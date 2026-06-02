use std::sync::Arc;
use clap::Parser;
use tracing::{info, error};
use rustyline::{CompletionType, Config, Editor};
use rustyline::error::ReadlineError;
use shlex::split;
use tokio::task::block_in_place;

use crate::domain::traits::DiskManager;
use crate::domain::validation::ComponentStatus;
use crate::domain::errors::DiskError;

#[cfg(target_os = "windows")]
use crate::infrastructure::{
    NativeValidator,
    iso_manager::IsoManager, 
    payload_manager::PayloadManager, 
    boot::BootManager,
    NativeDiskManager,
    autoplay::AutoPlayGuard
};

#[cfg(target_os = "linux")]
use crate::infrastructure::NativeValidator;

use crate::cli::commands::{Commands, IsekaiCli};
use crate::cli::helper::IsekaiHelper;

pub struct CliREPL {
    pub disk_manager: Arc<dyn DiskManager>,
}

impl CliREPL {
    pub fn new(disk_manager: Arc<dyn DiskManager>) -> Self {
        Self { disk_manager }
    }

    pub async fn handle_command(&self, command: Commands) -> bool {
        match command {
            Commands::Check => {
                self.handle_check().await;
            }
            Commands::List => {
                self.handle_list().await;
            }
            Commands::Parts { disk_id } => {
                self.handle_parts(&disk_id).await;
            }
            #[cfg(target_os = "windows")]
            Commands::ShrinkAndInstall { disk_id, partition_id, iso_path, boot_size_mb } => {
                if let Err(e) = self.execute_shrink_workflow(
                    disk_id, partition_id, iso_path, boot_size_mb
                ).await {
                    error!("FATAL: Shrink-and-Install workflow failed: {}", e);
                }
            }
            Commands::Exit | Commands::Quit => {
                println!("Exiting CLI...");
                return true;
            }
        }
        false
    }

