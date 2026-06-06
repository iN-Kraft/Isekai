use std::sync::{Arc, RwLock};
use serde::Serialize;
use crate::telemetry;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    ShrinkAndInstall,
    DownloadIso,
    WipeDisk
}

pub struct WorkflowGuard;

impl WorkflowGuard {
    pub fn start(workflow: WorkflowType) -> Self {
        telemetry!(start, workflow);
        Self
    }
}

impl Drop for WorkflowGuard {
    fn drop(&mut self) {
        telemetry!(end);
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AppState {
    pub active_workflow: Option<WorkflowType>,
    pub current_step: Option<String>,
    pub last_message: Option<String>,
    pub progress_percent: Option<u8>
}

pub type SharedState = Arc<RwLock<AppState>>;