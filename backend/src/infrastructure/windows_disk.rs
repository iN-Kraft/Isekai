#![allow(non_snake_case)]

use std::env::temp_dir;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use async_trait::async_trait;
use serde::Deserialize;
use wmi::WMIConnection;
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, InstallPlan, Partition};
use crate::domain::traits::DiskManager;
use std::string::String;
use std::sync::Arc;
use std::time::Duration;
use windows_sys::Win32::System::SystemInformation::{FirmwareTypeUefi, GetFirmwareType};

const MSR_RESERVE_BYTES: u64 = 16 * 1024 * 1024;
const PARTITION_ALIGNMENT_BYTES: u64 = 1024 * 1024;
const TOTAL_PLACEMENT_OVERHEAD_BYTES: u64 = MSR_RESERVE_BYTES + PARTITION_ALIGNMENT_BYTES;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MsftPhysicalDisk {
    MediaType: Option<u16>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MsftDisk {
    Number: u32,
    FriendlyName: Option<String>,
    Size: Option<u64>,
    IsSystem: Option<bool>,
    IsBoot: Option<bool>,
    BusType: Option<u16>,
    PartitionStyle: Option<u16>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MsftPartition {
    DiskNumber: u32,
    PartitionNumber: u32,
    Offset: Option<u64>,
    Size: Option<u64>,
    DriveLetter: Option<String>,
    GptType: Option<String>,
    MbrType: Option<u16>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MsftVolume {
    DriveLetter: Option<String>,
    FileSystem: Option<String>,
    SizeRemaining: Option<u64>,
    FileSystemLabel: Option<String>
}

struct Gap {
    start: u64,
    end: u64,
    size: u64
}

struct TempFileGuard(PathBuf);
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let path = self.0.clone();

        tokio::task::spawn_blocking(move || {
            let _ = fs::remove_file(&path);
        });
    }
}

struct SharedWmi(WMIConnection);
unsafe impl Send for SharedWmi {}
unsafe impl Sync for SharedWmi {}

pub struct WindowsDiskManager {
    debug_mode: bool,
    wmi_con: Arc<SharedWmi>,
}

impl WindowsDiskManager {
    pub fn new(debug_mode: bool) -> Result<Self, DiskError> {
        let wmi_con = WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage")
            .map_err(|e| DiskError::WmiError(format!("Failed to initialize WMI: {}", e)))?;

        Ok(Self {
            debug_mode,
            wmi_con: Arc::new(SharedWmi(wmi_con))
        })
    }

    pub async fn rollback_live_partitions(&self, disk_id: u32, _is_uefi: bool) -> Result<(), DiskError> {
        println!("ROLLBACK: Purging incomplete partitions on disk {}", disk_id);

        let dp_script = format!(
            "select disk {}\n\
            select volume LINUX_LIVE\n\
            delete volume\n\
            select volume LINUX_EFI\n\
            delete volume\n\
            exit\n",
            disk_id
        );
        self.run_diskpart_script(&dp_script, format!("rollback_{}", disk_id)).await
    }

    async fn get_partitions_fresh(
        &self,
        disk_id: &str,
        expected_partition_id: Option<&str>
    ) -> Result<Vec<Partition>, DiskError> {
        let delays = vec![500, 1000, 2000, 3000, 5000];

        for delay in delays {
            tokio::time::sleep(Duration::from_millis(delay)).await;
            let partitions = self.get_partitions(disk_id).await?;

            if expected_partition_id.is_none() || partitions.iter().any(|p| p.id == expected_partition_id.unwrap()) {
                return Ok(partitions);
            }
            println!("Partition not found yet. Retrying in WMI sync loop...");
        }

        Err(DiskError::DiskNotFound(
            format!("Partition {:?} did not appear in WMI after sync delays.", expected_partition_id)
        ))
    }

    async fn wipe_disk(&self, disk_id: u32, os_disk_id: u32, is_uefi: bool) -> Result<(), DiskError> {
        if disk_id == os_disk_id {
            return Err(DiskError::OsError(Error::new(
                ErrorKind::PermissionDenied,
                "SECURITY LOCKOUT: Refusing to wipe the disk containing the active Windows OS."
            )));
        }

        let partition_style = if is_uefi { "gpt" } else { "mbr" };
        println!("== STRATEGY: WIPE DISK {} (Style: {}) ==", disk_id, partition_style.to_uppercase());

        let dp_script = format!(
            "select disk {}\n\
             clean\n\
             convert {}\n\
             exit\n",
            disk_id, partition_style
        );

        self.run_diskpart_script(&dp_script, format!("wipe_{}", disk_id)).await?;

        tokio::time::sleep(Duration::from_secs(3)).await;

        Ok(())
    }

    pub async fn create_live_boot_partitions(
        &self,
        disk_id: u32,
        _target_offset_bytes: u64,
        iso_payload_size_mb: u32,
        is_uefi: bool
    ) -> Result<(String, Option<String>), DiskError> {
        let efi_driver_size_mb = 50;
        let dp_script = if is_uefi {
            format!(
                "select disk {}\n\
                create partition primary size={}\n\
                format fs=ntfs quick label=\"LINUX_LIVE\"\n\
                assign\n\
                create partition primary size={}\n\
                format fs=fat32 quick label=\"LINUX_EFI\"\n\
                assign\n\
                exit\n",
                disk_id, iso_payload_size_mb, efi_driver_size_mb
            )
        } else {
            println!("Legacy BIOS detected. Skipping FAT32 EFI partition creation to respect MBR limits.");
            format!(
                "select disk {}\n\
                create partition primary size={}\n\
                format fs=ntfs quick label=\"LINUX_LIVE\"\n\
                assign\n\
                exit\n",
                disk_id, iso_payload_size_mb
            )
        };

        println!("Creating Live Boot Partitions (NTFS Payload{})...", if is_uefi { " + FAT32 Driver Hook" } else { "" });
        self.run_diskpart_script(&dp_script, format!("create_live_{}", disk_id)).await?;

        let mut ntfs_letter = None;
        let mut fat32_letter = None;

        println!("Waiting for Windows VDS to map drive letters...");

        for _ in 0..6 {
            tokio::time::sleep(Duration::from_secs(2)).await;

            let fresh_parts = self.get_partitions(&disk_id.to_string()).await?;

            if ntfs_letter.is_none() {
                ntfs_letter = fresh_parts.iter()
                    .find(|p| p.label.contains("LINUX_LIVE"))
                    .and_then(|p| p.drive_letter.clone());
            }

            if is_uefi && fat32_letter.is_none() {
                fat32_letter = fresh_parts.iter()
                    .find(|p| p.label.contains("LINUX_EFI"))
                    .and_then(|p| p.drive_letter.clone());
            }

            if ntfs_letter.is_some() && (!is_uefi || fat32_letter.is_some()) {
                break;
            }
        }

        let ntfs_letter = ntfs_letter.ok_or_else(|| DiskError::DiskNotFound("Failed to mount NTFS Payload partition. WMI timeout.".into()))?;

        if is_uefi && fat32_letter.is_none() {
            return Err(DiskError::DiskNotFound("Failed to mount FAT32 EFI partition. WMI timeout.".into()));
        }

        println!("Partitions created! Payload: {}: | UEFI Hook: {:?}", ntfs_letter, fat32_letter);

        Ok((ntfs_letter, fat32_letter))
    }

    async fn run_diskpart_script(&self, script_content: &str, identifier: String) -> Result<(), DiskError> {
        let temp_dir = temp_dir();
        let script_path = temp_dir.join(format!("dp_{}.txt", identifier));
        let _guard = TempFileGuard(script_path.clone());

        tokio::fs::write(&script_path, script_content).await.map_err(DiskError::OsError)?;

        let output = tokio::process::Command::new("diskpart")
            .args(["/s", script_path.to_str().unwrap()])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if !stdout.to_lowercase().contains("successfully") {
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("DiskPart execution failed:\n{}", stdout)
            )));
        }

        Ok(())
    }

    pub fn is_uefi_host() -> bool {
        let mut fw_type = 0;
        unsafe {
            let _ = GetFirmwareType(&mut fw_type);
        }
        fw_type == FirmwareTypeUefi
    }

    pub async fn is_mechanical_drive(&self, disk_id: u32) -> Result<bool, DiskError> {
        let wmi_con = Arc::clone(&self.wmi_con);
        let is_hdd = tokio::task::spawn_blocking(move || -> Result<bool, DiskError> {
            let query = format!("SELECT MediaType FROM MSFT_PhysicalDisk WHERE DeviceId = '{}'", disk_id);
            let result: Vec<MsftPhysicalDisk> = wmi_con.0.raw_query(&query).map_err(|e| DiskError::WmiError(format!("PhysicalDisk Query failed: {}", e)))?;

            Ok(result.first().and_then(|d| d.MediaType) == Some(3))
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        Ok(is_hdd)
    }
}

fn determine_partition_label(drive_letter: Option<&str>, gpt_type: Option<&str>, mbr_type: Option<u16>) -> String {
    if let Some(dl) = drive_letter {
        let trimmed = dl.trim_matches('\0').trim();
        if trimmed == "C" {
            return "C: (Windows/NTFS)".to_string();
        } else if !trimmed.is_empty() {
            return format!("{}: drive", trimmed)
        }
    }

    if let Some(gpt) = gpt_type {
        let gpt_lower = gpt.to_lowercase();
        if gpt_lower.contains("de94bba4") {
            return "Recovery".to_string();
        } else if gpt_lower.contains("e3c9e316") {
            return "Microsoft Reversed".to_string();
        } else if gpt_lower.contains("c12a7328") {
            return "EFI System (ESP)".to_string();
        }
    }

    if let Some(pt) = mbr_type {
        if pt == 4 || pt == 39 {
            return "Recovery".to_string();
        }
    }

    return "Partition".to_string();
}

fn calculate_required_shrink_bytes(linux_size_mb: u32, boot_size_mb: u32) -> u64 {
    let mb_to_bytes = 1024_u64 * 1024;
    let linux_bytes = (linux_size_mb as u64) * mb_to_bytes;
    let boot_bytes = (boot_size_mb as u64) * mb_to_bytes;

    return linux_bytes + boot_bytes + TOTAL_PLACEMENT_OVERHEAD_BYTES;
}

fn get_contiguous_install_plan(
    disk_size_bytes: u64,
    partitions: &[Partition],
    anchor_end_bytes: u64,
    boot_size_mb: u32,
    linux_size_mb: u32
) -> InstallPlan {
    let mut gaps = Vec::new();
    let mut prev_end: u64 = 0;

    for part in partitions {
        if part.offset_bytes > prev_end {
            let gap_size = part.offset_bytes - prev_end;
            if gap_size > PARTITION_ALIGNMENT_BYTES {
                gaps.push(Gap {
                    start: prev_end,
                    end: part.offset_bytes,
                    size: gap_size
                });
            }
        }
        prev_end = part.offset_bytes + part.size_bytes;
    }

    if disk_size_bytes > prev_end {
        let trailing_gap = disk_size_bytes - prev_end;
        if trailing_gap > PARTITION_ALIGNMENT_BYTES {
            gaps.push(Gap {
                start: prev_end,
                end: disk_size_bytes,
                size: trailing_gap
            });
        }
    }

    let boot_size_bytes = (boot_size_mb as u64) * 1024 * 1024;
    let min_gap_required = boot_size_bytes + TOTAL_PLACEMENT_OVERHEAD_BYTES;
    let usable_gaps: Vec<&Gap> = gaps.iter().filter(|g| g.size >= min_gap_required).collect();
    let mut result = InstallPlan {
        has_boot_space: false,
        has_requested_linux_space: false,
        boot_partition_offset_bytes: 0,
        linux_space_bytes: 0
    };

    if usable_gaps.is_empty() {
        return result;
    }

    let chosen_gap = usable_gaps.iter().find(|&&g| {
        let lower_bound = anchor_end_bytes.saturating_sub(PARTITION_ALIGNMENT_BYTES);
        let upper_bound = anchor_end_bytes.saturating_add(PARTITION_ALIGNMENT_BYTES);
        g.start >= lower_bound && g.start <= upper_bound
    }).copied().or_else(|| {
        usable_gaps.iter()
            .filter(|&&g| g.start >= anchor_end_bytes)
            .max_by_key(|&&g| g.size)
            .copied()
    });

    let chosen_gap = match chosen_gap {
        Some(g) => g,
        None => return result
    };

    let boot_end = chosen_gap.end.saturating_sub(MSR_RESERVE_BYTES);
    let raw_boot_offset = boot_end.saturating_sub(boot_size_bytes);
    let boot_partition_offset = (raw_boot_offset / PARTITION_ALIGNMENT_BYTES) * PARTITION_ALIGNMENT_BYTES;

    if boot_partition_offset < (chosen_gap.start + PARTITION_ALIGNMENT_BYTES) {
        return result;
    }

    let linux_space = boot_partition_offset - chosen_gap.start;
    let requested_linux_bytes = (linux_size_mb as u64) * 1024 * 1024;

    result.has_boot_space = true;
    result.has_requested_linux_space = linux_space >= requested_linux_bytes;
    result.boot_partition_offset_bytes = boot_partition_offset;
    result.linux_space_bytes = linux_space;

    return result;
}

async fn check_bitlocker_status(drive_letter: Option<&str>) -> Result<(), DiskError> {
    let letter = match drive_letter {
        Some(l) => l,
        None => return Ok(()) // No drive letter usually means no BootLicker
    };

    let output = tokio::process::Command::new("manage-bde")
        .args(["-status", letter])
        .output()
        .await
        .map_err(DiskError::OsError)?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.contains("Conversion Status:    Encrypted") || stdout.contains("Fully Encrypted") {
        return Err(DiskError::DriveEncrypted(letter.to_string()));
    }

    return Ok(())
}

async fn create_uefi_boot_entry(
    distro_name: &str,
    device_partition: &str,
    efi_path: &str
) -> Result<(), DiskError> {
    let copy_out = tokio::process::Command::new("bcdedit.exe")
        .args(["/copy", "{bootmgr}", "/d", distro_name])
        .output()
        .await
        .map_err(DiskError::OsError)?;

    let copy_str = String::from_utf8_lossy(&copy_out.stdout);

    let guid = if let (Some(start), Some(end)) = (copy_str.find('{'), copy_str.find('}')) {
        &copy_str[start..=end]
    } else {
        return Err(DiskError::OsError(Error::new(
            ErrorKind::Other,
            format!("Failed to parse GUID from bcdedit output: {}", copy_str)
        )));
    };

    println!("Created new EFI boot entry: {}", guid);

    let inherited_props = [
        "default", "displayorder", "toolsdisplayorder", "timneout", "resumeobject", "inhreit", "locale"
    ];

    for prop in inherited_props {
        let _ = tokio::process::Command::new("bcdedit.exe")
            .args(["/deletevalue", guid, prop])
            .output()
            .await;
    }

    println!("Setting device=partition={} path={}", device_partition, efi_path);

    let run_cmd = |args: Vec<String>| async move {
        let out = tokio::process::Command::new("bcdedit.exe")
            .args(&args)
            .output()
            .await
            .map_err(DiskError::OsError)?;

        if !out.status.success() {
            Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("bcdedit {:?} failed with code {:?}", args, out.status.code())
            )))
        } else {
            Ok(())
        }
    };

    let device_arg = format!("partition={}", device_partition);
    let guid_str = guid.to_string();
    let distro_str = distro_name.to_string();
    let efi_str = efi_path.to_string();

    let config_result = async {
        run_cmd(vec!["/set".to_string(), guid_str.clone(), "device".to_string(), device_arg]).await?;
        run_cmd(vec!["/set".to_string(), guid_str.clone(), "path".to_string(), efi_str]).await?;
        run_cmd(vec!["/set".to_string(), guid_str.clone(), "description".to_string(), distro_str]).await?;

        run_cmd(vec!["/set".to_string(), "{fwbootmgr}".to_string(), "displayorder".to_string(), guid_str.clone(), "/addfirst".to_string()]).await?;
        run_cmd(vec!["/set".to_string(), "{fwbootmgr}".to_string(), "default".to_string(), guid_str]).await?;

        Ok::<(), DiskError>(())
    }.await;

    if let Err(e) = config_result {
        println!("Error configuration boot entry: {}. Rolling back...", e);
        let _ = tokio::process::Command::new("bcdedit.exe")
            .args(["/delete", guid])
            .output()
            .await;

        return Err(e);
    }

    println!("UEFI boot entry created and set as default!");
    return Ok(());
}

