use std::error::Error;
use std::fmt::{Display, Formatter};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", content = "version")]
pub enum ComponentStatus {
    Installed(String),
    Missing
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemComponent {
    pub name: String,
    pub status: ComponentStatus,
    pub is_critical: bool
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub os_name: String,
    pub components: Vec<SystemComponent>,
    pub is_ready: bool
}

#[derive(Debug)]
pub enum ValidationError {
    CheckFailed(String)
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self { 
            Self::CheckFailed(msg) => write!(f, "System validation failed: {}", msg)
        }
    }
}

impl Error for ValidationError {}

