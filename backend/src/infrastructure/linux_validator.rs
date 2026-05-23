use crate::domain::validation::{ValidationReport, ValidationError, ComponentStatus, SystemComponent};
use which::which;

pub struct LinuxValidator;

impl LinuxValidator {
    pub async fn run_checks() -> Result<ValidationReport, ValidationError> {
        let mut components = Vec::new();
        let required_cmds = ["parted", "partprobe", "e2fsck", "resize2fs", "mount", "umount", "btrfs"];

        for cmd in required_cmds {
            let status = match which(cmd) {
                Ok(path) => ComponentStatus::Installed(path.to_string_lossy().into_owned()),
                Err(_) => ComponentStatus::Missing
            };
            components.push(SystemComponent {
                name: cmd.to_string(),
                status,
                is_critical: true
            });
        }

        let is_ready = components.iter().filter(|c| c.is_critical).all(|c| matches!(c.status, ComponentStatus::Installed(_)));

        Ok(ValidationReport {
            os_name: "Linux".to_string(),
            components,
            is_ready
        })
    }
}