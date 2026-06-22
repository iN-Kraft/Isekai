use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{debug, trace};
use crate::domain::traits::DiskManager;
use crate::infrastructure::windows::bitlocker::BitLocker;
use crate::ipc::protocol::{IPCRequest, IPCResponse, OutgoingMessage, ResponseData};
use crate::application::state::{WorkflowGuard, WorkflowType};
use crate::application::workflow::shrink_install_local::ShrinkInstallWorkflow;
use crate::application::workflow::uninstall::{UninstallWorkflow};
use crate::application::workflow::WorkflowRunner;
use crate::domain::errors::DiskError;
use crate::infrastructure::network::NetworkManager;
use crate::infrastructure::windows::boot::BootManager;
use crate::telemetry;

pub async fn process_request(
    req: IPCRequest,
    disk_manager: Arc<dyn DiskManager>,
    tx: Sender<OutgoingMessage>
) {
    debug!("Processing Request: {:?}", req);
    let response = match req {
        IPCRequest::GetDisks { id } => {
            match disk_manager.get_disks().await {
                Ok(disks) => IPCResponse {
                    id: id.clone(),
                    success: true,
                    data: Some(ResponseData::Disks(disks)),
                    error: None,
                },
                Err(e) => build_error(&id, e.to_string()),
            }
        }

        IPCRequest::GetPartitions { id, disk_id } => {
            match disk_manager.get_partitions(&disk_id).await {
                Ok(parts) => IPCResponse {
                    id: id.clone(),
                    success: true,
                    data: Some(ResponseData::Partitions(parts)),
                    error: None,
                },
                Err(e) => build_error(&id, e.to_string()),
            }
        }

        IPCRequest::UnlockBitlocker { id, drive_letter } => {
            match BitLocker::prompt_unlock(&drive_letter).await {
                Ok(_) => {
                    IPCResponse {
                        id: id.clone(),
                        success: true,
                        data: Some(ResponseData::Empty),
                        error: None
                    }
                }
                Err(e) => build_error(&id, e.to_string())
            }
        }

        IPCRequest::SuspendBitlocker { id, drive_letter } => {
            match BitLocker::suspend(&drive_letter).await {
                Ok(_) => {
                    IPCResponse {
                        id: id.clone(),
                        success: true,
                        data: Some(ResponseData::Empty),
                        error: None
                    }
                }
                Err(e) => build_error(&id, e.to_string())
            }
        }

        IPCRequest::GetDistroInfo { id } => {
            let info_map = NetworkManager::get_public_config().await;

            IPCResponse {
                id: id.clone(),
                success: true,
                data: Some(ResponseData::DistroInfo(info_map)),
                error: None
            }
        }

        // --- LONG RUNNING WORKFLOWS --- \\

        IPCRequest::ShrinkInstallLocal { id, disk_id, partition_id, iso_path } => {
            let workflow = ShrinkInstallWorkflow {
                disk_manager: disk_manager.clone(),
                disk_id,
                partition_id,
                iso_path
            };

            match WorkflowRunner::run(workflow).await {
                Ok(_) => {
                    IPCResponse {
                        id: id.clone(),
                        success: true,
                        data: Some(ResponseData::Empty),
                        error: None
                    }
                }
                Err(e) => build_error(&id, e.to_string())
            }
        }

        IPCRequest::Uninstall { id, disk_id } => {
            let workflow = UninstallWorkflow {
                disk_manager: disk_manager.clone(),
                disk_id
            };

            match WorkflowRunner::run(workflow).await {
                Ok(_) => {
                    IPCResponse {
                        id: id.clone(),
                        success: true,
                        data: Some(ResponseData::Empty),
                        error: None
                    }
                }
                Err(e) => build_error(&id, e.to_string())
            }
        }
    };

    let _ = tx.send(OutgoingMessage::Response(response)).await;
}

fn build_error(id: &str, error_msg: String) -> IPCResponse {
    IPCResponse {
        id: id.to_string(),
        success: false,
        data: None,
        error: Some(error_msg),
    }
}