#[async_trait]
impl DiskManager for WindowsDiskManager {
    async fn get_disks(&self) -> Result<Vec<Disk>, DiskError> {
        let wmi_con = Arc::clone(&self.wmi_con);
        let wmi_disks = tokio::task::spawn_blocking(move || -> Result<Vec<MsftDisk>, DiskError> {
            let results: Vec<MsftDisk> = wmi_con.0
                .raw_query("SELECT * FROM MSFT_Disk")
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed for Disk: {}", e)))?;

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
                let is_gpt = wmi_disk.PartitionStyle.unwrap_or(0) == 2;

                disks.push(Disk {
                    stable_id: wmi_disk.Number.to_string(),
                    name: friendly_name.to_string(),
                    total_gb: size_gb,
                    free_gb: 0,
                    is_system_drive: is_sys,
                    is_gpt
                });
            }

        disks.sort_by_key(|d| d.stable_id.parse::<u32>().unwrap_or(u32::MAX));
        Ok(disks)
    }

    async fn get_partitions(&self, disk_id: &str) -> Result<Vec<Partition>, DiskError> {
        let disk_index: u32 = disk_id.parse().map_err(|_| {
            DiskError::DiskNotFound(disk_id.to_string())
        })?;
        let wmi_con = Arc::clone(&self.wmi_con);

        let (wmi_parts, volumes) = tokio::task::spawn_blocking(move || -> Result<(Vec<MsftPartition>, Vec<MsftVolume>), DiskError> {
            let query = format!("SELECT * FROM MSFT_Partition WHERE DiskNumber = {}", disk_index);

            let results: Vec<MsftPartition> = wmi_con.0
                .raw_query(&query)
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed for Partition: {}", e)))?;

            let volumes: Vec<MsftVolume> = wmi_con.0
                .raw_query("SELECT * FROM MSFT_Volume")
                .map_err(|e| DiskError::WmiError(format!("WMI Volume Query failed: {}", e)))?;

            Ok((results, volumes))
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let mut partitions = Vec::with_capacity(wmi_parts.len());

        for part in wmi_parts {
            let size_bytes = part.Size.unwrap_or(0);
            let size_gb = (size_bytes / 1024 / 1024 / 1024) as u32;

            let mut fs = "Unknown".to_string();
            let mut drive_letter = None;
            let mut free_bytes = 0;
            let mut vol_label_str = None;
            
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
                        if let Some(vol_free) = vol.SizeRemaining {
                            free_bytes = vol_free;
                        }
                        if let Some(lbl) = &vol.FileSystemLabel {
                            vol_label_str = Some(lbl.clone());
                        }
                    }
                }
            }

            let label = vol_label_str.unwrap_or_else(|| determine_partition_label(
                part.DriveLetter.as_deref(),
                part.GptType.as_deref(),
                part.MbrType
            ));

            partitions.push(Partition {
                id: part.PartitionNumber.to_string(),
                drive_letter,
                size_gb,
                file_system: fs,
                label,
                offset_bytes: part.Offset.unwrap_or(0),
                size_bytes: size_bytes,
                free_bytes
            });
        }

        partitions.sort_by_key(|p| p.id.parse::<u32>().unwrap_or(u32::MAX));
        Ok(partitions)
    }

