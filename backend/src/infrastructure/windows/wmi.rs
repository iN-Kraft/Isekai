#![allow(non_snake_case)]
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MsftPhysicalDisk {
    pub MediaType: Option<u16>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MsftDisk {
    pub Number: u32,
    pub FriendlyName: Option<String>,
    pub Size: Option<u64>,
    pub IsSystem: Option<bool>,
    pub IsBoot: Option<bool>,
    pub BusType: Option<u16>,
    pub PartitionStyle: Option<u16>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MsftPartition {
    pub DiskNumber: u32,
    pub PartitionNumber: u32,
    pub Offset: Option<u64>,
    pub Size: Option<u64>,
    pub DriveLetter: Option<String>,
    pub GptType: Option<String>,
    pub MbrType: Option<u16>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MsftVolume {
    pub DriveLetter: Option<String>,
    pub FileSystem: Option<String>,
    pub SizeRemaining: Option<u64>,
    pub FileSystemLabel: Option<String>
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_EncryptableVolume")]
#[serde(rename_all = "PascalCase")]
pub struct EncryptableVolume {
    #[serde(rename = "__Path")]
    pub path: String,
    pub drive_letter: Option<String>,
    pub protection_status: Option<u32>,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum BitLockerState {
    Unprotected, // Safe to modify
    Locked, // Needs unlock
    Protected // Needs suspend
}