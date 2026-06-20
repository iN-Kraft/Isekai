use std::sync::{Arc, RwLock};
use serde::Serialize;
use crate::telemetry;

#[derive(Clone, Debug, Serialize, PartialEq, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    ShrinkAndInstall,
    DownloadIso,
    WipeDisk,
    Uninstall
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
    pub step_details: Option<String>,
    pub step_progress: Option<u8>
}

pub type SharedState = Arc<RwLock<AppState>>;