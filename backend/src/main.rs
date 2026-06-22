use std::sync::Arc;
use domain::traits::DiskManager;
use infrastructure::NativeDiskManager;
use crate::infrastructure::logger::Logger;
use crate::ipc::server::IpcServer;
use crate::ipc::server::PIPE_NAME;

pub mod cli;
pub mod domain;
pub mod infrastructure;
pub mod ipc;
pub mod application;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let _ = try_enable_vt_processing();

    use clap::Parser;
    let cli = cli::commands::IsekaiCli::parse();
    let is_daemon = !cli.cli && cli.command.is_none();
    let _log_guard = Logger::init(cli.debug, is_daemon);

    if cli.debug {
        tracing::debug!("Debug mode enabled.");
    }

    let disk_manager: Arc<dyn DiskManager> = Arc::new(NativeDiskManager::new(cli.debug));

    if let Some(command) = cli.command {
        let repl = cli::repl::CliREPL::new(disk_manager);
        repl.handle_command(command).await;
    } else if cli.cli {
        let repl = cli::repl::CliREPL::new(disk_manager);
        repl.start().await?;
    } else {
        tracing::info!("Project Isekai Daemon starting...");
        let ipc_server = IpcServer::new(disk_manager, PIPE_NAME);
        ipc_server.start().await?;
    }

    Ok(())
}

#[cfg(windows)]
fn try_enable_vt_processing() -> Result<(), ()> {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };

    unsafe {
        for handle_id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let handle = GetStdHandle(handle_id);
            if handle.is_null() || handle == -1isize as *mut _ {
                continue;
            }

            let mut mode = 0;
            if GetConsoleMode(handle, &mut mode) != 0 {
                mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                let _ = SetConsoleMode(handle, mode);
            }
        }
    }
    Ok(())
}
