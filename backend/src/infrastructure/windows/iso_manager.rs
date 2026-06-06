use std::fs::{read_dir, File};
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::Path;
use sha2::{Digest, Sha256};
use tokio::process::Command;
use crate::application::spawn_blocking_with_context;
use crate::domain::errors::DiskError;
use crate::infrastructure::assets::COMMAND_NO_WINDOW;
use crate::telemetry;

pub struct IsoManager;

impl IsoManager {
    pub async fn mount_iso(iso_path: &str) -> Result<String, DiskError> {
        let ps_script = format!(
            "$vol = (Mount-DiskImage -ImagePath '{}' -StorageType ISO -PassThru | Get-Volume -ErrorAction Stop | Select-Object -First 1); if ($vol) {{ Write-Output $vol.DriveLetter }}",
            iso_path
        );

        let output = Command::new("powershell.exe")
            .kill_on_drop(true)
            .creation_flags(COMMAND_NO_WINDOW)
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("Failed to mount ISO: {}", err)
            )));
        }

        let drive_letter = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if drive_letter.is_empty() {
            let dismount_script = format!("Dismount-DiskImage -ImagePath '{}' -ErrorAction SilentlyContinue", iso_path);
            let _ = Command::new("powershell.exe")
                .kill_on_drop(true)
                .creation_flags(COMMAND_NO_WINDOW)
                .args(["-NoProfile", "-NonInteractive", "-Command", &dismount_script])
                .output()
                .await;

            return Err(DiskError::OsError(Error::new(
                ErrorKind::NotFound,
                "ISO mounted but no drive letter was assigned by Windows"
            )));
        }

        Ok(drive_letter)
    }

    pub async fn dismount_iso(iso_path: &str) -> Result<(), DiskError> {
        let ps_script = format!("Dismount-DiskImage -ImagePath '{}' -ErrorAction SilentlyContinue", iso_path);

        let _ = Command::new("powershell.exe")
            .kill_on_drop(true)
            .creation_flags(COMMAND_NO_WINDOW)
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
            .await;

        Ok(())
    }

    pub async fn verify_bootable_iso(drive_letter: &str) -> bool {
        let indicators = ["isolinux", "boot", "EFI", "efi", "casper", "live", "squashfs"];
        let clean_letter = drive_letter.trim_matches(|c| c == '\\' || c == '/' || c == ':');
        let base_path = format!("{}:\\", clean_letter);

        telemetry!(debug, "Verifying ISO at base path: [{}]", base_path);

        let exists = spawn_blocking_with_context(move || {
            telemetry!(debug, "Directory listing for [{}]:", base_path);
            match read_dir(&base_path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        telemetry!(debug, "  -> {:?}", entry.file_name());
                    }
                }
                Err(e) => {
                    telemetry!(error, "FATAL: Rust cannot read the root directory of {}! Error: {}", base_path, e);
                    telemetry!(error, "If the error is 'System cannot find the path specified', this is UAC Drive Isolation.");
                }
            }

            for indicator in indicators {
                let check_path = Path::new(&base_path).join(indicator);
                if check_path.exists() {
                    return true;
                }
            }
            false
        }).await.unwrap_or(false);

        exists
    }

    pub async fn calculate_sha256(iso_path: &str) -> Result<String, DiskError> {
        let path = iso_path.to_string();

        let hash_result = spawn_blocking_with_context(move || -> Result<String, DiskError> {
            let file = File::open(&path).map_err(DiskError::OsError)?;
            let mut reader = BufReader::with_capacity(1024 * 1024, file);
            let mut hasher = Sha256::new();

            let mut buffer = [0; 1024 * 1024];
            loop {
                let n = reader.read(&mut buffer).map_err(DiskError::OsError)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }

            let result = hasher.finalize();
            Ok(result.iter().map(|b| format!("{:02x}", b)).collect())
        }).await.map_err(|e| DiskError::OsError(Error::new(
            ErrorKind::Other,
            format!("Hash calculation task failed: {}", e)
        )))??;

        Ok(hash_result)
    }

    pub async fn verify_iso_hash(iso_path: &str, expected_hash: &str) -> Result<bool, DiskError> {
        let computed_hash = Self::calculate_sha256(iso_path).await?;

        Ok(computed_hash.eq_ignore_ascii_case(expected_hash))
    }
}