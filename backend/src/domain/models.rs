use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Disk {
    pub disk_num: u32,
    pub name: String,
    pub total_gb: u32,
    pub free_gb: u32,
    pub is_system_drive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Partition {
    pub partition_num: u32,
    pub drive_letter: Option<String>,
    pub size_gb: u32,
    pub file_system: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_is_almost_full() {
        let disk = Disk {
            disk_num: 0,
            name: "Test Drive".to_string(),
            total_gb: 100,
            free_gb: 5,
            is_system_drive: false,
        };

        assert!(disk.free_gb < 10, "Disk should be flagged as almost full");
    }
}