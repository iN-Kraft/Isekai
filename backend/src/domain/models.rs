use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Disk {
    pub stable_id: String,
    pub name: String,
    pub total_gb: u32,
    pub free_gb: u32,
    pub is_system_drive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Partition {
    pub uuid: String,
    pub drive_letter: Option<String>,
    pub size_gb: u32,
    pub file_system: String,
}
