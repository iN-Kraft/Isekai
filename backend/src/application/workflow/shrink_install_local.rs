use std::env::temp_dir;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use tokio::fs::{create_dir_all, remove_file};
use tokio::process::Command;
use tokio::runtime::Runtime;
use crate::application::spawn_blocking_with_context;
use crate::application::state::{WorkflowGuard, WorkflowType};
use crate::application::workflow::ExecutableWorkflow;
use crate::domain::errors::DiskError;
use crate::domain::models::Partition;
use crate::domain::traits::DiskManager;
use crate::infrastructure::{CommandExt, NativeDiskManager};
use crate::infrastructure::windows::autoplay::AutoPlayGuard;
use crate::infrastructure::windows::bitlocker::BitLocker;
use crate::infrastructure::windows::boot::BootManager;
use crate::infrastructure::windows::iso_manager::IsoManager;
use crate::infrastructure::windows::payload_manager::PayloadManager;
use crate::infrastructure::windows::saga::{Compensation, SagaOrchestrator};
use crate::infrastructure::windows::wmi::BitLockerState;
use crate::ipc::protocol::IPCEvent;
use crate::telemetry;

pub struct ShrinkInstallWorkflow {
    pub disk_manager: Arc<dyn DiskManager>,
    pub disk_id: String,
    pub partition_id: String,
    pub iso_path: String
}

#[async_trait]
impl ExecutableWorkflow for ShrinkInstallWorkflow {
    fn workflow_type(&self) -> WorkflowType {
        WorkflowType::ShrinkAndInstall
    }

    async fn execute(&self) -> Result<(), DiskError> {
        let _autoplay = AutoPlayGuard::new();
        let native_manager = NativeDiskManager::new(false);
        let mb_to_bytes = 1024_u64 * 1024;

        telemetry!(IPCEvent::StepMountingISO);
        let (iso_drive_letter, is_pre_mounted) = self.prepare_iso().await?;

        let _iso_guard = IsoDropGuard {
            iso_path: self.iso_path.clone(),
            is_pre_mounted
        };

        telemetry!(IPCEvent::StepCalculatingSpace);
        let iso_base_path = format!("{}:\\", iso_drive_letter.trim_end_matches(':'));
        let (required_bytes, boot_size_mb) = self.calculate_requirements(&iso_base_path, mb_to_bytes).await?;

        telemetry!(IPCEvent::StepPreFlightChecks);
        let partitions = self.disk_manager.get_partitions(&self.disk_id).await?;
        let target_part = partitions.iter().find(|p| p.id == self.partition_id)
            .ok_or_else(|| DiskError::PartitionNotFound(self.partition_id.clone(), self.disk_id.clone()))?;

        self.run_preflight_checks(target_part, required_bytes, mb_to_bytes, &native_manager).await?;

        let bcd_backup_path = self.backup_bcd().await?;
        let target_size_bytes = target_part.size_bytes - required_bytes;
        let destructive_result = self.perform_destructive_installation(
            &native_manager,
            target_size_bytes,
            boot_size_mb,
            &iso_drive_letter,
            target_part,
            bcd_backup_path.clone(),
        ).await;

        let _ = remove_file(&bcd_backup_path).await;
        destructive_result?;

        Ok(())
    }
}

impl ShrinkInstallWorkflow {
    async fn prepare_iso(&self) -> Result<(String, bool), DiskError> {
        let is_pre_mounted = self.iso_path.len() <= 3 && (self.iso_path.ends_with(':') || self.iso_path.ends_with(":\\"));
        let iso_drive_letter = if is_pre_mounted {
            self.iso_path.trim_end_matches('\\').to_string()
        } else {
            IsoManager::mount_iso(&self.iso_path).await?
        };

        if !IsoManager::verify_bootable_iso(&iso_drive_letter).await {
            return Err(DiskError::DataValidation("ISO is not bootable or missing EFI/boot configuration.".into()));
        }

        Ok((iso_drive_letter, is_pre_mounted))
    }

    async fn calculate_requirements(&self, iso_base_path: &str, mb_to_bytes: u64) -> Result<(u64, u32), DiskError> {
        let base_path = iso_base_path.to_string();
        let payload_size_bytes = spawn_blocking_with_context(move || {
            PayloadManager::get_dir_size(base_path)
        }).await.unwrap_or(0);

        if payload_size_bytes <= 0 {
            return Err(DiskError::DataValidation("Failed to read ISO contents or ISO is empty.".into()));
        }

        let buffer_bytes = 512 * mb_to_bytes;
        let required_free_space_bytes = payload_size_bytes + buffer_bytes;
        let boot_size_mb = (required_free_space_bytes / mb_to_bytes) as u32;

        Ok((required_free_space_bytes, boot_size_mb))
    }

