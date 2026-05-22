use async_trait::async_trait;
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, Partition};
use crate::domain::traits::DiskManager;
use crate::infrastructure::blockdev::{get_devices, BlockDevice};

pub struct LinuxDiskManager;

impl LinuxDiskManager {
    pub fn new() -> Self {
        Self {}
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
            if !device.is_disk() {
                continue;
            }

            let stable_id = match Self::get_stable_id(device) {
                Some(id) => id,
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
            .filter(|d| d.is_disk())
            .find(|d| Self::get_stable_id(d).as_deref() == Some(&disk_id_owned))
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

    async fn shrink_partition(&self, _partition_uuid: &str, _target_size_gb: u32) -> Result<(), DiskError> {
        Err(DiskError::DataValidation("Shrink not implemented".to_string()))
    }
}
