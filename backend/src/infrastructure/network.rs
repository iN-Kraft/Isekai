use std::collections::HashMap;
use std::time::Duration;
use reqwest::Client;
use tokio::sync::OnceCell;
use tracing::{debug, error, info, warn};
use crate::domain::errors::DiskError;
use crate::domain::models::{PublicConfig, RemoteConfig};

static CONFIG_CACHE: OnceCell<RemoteConfig> = OnceCell::const_new();
pub struct NetworkManager;

impl NetworkManager {
    fn get_client() -> Result<Client, DiskError> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| DiskError::DataValidation(format!("Failed to build HTTP client: {}", e)))
    }

    pub async fn fetch_distro_config() -> RemoteConfig {
        let client = match Self::get_client() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create HTTP client: {}", e);
                return HashMap::new();
            }
        };
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
        let client = match Self::get_client() {
            Ok(c) => c,
            Err(_) => return false
        };

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
}