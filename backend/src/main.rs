use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use domain::traits::DiskManager;
use infrastructure::NativeDiskManager;

pub mod domain;
pub mod infrastructure;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Project Isekai Daemon starting...");

    let disk_manager: Arc<dyn DiskManager> = Arc::new(NativeDiskManager::new());

    match disk_manager.get_disks().await {
        Ok(disks) => {
            println!("Disks found: {:?}\n", disks.len());

            for disk in disks {
                let sys_tag = if disk.is_system_drive { "[SYSTEM]" } else { "" };
                println!(
                    "Drive {} | Name: {:<15} | Size: {} GB {}",
                    disk.disk_num,
                    disk.name,
                    disk.total_gb,
                    sys_tag
                );

                match disk_manager.get_partitions(disk.disk_num).await {
                    Ok(partitions) => {
                        for part in partitions {
                            let mount = part.drive_letter.unwrap_or_else(|| "Unmounted".to_string());
                            println!(
                                "└─ Part {} | {:>4} GB | Mount: {}",
                                part.partition_num, part.size_gb, mount
                            )
                        }
                    }
                    Err(e) => {
                        println!("└─ Error reading partitions: {}", e)
                    }
                }
                println!()
            }
            println!();
        }
        Err(err) => {
            eprintln!("Error getting disks: {}\n", err);
        }
    }

    Ok(())
}
