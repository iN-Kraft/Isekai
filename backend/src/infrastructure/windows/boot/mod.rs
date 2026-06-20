pub mod legacy;
pub mod uefi;
pub mod sniffer;

use async_trait::async_trait;
use tokio::process::Command;
use crate::domain::errors::DiskError;
use crate::infrastructure::CommandExt;
use crate::infrastructure::windows::boot::legacy::LegacyBootManager;
use crate::infrastructure::windows::boot::uefi::UefiBootManager;
use crate::telemetry;

#[async_trait]
pub trait BootStrategy: Send + Sync {
    async fn inject_boot_binaries(&self, os_drive: &str, efi_drive: Option<&str>) -> Result<(), DiskError>;
    async fn patch_windows_bcd(&self, distro_name: &str, os_drive: &str) -> Result<(), DiskError>;
    async fn write_boot_config(&self, payload_drive: &str) -> Result<(), DiskError>;
}

pub struct BootManager;

impl BootManager {
    pub fn get_strategy(is_uefi: bool) -> Box<dyn BootStrategy> {
        if is_uefi {
            Box::new(UefiBootManager)
        } else {
            Box::new(LegacyBootManager)
        }
    }

    pub async fn remove_isekai_boot_entries(distro_name: &str) -> Result<(), DiskError> {
        telemetry!(info, "Scanning Windows Boot Manager for '{}' entries...", distro_name);

        let out = Command::new("bcdedit.exe")
            .kill_on_drop(true)
            .no_window()
            .args(["/enum", "all", "/v"])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut current_guid = String::new();
        let mut guids_to_delete = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();

            if line.chars().all(|c| c == '-') && !line.is_empty() {
                current_guid.clear();
                continue;
            }

            if current_guid.is_empty() && line.contains('{') && line.contains('}') {
                if let (Some(start), Some(end)) = (line.find('{'), line.find('}')) {
                    current_guid = line[start..=end].to_string();
                }
            }

            if line.contains(distro_name) {
                if !current_guid.is_empty() && current_guid != "{bootmgr}" && current_guid != "{fwbootmgr}" {
                    guids_to_delete.push(current_guid.clone());
                }
            }
        }

        if guids_to_delete.is_empty() {
            telemetry!(info, "No active boot entries found for '{}'", distro_name);
            return Ok(());
        }

        for guid in guids_to_delete {
            telemetry!(info, "Deleting BCD entry: {}", guid);

            let _ = Command::new("bcdedit.exe")
                .kill_on_drop(true)
                .no_window()
                .args(["/delete", &guid])
                .output()
                .await;
        }

        Ok(())
    }
}