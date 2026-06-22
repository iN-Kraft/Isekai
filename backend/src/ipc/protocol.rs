use serde::{Deserialize, Serialize};
use crate::domain::models::{Disk, Partition};
use crate::application::state::{WorkflowType};
use crate::define_telemetry;

#[derive(Deserialize, Debug)]
#[serde(tag = "method")]
pub enum IpcProtocol {
    GetDisks,
    GetPartitions { disk_id: String },
    UnlockBitlocker { drive_letter: String },
    SuspendBitlocker { drive_letter: String },
    ShrinkInstallLocal {
        disk_id: String,
        partition_id: String,
        iso_path: String
    },
    Uninstall {
        disk_id: String
    }
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
    Disks(Vec<Disk>),
    Partitions(Vec<Partition>),
    Empty,
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

define_telemetry! {
    pub enum IPCEvent {
        #[telemetry(start, "Starting workflow: {workflow_type:?}")]
        WorkflowStarted { workflow_type: WorkflowType },

        #[telemetry(end, "Workflow completed. Success: {success}")]
        WorkflowEnded { success: bool, message: Option<String> },

        // --- UNINSTALL STEPS ---
        #[telemetry(step, "Cleaning Windows Boot Manager entries...")]
        StepCleaningBootloader,

        #[telemetry(step, "Deleting partitions and reclaiming space...")]
        StepDeletingPartitions,

        // --- INSTALL STEPS ---
        #[telemetry(step, "Mounting and verifying ISO payload...")]
        StepMountingISO,

        #[telemetry(step, "Analyzing disk space requirements...")]
        StepCalculatingSpace,

        #[telemetry(step, "Running pre-flight checks and verifying filesystem...")]
        StepPreFlightChecks,

        #[telemetry(step, "Shrinking Windows partition {partition_id}...")]
        StepShrinkingPartition { partition_id: String },

        #[telemetry(step, "Creating live boot partitions...")]
        StepCreatingBootPartitions,

        #[telemetry(step, "Cloning OS payload to new partition...")]
        StepCopyingPayload,

        #[telemetry(step, "Configuring Windows Boot Manager...")]
        StepConfiguringBootloader,

        #[telemetry(warn, "{message}")]
        Warning { message: String },

        #[telemetry(progress, "Copying Payload: {percent}%")]
        ProgressCopyingPayload { copied_bytes: u64, total_bytes: u64, percent: u8 },

        #[telemetry(info, "Hardware changes detected. Refreshing system state...")]
        SystemHardwareChanged
    }
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum OutgoingMessage {
    Response(IpcResponse),
    Event { payload: IPCEvent },
}