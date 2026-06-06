use std::sync::Arc;
use domain::traits::DiskManager;
use infrastructure::NativeDiskManager;
use crate::ipc::server::IpcServer;

pub mod cli;
pub mod domain;
pub mod infrastructure;
pub mod ipc;
pub mod application;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    tracing_subscriber::fmt::init();
    
    let cli = cli::commands::IsekaiCli::parse();

    if cli.debug {
        println!("Debug mode enabled.");
    }

    let disk_manager: Arc<dyn DiskManager> = Arc::new(NativeDiskManager::new(cli.debug));

    if let Some(command) = cli.command {
        let repl = cli::repl::CliREPL::new(disk_manager);
        repl.handle_command(command).await;
    } else if cli.cli {
        let repl = cli::repl::CliREPL::new(disk_manager);
        repl.start().await?;
    } else {
        println!("Project Isekai Daemon starting...");
        let ipc_server = IpcServer::new(disk_manager, 45454);
        ipc_server.start().await?;
    }

    Ok(())
}
