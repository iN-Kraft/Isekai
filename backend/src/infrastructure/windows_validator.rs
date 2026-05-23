use tokio::task::spawn_blocking;
use crate::domain::validation::{ValidationReport, ValidationError, ComponentStatus, SystemComponent};
use which::which;
use wmi::WMIConnection;

pub struct WindowsValidator;

impl WindowsValidator {
    pub async fn run_checks() -> Result<ValidationReport, ValidationError> {
        let mut components = Vec::new();

        let ps_status = match which("powershell.exe") {
            Ok(path) => ComponentStatus::Installed(path.to_string_lossy().into_owned()),
            Err(_) => ComponentStatus::Missing
        };
        components.push(SystemComponent {
            name: "powershell.exe".to_string(),
            status: ps_status,
            is_critical: true
        });

        let wmi_status = spawn_blocking(|| {
            WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage").is_ok()
        }).await.map_err(|e| ValidationError::CheckFailed(format!("Thread pool crashed: {:?}", e)))?;

        components.push(SystemComponent {
            name: "WMI Storage API".to_string(),
            status: if wmi_status { ComponentStatus::Installed("System API".to_string()) } else { ComponentStatus::Missing },
            is_critical: true
        });

        let is_ready = components.iter().filter(|c| c.is_critical).all(|c| matches!(c.status, ComponentStatus::Installed(_)));

        Ok(ValidationReport {
            os_name: "Windows".to_string(),
            components,
            is_ready,
        })
    }
}