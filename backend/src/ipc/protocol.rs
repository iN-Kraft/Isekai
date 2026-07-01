use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::domain::models::{Disk, Partition, PublicConfig};
use crate::application::state::{WorkflowType};
use crate::define_telemetry;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum IPCRequest {
    GetDisks { id: String },
    GetPartitions { id: String, disk_id: String },
    UnlockBitlocker { id: String, drive_letter: String },
    SuspendBitlocker { id: String, drive_letter: String },
    GetDistroInfo { id: String },

    ShrinkInstallLocal {
        id: String,
        disk_id: String,
        partition_id: String,
        iso_path: String
    },
    ShrinkInstallRemote {
        id: String,
        disk_id: String,
        partition_id: String,
        distro_id: String
    },

    PauseWorkflow { id: String },
    CancelWorkflow { id: String },

    Uninstall { id: String, disk_id: String }
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum ResponseData {
    Disks(Vec<Disk>),
    Partitions(Vec<Partition>),
    DistroInfo(HashMap<String, PublicConfig>),
    Empty,
}

#[derive(Serialize, Debug)]
pub struct IPCResponse {
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
        SystemHardwareChanged,

        #[telemetry(step, "Initializing download connection...")]
        StepInitializingDownload,

        #[telemetry(progress, "Downloading: {percent}% (ETA: {eta_seconds}s)")]
        ProgressDownload { downloaded_bytes: u64, total_bytes: u64, percent: u8, eta_seconds: u64 }

    }
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum OutgoingMessage {
    Response(IPCResponse),
    Event { payload: IPCEvent },
}