use std::path::Path;
use tokio::fs::read_dir;

pub enum IsoFlavor {
    Ubuntu { initrd_file: String },
    Arch { kernel_path: String, initrd_path: String },
    Fedora { kernel_path: String, initrd_path: String },
    Unknown
}

pub async fn detect_payload(payload_drive_letter: &str) -> IsoFlavor {
    let base_path = format!("{}:\\", payload_drive_letter.trim_end_matches(':'));

    let casper_dir = Path::new(&base_path).join("casper");
    if casper_dir.exists() {
        let mut initrd = "initrd".to_string();
        if casper_dir.join("initrd.lz").exists() { initrd = "initrd.lz".to_string(); }
        else if casper_dir.join("initrd.gz").exists() { initrd = "initrd.gz".to_string(); }
        return IsoFlavor::Ubuntu { initrd_file: initrd };
    }

    if Path::new(&base_path).join("LiveOS").exists() {
        let mut k_path = "/images/pxeboot/vmlinuz".to_string();
        let mut i_path = "/images/pxeboot/initrd.img".to_string();

        let possible_kernels = [
            ("boot/x86_64/loader/linux", "boot/x86_64/loader/initrd"), // <-- NEW: Bleeding Edge Fedora 44+ / SUSE
            ("images/pxeboot/vmlinuz", "images/pxeboot/initrd.img"),   // Legacy Fedora
            ("isolinux/vmlinuz", "isolinux/initrd.img"),               // Old BIOS Fedora
            ("boot/vmlinuz", "boot/initrd.img"),                       // Modern Fedora Workstation
            ("EFI/BOOT/vmlinuz", "EFI/BOOT/initrd.img")                // Modern UEFI ISOs
        ];

        for (k, i) in possible_kernels.iter() {
            if Path::new(&base_path).join(k).exists() {
                k_path = format!("/{}", k);
                i_path = format!("/{}", i);
                break;
            }
        }

        return IsoFlavor::Fedora { kernel_path: k_path, initrd_path: i_path };
    }

    if Path::new(&base_path).join("arch").exists() {
        let mut k_path = "/arch/boot/x86_64/vmlinuz-linux".to_string();
        let mut i_path = "/arch/boot/x86_64/initramfs-linux.img".to_string();

        let arch_boot_dir = Path::new(&base_path).join("arch").join("boot").join("x86_64");
        if let Ok(mut entries) = read_dir(arch_boot_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let fname = entry.file_name().into_string().unwrap_or_default();
                if fname.starts_with("vmlinuz-linux") && !fname.contains("-lts") {
                    k_path = format!("/arch/boot/x86_64/{}", fname);
                } else if fname.starts_with("initramfs-linux") && !fname.contains("-lts") && fname.ends_with(".img") {
                    i_path = format!("/arch/boot/x86_64/{}", fname);
                }
            }
        }

        let mut initrd_string = String::new();
        if Path::new(&base_path).join("arch").join("boot").join("intel-ucode.img").exists() {
            initrd_string.push_str("/arch/boot/intel-ucode.img ");
        }
        if Path::new(&base_path).join("arch").join("boot").join("amd-ucode.img").exists() {
            initrd_string.push_str("/arch/boot/amd-ucode.img ");
        }
        initrd_string.push_str(&i_path);

        return IsoFlavor::Arch { kernel_path: k_path, initrd_path: initrd_string };
    }

    IsoFlavor::Unknown
}