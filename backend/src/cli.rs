use std::io::Write;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use crate::domain::traits::DiskManager;

pub struct CliREPL {
    disk_manager: Arc<dyn DiskManager>,
}

impl CliREPL {
    pub fn new(disk_manager: Arc<dyn DiskManager>) -> Self {
        Self { disk_manager }
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

            let mut parts = input.split_whitespace();
            let command = parts.next().unwrap_or("");

            match command {
                "help" => {
                    println!("Available commands:");
                    println!("  list                          - List all physical disks");
                    println!("  parts <disk_id>               - List partitions on a specific disk");
                    println!("  shrink <disk_id> <part_id> <gb> - Shrink a partition to target GB");
                    println!("  exit | quit                   - Exit the CLI");
                }
                "list" => {
                    self.handle_list().await;
                }
                "parts" => {
                    let disk_id = parts.next();
                    if let Some(id) = disk_id {
                        self.handle_parts(id).await;
                    } else {
                        println!("Usage: parts <disk_id>");
                    }
                }
                "shrink" => {
                    let disk_id = parts.next();
                    let part_id = parts.next();
                    let gb = parts.next();

                    if let (Some(d_id), Some(p_id), Some(gb_str)) = (disk_id, part_id, gb) {
                        if let Ok(gb_val) = gb_str.parse::<u32>() {
                            self.handle_shrink(d_id, p_id, gb_val).await;
                        } else {
                            println!("Error: <gb> must be an integer.");
                        }
                    } else {
                        println!("Usage: shrink <disk_id> <part_id> <gb>");
                    }
                }
                "exit" | "quit" => {
                    println!("Exiting CLI...");
                    break;
                }
                _ => {
                    println!("Unknown command: '{}'. Type 'help' for usage.", command);
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
