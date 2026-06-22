use tokio::process::Command;
use tracing::{error, info};
use crate::infrastructure::{CommandExt, NativeDiskManager};
use crate::infrastructure::windows::diskpart::DiskPart;
use crate::telemetry;

#[derive(Debug)]
pub enum Compensation {
    DeletePartitions { disk_id: u32, is_uefi: bool },
    ExtendSystemPartition { disk_id: String, partition_id: String },
    RestoreBcdBackup { backup_path: String }
}

pub struct SagaOrchestrator {
    rollbacks: Vec<Compensation>
}

impl SagaOrchestrator {
    pub fn new() -> Self {
        Self { rollbacks: Vec::new() }
    }

    pub fn push(&mut self, action: Compensation) {
        self.rollbacks.push(action)
    }

    pub async fn abort(self, native_manager: &NativeDiskManager) {
        error!("CRITICAL: Initiating Saga Rollback...");

        for rollback in self.rollbacks.into_iter().rev() {
            match rollback {
                Compensation::DeletePartitions { disk_id, is_uefi } => {
                    info!("Saga Step: Deleting incomplete partitions...");
                    if let Err(e) = native_manager.rollback_live_partitions(disk_id, is_uefi).await {
                        error!("Failed to delete partitions during rollback: {}", e);
                    }
                }
                Compensation::ExtendSystemPartition { disk_id, partition_id } => {
                    info!("Saga Step: Extending Windows partition back to original size...");
                    let extend_script = format!(
                        "select disk {}\n\
                        select partition {}\n\
                        extend\n\
                        exit\n",
                        disk_id, partition_id
                    );

                    if let Err(e) = DiskPart::run_script(
                        &extend_script,
                        format!("extend_{}", disk_id)
                    ).await {
                        error!("Failed to extend partition during rollback: {}", e);
                    }
                }
                Compensation::RestoreBcdBackup { backup_path } => {
                    info!("Saga Step: Restoring Windows BCD from backup...");
                    let output = Command::new("bcdedit.exe")
                        .kill_on_drop(true)
                        .no_window()
                        .args(["/import", &backup_path])
                        .output()
                        .await;

                    match output {
                        Ok(out) if !out.status.success() => {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            error!("Failed to restore BCD backup: {}", stderr);
                        }
                        Err(e) => {
                            error!("Failed to execute bcdedit for restore: {}", e);
                        },
                        _ => {
                            info!("Windows BCD restored successfully.");
                            let _ = tokio::fs::remove_file(&backup_path).await;
                        }
                    }
                }
            }
        }

        info!("Saga Rollback completed.");
    }
}