use std::borrow::Cow;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::sync::LazyLock;
use async_trait::async_trait;
use regex::{Captures, Regex};
use tokio::fs;
use crate::domain::errors::DiskError;
use crate::infrastructure::assets::{BOOT_X64_EFI, COMMAND_NO_WINDOW, EXFAT_X64_EFI, NTFS_X64_EFI};
use crate::infrastructure::windows::boot::BootStrategy;
use crate::telemetry;

static BOOT_CONFIG_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| vec![
    Regex::new(r"(?m)(root=live:(?:CD)?LABEL=)[^\s\\]+").unwrap(),
    Regex::new(r"(?m)(set\s+isolabel=)[^\s]+").unwrap(),
    Regex::new(r"(?m)(CDLABEL=)[^\s\\]+").unwrap(),
    Regex::new(r"(?m)(archiso(?:search)?label=)[^\s\\]+").unwrap(),
    Regex::new(r"(?m)(archisodevice=/dev/disk/by-label/)[^\s\\]+").unwrap(),
    Regex::new(r"(?m)(search\s+[^\r\n]*?--(?:label|fs-label|-l)\s+[']?)[^'\s]+([']?)").unwrap(),
    Regex::new(r"(?m)(search\s+[^\r\n]*?--(?:label|fs-label|-l)\s+[`]?)[^`\s]+([`]?)").unwrap(),
    Regex::new(r"(?m)(search\s+[^\r\n]*?--(?:label|fs-label|-l)\s+[\x22]?)[^\x22\s]+([\x22]?)").unwrap(),
]);

pub struct UefiBootManager;

#[async_trait]
impl BootStrategy for UefiBootManager {
    async fn inject_boot_binaries(&self, _os_drive: &str, efi_drive: Option<&str>) -> Result<(), DiskError> {
        let efi_letter = efi_drive.unwrap();
        let efi_boot_dir = format!("{}:\\EFI\\Boot", efi_letter);
        let rufus_driver_dir = format!("{}:\\EFI\\Rufus", efi_letter);

        fs::create_dir_all(&efi_boot_dir).await.map_err(DiskError::OsError)?;
        fs::create_dir_all(&rufus_driver_dir).await.map_err(DiskError::OsError)?;

        let boot_path = Path::new(&efi_boot_dir).join("bootx64.efi");
        fs::write(&boot_path, BOOT_X64_EFI).await.map_err(DiskError::OsError)?;

        let ntfs_path = Path::new(&rufus_driver_dir).join("ntfs_x64.efi");
        fs::write(&ntfs_path, NTFS_X64_EFI).await.map_err(DiskError::OsError)?;

        let exfat_path = Path::new(&rufus_driver_dir).join("exfat_x64.efi");
        fs::write(&exfat_path, EXFAT_X64_EFI).await.map_err(DiskError::OsError)?;

        telemetry!(info, "Embedded UEFI drivers successfully written to {}", efi_letter);
        Ok(())
    }

