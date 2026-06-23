use std::sync::Arc;
use async_trait::async_trait;
use tokio::fs::remove_file;
use tokio::sync::watch;
use tracing::info;
use crate::application::state::WorkflowType;
use crate::application::workflow::ExecutableWorkflow;
use crate::application::workflow::shrink_install_local::ShrinkInstallWorkflow;
use crate::domain::errors::DiskError;
use crate::domain::models::WorkflowState;
use crate::domain::traits::DiskManager;
use crate::infrastructure::network::NetworkManager;

pub struct ShrinkInstallRemoteWorkflow {
    pub disk_manager: Arc<dyn DiskManager>,
    pub disk_id: String,
    pub partition_id: String,
    pub distro_id: String,
    pub state_rx: watch::Receiver<WorkflowState>
}

#[async_trait]
impl ExecutableWorkflow for ShrinkInstallRemoteWorkflow {
    fn workflow_type(&self) -> WorkflowType {
        WorkflowType::ShrinkAndInstall
    }

    async fn execute(&self) -> Result<(), DiskError> {
        let (url, checksum, dest_path) = NetworkManager::prepare_download(&self.distro_id).await?;
        NetworkManager::download_and_verify(&url, &dest_path, &checksum, self.state_rx.clone()).await?;

        info!("Download complete! Transitioning to disk installation phase...");

        let local_workflow = ShrinkInstallWorkflow {
            disk_manager: self.disk_manager.clone(),
            disk_id: self.disk_id.clone(),
            partition_id: self.partition_id.clone(),
            iso_path: dest_path.to_string_lossy().to_string(),
            state_rx: self.state_rx.clone()
        };
        let result = local_workflow.execute().await;
        let _ = remove_file(&dest_path).await;

        result
    }
}