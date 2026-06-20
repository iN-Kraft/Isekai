use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use crate::application::spawn_blocking_with_context;
use crate::application::state::{WorkflowGuard, WorkflowType};
use crate::domain::errors::DiskError;
use crate::domain::traits::DiskManager;
use crate::infrastructure::{CommandExt, NativeDiskManager};
use crate::infrastructure::windows::autoplay::AutoPlayGuard;
use crate::infrastructure::windows::bitlocker::BitLocker;
use crate::infrastructure::windows::boot::BootManager;
use crate::infrastructure::windows::iso_manager::IsoManager;
use crate::infrastructure::windows::payload_manager::PayloadManager;
use crate::infrastructure::windows::saga::{Compensation, SagaOrchestrator};
use crate::infrastructure::windows::wmi::BitLockerState;
use crate::telemetry;

pub async fn shrink_install_local(
    disk_manager: Arc<dyn DiskManager>,
    disk_id: String,
    partition_id: String,
    iso_path: String,
) -> Result<(), DiskError> {
    let _workflow = WorkflowGuard::start(WorkflowType::ShrinkAndInstall);
    let _autoplay = AutoPlayGuard::new();

    let is_pre_mounted = iso_path.len() <= 3 && (iso_path.ends_with(':') || iso_path.ends_with(":\\"));
    let iso_drive_letter = if is_pre_mounted {
        let letter = iso_path.trim_end_matches('\\').to_string();
        telemetry!(info, "Using pre-mounted ISO on drive: {}", letter);
        letter
    } else {
        telemetry!(info, "Mounting ISO Payload: {}", iso_path);
        let letter = IsoManager::mount_iso(&iso_path).await?;
        tokio::time::sleep(Duration::from_millis(1500)).await;
        letter
    };

    let is_bootable = IsoManager::verify_bootable_iso(&iso_drive_letter).await;
    if !is_bootable {
        if !is_pre_mounted {
            let _ = IsoManager::dismount_iso(&iso_path).await;
        }
        return Err(DiskError::DataValidation("ISO is not bootable or missing EFI/boot configuration.".into()));
    }

    let workflow_result = async {
        let mb_to_bytes = 1024_u64 * 1024;

        telemetry!(info, "Calculating required partition size based on payload...");
        let iso_base_path = format!("{}:\\", iso_drive_letter.trim_end_matches(':'));
        let payload_size_bytes = spawn_blocking_with_context(move || {
            PayloadManager::get_dir_size(iso_base_path)
        }).await.unwrap_or(0);

        if payload_size_bytes <= 0 {
            return Err(DiskError::DataValidation("Failed to read ISO contents or ISO is empty.".into()));
        }

        let buffer_bytes = 512 * mb_to_bytes;
        let required_free_space_bytes = payload_size_bytes + buffer_bytes;
        let boot_size_mb = (required_free_space_bytes / mb_to_bytes) as u32;

        telemetry!(info, "Fetching live volume parameters for {}...", disk_id);
        let partitions = disk_manager.get_partitions(&disk_id).await?;
        let target_part = partitions.iter().find(|p| p.id == partition_id).ok_or_else(|| DiskError::PartitionNotFound(partition_id.clone(), disk_id.clone()))?;

        if target_part.size_bytes <= required_free_space_bytes {
            return Err(DiskError::InsufficientSpace {
                required: (required_free_space_bytes / mb_to_bytes) as u32,
                available: (target_part.size_bytes / mb_to_bytes) as u32
            });
        }

        let target_size_bytes = target_part.size_bytes - required_free_space_bytes;
        let native_manager = NativeDiskManager::new(false);
        let is_uefi = NativeDiskManager::is_uefi_host();
        let target_letter = target_part.drive_letter.as_deref().unwrap_or("C:");
        native_manager.repair_disk(target_letter, &target_part.file_system).await?;

        telemetry!(info, "Querying NTFS driver for maximum shrink boundary on {}...", target_letter);
        let size_min_bytes = native_manager.get_max_shrink_size(target_letter).await?;

        if target_size_bytes < size_min_bytes {
            return Err(DiskError::DataValidation(format!(
                "Windows refuses to shrink the volume this much due to unmoveable system files. The smallest possible size is {} MB",
                size_min_bytes / mb_to_bytes
            )));
        }

        telemetry!(info, "Checking BitLocker status for {}...", partition_id);
        let bitlocker_state = BitLocker::get_state(target_part.drive_letter.as_deref()).await?;

        if bitlocker_state != BitLockerState::Unprotected {
            telemetry!(warn, "BITLOCKER DETECTED!");
            return Err(DiskError::DriveEncrypted("BitLocker must be disabled or suspended via the UI before starting the installation.".into()));
        }

        let bcd_backup_path = if let Some(proj_dirs) = directories::ProjectDirs::from("dev", "iNKraft", "Isekai") {
            let data_dir = proj_dirs.data_local_dir();
            tokio::fs::create_dir_all(data_dir).await.map_err(DiskError::OsError)?;
            data_dir.join("bcd.bak").to_string_lossy().to_string()
        } else {
            std::env::temp_dir().join("isekai_bcd.bak").to_string_lossy().to_string()
        };

        telemetry!(info, "Creating Windows BCD backup at {}...", bcd_backup_path);
        let bcd_export = Command::new("bcdedit.exe")
            .kill_on_drop(true)
            .no_window()
            .args(["/export", &bcd_backup_path])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        if !bcd_export.status.success() {
            let err_msg = String::from_utf8_lossy(&bcd_export.stderr);
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("Failed to backup Windows BCD. Refusing to proceed. Error: {}", err_msg)
            )));
        }

        let mut saga = SagaOrchestrator::new();
        saga.push(Compensation::RestoreBcdBackup { backup_path: bcd_backup_path.clone() });

        let destructive_phase = async {
            telemetry!(info, "Shrinking NTFS partition {} to {} bytes...", partition_id, target_size_bytes);
            disk_manager.shrink_partition(&disk_id, &partition_id, target_size_bytes).await?;

            saga.push(Compensation::ExtendSystemPartition {
                disk_id: disk_id.clone(),
                partition_id: partition_id.clone()
            });

            let payload_size_mb = if is_uefi { boot_size_mb.saturating_sub(50) } else { boot_size_mb };
            let disk_num = disk_id.parse::<u32>().map_err(|_| DiskError::DataValidation("Invalid Disk ID parameter".into()))?;

            let (payload_letter, boot_letter_opt) = match native_manager.create_live_boot_partitions(disk_num, payload_size_mb, is_uefi).await {
                Ok(letters) => letters,
                Err(e) => return Err(e),
            };

            saga.push(Compensation::DeletePartitions { disk_id: disk_num, is_uefi });

            telemetry!(info, "Cloning OS Payload: {} -> {}", iso_drive_letter, payload_letter);
            let is_hdd = native_manager.is_mechanical_drive(disk_num).await.unwrap_or(false);
            PayloadManager::copy_payload(&iso_drive_letter, &payload_letter, is_hdd).await?;

            let boot_strategy = BootManager::get_strategy(is_uefi);
            let target_bcd_drive = if is_uefi {
                boot_letter_opt.as_deref().ok_or_else(|| DiskError::DataValidation("Missing FAT32 EFI partition letter".into()))?
            } else {
                target_part.drive_letter.as_deref().unwrap_or("C:")
            };

            telemetry!(info, "Injecting boot binaries...");
            boot_strategy.inject_boot_binaries(target_bcd_drive, boot_letter_opt.as_deref()).await?;

            telemetry!(info, "Patching Windows BCD...");
            boot_strategy.patch_windows_bcd("Project Isekai", target_bcd_drive).await?;

            telemetry!(info, "Writing native boot configurations...");
            boot_strategy.write_boot_config(&payload_letter).await?;

            Ok::<(), DiskError>(())
        };

        tokio::select! {
            result = destructive_phase => {
                if let Err(e) = result {
                    saga.abort(&native_manager).await;
                    return Err(e);
                }
            }
        }

        telemetry!(info, "Cleaning up temporary backup files...");
        let _ = tokio::fs::remove_file(&bcd_backup_path).await;

        Ok(())
    }.await;

    if !is_pre_mounted {
        telemetry!(info, "Detaching Virtual ISO Image...");
        let _ = IsoManager::dismount_iso(&iso_path).await;
    }

    workflow_result
}