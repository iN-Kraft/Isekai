use std::env::temp_dir;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use crate::application::spawn_blocking_with_context;
use crate::domain::errors::DiskError;
use crate::infrastructure::CommandExt;

struct TempFileGuard(PathBuf);
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let path = self.0.clone();

        spawn_blocking_with_context(move || {
            let _ = std::fs::remove_file(&path);
        });
    }
}

pub async fn run_diskpart_script(script_content: &str, identifier: String) -> Result<(), DiskError> {
    let temp_dir = temp_dir();
    let script_path = temp_dir.join(format!("dp_{}.txt", identifier));
    let _guard = TempFileGuard(script_path.clone());

    tokio::fs::write(&script_path, script_content).await.map_err(DiskError::OsError)?;

    let script_path_str = script_path.to_str().ok_or_else(|| {
        DiskError::OsError(Error::new(ErrorKind::InvalidData, "Temp path contains invalid UTF-8"))
    })?;

    let output = tokio::process::Command::new("diskpart")
        .kill_on_drop(true)
        .no_window()
        .args(["/s", script_path_str])
        .output()
        .await
        .map_err(DiskError::OsError)?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        return Err(DiskError::OsError(Error::new(
            ErrorKind::Other,
            format!("DiskPart execution failed: {}\nError: {}", stdout, stderr)
        )));
    }

    Ok(())
}