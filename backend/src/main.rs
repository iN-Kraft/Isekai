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
    #[cfg(windows)]
    let _ = try_enable_vt_processing();

    use clap::Parser;
    let cli = cli::commands::IsekaiCli::parse();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            let level = if cli.debug { "debug" } else { "info" };
            tracing_subscriber::EnvFilter::new(format!("{},rustyline=warn,wmi=warn", level))
        });

    let is_daemon = !cli.cli && cli.command.is_none();

    let builder = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .compact();

    if is_daemon {
        builder.init();
    } else {
        builder.without_time().init();
    }

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
        let ipc_server = IpcServer::new(disk_manager, 45454);
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
