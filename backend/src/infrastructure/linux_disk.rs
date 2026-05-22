use async_trait::async_trait;
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, Partition};
use crate::domain::traits::DiskManager;
use crate::infrastructure::blockdev::{get_devices, BlockDevice, DeviceType};

pub struct LinuxDiskManager {
    debug_mode: bool,
}

impl LinuxDiskManager {
    pub fn new(debug_mode: bool) -> Self {
        Self { debug_mode }
    }

    fn get_stable_id(device: &BlockDevice) -> Option<String> {
        device.wwn.clone()
            .or_else(|| device.serial.clone())
            .or_else(|| device.uuid.clone())
    }
}

#[async_trait]
impl DiskManager for LinuxDiskManager {
    async fn get_disks(&self) -> Result<Vec<Disk>, DiskError> {
        let block_devices = tokio::task::spawn_blocking(|| {
            get_devices().map_err(|e| DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )))
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let mut disks = Vec::new();

        for device in block_devices.iter() {
            let is_allowed_type = device.is_disk() || (self.debug_mode && device.device_type == DeviceType::Loop);
            
            if !is_allowed_type {
                continue;
            }

            let stable_id = match Self::get_stable_id(device) {
                Some(id) => id,
                None if self.debug_mode && device.device_type == DeviceType::Loop => device.name.clone(),
                None => continue, // Skip devices without a stable ID
            };

            disks.push(Disk {
                stable_id,
                name: device.name.clone(),
                total_gb: (device.size / 1024 / 1024 / 1024) as u32,
                free_gb: 0, // Placeholder
                is_system_drive: device.is_system(),
            });
        }

        Ok(disks)
    }

    async fn get_partitions(&self, disk_id: &str) -> Result<Vec<Partition>, DiskError> {
        let disk_id_owned = disk_id.to_string();
        
        let block_devices = tokio::task::spawn_blocking(|| {
            get_devices().map_err(|e| DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )))
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let target_device = block_devices.iter_all()
            .filter(|d| d.is_disk() || (self.debug_mode && d.device_type == DeviceType::Loop))
            .find(|d| {
                let id = Self::get_stable_id(d).unwrap_or_else(|| {
                    if d.device_type == DeviceType::Loop {
                        d.name.clone()
                    } else {
                        "".to_string()
                    }
                });
                id == disk_id_owned
            })
            .ok_or_else(|| DiskError::DiskNotFound(disk_id_owned))?;

        let mut partitions = Vec::new();

        for child in target_device.children_iter() {
            if !child.is_partition() {
                continue;
            }

            let uuid = match &child.uuid {
                Some(u) => u.clone(),
                None => continue, // Skip partitions without UUID
            };

            partitions.push(Partition {
                uuid,
                drive_letter: child.active_mountpoints().first().map(|s| s.to_string()),
                size_gb: (child.size / 1024 / 1024 / 1024) as u32,
                file_system: child.fstype.clone().unwrap_or_else(|| "Unknown".to_string()),
            });
        }

        Ok(partitions)
    }

    async fn shrink_partition(&self, partition_uuid: &str, target_size_gb: u32) -> Result<(), DiskError> {
        let uuid = partition_uuid.to_string();

        tokio::task::spawn_blocking(move || {
            // 1. Resolve UUID to Device Path using blkid
            let output = std::process::Command::new("blkid")
                .arg("-U")
                .arg(&uuid)
                .output()
                .map_err(|e| DiskError::OsError(e))?;

            if !output.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(DiskError::OsError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("blkid failed for UUID {}: {}", uuid, err_msg),
                )));
            }

            let device_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if device_path.is_empty() {
                return Err(DiskError::PartitionNotFound(uuid, "Unknown (Resolved via blkid)".to_string()));
            }

            // 2. Filesystem Check (Required before resizing ext4)
            // -f forces check even if clean, -p auto-repairs safe errors
            let check_output = std::process::Command::new("e2fsck")
                .arg("-f")
                .arg("-p")
                .arg(&device_path)
                .output()
                .map_err(|e| DiskError::OsError(e))?;

            // e2fsck exit codes: 0 (No errors), 1 (Errors corrected), 2 (System should be rebooted)
            // We accept 0 and 1.
            if !check_output.status.success() && check_output.status.code() != Some(1) {
                let err_msg = String::from_utf8_lossy(&check_output.stderr).trim().to_string();
                return Err(DiskError::OsError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("e2fsck failed for {}: {}", device_path, err_msg),
                )));
            }

            // 3. Filesystem Shrink using resize2fs
            let resize_output = std::process::Command::new("resize2fs")
                .arg(&device_path)
                .arg(format!("{}G", target_size_gb))
                .output()
                .map_err(|e| DiskError::OsError(e))?;

            if !resize_output.status.success() {
                let err_msg = String::from_utf8_lossy(&resize_output.stderr).trim().to_string();
                return Err(DiskError::OsError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("resize2fs failed for {}: {}", device_path, err_msg),
                )));
            }

            Ok(())
        })
        .await
        .map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))?
    }
}
