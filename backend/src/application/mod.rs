pub mod telemetry;

use tokio::sync::mpsc::Sender;
use crate::ipc::protocol::OutgoingMessage;
use crate::ipc::state::SharedState;

#[derive(Clone)]
pub enum AppContext {
    CLI(SharedState),
    IPC(Sender<OutgoingMessage>, SharedState)
}

tokio::task_local! {
    pub static APP_CONTEXT: AppContext;
}