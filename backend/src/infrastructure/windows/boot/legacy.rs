use std::io::{Error, ErrorKind};
use std::path::Path;
use async_trait::async_trait;
use tokio::process::Command;
use crate::domain::errors::DiskError;
use crate::domain::PARTITION_LABEL_LIVE;
use crate::infrastructure::assets::{GRLDR, GRLDR_MBR};
use crate::infrastructure::CommandExt;
use crate::infrastructure::windows::boot::BootStrategy;
use crate::infrastructure::windows::boot::sniffer::{detect_payload, IsoFlavor};
use crate::telemetry;

pub struct LegacyBootManager;

#[async_trait]
impl BootStrategy for LegacyBootManager {
    async fn inject_boot_binaries(&self, os_drive: &str, _efi_drive: Option<&str>) -> Result<(), DiskError> {
        let base_path = format!("{}:\\", os_drive.trim_end_matches(':'));
        let mbr_path = Path::new(&base_path).join("grldr.mbr");
        let grldr_path = Path::new(&base_path).join("grldr");

        telemetry!(info, "Dropping Legacy BootLoader payloads to {}...", base_path);

        tokio::fs::write(&mbr_path, GRLDR_MBR).await.map_err(DiskError::OsError)?;
        tokio::fs::write(&grldr_path, GRLDR).await.map_err(DiskError::OsError)?;

        Ok(())
    }

    async fn patch_windows_bcd(&self, distro_name: &str, os_drive: &str) -> Result<(), DiskError> {
        telemetry!(info, "Patching Windows BCD for Legacy Chainloading...");

        let create_out = Command::new("bcdedit.exe")
            .kill_on_drop(true)
            .no_window()
            .args(["/create", "/d", distro_name, "/application", "bootsector"])
            .output()
            .await
            .map_err(DiskError::OsError)?;

        let create_str = String::from_utf8_lossy(&create_out.stdout);
        let guid = if let (Some(start), Some(end)) = (create_str.find('{'), create_str.find('}')) {
            &create_str[start..=end]
        } else {
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("Failed to parse GUID from bcdedit output: {}", create_str)
            )));
        };

        telemetry!(info, "Created Legacy Boot Entry: {}", guid);

        let run_cmd = |args: Vec<String>| async move {
            let out = Command::new("bcdedit.exe")
                .kill_on_drop(true)
                .no_window()
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

        let device_arg = format!("partition={}", os_drive.trim_end_matches('\\'));
        let guid_str = guid.to_string();

        let config_result = async {
            run_cmd(vec!["/set".to_string(), guid_str.clone(), "device".to_string(), device_arg]).await?;
            run_cmd(vec!["/set".to_string(), guid_str.clone(), "path".to_string(), "\\grldr.mbr".to_string()]).await?;
            run_cmd(vec!["/displayorder".to_string(), guid_str.clone(), "/addlast".to_string()]).await?;
            run_cmd(vec!["/set".to_string(), "{bootmgr}".to_string(), "displaybootmenu".to_string(), "yes".to_string()]).await?;
            run_cmd(vec!["/timeout".to_string(), "5".to_string()]).await?;

            telemetry!(info, "Disabling Windows Fast Startup...");
            let _ = Command::new("powercfg.exe")
                .kill_on_drop(true)
                .no_window()
                .args(["/h", "off"])
                .output()
                .await;

            Ok::<(), DiskError>(())
        }.await;

        if let Err(e) = config_result {
            telemetry!(error, "Error configuring BCD entry: {}. Rolling back...", e);

            let _ = Command::new("bcdedit.exe")
                .kill_on_drop(true)
                .no_window()
                .args(["/delete", guid])
                .output()
                .await;
            return Err(e);
        }

        telemetry!(info, "Legacy BCD patch successful!");
        Ok(())
    }

    async fn write_boot_config(&self, payload_drive: &str) -> Result<(), DiskError> {
        let flavor = detect_payload(payload_drive).await;
        let base_path = format!("{}:\\", payload_drive.trim_end_matches(':'));
        let menu_path = Path::new(&base_path).join("menu.lst");

        let mut menu_content = String::from(
            "default 0\n\
            timeout 5\n\
            color normal=white/black highlight=black/light-gray\n\
            \n\n"
        );

        match flavor {
            IsoFlavor::Ubuntu { initrd_file } => {
                menu_content.push_str(&format!("\
                    title Project Isekai
                    find --set-root /casper/vmlinuz
                    kernel /casper/vmlinuz boot=casper live-media-path=/casper quiet splash ---
                    initrd /casper/{}

                    title Project Isekai (Fallback)
                    find --set-root /casper/vmlinuz
                    kernel /casper/vmlinuz boot=casper live-media-path=/casper nomodeset ---
                    initrd /casper/{}
                ", initrd_file, initrd_file));
            }
            IsoFlavor::Arch { kernel_path, initrd_path } => {
                menu_content.push_str(&format!("\
                    title Project Isekai
                    find --set-root {}
                    kernel {} archisobasedir=arch archisolabel={} quiet splash
                    initrd {}

                    title Project Isekai (Fallback)
                    find --set-root {}
                    kernel {} archisobasedir=arch archisolabel={} nomodeset
                    initrd {}
                ", kernel_path, kernel_path, PARTITION_LABEL_LIVE, initrd_path, kernel_path, kernel_path, PARTITION_LABEL_LIVE, initrd_path));
            }
            IsoFlavor::Fedora { kernel_path, initrd_path } => {
                menu_content.push_str(&format!("\
                    title Project Isekai (Live Desktop Mode)
                    find --set-root {}
                    kernel {} root=live:LABEL={} rd.live.image quiet splash
                    initrd {}

                    title Project Isekai (Fallback - Live Desktop Mode)
                    find --set-root {}
                    kernel {} root=live:LABEL={} rd.live.image nomodeset
                    initrd {}

                    title Project Isekai (Installer Mode - 8GB+ RAM Recommended)
                    find --set-root {}
                    kernel {} root=live:LABEL={} rd.live.image rd.live.ram=1 quiet splash
                    initrd {}

                    title Project Isekai (Fallback - Installer Mode)
                    find --set-root {}
                    kernel {} root=live:LABEL={} rd.live.image rd.live.ram=1 nomodeset
                    initrd {}
                ", kernel_path, kernel_path, PARTITION_LABEL_LIVE, initrd_path, kernel_path, kernel_path, PARTITION_LABEL_LIVE, initrd_path, kernel_path, kernel_path, PARTITION_LABEL_LIVE, initrd_path, kernel_path, kernel_path, PARTITION_LABEL_LIVE, initrd_path));
            }
            IsoFlavor::Unknown => {
                menu_content.push_str("\
                    title Project Isekai
                    find --set-root /casper/vmlinuz
                    kernel /casper/vmlinuz boot=casper live-media-path=/casper quiet splash ---
                    initrd /casper/initrd.lz
                ");
            }
        }

        tokio::fs::write(&menu_path, menu_content).await.map_err(DiskError::OsError)?;
        Ok(())
    }
}