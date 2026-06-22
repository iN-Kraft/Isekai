use std::env::temp_dir;
use std::fs::create_dir_all;
use tracing_appender::non_blocking;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::daily;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct Logger;

impl Logger {
    pub fn init(debug_mode: bool, is_daemon: bool) -> Option<WorkerGuard> {
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| {
                let level = if debug_mode { "debug" } else { "info" };
                tracing_subscriber::EnvFilter::new(format!("{},rustyline=warn,wmi=warn", level))
            });

        let console_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .compact();

        let console_layer = if is_daemon {
            console_layer.boxed()
        } else {
            console_layer.without_time().boxed()
        };

        let log_dir = directories::ProjectDirs::from("dev", "iNKraft", "Isekai")
            .map(|dirs| dirs.data_local_dir().to_path_buf())
            .unwrap_or_else(|| temp_dir().join("Isekai"));

        let (file_layer, guard) = match create_dir_all(&log_dir) {
            Ok(_) => {
                let file_appender = daily(&log_dir, "daemon.log");
                let (non_blocking, guard) = non_blocking(file_appender);

                let layer = tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .boxed();

                (Some(layer), Some(guard))
            }
            Err(e) => {
                eprintln!("Warning: Failed to initialize file logging at {:?}. Error: {}", log_dir, e);
                (None, None)
            }
        };

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .init();

        guard
    }
}