    async fn run_preflight_checks(
        &self,
        target_part: &Partition,
        required_bytes: u64,
        mb_to_bytes: u64,
        native_manager: &NativeDiskManager
    ) -> Result<(), DiskError> {
        if target_part.size_bytes <= required_bytes {
            return Err(DiskError::InsufficientSpace {
                required: (required_bytes / mb_to_bytes) as u32,
                available: (target_part.size_bytes / mb_to_bytes) as u32
            });
        }

        let bitlocker_state = BitLocker::get_state(target_part.drive_letter.as_deref()).await?;
        if bitlocker_state != BitLockerState::Unprotected {
            return Err(DiskError::DriveEncrypted("BitLocker must be disabled or suspended before starting the installation.".into()));
        }

        let target_letter = target_part.drive_letter.as_deref().unwrap_or("C:");
        native_manager.repair_disk(target_letter, &target_part.file_system).await?;

        let size_min_bytes = native_manager.get_max_shrink_size(target_letter).await?;
        let target_size_bytes = target_part.size_bytes - required_bytes;

        if target_size_bytes < size_min_bytes {
            return Err(DiskError::DataValidation(format!(
                "Windows refuses to shrink the volume this much due to unmoveable system files. The smallest possible size is {}MB",
                size_min_bytes / mb_to_bytes
            )));
        }

        Ok(())
    }

    async fn backup_bcd(&self) -> Result<String, DiskError> {
        let bcd_backup_path = if let Some(proj_dirs) = directories::ProjectDirs::from("dev", "iNKraft", "Isekai") {
            let data_dir = proj_dirs.data_local_dir();
            create_dir_all(data_dir).await.map_err(DiskError::OsError)?;
            data_dir.join("bcd.bak").to_string_lossy().to_string()
        } else {
            temp_dir().join("isekai_bcd.bak").to_string_lossy().to_string()
        };

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

        Ok(bcd_backup_path)
    }

    async fn perform_destructive_installation(
        &self,
        native_manager: &NativeDiskManager,
        target_size_bytes: u64,
        boot_size_mb: u32,
        iso_drive_letter: &str,
        target_part: &Partition,
        bcd_backup_path: String
    ) -> Result<(), DiskError> {
        let mut saga = SagaOrchestrator::new();
        saga.push(Compensation::RestoreBcdBackup { backup_path: bcd_backup_path });

        let is_uefi = NativeDiskManager::is_uefi_host();
        let disk_num = self.disk_id.parse::<u32>().map_err(|_| DiskError::DataValidation("Invalid Disk ID parameter".into()))?;

        telemetry!(IPCEvent::StepShrinkingPartition { partition_id: self.partition_id.clone() });
        self.disk_manager.shrink_partition(&self.disk_id, &self.partition_id, target_size_bytes).await?;
        saga.push(Compensation::ExtendSystemPartition { disk_id: self.disk_id.clone(), partition_id: self.partition_id.clone() });

        telemetry!(IPCEvent::StepCreatingBootPartitions);
        let payload_size_mb = if is_uefi { boot_size_mb.saturating_sub(50) } else { boot_size_mb };
        let (payload_letter, boot_letter_opt) = match native_manager.create_live_boot_partitions(disk_num, payload_size_mb, is_uefi).await {
            Ok(letters) => letters,
            Err(e) => {
                saga.abort(native_manager).await;
                return Err(e);
            }
        };
        saga.push(Compensation::DeletePartitions { disk_id: disk_num, is_uefi });

        telemetry!(IPCEvent::StepCopyingPayload);
        let is_hdd = native_manager.is_mechanical_drive(disk_num).await.unwrap_or(false);
        let copy_result = PayloadManager::copy_payload(
            iso_drive_letter,
            &payload_letter,
            is_hdd,
            Some(|copied_bytes, total_bytes| {
                let percent = ((copied_bytes as f64 / total_bytes as f64) * 100.0) as u8;
                telemetry!(IPCEvent::ProgressCopyingPayload {
                    copied_bytes,
                    total_bytes,
                    percent
                });
            })
        ).await;

        if let Err(e) = copy_result {
            saga.abort(native_manager).await;
            return Err(e);
        }

        telemetry!(IPCEvent::StepConfiguringBootloader);
        let boot_strategy = BootManager::get_strategy(is_uefi);
        let target_bcd_drive = if is_uefi {
            boot_letter_opt.as_deref().ok_or_else(|| {
                DiskError::DataValidation("Missing FAT32 EFI partition letter".into())
            })?
        } else {
            target_part.drive_letter.as_deref().unwrap_or("C:")
        };

        if let Err(e) = boot_strategy.inject_boot_binaries(target_bcd_drive, boot_letter_opt.as_deref()).await {
            saga.abort(native_manager).await; return Err(e);
        }
        if let Err(e) = boot_strategy.patch_windows_bcd("Project Isekai", target_bcd_drive).await {
            saga.abort(native_manager).await; return Err(e);
        }
        if let Err(e) = boot_strategy.write_boot_config(&payload_letter).await {
            saga.abort(native_manager).await; return Err(e);
        }

        Ok(())
    }
}

struct IsoDropGuard {
    iso_path: String,
    is_pre_mounted: bool
}

impl Drop for IsoDropGuard {
    fn drop(&mut self) {
        if !self.is_pre_mounted {
            let path = self.iso_path.clone();
            spawn_blocking_with_context(move || {
                match Runtime::new() {
                    Ok(rt) => {
                        let _ = rt.block_on(IsoManager::dismount_iso(&path));
                    }
                    Err(_) => { }
                }
            });
        }
    }
}
