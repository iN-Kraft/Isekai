use std::cmp::Reverse;
use std::io::{Error, ErrorKind};
use std::process::Stdio;
use std::time::Duration;
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use windows_sys::Win32::System::SystemInformation::{FirmwareTypeUefi, GetFirmwareType};
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, Partition};
use crate::domain::traits::DiskManager;
use crate::infrastructure::windows::wmi::{BitLockerState, MsftDisk, MsftPartition, MsftPhysicalDisk, MsftVolume};
use crate::infrastructure::windows::utils::{ParsingUtils, PartitionUtils};
use wmi::WMIConnection;
use crate::application::spawn_blocking_with_context;
use crate::domain::{PARTITION_LABEL_EFI, PARTITION_LABEL_LIVE};
use crate::infrastructure::CommandExt;
use crate::infrastructure::windows::bitlocker::BitLocker;
use crate::infrastructure::windows::diskpart::run_diskpart_script;
use crate::ipc::protocol::{IpcEvent, OutgoingMessage};
use crate::telemetry;

pub struct WindowsDiskManager {
    debug_mode: bool,
}

impl WindowsDiskManager {
    pub fn new(debug_mode: bool) -> Self {

        Self { debug_mode }
    }

    fn create_wmi_connection() -> Result<WMIConnection, DiskError> {
        WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage")
            .map_err(|e| DiskError::WmiError(format!("Failed to initialize WMI: {}", e)))
    }

    pub async fn rollback_live_partitions(&self, disk_id: u32, _is_uefi: bool) -> Result<(), DiskError> {
        telemetry!(info, "ROLLBACK: Purging incomplete partitions on disk {}", disk_id);

        let dp_script = format!(
            "select disk {}\n\
            select volume {}\n\
            delete volume\n\
            select volume {}\n\
            delete volume\n\
            exit\n",
            disk_id, PARTITION_LABEL_LIVE, PARTITION_LABEL_EFI
        );
        crate::infrastructure::windows::diskpart::run_diskpart_script(&dp_script, format!("rollback_{}", disk_id)).await
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
            telemetry!(info, "Partition not found yet. Retrying in WMI sync loop...");
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
        telemetry!(step, format!("STRATEGY: WIPE DISK {} (Style: {})", disk_id, partition_style.to_uppercase()));

        let dp_script = format!(
            "select disk {}\n\
             clean\n\
             convert {}\n\
             exit\n",
            disk_id, partition_style
        );

        crate::infrastructure::windows::diskpart::run_diskpart_script(&dp_script, format!("wipe_{}", disk_id)).await?;

        tokio::time::sleep(Duration::from_secs(3)).await;

        Ok(())
    }

