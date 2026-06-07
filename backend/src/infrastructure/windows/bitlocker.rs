use std::io::{Error, ErrorKind};
use crate::domain::errors::DiskError;
use crate::infrastructure::windows::wmi::{BitLockerState, EncryptableVolume};
use wmi::{AuthLevel, Variant, WMIConnection};
use crate::application::spawn_blocking_with_context;
use crate::infrastructure::CommandExt;
use crate::telemetry;

pub struct BitLocker;

impl BitLocker {
    pub async fn get_state(drive_letter: Option<&str>) -> Result<BitLockerState, DiskError> {
        let letter = match drive_letter {
            Some(l) => l.trim_end_matches('\\'),
            None => return Ok(BitLockerState::Unprotected)
        };

        let target_drive = letter.to_string();
        let state = spawn_blocking_with_context(move || -> Result<BitLockerState, DiskError> {
            let wmi_con = WMIConnection::with_namespace_path("ROOT\\CIMV2\\Security\\MicrosoftVolumeEncryption")
                .map_err(|e| DiskError::WmiError(format!("Failed to connect to BitLocker WMI: {}", e)))?;
            wmi_con.set_proxy_blanket(AuthLevel::PktPrivacy).map_err(|e| DiskError::WmiError(format!("Failed to set proxy blanket: {}", e)))?;

            let results: Vec<EncryptableVolume> = wmi_con.query().map_err(|e| DiskError::WmiError(format!("BitLocker WMI query failed: {}", e)))?;

            let target_vol = results.iter().find(|v| {
                v.drive_letter.as_deref().is_some_and(|d| {
                    let d_clean = d.trim_end_matches(':').trim_end_matches('\\');
                    let target_clean = target_drive.trim_end_matches(':').trim_end_matches('\\');
                    d_clean.eq_ignore_ascii_case(target_clean)
                })
            });

            if let Some(vol) = target_vol {
                let method_result = wmi_con.exec_method(&vol.path, "GetLockStatus", None).map_err(|e| DiskError::WmiError(format!("Failed to execute GetLockStatus: {}", e)))?;

                if let Some(out_wrapper) = method_result {
                    let lock_variant = out_wrapper.get_property("LockStatus").map_err(|e| DiskError::WmiError(format!("Failed to parse LockStatus: {}", e)))?;
                    let lock_status: u32 = match lock_variant {
                        Variant::UI4(val) => val,
                        Variant::I4(val) => val as u32,
                        Variant::UI8(val) => val as u32,
                        Variant::I8(val) => val as u32,
                        _ => {
                            telemetry!(error, "Unexpected Variant type for LockStatus: {:?}", lock_variant);
                            0
                        }
                    };

                    if lock_status == 1 {
                        return Ok(BitLockerState::Locked);
                    }
                }

                if vol.protection_status == Some(1) {
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
            .no_window()
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
            .no_window()
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