pub mod telemetry;
pub mod state;

use tokio::sync::mpsc::Sender;
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