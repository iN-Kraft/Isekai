use std::io::Write;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use crate::domain::traits::DiskManager;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct IsekaiCli {
    /// Enable debug mode (shows virtual drives/loops)
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Enter interactive REPL mode
    #[arg(short, long)]
    pub cli: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List all available physical disks
    List,
    /// List partitions for a specific disk
    Parts {
        /// The Hardware ID of the disk
        disk_id: String,
    },
    /// Shrink a partition on a disk
    Shrink {
        /// The Hardware ID of the disk
        disk_id: String,
        /// The ID of the partition
        partition_id: String,
        /// Target size in GB
        target_size_gb: u32,
    },
    /// Exit the CLI
    Exit,
    /// Exit the CLI
    Quit,
}

pub struct CliREPL {
    disk_manager: Arc<dyn DiskManager>,
}

impl CliREPL {
    pub fn new(disk_manager: Arc<dyn DiskManager>) -> Self {
        Self { disk_manager }
    }

    pub async fn handle_command(&self, command: Commands) -> bool {
        match command {
            Commands::List => {
                self.handle_list().await;
            }
            Commands::Parts { disk_id } => {
                self.handle_parts(&disk_id).await;
            }
            Commands::Shrink { disk_id, partition_id, target_size_gb } => {
                self.handle_shrink(&disk_id, &partition_id, target_size_gb).await;
            }
            Commands::Exit | Commands::Quit => {
                println!("Exiting CLI...");
                return true;
            }
        }
        false
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Welcome to Project Isekai CLI.");
        println!("Type 'help' for a list of commands.");

        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            print!("isekai> ");
            std::io::stdout().flush()?;

            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break; // EOF
            }

            let input = line.trim();
            if input.is_empty() {
                continue;
            }

            let tokens = match shlex::split(input) {
                Some(t) => t,
                None => {
                    println!("Error: Invalid quoting in input.");
                    continue;
                }
            };

            // We prepend a dummy executable name because clap expects it
            let mut clap_args = vec!["isekai".to_string()];
            clap_args.extend(tokens);

            match IsekaiCli::try_parse_from(clap_args) {
                Ok(cli) => {
                    if let Some(cmd) = cli.command {
                        if self.handle_command(cmd).await {
                            break;
                        }
                    } else {
                        println!("No command provided. Type 'help' for usage.");
                    }
                }
                Err(e) => {
                    println!("{}", e);
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
                    let mount = part.drive_letter.unwrap_or_else(|| "-".to_string());
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

    async fn handle_shrink(&self, disk_id: &str, part_id: &str, target_size_gb: u32) {
        println!("Attempting to shrink partition {} on disk {} to {} GB...", part_id, disk_id, target_size_gb);
        
        match self.disk_manager.shrink_partition(disk_id, part_id, target_size_gb).await {
            Ok(_) => println!("Shrink operation completed successfully."),
            Err(e) => eprintln!("Shrink operation failed: {}", e),
        }
    }
}
