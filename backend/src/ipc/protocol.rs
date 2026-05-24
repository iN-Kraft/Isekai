use serde::{Deserialize, Serialize};
use crate::domain::models::{Disk, Partition};
use crate::domain::validation::ValidationReport;

#[derive(Deserialize, Debug)]
#[serde(tag = "protocol")]
pub enum IpcProtocol {
    CheckSystem,
    GetDisks,
    GetPartitions { disk_id: String },
    ShrinkPartition { disk_id: String, partition_id: String, target_size_gb: u32 },
}

#[derive(Deserialize, Debug)]
pub struct IpcRequest {
    pub id: String,
    #[serde(flatten)]
    pub payload: IpcProtocol,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ResponseData {
    Validation(ValidationReport),
    Disks(Vec<Disk>),
    Partitions(Vec<Partition>),
    Empty
}

#[derive(Serialize, Debug)]
pub struct IpcResponse {
    pub id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct IpcEvent {
    pub event_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum OutgoingMessage {
    Response(IpcResponse),
    Event(IpcEvent),
}