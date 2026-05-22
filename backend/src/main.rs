use std::sync::Arc;
use domain::traits::DiskManager;
use infrastructure::NativeDiskManager;
use crate::ipc::server::IpcServer;

pub mod cli;
pub mod domain;
pub mod infrastructure;
pub mod ipc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Project Isekai Daemon starting...");

    let args: Vec<String> = std::env::args().collect();
    let use_cli = args.iter().any(|arg| arg == "--cli" || arg == "-c");
    let debug_mode = args.iter().any(|arg| arg == "--debug" || arg == "-d");

    if debug_mode {
        println!("Debug mode enabled.");
    }

    let disk_manager: Arc<dyn DiskManager> = Arc::new(NativeDiskManager::new(debug_mode));

    if use_cli {
        let repl = cli::CliREPL::new(disk_manager);
        repl.start().await?;
    } else {
        let ipc_server = IpcServer::new(disk_manager, 45454);
        ipc_server.start().await?;
    }

    Ok(())
}
