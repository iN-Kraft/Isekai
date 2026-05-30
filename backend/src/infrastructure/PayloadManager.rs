use std::io::{Error, ErrorKind};
use tokio::process::Command;
use crate::domain::errors::DiskError;

pub struct PayloadManager;

impl PayloadManager {
    pub async fn copy_payload(source_drive_letter: &str, target_drive_letter: &str) -> Result<(), DiskError> {
        let source = format!("{}:\\", source_drive_letter.trim_end_matches(':'));
        let target = format!("{}:\\", target_drive_letter.trim_end_matches(':'));

        println!("Starting high-speed payload copy from {} to {}", source, target);
        println!("This may take a few minutes...");

        let output = Command::new("robocopy")
            .args([
                &source,
                &target,
                "/E", // Copy subdirectories, including empty ones
                "/R:3", // 3 retries on locked/failed files
                "/W:2", // wait 2 seconds between retries
                "/NP", // no progress
                "/NFL", // no file list
                "/NDL", // no directory list
                "/MT:8" // multi-threading (8 threads)
            ])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        let exit_code = output.status.code().unwrap_or(-1);

        // Microsoft Robocopy exit codes are non-standard:
        // 0 = No files copied (source and dest match)
        // 1 = Files copied successfully
        // 2-7 = Various success states with extra/mismatched files ignored
        // 8+ = Hard failure
        if exit_code >= 8 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("Robocopy failed with code {}.\nOut: {}\nErr: {}", exit_code, stdout, stderr)
            )));
        }

        println!("Payload copy completed successfully!");

        Self::strip_readonly_attributes(&target).await?;

        Ok(())
    }

    async fn strip_readonly_attributes(target_path: &str) -> Result<(), DiskError> {
        println!("Stripping read-only attributes from copied files...");

        let target_glob = format!("{}*.*", target_path);
        let output = Command::new("attrib")
            .args(["-R", &target_glob, "/S", "/D"])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        if !output.status.success() {
            println!("Note: Some systems files denied attribute changes (this is normal).");
        } else {
            println!("Read-only attributes removed.");
        }

        Ok(())
    }
}