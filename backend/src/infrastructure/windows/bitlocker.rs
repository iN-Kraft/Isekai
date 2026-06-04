use std::io::{Error, ErrorKind};
use crate::domain::errors::DiskError;
use crate::infrastructure::windows::wmi::{BitLockerState, Win32EncryptableVolume};
use wmi::WMIConnection;

pub struct BitLocker;

impl BitLocker {
    pub async fn get_state(drive_letter: Option<&str>) -> Result<BitLockerState, DiskError> {
        let letter = match drive_letter {
            Some(l) => l.trim_end_matches('\\'),
            None => return Ok(BitLockerState::Unprotected)
        };

        let target_drive = letter.to_string();
        let state = tokio::task::spawn_blocking(move || -> Result<BitLockerState, DiskError> {
            let wmi_con = WMIConnection::with_namespace_path("ROOT\\CIMV2\\Security\\MicrosoftVolumeEncryption")
                .map_err(|e| DiskError::WmiError(format!("Failed to connect to BitLocker WMI: {}", e)))?;

            let query = format!("SELECT ProtectionStatus, LockStatus FROM Win32_EncryptableVolume WHERE DriveLetter = '{}'", target_drive);
            let results: Vec<Win32EncryptableVolume> = wmi_con.raw_query(&query).map_err(|e| DiskError::WmiError(format!("BitLocker WMI query failed: {}", e)))?;

            if let Some(vol) = results.first() {
                if vol.LockStatus == Some(1) {
                    return Ok(BitLockerState::Locked);
                }

                if vol.ProtectionStatus == 1 {
                    return Ok(BitLockerState::Protected);
                }
            }

            Ok(BitLockerState::Unprotected)
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        Ok(state)
    }

    pub async fn prompt_unlock(drive_letter: &str) -> Result<(), DiskError> {
        let letter = drive_letter.trim_end_matches('\\');
        let mut child = tokio::process::Command::new("bdeunlock.exe")
            .kill_on_drop(true)
            .arg(letter)
            .spawn()
            .map_err(DiskError::OsError)?;

        let _ = child.wait().await;
        let new_state = Self::get_state(Some(letter)).await?;

        if new_state == BitLockerState::Locked {
            return Err(DiskError::DriveEncrypted(format!("User aborted unlock for drive: {}", letter)));
        }

        Ok(())
    }

    pub async fn suspend(drive_letter: &str) -> Result<(), DiskError> {
        let letter = drive_letter.trim_end_matches('\\');
        let output = tokio::process::Command::new("manage-bde.exe")
            .kill_on_drop(true)
            .args(["-protectors", "-disable", letter, "-RebootCount", "1"])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        if !output.status.success() {
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                "Failed to suspend BitLocker protection."
            )));
        }

        Ok(())
    }
}