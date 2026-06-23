use async_trait::async_trait;
use crate::application::state::{WorkflowGuard, WorkflowType};
use crate::domain::errors::DiskError;

pub mod shrink_install_local;
pub mod uninstall;
pub mod shrink_install_remote;

#[async_trait]
pub trait ExecutableWorkflow: Send + Sync {
    fn workflow_type(&self) -> WorkflowType;
    async fn execute(&self) -> Result<(), DiskError>;
}

pub struct WorkflowRunner;

impl WorkflowRunner {
    pub async fn run<W: ExecutableWorkflow>(workflow: W) -> Result<(), DiskError> {
        let guard = WorkflowGuard::start(workflow.workflow_type());

        match workflow.execute().await {
            Ok(_) => {
                guard.finish(true, None);
                Ok(())
            }
            Err(e) => {
                let err_msg = e.to_string();
                guard.finish(false, Some(err_msg));
                Err(e)
            }
        }
    }
}