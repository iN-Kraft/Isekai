use std::borrow::Cow;
use std::io::{Error, ErrorKind};
use std::path::Path;
use regex::{Captures, Regex};
use tokio::fs;
use tokio::process::Command;
use crate::domain::errors::DiskError;
use crate::infrastructure::assets::{BOOT_X64_EFI, EXFAT_X64_EFI, GRLDR, GRLDR_MBR, NTFS_X64_EFI};

pub struct BootManager;

impl BootManager {
    pub async fn install_uefi_driver(fat32_drive_letter: &str) -> Result<(), DiskError> {
        let efi_boot_dir = format!("{}:\\EFI\\Boot", fat32_drive_letter);
        let rufus_driver_dir = format!("{}:\\EFI\\Rufus", fat32_drive_letter);

        fs::create_dir_all(&efi_boot_dir).await.map_err(DiskError::OsError)?;
        fs::create_dir_all(&rufus_driver_dir).await.map_err(DiskError::OsError)?;

        let boot_path = Path::new(&efi_boot_dir).join("bootx64.efi");
        fs::write(&boot_path, BOOT_X64_EFI).await.map_err(DiskError::OsError)?;

        let ntfs_path = Path::new(&rufus_driver_dir).join("ntfs_x64.efi");
        fs::write(&ntfs_path, NTFS_X64_EFI).await.map_err(DiskError::OsError)?;

        let exfat_path = Path::new(&rufus_driver_dir).join("exfat_x64.efi");
        fs::write(&exfat_path, EXFAT_X64_EFI).await.map_err(DiskError::OsError)?;

        println!("Embedded UEFI drivers successfully written to {}", fat32_drive_letter);
        Ok(())
    }

    pub async fn patch_boot_configs(
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
            println!("Warning: No boot config files found to patch. This ISO might use an unknown bootloader.");
            return Ok(0);
        }

        let patterns = vec![
            Regex::new(r"(?m)(root=live:(?:CD)?LABEL=)[^\s\\]+").unwrap(),
            Regex::new(r"(?m)(set\s+isolabel=)[^\s]+").unwrap(),
            Regex::new(r"(?m)(CDLABEL=)[^\s\\]+").unwrap(),

            Regex::new(r"(?m)(archiso(?:search)?label=)[^\s\\]+").unwrap(),
            Regex::new(r"(?m)(archisodevice=/dev/disk/by-label/)[^\s\\]+").unwrap(),

            Regex::new(r"(?m)(search\s+[^\r\n]*?--(?:label|fs-label|-l)\s+[']?)[^'\s]+([']?)").unwrap(),
            Regex::new(r"(?m)(search\s+[^\r\n]*?--(?:label|fs-label|-l)\s+[`]?)[^`\s]+([`]?)").unwrap(),
            Regex::new(r"(?m)(search\s+[^\r\n]*?--(?:label|fs-label|-l)\s+[\x22]?)[^\x22\s]+([\x22]?)").unwrap(),
        ];

        let mut patched_count = 0;