    pub async fn create_live_boot_partitions(
        &self,
        disk_id: u32,
        iso_payload_size_mb: u32,
        is_uefi: bool
    ) -> Result<(String, Option<String>), DiskError> {
        let efi_driver_size_mb = 50;
        let dp_script = if is_uefi {
            format!(
                "select disk {}\n\
                create partition primary size={}\n\
                format fs=fat32 quick label=\"{}\"\n\
                assign\n\
                create partition primary size={}\n\
                format fs=fat32 quick label=\"{}\"\n\
                assign\n\
                exit\n",
                disk_id, iso_payload_size_mb, PARTITION_LABEL_LIVE, efi_driver_size_mb, PARTITION_LABEL_EFI
            )
        } else {
            telemetry!(info, "Legacy BIOS detected. Skipping FAT32 EFI partition creation to respect MBR limits.");
            format!(
                "select disk {}\n\
                create partition primary size={}\n\
                format fs=fat32 quick label=\"{}\"\n\
                assign\n\
                exit\n",
                disk_id, iso_payload_size_mb, PARTITION_LABEL_LIVE
            )
        };

        telemetry!(step, format!("Creating Live Boot Partitions (NTFS Payload{})...", if is_uefi { " + FAT32 Driver Hook" } else { "" }));
        crate::infrastructure::windows::diskpart::run_diskpart_script(&dp_script, format!("create_live_{}", disk_id)).await?;

        let mut ntfs_letter = None;
        let mut fat32_letter = None;

        telemetry!(info, "Waiting for Windows VDS to map drive letters...");

        for _ in 0..6 {
            tokio::time::sleep(Duration::from_secs(2)).await;

            let fresh_parts = self.get_partitions(&disk_id.to_string()).await?;

            if ntfs_letter.is_none() {
                ntfs_letter = fresh_parts.iter()
                    .find(|p| p.label.contains(PARTITION_LABEL_LIVE))
                    .and_then(|p| p.drive_letter.clone());
            }

            if is_uefi && fat32_letter.is_none() {
                fat32_letter = fresh_parts.iter()
                    .find(|p| p.label.contains(PARTITION_LABEL_EFI))
                    .and_then(|p| p.drive_letter.clone());
            }

            if ntfs_letter.is_some() && (!is_uefi || fat32_letter.is_some()) {
                break;
            }
        }

        if ntfs_letter.is_none() || (is_uefi && fat32_letter.is_none()) {
            telemetry!(warn, "WMI timeout mounting partitions. Executing rollback...");
            let _ = self.rollback_live_partitions(disk_id, is_uefi).await;
            return Err(DiskError::DiskNotFound("Failed to mount FAT32 partitions. WMI timeout.".into()));
        }

        let ntfs_letter = ntfs_letter.unwrap();

        if is_uefi && fat32_letter.is_none() {
            return Err(DiskError::DiskNotFound("Failed to mount FAT32 EFI partition. WMI timeout.".into()));
        }

        telemetry!(info, "Partitions created! Payload: {}: | UEFI Hook: {:?}", ntfs_letter, fat32_letter);

        Ok((ntfs_letter, fat32_letter))
    }

    pub fn is_uefi_host() -> bool {
        let mut fw_type = 0;
        unsafe {
            let _ = GetFirmwareType(&mut fw_type);
        }
        fw_type == FirmwareTypeUefi
    }

