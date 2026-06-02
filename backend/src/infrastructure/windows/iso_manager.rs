use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;
use tokio::process::Command;
use crate::domain::errors::DiskError;

pub struct IsoManager;

impl IsoManager {
    pub async fn mount_iso(iso_path: &str) -> Result<String, DiskError> {
        let ps_script = format!(
            "$vol = (Mount-DiskImage -ImagePath '{}' -StorageType ISO -PassThru | Get-Volume -ErrorAction Stop | Select-Object -First 1); if ($vol) {{ Write-Output $vol.DriveLetter }}",
            iso_path
        );

        let output = Command::new("powershell.exe")
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
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
            .await;

        Ok(())
    }

    pub async fn verify_bootable_iso(drive_letter: &str) -> bool {
        let indicators = ["isolinux", "boot", "EFI", "efi", "casper", "live", "squashfs"];
        let clean_letter = drive_letter.trim_matches(|c| c == '\\' || c == '/' || c == ':');
        let base_path = format!("{}:\\", clean_letter);

        println!("DEBUG: Verifying ISO at base path: [{}]", base_path);

        let exists = tokio::task::spawn_blocking(move || {
            println!("DEBUG: Directory listing for [{}]:", base_path);
            match fs::read_dir(&base_path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        println!("  -> {:?}", entry.file_name());
                    }
                }
                Err(e) => {
                    println!("DEBUG: FATAL: Rust cannot read the root directory of {}! Error: {}", base_path, e);
                    println!("DEBUG: If the error is 'System cannot find the path specified', this is UAC Drive Isolation.");
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
}