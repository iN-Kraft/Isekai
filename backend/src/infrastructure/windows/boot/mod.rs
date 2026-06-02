pub mod legacy;
pub mod uefi;
pub mod sniffer;

use async_trait::async_trait;
use crate::domain::errors::DiskError;
use crate::infrastructure::windows::boot::legacy::LegacyBootManager;
use crate::infrastructure::windows::boot::uefi::UefiBootManager;

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
}