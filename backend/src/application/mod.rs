pub mod telemetry;
pub mod state;

use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use crate::ipc::protocol::OutgoingMessage;
use state::SharedState;

#[derive(Clone)]
pub enum AppContext {
    CLI(SharedState),
    IPC(Sender<OutgoingMessage>, SharedState)
}

tokio::task_local! {
    pub static APP_CONTEXT: AppContext;
}

pub fn spawn_blocking_with_context<F, R>(f: F) -> JoinHandle<R> where F: FnOnce() -> R + Send + 'static, R: Send + 'static {
    match APP_CONTEXT.try_with(|c| c.clone()) {
        Ok(ctx) => {
            tokio::task::spawn_blocking(move || {
                APP_CONTEXT.sync_scope(ctx, f)
            })
        }
        Err(_) => {
            tokio::task::spawn_blocking(f)
        }
    }
}