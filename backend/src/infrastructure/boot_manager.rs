use std::path::Path;
use tokio::fs;
use crate::domain::errors::DiskError;
use crate::infrastructure::assets::{BOOT_X64_EFI, EXFAT_X64_EFI, NTFS_X64_EFI};

pub struct BootManager;

impl BootManager {
    pub async fn install_uefi_driver(fat32_drive_letter: &str) -> Result<(), DiskError> {
        let efi_boot_dir = format!("{}:\\EFI\\Boot", fat32_drive_letter);
        let rufus_driver_dir = format!("{}:\\EFI\\Rufus", fat32_drive_letter);

        fs::create_dir_all(&efi_boot_dir).await.map_err(DiskError::OsError)?;
        fs::create_dir_all(&rufus_driver_dir).await.map_err(DiskError::OsError)?;

        let boot_path = Path::new(&efi_boot_dir).join("bootx64.efi");
        fs::write(&boot_path, BOOT_X64_EFI).await.map_err(DiskError::OsError)?;

        let ntfs_path = Path::new(&rufus_driver_dir).join("ntfs_x64.efi");
        fs::write(&ntfs_path, NTFS_X64_EFI).await.map_err(DiskError::OsError)?;

        let exfat_path = Path::new(&rufus_driver_dir).join("exfat_x64.efi");
        fs::write(&exfat_path, EXFAT_X64_EFI).await.map_err(DiskError::OsError)?;

        println!("Embedded UEFI drivers successfully written to {}", fat32_drive_letter);
        Ok(())
    }
}