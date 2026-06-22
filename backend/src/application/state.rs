use serde::Serialize;
use crate::telemetry;
use crate::ipc::protocol::IPCEvent;

#[derive(Clone, Debug, Serialize, PartialEq, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    ShrinkAndInstall,
    DownloadIso,
    WipeDisk,
    Uninstall
}

pub struct WorkflowGuard {
    is_completed: bool,
}

impl WorkflowGuard {
    pub fn start(workflow: WorkflowType) -> Self {
        telemetry!(IPCEvent::WorkflowStarted { workflow_type: workflow });
        Self { is_completed: false }
    }

    pub fn finish(mut self, success: bool, message: Option<String>) {
        self.is_completed = true;
        telemetry!(IPCEvent::WorkflowEnded { success, message });
    }
}

impl Drop for WorkflowGuard {
    fn drop(&mut self) {
        if !self.is_completed {
            telemetry!(IPCEvent::WorkflowEnded {
                success: false,
                message: Some("Workflow was unexpectedly terminated or panicked.".to_string())
            });
        }
    }
}