use serde::{Deserialize, Serialize};
use crate::domain::models::Disk;

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum IsekaiCommand {
    GetDisks,
    ShrinkPartition { disk_num: u32, size_gb: u32 }
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum IsekaiEvent {
    Progress { step: String, percent: u8 },
    FatalError { message: String },
    DisksLoaded { disks: Vec<Disk> }
}