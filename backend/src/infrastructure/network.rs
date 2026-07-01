use std::collections::HashMap;
use std::env::temp_dir;
use std::path::{Path, PathBuf};
use std::time::Duration;
use const_hex::encode;
use directories::UserDirs;
use futures::StreamExt;
use reqwest::{Client, StatusCode};
use reqwest::header::RANGE;
use sha2::{Digest, Sha256};
use tokio::fs::{create_dir_all, metadata, remove_file, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{watch, OnceCell};
use tokio::time::Instant;
use tracing::{debug, error, info, warn};
use crate::domain::errors::DiskError;
use crate::domain::models::{PublicConfig, RemoteConfig, WorkflowState};
use crate::ipc::protocol::IPCEvent;
use crate::telemetry;

static CONFIG_CACHE: OnceCell<RemoteConfig> = OnceCell::const_new();
pub struct NetworkManager;

impl NetworkManager {
    fn get_client() -> Result<Client, DiskError> {
        Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| DiskError::DataValidation(format!("Failed to build HTTP client: {}", e)))
    }

    pub async fn fetch_distro_config() -> RemoteConfig {
        let client = Self::get_client().unwrap_or_else(|_| Client::new());
        let primary_url = "https://in-kraft.github.io/Isekai/assets/distros.json";
        let fallback_url = "https://raw.githubusercontent.com/iN-Kraft/Isekai/master/assets/distros.json";

        info!("Fetching distro configuration from primary CDN...");

        match Self::try_fetch(&client, primary_url).await {
            Ok(config) => {
                info!("Successfully loaded remote configuration from primary CDN.");
                return config;
            }
            Err(e) => {
                warn!("Primary config fetch failed ({}). Attempting raw repository fallback...", e);
            }
        }

        match Self::try_fetch(&client, fallback_url).await {
            Ok(config) => {
                info!("Successfully loaded remote configuration from fallback source.");
                config
            }
            Err(e) => {
                error!("CRITICAL: Both primary and fallback config fetches failed! {}", e);
                HashMap::new()
            }
        }
    }

    async fn try_fetch(client: &Client, url: &str) -> Result<RemoteConfig, String> {
        let response = client.get(url).send().await.map_err(|e| e.to_string())?;

        if response.status().is_success() {
            response.json::<RemoteConfig>().await.map_err(|e| e.to_string())
        } else {
            Err(format!("HTTP Status: {}", response.status()))
        }
    }

    pub async fn verify_mirror(iso_url: &str) -> bool {
        debug!("Verifying mirror reachability for: {}", iso_url);
        let client = Self::get_client().unwrap_or_else(|_| Client::new());

        match client.head(iso_url).send().await {
            Ok(res) => {
                let success = res.status().is_success();
                if !success {
                    warn!("Mirror rejected HEAD request with status: {}", res.status())
                }
                success
            }
            Err(e) => {
                warn!("Mirror verification failed: {}", e);
                false
            }
        }
    }

    pub async fn get_config() -> &'static RemoteConfig {
        CONFIG_CACHE.get_or_init(|| async {
            Self::fetch_distro_config().await
        }).await
    }

    pub async fn get_public_config() -> HashMap<String, PublicConfig> {
        let config = Self::get_config().await;
        let mut ui_map = HashMap::new();

        for (id, spec) in config.iter() {
            ui_map.insert(id.clone(), PublicConfig {
                available: true,
                version: spec.version.clone(),
                secure_boot: spec.secure_boot
            });
        }

        ui_map
    }

    pub async fn download_and_verify(
        url: &str,
        destination_path: &Path,
        expected_checksum: &str,
        mut state_rx: watch::Receiver<WorkflowState>
    ) -> Result<(), DiskError> {
        let client = Self::get_client().unwrap_or_else(|_| Client::new());
        let mut hasher = Sha256::new();
        let mut downloaded_bytes = 0u64;

        telemetry!(IPCEvent::StepInitializingDownload);

        if destination_path.exists() {
            downloaded_bytes = metadata(destination_path).await.map_or(0, |m| m.len());
            if downloaded_bytes > 0 {
                info!("Found existing partial file ({} bytes). Priming SHA-256 hasher...", downloaded_bytes);

                let mut file = File::open(destination_path).await.map_err(|e| DiskError::DataValidation(format!("Failed to open existing file: {}", e)))?;
                let mut buf = [0u8; 65536];
                while let Ok(n) = file.read(&mut buf).await {
                    if n == 0 { break; }
                    hasher.update(&buf[..n]);
                }
            }
        }

        let mut total_size = downloaded_bytes;
        loop {
            let state = state_rx.borrow_and_update().clone();

            match state {
                WorkflowState::Cancelled => {
                    info!("Download cancelled. Cleaning up file.");
                    let _ = tokio::fs::remove_file(destination_path).await;
                    return Err(DiskError::DataValidation("Download Cancelled.".into()))
                }
                WorkflowState::Paused => {
                    info!("Download paused. Waiting for resume signal...");
                    let _ = state_rx.changed().await;
                    continue;
                }
                WorkflowState::Running => { }
            }

            let mut request = client.get(url);
            if downloaded_bytes > 0 {
                info!("Resuming download from byte: {}", downloaded_bytes);
                request = request.header(RANGE, format!("bytes={}-", downloaded_bytes));
            }

            let response = request.send().await.map_err(|e| DiskError::DataValidation(format!("Network error: {}", e)))?;
            let status = response.status();
            if !status.is_success() {
                return Err(DiskError::DataValidation(format!("Server returned error status: {}", status)));
            }

            if downloaded_bytes > 0 && status != StatusCode::PARTIAL_CONTENT {
                warn!("Server ignored range request. Restarting download from scratch.");
                downloaded_bytes = 0;
                hasher = Sha256::new();
                let _ = remove_file(destination_path).await;
            } else if total_size == downloaded_bytes {
                let content_length = response.content_length().unwrap_or(0);
                total_size = downloaded_bytes + content_length;
            }

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(destination_path)
                .await
                .map_err(|e| DiskError::DataValidation(format!("Failed to open file for writing: {}", e)))?;

            let mut stream = response.bytes_stream();
            let mut last_percent = if total_size > 0 {
                ((downloaded_bytes as f64 / total_size as f64) * 100.0) as u8
            } else {
                0
            };
            let stream_start_time = Instant::now();
            let stream_start_bytes = downloaded_bytes;

            'chunk_loop: while let Some(chunk_res) = stream.next().await {
                if state_rx.has_changed().unwrap_or(false) {
                    let current_state = state_rx.borrow_and_update().clone();
                    if current_state != WorkflowState::Running {
                        break 'chunk_loop;
                    }
                }

                let chunk = match chunk_res {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Network stream interrupted: {}. Auto-resuming download...", e);
                        break 'chunk_loop;
                    }
                };

                file.write_all(&chunk).await.map_err(|e| DiskError::DataValidation(format!("Write error: {}", e)))?;
                hasher.update(&chunk);
                downloaded_bytes += chunk.len() as u64;

                if total_size > 0 {
                    let percent = ((downloaded_bytes as f64 / total_size as f64) * 100.0) as u8;
                    if percent > last_percent {
                        let elapsed = stream_start_time.elapsed().as_secs_f64();
                        let mut eta_seconds = 0u64;

                        if elapsed > 0.0 {
                            let speed = (downloaded_bytes - stream_start_bytes) as f64 / elapsed;
                            if speed > 0.0 {
                                eta_seconds = ((total_size - downloaded_bytes) as f64 / speed) as u64;
                            }
                        }

                        telemetry!(IPCEvent::ProgressDownload {
                            downloaded_bytes,
                            total_bytes: total_size,
                            percent,
                            eta_seconds
                        });
                        last_percent = percent;
                    }
                }
            }

            let current_state = state_rx.borrow().clone();
            if current_state == WorkflowState::Running && downloaded_bytes >= total_size {
                break;
            }
        }

        info!("Download complete. Verifying checksum...");
        let final_hash = encode(hasher.finalize());
        if final_hash.eq_ignore_ascii_case(expected_checksum) {
            info!("Checksum valid: {}", final_hash);
            Ok(())
        } else {
            error!("Checksum mismatch! Expected: {}, Got: {}", expected_checksum, final_hash);
            let _ = remove_file(destination_path).await;
            Err(DiskError::DataValidation("Checksum mismatch. File was corrupted during download.".into()))
        }
    }

    pub async fn prepare_download(distro_id: &str) -> Result<(String, String, PathBuf), DiskError> {
        let config = Self::get_config().await;
        let spec = config.get(distro_id).ok_or_else(|| {
            DiskError::DataValidation(format!("Distribution '{}' not found in remote config.", distro_id))
        })?;

        for mirror in &spec.mirrors {
            if Self::verify_mirror(mirror).await {
                let url_segment = mirror.rsplit('/').find(|s| !s.is_empty()).unwrap_or("");
                let filename = if url_segment.ends_with(".iso") {
                    url_segment.to_string()
                } else {
                    format!("{}.iso", distro_id)
                };
                let download_dir = UserDirs::new().and_then(|dirs| dirs.download_dir().map(|d| d.join("Isekai")))
                    .unwrap_or_else(|| temp_dir().join("Isekai"));

                create_dir_all(&download_dir).await.map_err(|e| {
                    DiskError::DataValidation(format!("Failed to create download directory: {}", e))
                })?;
                let dest_path = download_dir.join(filename);

                return Ok((mirror.clone(), spec.checksum.clone(), dest_path));
            }
        }

        Err(DiskError::DataValidation("All download mirrors are currently offline.".into()))
    }
}