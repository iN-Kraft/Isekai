use std::sync::Arc;
use async_trait::async_trait;
use crate::application::state::{WorkflowType};
use crate::application::workflow::ExecutableWorkflow;
use crate::domain::errors::DiskError;
use crate::domain::traits::DiskManager;
use crate::infrastructure::windows::boot::BootManager;
use crate::ipc::protocol::IPCEvent;
use crate::telemetry;

pub struct UninstallWorkflow {
    pub disk_manager: Arc<dyn DiskManager>,
    pub disk_id: String
}

#[async_trait]
impl ExecutableWorkflow for UninstallWorkflow {
    fn workflow_type(&self) -> WorkflowType {
        WorkflowType::Uninstall
    }

    async fn execute(&self) -> Result<(), DiskError> {
        telemetry!(IPCEvent::StepCleaningBootloader);
        BootManager::remove_isekai_boot_entries("Project Isekai").await?;

        telemetry!(IPCEvent::StepDeletingPartitions);
        self.disk_manager.uninstall_isekai(&self.disk_id).await?;

        Ok(())
    }
}