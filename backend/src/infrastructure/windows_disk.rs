use async_trait::async_trait;
use serde::Deserialize;
use wmi::WMIConnection;
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, Partition};
use crate::domain::traits::DiskManager;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Win32DiskDrive {
    Index: u32,
    Model: String,
    Size: Option<u64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Win32DiskPartition {
    DiskIndex: u32,
    Index: u32,
    Size: Option<u64>,
    Type: Option<String>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Win32LogicalDisk {
    DeviceID: String,
    Size: Option<u64>,
    FreeSpace: Option<u64>,
    FileSystem: Option<String>
}

pub struct WindowsDiskManager;

impl WindowsDiskManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl DiskManager for WindowsDiskManager {
    async fn get_disks(&self) -> Result<Vec<Disk>, DiskError> {
        let wmi_disks = tokio::task::spawn_blocking(|| -> Result<Vec<Win32DiskDrive>, DiskError> {
            let wmi_con = WMIConnection::new().map_err(|e| {
                DiskError::OsError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("WMI Connection failed: {}", e)
                ))
            })?;

            let results: Vec<Win32DiskDrive> = wmi_con
                .raw_query("SELECT Index, Model, Size FROM Win32_DiskDrive")
                .map_err(|e| {
                    DiskError::OsError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("WMI Query failed: {}", e)
                    ))
                })?;

            Ok(results)
        }).await.expect("Thread Pool crashed")?;

        let mut disks = Vec::new();

        for wmi_disk in wmi_disks {
            let size_bytes = wmi_disk.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;

            disks.push(Disk {
                disk_num: wmi_disk.Index,
                name: wmi_disk.Model,
                total_gb: size_gb,
                free_gb: 0,
                is_system_drive: false,
            });
        }

        Ok(disks)
    }

    async fn get_partitions(&self, disk_num: u32) -> Result<Vec<Partition>, DiskError> {
        let wmi_parts = tokio::task::spawn_blocking(move || -> Result<Vec<Win32DiskPartition>, DiskError> {
            let wmi_con = WMIConnection::new().map_err(|e| {
                DiskError::OsError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("WMI Connection failed: {}", e),
                ))
            })?;

            let query = format!("SELECT DiskIndex, Index, Size, Type FROM Win32_DiskPartition WHERE DiskIndex = {}", disk_num);

            let results: Vec<Win32DiskPartition> = wmi_con
                .raw_query(&query)
                .map_err(|e| {
                    DiskError::OsError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("WMI Query failed: {}", e)
                    ))
                })?;

            Ok(results)
        }).await.expect("Thread Pool crashed")?;

        let mut partitions = Vec::new();

        for part in wmi_parts {
            let size_bytes = part.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;

            partitions.push(Partition {
                partition_num: part.Index + 1, // WMI partitions are 0-indexed
                drive_letter: None, // Resolving "C:" requires querying Win32_LogicalDiskToPartition
                size_gb,
                file_system: part.Type.unwrap_or_else(|| "Unknown".to_string()),
            });
        }

        Ok(partitions)
    }

    async fn shrink_partition(&self, disk_num: &str, partition_num: u32, shrink_by_gb: u32) -> Result<(), DiskError> {
        todo!()
    }
}