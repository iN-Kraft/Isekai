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
    IsSystem: Option<bool>,
    IsBoot: Option<bool>,
    BusType: Option<u16>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MsftPartition {
    DiskNumber: u32,
    PartitionNumber: u32,
    Size: Option<u64>,
    DriveLetter: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MsftVolume {
    DriveLetter: Option<String>,
    FileSystem: Option<String>,
}

pub struct WindowsDiskManager {
    debug_mode: bool,
}

impl WindowsDiskManager {
    pub fn new(debug_mode: bool) -> Self {
        Self { debug_mode }
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
                .raw_query("SELECT Number, FriendlyName, Size, IsSystem, IsBoot, BusType FROM MSFT_Disk")
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed: {}", e)))?;

            Ok(results)
            }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

            let mut disks = Vec::with_capacity(wmi_disks.len());

            for wmi_disk in wmi_disks {
            let bus_type = wmi_disk.BusType.unwrap_or(0);
            let friendly_name = wmi_disk.FriendlyName.as_deref().unwrap_or("Unknown");
            let is_virtual = bus_type == 14 || bus_type == 15 || friendly_name.to_lowercase().contains("virtual");

            if is_virtual && !self.debug_mode {
                continue;
            }

            let size_bytes = wmi_disk.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;
            let is_sys = wmi_disk.IsSystem.unwrap_or(false) || wmi_disk.IsBoot.unwrap_or(false);

            disks.push(Disk {
                stable_id: wmi_disk.Number.to_string(),
                name: friendly_name.to_string(),
                total_gb: size_gb,
                free_gb: 0,
                is_system_drive: is_sys,
            });
            }

        disks.sort_by_key(|d| d.stable_id.parse::<u32>().unwrap_or(u32::MAX));
        Ok(disks)
    }

    async fn get_partitions(&self, disk_id: &str) -> Result<Vec<Partition>, DiskError> {
        let disk_index: u32 = disk_id.parse().map_err(|_| {
            DiskError::DiskNotFound(disk_id.to_string())
        })?;

        let (wmi_parts, volumes) = tokio::task::spawn_blocking(move || -> Result<(Vec<MsftPartition>, Vec<MsftVolume>), DiskError> {
            let wmi_con = WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage").map_err(|e| {
                DiskError::WmiError(format!("WMI Connection failed: {}", e))
            })?;

            let query = format!("SELECT DiskNumber, PartitionNumber, Size, DriveLetter FROM MSFT_Partition WHERE DiskNumber = {}", disk_index);

            let results: Vec<MsftPartition> = wmi_con
                .raw_query(&query)
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed: {}", e)))?;

            let volumes: Vec<MsftVolume> = wmi_con
                .raw_query("SELECT DriveLetter, FileSystem FROM MSFT_Volume")
                .unwrap_or_default();

            Ok((results, volumes))
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let mut partitions = Vec::with_capacity(wmi_parts.len());

        for part in wmi_parts {
            let size_bytes = part.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;

            let mut fs = "Unknown".to_string();
            let mut drive_letter = None;
            
            if let Some(dl) = &part.DriveLetter {
                let trimmed = dl.trim_matches('\0').trim();
                if !trimmed.is_empty() {
                    drive_letter = Some(format!("{}:", trimmed));
                    
                    if let Some(vol) = volumes.iter().find(|v| {
                        v.DriveLetter.as_deref().map(|s| s.trim_matches('\0').trim()) == Some(trimmed)
                    }) {
                        if let Some(vol_fs) = &vol.FileSystem {
                            fs = vol_fs.clone();
                        }
                    }
                }
            }

            partitions.push(Partition {
                id: part.PartitionNumber.to_string(),
                drive_letter,
                size_gb,
                file_system: fs,
            });
        }

        partitions.sort_by_key(|p| p.id.parse::<u32>().unwrap_or(u32::MAX));
        Ok(partitions)
    }

    async fn shrink_partition(&self, disk_id: &str, partition_id: &str, target_size_gb: u32) -> Result<(), DiskError> {
        let cmd_str = format!("Resize-Partition -DiskNumber {} -PartitionNumber {} -Size {}GB", disk_id, partition_id, target_size_gb);
        
        let output_fut = tokio::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &cmd_str])
            .output();

        let output = tokio::time::timeout(std::time::Duration::from_secs(300), output_fut)
            .await
            .map_err(|_| DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Resize-Partition timed out after 5 minutes",
            )))?
            .map_err(DiskError::OsError)?;

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
    }
}
