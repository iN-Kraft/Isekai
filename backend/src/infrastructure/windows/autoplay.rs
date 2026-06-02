use windows_registry::CURRENT_USER;

pub struct AutoPlayGuard {
    original_value: Option<u32>
}

impl AutoPlayGuard {
    pub fn new() -> Self {
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer";
        let mut original_value = None;

        if let Ok(key) = CURRENT_USER.create(path) {
            if let Ok(val) = key.get_u32("NoDriveTypeAutoRun") {
                original_value = Some(val);
            }

            let _ = key.set_u32("NoDriveTypeAutoRun", 255);
        }

        Self { original_value }
    }
}

impl Drop for AutoPlayGuard {
    fn drop(&mut self) {
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer";

        if let Ok(key) = CURRENT_USER.create(path) {
            if let Some(val) = self.original_value {
                let _ = key.set_u32("NoDriveTypeAutoRun", val);
            } else {
                let _ = key.remove_value("NoDriveTypeAutoRun");
            }
        }
    }
}