        for file_path in config_files {
            let original_content = match fs::read_to_string(&file_path).await {
                Ok(c) => c,
                Err(_) => {
                    println!("Warning: Could not read {:?} (might be a binary or locked)", file_path.file_name().unwrap());
                    continue;
                }
            };

            let mut current_content = original_content;
            let mut file_was_patched = false;

            for regex in &patterns {
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
                    println!("Warning: Failed to save patched config {:?} - {}", file_path.file_name().unwrap(), e);
                } else {
                    println!("Patched boot config: {:?}", file_path.file_name().unwrap());
                    patched_count += 1;
                }
            }
        }

        println!("Successfully patched {} boot config file(s) with label '{}'", patched_count, new_label);
        Ok(patched_count)
    }

    pub async fn install_legacy_chainloader(os_drive_letter: &str) -> Result<(), DiskError> {
        let base_path = format!("{}:\\", os_drive_letter.trim_end_matches(':'));
        let mbr_path = Path::new(&base_path).join("grldr.mbr");
        let grldr_path = Path::new(&base_path).join("grldr");

        println!("Dropping Legacy BootLoader payloads to {}...", base_path);

        tokio::fs::write(&mbr_path, GRLDR_MBR).await.map_err(DiskError::OsError)?;
        tokio::fs::write(&grldr_path, GRLDR).await.map_err(DiskError::OsError)?;

        Ok(())
    }

    pub async fn patch_legacy_bcd(
        distro_name: &str,
        os_drive_letter: &str
    ) -> Result<(), DiskError> {
        println!("Patching Windows BCD for Legacy Chainloading...");

        let create_out = Command::new("bcdedit.exe")
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

        println!("Created Legacy Boot Entry: {}", guid);

        let run_cmd = |args: Vec<String>| async move {
            let out = Command::new("bcdedit.exe")
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

        let device_arg = format!("partition={}", os_drive_letter.trim_end_matches('\\'));
        let guid_str = guid.to_string();

        let config_result = async {
            run_cmd(vec!["/set".to_string(), guid_str.clone(), "device".to_string(), device_arg]).await?;
            run_cmd(vec!["/set".to_string(), guid_str.clone(), "path".to_string(), "\\grldr.mbr".to_string()]).await?;
            run_cmd(vec!["/displayorder".to_string(), guid_str.clone(), "/addlast".to_string()]).await?;
            run_cmd(vec!["/set".to_string(), "{bootmgr}".to_string(), "displaybootmenu".to_string(), "yes".to_string()]).await?;
            run_cmd(vec!["/timeout".to_string(), "5".to_string()]).await?;

            println!("Disabling Windows Fast Startup...");
            let _ = Command::new("powercfg.exe")
                .args(["/h", "off"])
                .output()
                .await;

            Ok::<(), DiskError>(())
        }.await;

        if let Err(e) = config_result {
            println!("Error configuring BCD entry: {}. Rolling back...", e);

            let _ = Command::new("bcdedit.exe")
                .args(["/delete", guid])
                .output()
                .await;
            return Err(e);
        }

        println!("Legacy BCD patch successful!");
        Ok(())
    }

    pub async fn write_grub4dos_config(payload_drive_letter: &str) -> Result<(), DiskError> {
        println!("Writing GRUB4DOS menu.lst configuration...");

        let base_path = format!("{}:\\", payload_drive_letter.trim_end_matches(':'));
        let menu_path = Path::new(&base_path).join("menu.lst");

        let mut menu_content = String::from(
            "default 0\n\
            timeout 5\n\
            color normal=white/black highlight=black/light-gray\n\
            \n\n"
        );

        let casper_dir = Path::new(&base_path).join("casper");
        if casper_dir.exists() {
            println!("Detected Debian/Ubuntu based payload (casper).");

            let mut initrd_file = "initrd";
            if casper_dir.join("initrd.lz").exists() {
                initrd_file = "initrd.lz";
            } else  if casper_dir.join("initrd.gz").exists() {
                initrd_file = "initrd.gz";
            }

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
        } else if Path::new(&base_path).join("arch").exists() {
            println!("Detected Arch based payload (archiso).");

            menu_content.push_str("\
                title Project Isekai
                find --set-root /arch/boot/x86_64/vmlinuz-linux
                kernel /arch/boot/x86_64/vmlinuz-linux archisobasedir=arch archisolabel=LINUX_LIVE copytoram=y quiet splash
                initrd /arch/boot/intel-ucode.img /arch/boot/amd-ucode.img /arch/boot/x86_64/initramfs-linux.img

                title Project Isekai (Fallback)
                find --set-root /arch/boot/x86_64/vmlinuz-linux
                kernel /arch/boot/x86_64/vmlinuz-linux archisobasedir=arch archisolabel=LINUX_LIVE copytoram=y nomodeset
                initrd /arch/boot/intel-ucode.img /arch/boot/amd-ucode.img /arch/boot/x86_64/initramfs-linux.img
            ");
        } else if Path::new(&base_path).join("images").join("pxeboot").exists() {
            println!("Detected Fedora/RHEL based payload (dracut).");

            menu_content.push_str("\
                title Project Isekai
                find --set-root /images/pxeboot/vmlinuz
                kernel /images/pxeboot/vmlinuz root=live:LABEL=LINUX_LIVE rd.live.image quiet splash
                initrd /images/pxeboot/initrd.img

                title Project Isekai (Fallback)
                find --set-root /images/pxeboot/vmlinuz
                kernel /images/pxeboot/vmlinuz root=live:LABEL=LINUX_LIVE rd.live.image nomodeset
                initrd /images/pxeboot/initrd.img
            ");
        } else {
            println!("Warning: Could not automatically detect ISO type. Writing safe default...");

            menu_content.push_str("\
                title Project Isekai
                find --set-root /casper/vmlinuz
                kernel /casper/vmlinuz boot=casper live-media-path=/casper quiet splash ---
                initrd /casper/initrd.lz
            ");
        }

        tokio::fs::write(&menu_path, menu_content).await.map_err(DiskError::OsError)?;

        Ok(())
    }
}