#![allow(non_snake_case)]
use async_trait::async_trait;
use serde::Deserialize;
use wmi::WMIConnection;
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, Partition};
use crate::domain::traits::DiskManager;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MsftDisk {
    Number: u32,
    FriendlyName: Option<String>,
    Size: Option<u64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MsftPartition {
    DiskNumber: u32,
    PartitionNumber: u32,
    Size: Option<u64>,
    DriveLetter: Option<String>,
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
        let wmi_disks = tokio::task::spawn_blocking(|| -> Result<Vec<MsftDisk>, DiskError> {
            let wmi_con = WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage").map_err(|e| {
                DiskError::WmiError(format!("WMI Connection failed: {}", e))
            })?;

            let results: Vec<MsftDisk> = wmi_con
                .raw_query("SELECT Number, FriendlyName, Size FROM MSFT_Disk")
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed: {}", e)))?;

            Ok(results)
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let mut disks = Vec::with_capacity(wmi_disks.len());

        for wmi_disk in wmi_disks {
            let size_bytes = wmi_disk.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;

            disks.push(Disk {
                stable_id: wmi_disk.Number.to_string(),
                name: wmi_disk.FriendlyName.unwrap_or_else(|| "Unknown".to_string()),
                total_gb: size_gb,
                free_gb: 0,
                is_system_drive: false,
            });
        }

        disks.sort_by_key(|d| d.stable_id.parse::<u32>().unwrap_or(u32::MAX));
        Ok(disks)
    }

    async fn get_partitions(&self, disk_id: &str) -> Result<Vec<Partition>, DiskError> {
        let disk_index: u32 = disk_id.parse().map_err(|_| {
            DiskError::DiskNotFound(disk_id.to_string())
        })?;

        let wmi_parts = tokio::task::spawn_blocking(move || -> Result<Vec<MsftPartition>, DiskError> {
            let wmi_con = WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage").map_err(|e| {
                DiskError::WmiError(format!("WMI Connection failed: {}", e))
            })?;

            let query = format!("SELECT DiskNumber, PartitionNumber, Size, DriveLetter FROM MSFT_Partition WHERE DiskNumber = {}", disk_index);

            let results: Vec<MsftPartition> = wmi_con
                .raw_query(&query)
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed: {}", e)))?;

            Ok(results)
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let mut partitions = Vec::with_capacity(wmi_parts.len());

        for part in wmi_parts {
            let size_bytes = part.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;

            let mut drive_letter = None;
            if let Some(dl) = &part.DriveLetter {
                let trimmed = dl.trim_matches('\0').trim();
                if !trimmed.is_empty() {
                    drive_letter = Some(format!("{}:", trimmed));
                }
            }

            partitions.push(Partition {
                id: part.PartitionNumber.to_string(),
                drive_letter,
                size_gb,
                file_system: "Unknown".to_string(),
            });
        }

        partitions.sort_by_key(|p| p.id.parse::<u32>().unwrap_or(u32::MAX));
        Ok(partitions)
    }

    async fn shrink_partition(&self, disk_id: &str, partition_id: &str, target_size_gb: u32) -> Result<(), DiskError> {
        let disk_num = disk_id.to_string();
        let part_num = partition_id.to_string();

        tokio::task::spawn_blocking(move || {
            // 2. Execute PowerShell
            let cmd_str = format!("Resize-Partition -DiskNumber {} -PartitionNumber {} -Size {}GB", disk_num, part_num, target_size_gb);
            
            let output = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", &cmd_str])
                .output()
                .map_err(|e| DiskError::OsError(e))?;

            // 3. Handle Errors
            if !output.status.success() {
                let stdout_err = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr_err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                
                let mut combined_err = String::new();
                if !stdout_err.is_empty() {
                    combined_err.push_str(&stdout_err);
                }
                if !stderr_err.is_empty() {
                    if !combined_err.is_empty() {
                        combined_err.push_str(" | ");
                    }
                    combined_err.push_str(&stderr_err);
                }
                if combined_err.is_empty() {
                    combined_err = "Unknown error occurred".to_string();
                }

                return Err(DiskError::OsError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("PowerShell Resize-Partition failed: {}", combined_err),
                )));
            }

            Ok(())
        })
        .await
        .map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))?
    }
}