    async fn shrink_partition(&self, disk_id: &str, partition_id: &str, target_size_bytes: u64) -> Result<(), DiskError> {
        let partitions = self.get_partitions_fresh(disk_id, Some(partition_id)).await?;
        let target_part = partitions.iter().find(|p| p.id == partition_id).ok_or_else(|| DiskError::DiskNotFound(format!("Partition {} disappeared", partition_id)))?;

        check_bitlocker_status(target_part.drive_letter.as_deref()).await?;

        println!("Attempting primary shrink method: Resize-Partition");
        let cmd_str = format!("Resize-Partition -DiskNumber {} -PartitionNumber {} -Size {}", disk_id, partition_id, target_size_bytes);
        
        let output_fut = tokio::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &cmd_str])
            .output();

        let ps_output = tokio::time::timeout(std::time::Duration::from_secs(300), output_fut)
            .await
            .map_err(|_| DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Resize-Partition timed out after 5 minutes",
            )))?
            .map_err(DiskError::OsError)?;

        if ps_output.status.success() {
            tokio::time::sleep(Duration::from_secs(3)).await;
            return Ok(());
        }

        println!("Resize-Partition failed. Attempting robust diskpart fallback...");
        let shrink_amount_bytes = target_part.size_bytes.saturating_sub(target_size_bytes);
        let shrink_amount_mb = shrink_amount_bytes / (1024 * 1024);

        let dp_script = format!(
            "select disk {}\nselect partition {}\nshrink desired={}\nexit\n",
            disk_id, partition_id, shrink_amount_mb
        );

        self.run_diskpart_script(&dp_script, format!("shrink_{}_{}", disk_id, partition_id)).await?;
        tokio::time::sleep(Duration::from_secs(10)).await;

        Ok(())
    }
}
