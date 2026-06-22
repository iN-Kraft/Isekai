use tracing::{debug, warn};
use windows_registry::LOCAL_MACHINE;

pub struct AutoPlayGuard {
    original_value: Option<u32>
}

impl AutoPlayGuard {
    pub fn new() -> Self {
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer";
        let mut original_value = None;

        match LOCAL_MACHINE.create(path) {
            Ok(key) => {
                if let Ok(val) = key.get_u32("NoDriveTypeAutoRun") {
                    original_value = Some(val);
                }

                if let Err(e) = key.set_u32("NoDriveTypeAutoRun", 255) {
                    warn!("Failed to set NoDriveTypeAutoRun registry key: {}", e);
                } else {
                    debug!("AutoPlayGuard engaged: Suppressing Windows AutoRun.");
                }
            }
            Err(e) => warn!("Failed to open registry for AutoPlay configuration: {}", e)
        }

        Self { original_value }
    }
}

impl Drop for AutoPlayGuard {
    fn drop(&mut self) {
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer";

        if let Ok(key) = LOCAL_MACHINE.create(path) {
            if let Some(val) = self.original_value {
                let _ = key.set_u32("NoDriveTypeAutoRun", val);
                debug!("AutoPlayGuard disengaged: Restore original AutoRun policy.");
            } else {
                let _ = key.remove_value("NoDriveTypeAutoRun");
                debug!("AutoPlayGuard disengaged: Removed temporary AutoRun policy.");
            }
        }
    }
}