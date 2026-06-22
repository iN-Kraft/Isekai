use std::io::{stdin, stdout, Error, ErrorKind, Write};
use std::sync::Arc;
use clap::Parser;
use rustyline::{CompletionType, Config, Editor};
use rustyline::error::ReadlineError;
use shlex::split;
use tokio::task::block_in_place;

use crate::domain::traits::DiskManager;
use crate::domain::errors::DiskError;
use crate::application::{spawn_blocking_with_context, AppContext, APP_CONTEXT};
use crate::application::state::{WorkflowGuard, WorkflowType};
use std::sync::RwLock;
use tokio::process::Command;
use tracing::error;
use crate::infrastructure::{CommandExt, NativeDiskManager};

use crate::cli::commands::{Commands, IsekaiCli};
use crate::cli::helper::IsekaiHelper;
use crate::infrastructure::windows::autoplay::AutoPlayGuard;
use crate::infrastructure::windows::boot::BootManager;
use crate::infrastructure::windows::wmi::BitLockerState;
use crate::infrastructure::windows::iso_manager::IsoManager;
use crate::infrastructure::windows::payload_manager::PayloadManager;
use crate::infrastructure::windows::bitlocker::BitLocker;
use crate::infrastructure::windows::saga::{Compensation, SagaOrchestrator};
use crate::telemetry;

pub struct CliREPL {
    pub disk_manager: Arc<dyn DiskManager>,
}

impl CliREPL {
    pub fn new(disk_manager: Arc<dyn DiskManager>) -> Self {
        Self { 
            disk_manager,
        }
    }

    pub async fn handle_command(&self, command: Commands) -> bool {
        let ctx = AppContext::CLI();

        APP_CONTEXT.scope(ctx, async move {
            match command {
                Commands::List => {
                    self.handle_list().await;
                }
                Commands::Parts { disk_id } => {
                    self.handle_parts(&disk_id).await;
                }
                Commands::ShrinkAndInstall { disk_id, partition_id, iso_path } => {
                    if let Err(e) = self.execute_shrink_workflow(
                        disk_id, partition_id, iso_path
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
        }).await
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

    async fn execute_shrink_workflow(
        &self,
        disk_id: String,
        partition_id: String,
        iso_path: String,
    ) -> Result<(), DiskError> {
        todo!("Use workflow")
    }
}