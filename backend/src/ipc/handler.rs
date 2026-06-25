use std::sync::{Arc, OnceLock};
use tokio::spawn;
use tokio::sync::mpsc::Sender;
use tokio::sync::{watch, Mutex};
use tracing::{debug, error, info, trace};
use crate::application::APP_CONTEXT;
use crate::domain::traits::DiskManager;
use crate::infrastructure::windows::bitlocker::BitLocker;
use crate::ipc::protocol::{IPCRequest, IPCResponse, OutgoingMessage, ResponseData};
use crate::application::state::{WorkflowGuard, WorkflowType};
use crate::application::workflow::shrink_install_local::ShrinkInstallWorkflow;
use crate::application::workflow::shrink_install_remote::ShrinkInstallRemoteWorkflow;
use crate::application::workflow::uninstall::{UninstallWorkflow};
use crate::application::workflow::WorkflowRunner;
use crate::domain::errors::DiskError;
use crate::domain::models::WorkflowState;
use crate::infrastructure::network::NetworkManager;
use crate::infrastructure::windows::boot::BootManager;
use crate::telemetry;


static ACTIVE_WORKFLOW: OnceLock<Mutex<Option<watch::Sender<WorkflowState>>>> = OnceLock::new();

fn get_active_workflow() -> &'static Mutex<Option<watch::Sender<WorkflowState>>> {
    ACTIVE_WORKFLOW.get_or_init(|| Mutex::new(None))
}
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
            let mut active_lock = get_active_workflow().lock().await;
            if active_lock.is_some() {
                build_error(&id, "A workflow is already running.".to_string())
            } else {
                let (state_tx, state_rx) = watch::channel(WorkflowState::Running);
                *active_lock = Some(state_tx);

                let workflow = ShrinkInstallWorkflow {
                    disk_manager,
                    disk_id,
                    partition_id,
                    iso_path,
                    state_rx
                };

                let result = match WorkflowRunner::run(workflow).await {
                    Ok(_) => {
                        IPCResponse { id, success: true, data: Some(ResponseData::Empty), error: None }
                    }
                    Err(e) => build_error(&id, e.to_string())
                };

                *get_active_workflow().lock().await = None;
                result
            }
        }

        IPCRequest::ShrinkInstallRemote { id, disk_id, partition_id, distro_id } => {
            let mut active_lock = get_active_workflow().lock().await;
            if active_lock.is_some() {
                build_error(&id, "An installation is already running.".to_string())
            } else {
                let (state_tx, state_rx) = watch::channel(WorkflowState::Running);
                *active_lock = Some(state_tx);

                let workflow = ShrinkInstallRemoteWorkflow {
                    disk_manager,
                    disk_id,
                    partition_id,
                    distro_id,
                    state_rx
                };

                let result = match WorkflowRunner::run(workflow).await {
                    Ok(_) => {
                        IPCResponse {
                            id,
                            success: true,
                            data: Some(ResponseData::Empty),
                            error: None
                        }
                    },
                    Err(e) => build_error(&id, e.to_string())
                };

                *get_active_workflow().lock().await = None;
                result
            }
        }

        IPCRequest::PauseWorkflow { id } => {
            if let Some(state_tx) = get_active_workflow().lock().await.as_ref() {
                let current_state = state_tx.borrow().clone();
                let new_state = if current_state == WorkflowState::Paused {
                    WorkflowState::Running
                } else {
                    WorkflowState::Paused
                };
                let _ = state_tx.send(new_state);
            }
            IPCResponse { id, success: true, data: Some(ResponseData::Empty), error: None }
        }

        IPCRequest::CancelWorkflow { id } => {
            if let Some(state_tx) = get_active_workflow().lock().await.as_ref() {
                let _ = state_tx.send(WorkflowState::Cancelled);
            }
            IPCResponse { id, success: true, data: Some(ResponseData::Empty), error: None }
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