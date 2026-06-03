#![allow(non_snake_case)]
use serde::Deserialize;
use wmi::WMIConnection;

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
#[serde(rename_all = "PascalCase")]
pub struct Win32EncryptableVolume {
    pub ProtectionStatus: u32,
}