    async fn patch_windows_bcd(&self, distro_name: &str, efi_drive: &str) -> Result<(), DiskError> {
        let efi_path = "\\EFI\\Boot\\bootx64.efi";
        let copy_out = tokio::process::Command::new("bcdedit.exe")
            .kill_on_drop(true)
            .creation_flags(COMMAND_NO_WINDOW)
            .args(["/copy", "{bootmgr}", "/d", distro_name])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        let copy_str = String::from_utf8_lossy(&copy_out.stdout);

        let guid = if let (Some(start), Some(end)) = (copy_str.find('{'), copy_str.find('}')) {
            &copy_str[start..=end]
        } else {
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("Failed to parse GUID from bcdedit output: {}", copy_str)
            )));
        };

        telemetry!(info, "Created new EFI boot entry: {}", guid);

        let inherited_props = [
            "default", "displayorder", "toolsdisplayorder", "timeout", "resumeobject", "inherit", "locale"
        ];

        for prop in inherited_props {
            let _ = tokio::process::Command::new("bcdedit.exe")
                .kill_on_drop(true)
                .creation_flags(COMMAND_NO_WINDOW)
                .args(["/deletevalue", guid, prop])
                .output()
                .await;
        }

        telemetry!(info, "Setting device=partition={} path={}", efi_drive, efi_path);

        let run_cmd = |args: Vec<String>| async move {
            let out = tokio::process::Command::new("bcdedit.exe")
                .kill_on_drop(true)
                .creation_flags(COMMAND_NO_WINDOW)
                .args(&args)
                .output()
                .await
                .map_err(DiskError::OsError)?;

            if !out.status.success() {
                Err(DiskError::OsError(Error::new(
                    ErrorKind::Other,
                    format!("bcdedit {:?} failed with code {:?}", args, out.status.code())
                )))
            } else {
                Ok(())
            }
        };

        let device_arg = format!("partition={}", efi_drive);
        let guid_str = guid.to_string();
        let distro_str = distro_name.to_string();
        let efi_str = efi_path.to_string();

        let config_result = async {
            run_cmd(vec!["/set".to_string(), guid_str.clone(), "device".to_string(), device_arg]).await?;
            run_cmd(vec!["/set".to_string(), guid_str.clone(), "path".to_string(), efi_str]).await?;
            run_cmd(vec!["/set".to_string(), guid_str.clone(), "description".to_string(), distro_str]).await?;

            run_cmd(vec!["/set".to_string(), "{fwbootmgr}".to_string(), "displayorder".to_string(), guid_str.clone(), "/addfirst".to_string()]).await?;
            run_cmd(vec!["/set".to_string(), "{fwbootmgr}".to_string(), "default".to_string(), guid_str]).await?;

            Ok::<(), DiskError>(())
        }.await;

        if let Err(e) = config_result {
            telemetry!(error, "Error configuration boot entry: {}. Rolling back...", e);
            let _ = tokio::process::Command::new("bcdedit.exe")
                .kill_on_drop(true)
                .creation_flags(COMMAND_NO_WINDOW)
                .args(["/delete", guid])
                .output()
                .await;

            return Err(e);
        }

        telemetry!(info, "UEFI boot entry created and set as default!");
        Ok(())
    }

    async fn write_boot_config(&self, payload_drive: &str) -> Result<(), DiskError> {
        Self::patch_boot_configs(payload_drive, "LINUX_LIVE").await?;
        Ok(())
    }
}

impl UefiBootManager {
    async fn patch_boot_configs(
        target_drive_letter: &str,
        new_label: &str
    ) -> Result<u32, DiskError> {
        let base_path = format!("{}:\\", target_drive_letter.trim_end_matches(':'));
        let search_paths = vec![
            "EFI\\BOOT\\grub.cfg",
            "EFI\\BOOT\\BOOT.conf",
            "boot\\grub2\\grub.cfg",
            "boot\\grub\\grub.cfg",
            "isolinux\\isolinux.cfg",
            "isolinux\\grub.conf",
            "syslinux\\syslinux.cfg",
            "syslinux\\archiso_sys-linux.cfg",
            "syslinux\\archiso_pxe-linux.cfg",
            "syslinux\\archiso_sys.cfg",
            "syslinux\\archiso_pxe.cfg",
        ];

        let mut config_files = Vec::new();

        for path in search_paths {
            let full_path = Path::new(&base_path).join(path);
            if full_path.exists() {
                config_files.push(full_path);
            }
        }

        let loader_entries_dir = Path::new(&base_path).join("loader\\entries");
        if loader_entries_dir.exists() {
            let mut entries = fs::read_dir(loader_entries_dir).await.map_err(DiskError::OsError)?;
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("conf") {
                    config_files.push(path);
                }
            }
        }

        if config_files.is_empty() {
            telemetry!(warn, "Warning: No boot config files found to patch. This ISO might use an unknown bootloader.");
            return Ok(0);
        }

        let mut patched_count = 0;

        for file_path in config_files {
            let original_content = match fs::read_to_string(&file_path).await {
                Ok(c) => c,
                Err(_) => {
                    telemetry!(warn, "Warning: Could not read {:?} (might be a binary or locked)", file_path.file_name().unwrap());
                    continue;
                }
            };

            let mut current_content = original_content;
            let mut file_was_patched = false;

            for regex in BOOT_CONFIG_PATTERNS.iter() {
                if let Cow::Owned(new_string) = regex.replace_all(&current_content, |caps: &Captures| {
                    let prefix = caps.get(1).map_or("", |m| m.as_str());
                    let suffix = caps.get(2).map_or("", |m| m.as_str());
                    format!("{}{}{}", prefix, new_label, suffix)
                }) {
                    current_content = new_string;
                    file_was_patched = true;
                }
            }

            if file_was_patched {
                if let Err(e) = fs::write(&file_path, &current_content).await {
                    telemetry!(warn, "Warning: Failed to save patched config {:?} - {}", file_path.file_name().unwrap(), e);
                } else {
                    telemetry!(info, "Patched boot config: {:?}", file_path.file_name().unwrap());
                    patched_count += 1;
                }
            }
        }

        telemetry!(info, "Successfully patched {} boot config file(s) with label '{}'", patched_count, new_label);
        Ok(patched_count)
    }
}