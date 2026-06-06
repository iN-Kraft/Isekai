use std::io::{Error, ErrorKind};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::interval;
use crate::application::spawn_blocking_with_context;
use crate::domain::errors::DiskError;
use crate::infrastructure::assets::COMMAND_NO_WINDOW;
use crate::telemetry;

pub struct PayloadManager;

impl PayloadManager {
    pub async fn copy_payload(source_drive_letter: &str, target_drive_letter: &str, is_hdd: bool) -> Result<(), DiskError> {
        let source = format!("{}:\\", source_drive_letter.trim_end_matches(':'));
        let target = format!("{}:\\", target_drive_letter.trim_end_matches(':'));
        let mt_flag = if is_hdd { "/MT:1" } else { "/MT:8" };

        telemetry!(step, "Preparing OS Payload");
        telemetry!(info, "Calculating payload size for {}...", source);

        let source_path = source.clone();
        let total_bytes = spawn_blocking_with_context(move || Self::get_dir_size(source_path))
            .await
            .unwrap_or(0);

        if total_bytes == 0 {
            return Err(DiskError::DataValidation("Source ISO appears to be empty!".into()));
        }

        telemetry!(step, "Cloning OS Payload");
        telemetry!(info, "Starting high-speed payload copy (Mode: {})", mt_flag);

        let mut child = Command::new("robocopy")
            .kill_on_drop(true)
            .creation_flags(COMMAND_NO_WINDOW)
            .args([
                &source,
                &target,
                "/E", // Copy subdirectories, including empty ones
                "/R:3", // 3 retries on locked/failed files
                "/W:2", // wait 2 seconds between retries
                "/NP", // no progress
                "/NFL", // no file list
                "/NDL", // no directory list
                mt_flag // multi-threading (8 threads)
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(DiskError::OsError)?;

        let mut ticker = interval(Duration::from_millis(500));
        let mut last_percent = 0u8;
        telemetry!(progress, last_percent, "{:.2} GB / {:.2} GB", 0f64, total_bytes as f64 / 1_073_741_824.0);

        let exit_status = loop {
            tokio::select! {
                status_result = child.wait() => {
                    break status_result.map_err(DiskError::OsError)?;
                }

                _ = ticker.tick() => {
                    let target_path = target.clone();
                    let current_bytes = spawn_blocking_with_context(move || Self::get_dir_size(target_path))
                        .await
                        .unwrap_or(0);

                    let raw_percent = (current_bytes as f64 / total_bytes as f64 * 100.0).clamp(0.0, 100.0) as u8;

                    if raw_percent > last_percent {
                        telemetry!(
                            progress,
                            raw_percent,
                            "{:.2} GB / {:.2} GB",
                            current_bytes as f64 / 1_073_741_824.0,
                            total_bytes as f64 / 1_073_741_824.0
                        );
                        last_percent = raw_percent
                    }
                }
            }
        };

        let exit_code = exit_status.code().unwrap_or(-1);

        // Microsoft Robocopy exit codes are non-standard:
        // 0 = No files copied (source and dest match)
        // 1 = Files copied successfully
        // 2-7 = Various success states with extra/mismatched files ignored
        // 8+ = Hard failure
        if exit_code >= 8 {
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("Robocopy failed with code {}.", exit_code,)
            )));
        }

        telemetry!(info, "Payload copy completed successfully!");

        Self::strip_readonly_attributes(&target).await?;

        Ok(())
    }

    async fn strip_readonly_attributes(target_path: &str) -> Result<(), DiskError> {
        telemetry!(step, "Finalizing File Permissions");
        telemetry!(info, "Stripping read-only attributes from copied files...");

        let target_glob = format!("{}*", target_path);
        let status = Command::new("attrib")
            .kill_on_drop(true)
            .creation_flags(COMMAND_NO_WINDOW)
            .args(["-R", &target_glob, "/S", "/D"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(DiskError::OsError)?;

        if !status.success() {
            telemetry!(warn, "Note: Some systems files denied attribute changes (this is normal).");
        } else {
            telemetry!(info, "Read-only attributes removed.");
        }

        Ok(())
    }

    fn get_dir_size(path: impl AsRef<Path>) -> u64 {
        let mut size = 0;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        size += Self::get_dir_size(entry.path());
                    } else {
                        size += metadata.len();
                    }
                }
            }
        }
        size
    }
}