    pub async fn is_mechanical_drive(&self, disk_id: u32) -> Result<bool, DiskError> {
        let is_hdd = spawn_blocking_with_context(move || -> Result<bool, DiskError> {
            let wmi_con = WindowsDiskManager::create_wmi_connection()?;
            let query = format!("SELECT MediaType FROM MSFT_PhysicalDisk WHERE DeviceId = '{}'", disk_id);
            let result: Vec<MsftPhysicalDisk> = wmi_con.raw_query(&query).map_err(|e| DiskError::WmiError(format!("PhysicalDisk Query failed: {}", e)))?;

            Ok(result.first().and_then(|d| d.MediaType) == Some(3))
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        Ok(is_hdd)
    }

    pub async fn get_max_shrink_size(&self, drive_letter: &str) -> Result<u64, DiskError> {
        let clean_letter = drive_letter.trim_end_matches(':').trim_end_matches('\\');
        let ps_script = format!(
            "$size = Get-PartitionSupportedSize -DriveLetter '{}' -ErrorAction Stop; [PSCustomObject]@{{ SizeMin = $size.SizeMin; SizeMax = $size.SizeMax }} | ConvertTo-Json -Compress",
            clean_letter
        );

        let output = Command::new("powershell.exe")
            .kill_on_drop(true)
            .no_window()
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        if !output.status.success() {
            return Err(DiskError::DataValidation("Failed to calculate maximum shrink size".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            let size_min = json["SizeMin"].as_u64().unwrap_or(u64::MAX);
            return Ok(size_min);
        }

        Err(DiskError::DataValidation("Failed to parse shrink size from Windows API".into()))
    }

    pub async fn repair_disk(&self, drive_letter: &str, file_system: &str) -> Result<(), DiskError> {
        let letter = drive_letter.trim_end_matches('\\');
        let is_ntfs = file_system.eq_ignore_ascii_case("NTFS");
        let args = if is_ntfs {
            vec![letter, "/scan", "/perf"]
        } else {
            vec![letter, "/f", "/x"]
        };
        telemetry!(info, "Running chkdsk repair on {} to clear volume errors...", letter);

        let output = Command::new("chkdsk.exe")
            .kill_on_drop(true)
            .no_window()
            .args(&args)
            .output()
            .await
            .map_err(DiskError::OsError)?;

        // chkdsk exit codes:
        // 0 = No errors
        // 1 = Errors found and fixed
        // 2 = Cleanup needed (but safe to proceed)
        // >2 = Could not fix online or fatal error
        if let Some(code) = output.status.code() {
            if code > 2 {
                telemetry!(warn, "Chkdsk reported unfixable errors (Code: {}). The shrink operation might be rejected by Windows.", code);
            } else {
                telemetry!(info, "Filesystem check complete. Volume is clean.");
            }
        } else {
            telemetry!(warn, "Chkdsk terminated unexpectedly without an exit code.");
        }

        Ok(())
    }

    pub fn start_hardware_watcher() {
        spawn_blocking_with_context(move || {
            let wmi_con = match WMIConnection::with_namespace_path("ROOT\\CIMV2") {
                Ok(w) => w,
                Err(e) => {
                    telemetry!(error, "Failed to connect to WMI for hardware watcher: {}", e);
                    return;
                }
            };
            let query = "SELECT * FROM Win32_VolumeChangeEvent";
            let iterator = match wmi_con.exec_notification_query(query) {
                Ok(i) => i,
                Err(e) => {
                    telemetry!(error, "Failed to execute WMI event query: {}", e);
                    return;
                }
            };

            telemetry!(info, "Hardware watcher started. Listening for volume interrupts...");

            for event in iterator {
                match event {
                    Ok(_) => {
                        telemetry!(info, "HardwareChanged");
                    }
                    Err(e) => {
                        telemetry!(error, "WMI watcher event error: {}", e);
                    }
                }
            }
        });
    }
}

#[async_trait]
impl DiskManager for WindowsDiskManager {

    async fn get_disks(&self) -> Result<Vec<Disk>, DiskError> {
        let wmi_disks = spawn_blocking_with_context(move || -> Result<Vec<MsftDisk>, DiskError> {
            let wmi_con = WindowsDiskManager::create_wmi_connection()?;
            let results: Vec<MsftDisk> = wmi_con
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
        let (wmi_parts, volumes) = spawn_blocking_with_context(move || -> Result<(Vec<MsftPartition>, Vec<MsftVolume>), DiskError> {
            let wmi_con = WindowsDiskManager::create_wmi_connection()?;
            let query = format!("SELECT * FROM MSFT_Partition WHERE DiskNumber = {}", disk_index);

            let results: Vec<MsftPartition> = wmi_con
                .raw_query(&query)
                .map_err(|e| DiskError::WmiError(format!("WMI Query failed for Partition: {}", e)))?;

            let volumes: Vec<MsftVolume> = wmi_con
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

            let label = vol_label_str.unwrap_or_else(|| PartitionUtils::determine_partition_label(
                part.DriveLetter.as_deref(),
                part.GptType.as_deref(),
                part.MbrType
            ));

            let mut bl_state = BitLockerState::Unprotected;
            if let Some(ref dl) = drive_letter {
                if let Ok(state) = BitLocker::get_state(Some(dl)).await {
                    bl_state = state;
                }
            }

            partitions.push(Partition {
                id: part.PartitionNumber.to_string(),
                drive_letter,
                size_gb,
                file_system: fs,
                label,
                offset_bytes: part.Offset.unwrap_or(0),
                size_bytes: size_bytes,
                free_bytes,
                bitlocker_state: bl_state
            });
        }

        partitions.sort_by_key(|p| p.id.parse::<u32>().unwrap_or(u32::MAX));
        Ok(partitions)
    }

    async fn shrink_partition(&self, disk_id: &str, partition_id: &str, target_size_bytes: u64) -> Result<(), DiskError> {
        let partitions = self.get_partitions_fresh(disk_id, Some(partition_id)).await?;
        let target_part = partitions.iter().find(|p| p.id == partition_id).ok_or_else(|| DiskError::DiskNotFound(format!("Partition {} disappeared", partition_id)))?;

        telemetry!(info, "Attempting primary shrink method: Resize-Partition");
        let cmd_str = format!("Resize-Partition -DiskNumber {} -PartitionNumber {} -Size {}", disk_id, partition_id, target_size_bytes);

        let output_fut = tokio::process::Command::new("powershell.exe")
            .kill_on_drop(true)
            .no_window()
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

        telemetry!(warn, "Resize-Partition failed. Attempting robust diskpart fallback...");
        let shrink_amount_bytes = target_part.size_bytes.saturating_sub(target_size_bytes);
        let shrink_amount_mb = shrink_amount_bytes / (1024 * 1024);

        let dp_script = format!(
            "select disk {}\nselect partition {}\nshrink desired={}\nexit\n",
            disk_id, partition_id, shrink_amount_mb
        );

        crate::infrastructure::windows::diskpart::run_diskpart_script(&dp_script, format!("shrink_{}_{}", disk_id, partition_id)).await?;
        tokio::time::sleep(Duration::from_secs(10)).await;

        let fresh_parts = self.get_partitions_fresh(disk_id, Some(partition_id)).await?;
        let shrunken_part = fresh_parts.iter().find(|p| p.id == partition_id).ok_or_else(|| DiskError::DiskNotFound(format!("Partition {} disappeared after diskpart", partition_id)))?;
        let tolerance_bytes = 5 * 1024 * 1024;

        if shrunken_part.size_bytes.abs_diff(target_size_bytes) > tolerance_bytes {
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("Diskpart silent failure: Partition size is {} bytes, but expected roughly {} bytes. Shrink aborted.", shrunken_part.size_bytes, target_size_bytes)
            )));
        }

        Ok(())
    }

    async fn uninstall_isekai(&self, disk_id: &str) -> Result<(), DiskError> {
        let partitions = self.get_partitions(disk_id).await?;
        let mut parts_to_delete = Vec::new();

        for p in &partitions {
            if p.label.contains(PARTITION_LABEL_LIVE) || p.label.contains(PARTITION_LABEL_EFI) {
                parts_to_delete.push(p.clone())
            }
        }

        if parts_to_delete.is_empty() {
            telemetry!(warn, "No Isekai partitions found on disk {}. BCD was cleaned, but no disk space was reclaimed.", disk_id);
            return Ok(());
        }

        parts_to_delete.sort_by_key(|p| Reverse(p.id.parse::<u32>().unwrap_or(0)));

        for p in parts_to_delete {
            telemetry!(info, "Deleting partition {} (Label: '{}')...", p.id, p.label);

            let dp_script = format!(
                "select disk {}\n\
                select partition {}\n\
                delete partition override\n\
                exit\n",
                disk_id, p.id
            );
            run_diskpart_script(&dp_script, format!("del_{}", p.id)).await?;
        }
        tokio::time::sleep(Duration::from_secs(4)).await;

        let fresh_partitions = self.get_partitions(disk_id).await?;
        let mut main_part = fresh_partitions.iter().find(|p| p.drive_letter.as_deref() == Some("C:"));

        if main_part.is_none() {
            main_part = fresh_partitions.iter()
                .filter(|p| p.file_system.eq_ignore_ascii_case("NTFS"))
                .max_by_key(|p| p.size_bytes)
        }

        if let Some(main) = main_part {
            telemetry!(info, "Reclaiming unallocated space into main partition {}...", main.id);

            let dp_script = format!(
                "select disk {}\n\
                select partition {}\n\
                extend\n\
                exit\n",
                disk_id, main.id
            );

            if let Err(e) = run_diskpart_script(&dp_script, format!("ext_{}", main.id)).await {
                telemetry!(warn, "Could not extend partition. Space remains unallocated. ({})", e);
            } else {
                telemetry!(info, "Successfully extended main partition.");
            }
        } else {
            telemetry!(warn, "Could not confidently determine the main partition. Leaving space unallocated for safety.");
        }

        Ok(())
    }
}