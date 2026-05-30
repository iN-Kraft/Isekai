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
    pub id: String,
    pub drive_letter: Option<String>,
    pub size_gb: u32,
    pub file_system: String,
    pub label: String,
    pub offset_bytes: u64,
    pub size_bytes: u64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstallPlan {
    pub has_boot_space: bool,
    pub has_requested_linux_space: bool,
    pub boot_partition_offset_bytes: u64,
    pub linux_space_bytes: u64
}
