use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use crate::domain::traits::DiskManager;
use crate::infrastructure::NativeValidator;
use crate::infrastructure::windows::bitlocker::BitLocker;
use crate::ipc::protocol::{IpcEvent, IpcProtocol, IpcRequest, IpcResponse, OutgoingMessage, ResponseData};
use crate::application::state::{SharedState, WorkflowGuard, WorkflowType};
use crate::telemetry;

pub async fn process_request(
    req: IpcRequest,
    disk_manager: Arc<dyn DiskManager>,
    tx: Sender<OutgoingMessage>,
    state: SharedState
) {
    let response = match req.payload {
        IpcProtocol::GetState => {
            let current_state = state.read().unwrap().clone();

            IpcResponse {
                id: req.id.clone(),
                success: true,
                data: Some(ResponseData::AppState(current_state)),
                error: None
            }
        }

        IpcProtocol::CheckSystem => {
            match NativeValidator::run_checks().await {
                Ok(report) => IpcResponse {
                    id: req.id.clone(),
                    success: true,
                    data: Some(ResponseData::Validation(report)),
                    error: None,
                },
                Err(e) => build_error(&req.id, e.to_string()),
            }
        }

        IpcProtocol::GetDisks => {
            match disk_manager.get_disks().await {
                Ok(disks) => IpcResponse {
                    id: req.id.clone(),
                    success: true,
                    data: Some(ResponseData::Disks(disks)),
                    error: None,
                },
                Err(e) => build_error(&req.id, e.to_string()),
            }
        }

        IpcProtocol::GetPartitions { disk_id } => {
            match disk_manager.get_partitions(&disk_id).await {
                Ok(parts) => IpcResponse {
                    id: req.id.clone(),
                    success: true,
                    data: Some(ResponseData::Partitions(parts)),
                    error: None,
                },
                Err(e) => build_error(&req.id, e.to_string()),
            }
        }

        IpcProtocol::ShrinkPartition { disk_id, partition_id, target_size_gb } => {
            let _workflow = WorkflowGuard::start(WorkflowType::ShrinkAndInstall);
            telemetry!(step, format!("Initializing shrink for Partition {}...", partition_id));

            match disk_manager.shrink_partition(&disk_id, &partition_id, target_size_gb as u64).await {
                Ok(_) => {
                    telemetry!(info, "Shrink complete.");

                    IpcResponse {
                        id: req.id.clone(),
                        success: true,
                        data: Some(ResponseData::Empty),
                        error: None,
                    }
                }
                Err(e) => build_error(&req.id, e.to_string()),
            }
        }

        IpcProtocol::UnlockBitLocker { drive_letter } => {
            telemetry!(info, "Waiting for user to unlock BitLocker.");

            match BitLocker::prompt_unlock(&drive_letter).await {
                Ok(_) => {
                    IpcResponse {
                        id: req.id.clone(),
                        success: true,
                        data: Some(ResponseData::Empty),
                        error: None
                    }
                }
                Err(e) => build_error(&req.id, e.to_string())
            }
        }
    };

    let _ = tx.send(OutgoingMessage::Response(response)).await;
}

fn build_error(id: &str, error_msg: String) -> IpcResponse {
    IpcResponse {
        id: id.to_string(),
        success: false,
        data: None,
        error: Some(error_msg),
    }
}