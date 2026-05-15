use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use domain::traits::DiskManager;
use infrastructure::NativeDiskManager;
use crate::ipc::server::IpcServer;

pub mod domain;
pub mod infrastructure;
pub mod ipc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Project Isekai Daemon starting...");

    let disk_manager: Arc<dyn DiskManager> = Arc::new(NativeDiskManager::new());
    let ipc_server = IpcServer::new(disk_manager, 45454);

    ipc_server.start().await?;

    Ok(())
}