    pub async fn handle_check(&self) {
        match NativeValidator::run_checks().await {
            Ok(report) => {
                println!("{:-<60}", "");
                println!("{:<20} | {:<50} | {:<5}", "Component", "Status", "Crit");
                println!("{:-<85}", "");
                for comp in report.components {
                    let mut status_str = match comp.status {
                        ComponentStatus::Installed(path) => format!("✅ {}", path),
                        ComponentStatus::Missing => "❌ Missing".to_string(),
                    };

                    if status_str.chars().count() > 48 {
                        status_str = format!("{}...", status_str.chars().take(45).collect::<String>());
                    }

                    let crit_str = if comp.is_critical { "Yes" } else { "No" };
                    println!("{:<20} | {:<50} | {:<5}", comp.name, status_str, crit_str);
                }
                println!("{:-<85}", "");
                if report.is_ready {
                    println!("System is READY for disk operations.");
                } else {
                    println!("System is NOT READY. Missing critical components.");
                }
            }
            Err(err) => eprintln!("Failed to run checks: {}", err),
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Welcome to Project Isekai CLI.");
        println!("Type 'help' for a list of commands.");

        let config = Config::builder()
            .completion_type(CompletionType::List)
            .build();
        let mut rl = Editor::with_config(config)?;
        rl.set_helper(Some(IsekaiHelper { disk_manager: self.disk_manager.clone() }));

        loop {
            let readline = block_in_place(|| rl.readline("isekai> "));

            match readline {
                Ok(line) => {
                    let input = line.trim();
                    if input.is_empty() { continue; }

                    let _ = rl.add_history_entry(input);
                    let tokens = match split(input) {
                        Some(t) => t,
                        None => {
                            println!("Invalid input");
                            continue
                        },
                    };

                    let mut clap_args = vec!["isekai".to_string()];
                    clap_args.extend(tokens);

                    match IsekaiCli::try_parse_from(clap_args) {
                        Ok(cli) => {
                            if let Some(cmd) = cli.command {
                                if self.handle_command(cmd).await { break; }
                            } else {
                                println!("No command provided. Type 'help' for usage.");
                            }
                        }
                        Err(e) => println!("{}", e)
                    }
                }
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_list(&self) {
        match self.disk_manager.get_disks().await {
            Ok(disks) => {
                if disks.is_empty() {
                    println!("No disks found.");
                    return;
                }
                println!("{:-<85}", "");
                println!("{:<40} | {:<20} | {:<10} | {:<5}", "Stable ID", "Name", "Size (GB)", "Sys");
                println!("{:-<85}", "");
                for disk in disks {
                    let sys_flag = if disk.is_system_drive { "*" } else { " " };
                    let truncated_id = if disk.stable_id.len() > 38 {
                        format!("{}...", &disk.stable_id[..35])
                    } else {
                        disk.stable_id.clone()
                    };
                    
                    println!("{:<40} | {:<20} | {:<10} | {:<5}", 
                        truncated_id, 
                        disk.name.chars().take(20).collect::<String>(), 
                        disk.total_gb, 
                        sys_flag
                    );
                }
                println!("{:-<85}", "");
            }
            Err(e) => {
                eprintln!("Failed to get disks: {:?}", e);
            }
        }
    }

    async fn handle_parts(&self, disk_id: &str) {
        match self.disk_manager.get_partitions(disk_id).await {
            Ok(partitions) => {
                if partitions.is_empty() {
                    println!("No partitions found for disk ID: {}", disk_id);
                    return;
                }
                println!("Partitions for disk: {}", disk_id);
                println!("{:-<85}", "");
                println!("{:<40} | {:<10} | {:<10} | {:<10}", "ID", "Mount", "Size (GB)", "FS");
                println!("{:-<85}", "");
                for part in partitions {
                    let mount = part.drive_letter.as_deref().unwrap_or("-");
                    let truncated_uuid = if part.id.len() > 38 {
                        format!("{}...", &part.id[..35])
                    } else {
                        part.id.clone()
                    };

                    println!("{:<40} | {:<10} | {:<10} | {:<10}", 
                        truncated_uuid, 
                        mount, 
                        part.size_gb, 
                        part.file_system
                    );
                }
                println!("{:-<85}", "");
            }
            Err(e) => {
                eprintln!("Failed to get partitions: {:?}", e);
            }
        }
    }

    #[cfg(target_os = "windows")]
    async fn execute_shrink_workflow(
        &self,
        disk_id: String,
        partition_id: String,
        iso_path: String,
        boot_size_mb: u32,
    ) -> Result<(), DiskError> {
        let _autoplay_guard = AutoPlayGuard::new();
        let is_pre_mounted = iso_path.len() <= 3 && (iso_path.ends_with(':') || iso_path.ends_with(":\\"));
        let iso_drive_letter = if is_pre_mounted {
            let letter = iso_path.trim_end_matches('\\').to_string();
            info!("Using pre-mounted ISO on drive: {}", letter);
            letter
        } else {
            info!("Mounting ISO Payload: {}", iso_path);
            IsoManager::mount_iso(&iso_path).await?
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
            let alignment_buffer_mb = 20;
            let total_shrink_mb = boot_size_mb + alignment_buffer_mb;
            let required_free_space_bytes = (total_shrink_mb as u64) * mb_to_bytes;
            
            info!("Fetching live volume parameters for {}...", disk_id);
            let partitions = self.disk_manager.get_partitions(&disk_id).await?;
            let target_part = partitions.iter().find(|p| p.id == partition_id)
                .ok_or_else(|| DiskError::PartitionNotFound(partition_id.clone(), disk_id.clone()))?;
                
            if target_part.size_bytes <= required_free_space_bytes {
                return Err(DiskError::InsufficientSpace { 
                    required: (required_free_space_bytes / mb_to_bytes) as u32, 
                    available: (target_part.size_bytes / mb_to_bytes) as u32 
                });
            }

            let target_size_bytes = target_part.size_bytes - required_free_space_bytes;

            info!("Shrinking NTFS partition {} to {} bytes...", partition_id, target_size_bytes);
            self.disk_manager.shrink_partition(&disk_id, &partition_id, target_size_bytes).await?;
            
            info!("Recalculating Virtual Disk Offsets...");
            let refreshed_partitions = self.disk_manager.get_partitions(&disk_id).await?;
            let refreshed_target_part = refreshed_partitions.iter().find(|p| p.id == partition_id)
                .ok_or_else(|| DiskError::PartitionNotFound(partition_id.clone(), disk_id.clone()))?;
                
            let target_offset_bytes = refreshed_target_part.offset_bytes + refreshed_target_part.size_bytes;
            let disk_num = disk_id.parse::<u32>().map_err(|_| DiskError::DataValidation("Invalid Disk ID parameter".into()))?;
            let native_manager = NativeDiskManager::new(false)?;
            let is_uefi = NativeDiskManager::is_uefi_host();

            let payload_size_mb = if is_uefi {
                boot_size_mb.saturating_sub(50)
            } else {
                boot_size_mb
            };

            info!("Allocating native LINUX_LIVE & LINUX_EFI bounds at offset {}...", target_offset_bytes);
            let disk_num = disk_id.parse::<u32>().map_err(|_| DiskError::DataValidation("Invalid Disk ID parameter".into()))?;
            
            let (ntfs_letter, fat32_letter_opt) = match native_manager.create_live_boot_partitions(
                disk_num,
                target_offset_bytes,
                payload_size_mb,
                is_uefi
            ).await {
                Ok(letters) => letters,
                Err(e) => return Err(e),
            };

            let is_hdd = native_manager.is_mechanical_drive(disk_num).await.unwrap_or(false);

            let post_creation_result = async {
                info!("Cloning OS Payload: {} -> {}", iso_drive_letter, ntfs_letter);
                PayloadManager::copy_payload(&iso_drive_letter, &ntfs_letter, is_hdd).await?;

                let boot_strategy = BootManager::get_strategy(is_uefi);
                let target_bcd_drive = if is_uefi {
                    fat32_letter_opt.as_deref().unwrap()
                } else {
                    target_part.drive_letter.as_deref().unwrap_or("C:")
                };

                info!("Injecting boot binaries...");
                boot_strategy.inject_boot_binaries(target_bcd_drive, fat32_letter_opt.as_deref()).await?;

                info!("Patching Windows BCD...");
                boot_strategy.patch_windows_bcd("Project Isekai Live", target_bcd_drive).await?;

                info!("Writing native boot configurations...");
                boot_strategy.write_boot_config(&ntfs_letter).await?;
                
                Ok::<(), DiskError>(())
            }.await;

            if let Err(e) = post_creation_result {
                error!("Saga failure detected. Executing explicit diskpart rollback...");
                let _ = native_manager.rollback_live_partitions(disk_num, is_uefi).await;
                return Err(e);
            }

            Ok(())
        }.await;

        if !is_pre_mounted {
            info!("Detaching Virtual ISO Image...");
            let _ = IsoManager::dismount_iso(&iso_path).await;
        }

        workflow_result
    }
}