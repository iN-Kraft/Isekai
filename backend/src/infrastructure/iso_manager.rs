use std::io::{Error, ErrorKind};
use std::path::Path;
use tokio::process::Command;
use crate::domain::errors::DiskError;

pub struct IsoManager;

impl IsoManager {
    pub async fn mount_iso(iso_path: &str) -> Result<String, DiskError> {
        let ps_script = format!(
            "$$vol = (Mount-DiskImage -ImagePath '{}' -StorageType ISO -PassThru | Get-Volume -ErrorAction Stop | Select-Object -First 1); if ($vol) {{ Write-Output $vol.DriveLetter }}",
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
        let indicators = ["isolinux", "boot", "EFI", "casper", "live", "squashfs"];
        let base_path = format!("{}:\\", drive_letter);

        let exists = tokio::task::spawn_blocking(move || {
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