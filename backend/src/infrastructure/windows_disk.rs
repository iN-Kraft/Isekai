#![allow(non_snake_case)]
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
    PNPDeviceID: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Win32DiskPartition {
    DeviceID: String,
    Size: Option<u64>,
    Type: Option<String>,
}

pub struct WindowsDiskManager {
    _debug_mode: bool,
}

impl WindowsDiskManager {
    pub fn new(debug_mode: bool) -> Self {
        Self { _debug_mode: debug_mode }
    }
}

#[async_trait]
impl DiskManager for WindowsDiskManager {
    async fn get_disks(&self) -> Result<Vec<Disk>, DiskError> {
        let wmi_disks = tokio::task::spawn_blocking(|| -> Result<Vec<Win32DiskDrive>, DiskError> {
            let wmi_con = WMIConnection::new().map_err(|e| {
                DiskError::WmiError(format!("WMI Connection failed: {}", e))
            })?;

            let results: Vec<Win32DiskDrive> = wmi_con
                .raw_query("SELECT Index, Model, Size, PNPDeviceID FROM Win32_DiskDrive")
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed: {}", e)))?;

            Ok(results)
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let mut disks = Vec::with_capacity(wmi_disks.len());

        for wmi_disk in wmi_disks {
            let size_bytes = wmi_disk.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;

            if wmi_disk.PNPDeviceID.is_empty() {
                continue;
            }

            disks.push(Disk {
                stable_id: wmi_disk.PNPDeviceID,
                name: wmi_disk.Model,
                total_gb: size_gb,
                free_gb: 0,
                is_system_drive: false,
            });
        }

        Ok(disks)
    }

    async fn get_partitions(&self, disk_id: &str) -> Result<Vec<Partition>, DiskError> {
        let disk_id_owned = disk_id.to_string();

        let wmi_parts = tokio::task::spawn_blocking(move || -> Result<Vec<Win32DiskPartition>, DiskError> {
            let wmi_con = WMIConnection::new().map_err(|e| {
                DiskError::WmiError(format!("WMI Connection failed: {}", e))
            })?;

            // Resolve stable PNPDeviceID to volatile Index
            let drives: Vec<Win32DiskDrive> = wmi_con
                .raw_query("SELECT Index, PNPDeviceID FROM Win32_DiskDrive")
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed: {}", e)))?;

            let disk_index = match drives.into_iter().find(|d| d.PNPDeviceID == disk_id_owned) {
                Some(d) => d.Index,
                None => return Err(DiskError::DiskNotFound(disk_id_owned)),
            };

            let query = format!("SELECT DeviceID, Size, Type FROM Win32_DiskPartition WHERE DiskIndex = {}", disk_index);

            let results: Vec<Win32DiskPartition> = wmi_con
                .raw_query(&query)
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed: {}", e)))?;

            Ok(results)
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let mut partitions = Vec::with_capacity(wmi_parts.len());

        for part in wmi_parts {
            let size_bytes = part.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;

            if part.DeviceID.is_empty() {
                continue;
            }

            partitions.push(Partition {
                uuid: part.DeviceID,
                drive_letter: None, // Resolving "C:" requires querying Win32_LogicalDiskToPartition
                size_gb,
                file_system: part.Type.unwrap_or_else(|| "Unknown".to_string()),
            });
        }

        Ok(partitions)
    }

    async fn shrink_partition(&self, _partition_uuid: &str, _target_size_gb: u32) -> Result<(), DiskError> {
        Err(DiskError::DataValidation("Shrink not implemented on Windows".to_string()))
    }
}
