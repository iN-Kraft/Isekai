use async_trait::async_trait;
use blockdev::{get_devices, DeviceType};
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, Partition};
use crate::domain::traits::DiskManager;

pub struct LinuxDiskManager;

impl LinuxDiskManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl DiskManager for LinuxDiskManager {
    async fn get_disks(&self) -> Result<Vec<Disk>, DiskError> {
        println!("Linux: Querying block devices...");

        let devices = tokio::task::spawn_blocking(|| {
            get_devices()
        }).await.expect("Thread Pool crashed").map_err(|e| {
            DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Blockdev failed: {:?}", e)
            ))
        })?;

        let mut mapped_disks= Vec::new();

        for (index, device) in devices.into_iter().enumerate() {
            let size_gb = (device.size / 1024 / 1024 / 1024) as u32;

            mapped_disks.push(Disk {
                disk_num: index as u32,
                total_gb: size_gb,
                free_gb: 0,
                is_system_drive: device.is_system(),
                name: device.name,
            });
        }

        Ok(mapped_disks)
    }

    async fn get_partitions(&self, disk_num: u32) -> Result<Vec<Partition>, DiskError> {
        println!("Linux: Querying partitions for disk {}...", disk_num);

        let block_devices = tokio::task::spawn_blocking(|| {
            get_devices()
        }).await.expect("Thread Pool crashed").map_err(|e| {
            DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to parse lsblk: {:?}", e)
            ))
        })?;

        let mut target_disk = None;
        for (index, device) in block_devices.into_iter().enumerate() {
            if device.is_disk() && index as u32 == disk_num {
                target_disk = Some(device);
                break;
            }
        }

        let disk = match target_disk {
            Some(d) => d,
            None => return Err(DiskError::DiskNotFound(disk_num.to_string())),
        };

        let mut partitions = Vec::new();

        if let Some(children) = disk.children {
            for (p_index, child) in children.into_iter().filter(|c| c.is_partition()).enumerate() {
                let size_gb = (child.size / 1024 / 1024 / 1024) as u32;
                let mut mountpoint = child.active_mountpoints().first().map(|s| s.to_string());
                let mut file_system = "Unknown".to_string();

                if mountpoint.is_none() {
                    for descendant in child.descendants() {
                        if descendant.device_type == DeviceType::Crypt || descendant.device_type == DeviceType::Lvm {
                            file_system = if descendant.device_type == DeviceType::Crypt {
                                "LUKS Encrypted".to_string()
                            } else {
                                "LVM Volume".to_string()
                            };

                            let desc_mounts = descendant.active_mountpoints();
                            mountpoint = if desc_mounts.contains(&"/") {
                                Some("/".to_string())
                            } else {
                                desc_mounts.first().map(|s| s.to_string())
                            };

                            if mountpoint.is_some() { break; }
                        }
                    }
                }

                partitions.push(Partition {
                    partition_num: p_index as u32 + 1,
                    drive_letter: mountpoint,
                    size_gb,
                    file_system,
                });
            }
        }

        Ok(partitions)
    }

    async fn shrink_partition(&self, disk_num: &str, partition_num: u32, shrink_by_gb: u32) -> Result<(), DiskError> {
        todo!("Implement blockdev partition shrinking")
    }
}