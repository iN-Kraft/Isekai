use async_trait::async_trait;
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, Partition};
use crate::domain::traits::DiskManager;
use crate::infrastructure::blockdev::{get_devices, BlockDevice, DeviceType};

enum ShrinkStrategy {
    Offline {
        check_args: Vec<&'static str>,
        resize_args: Vec<&'static str>,
    },
    Online {
        resize_args: Vec<&'static str>,
    },
    Unsupported(String),
}

fn get_shrink_strategy(fstype: &str) -> ShrinkStrategy {
    match fstype {
        "ext2" | "ext3" | "ext4" => ShrinkStrategy::Offline {
            check_args: vec!["e2fsck", "-f", "-p", "{dev}"],
            resize_args: vec!["resize2fs", "{dev}", "{size}"],
        },
        "ntfs" => ShrinkStrategy::Offline {
            check_args: vec!["ntfsresize", "-f", "-i", "{dev}"],
            resize_args: vec!["ntfsresize", "-f", "-s", "{size}", "{dev}"],
        },
        "btrfs" => ShrinkStrategy::Online {
            resize_args: vec!["btrfs", "filesystem", "resize", "{size}", "{mnt}"],
        },
        "xfs" | "zfs_member" => ShrinkStrategy::Unsupported(
            "This filesystem fundamentally does not support shrinking.".to_string(),
        ),
        _ => ShrinkStrategy::Unsupported(
            "Shrinking not yet implemented for this filesystem.".to_string(),
        ),
    }
}

async fn execute_strategy_cmd(args: &[&'static str], dev: &str, size_gb: u32, mnt: &str) -> Result<(), DiskError> {
    if args.is_empty() { return Ok(()); }
    let cmd_name = args[0];
    let mut cmd = tokio::process::Command::new(cmd_name);
    for &arg in &args[1..] {
        let processed_arg = arg
            .replace("{dev}", dev)
            .replace("{size}", &format!("{}G", size_gb))
            .replace("{mnt}", mnt);
        cmd.arg(processed_arg);
    }
    
    let output = tokio::time::timeout(std::time::Duration::from_secs(300), cmd.output())
        .await
        .map_err(|_| DiskError::OsError(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("{} timed out after 5 minutes", cmd_name),
        )))?
        .map_err(DiskError::OsError)?;

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        if cmd_name == "e2fsck" && exit_code == 1 {
            // e2fsck corrected errors safely, acceptable exit code.
        } else {
            let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{} failed for {}: {}", cmd_name, dev, err_msg),
            )));
        }
    }
    Ok(())
}

struct MountGuard {
    pub mountpoint: String,
    needs_unmount: bool,
}

impl MountGuard {
    async fn new(device_path: &str, uuid: &str) -> Result<Self, DiskError> {
        let mountpoint = format!("/tmp/isekai_fs_{}", uuid);
        std::fs::create_dir_all(&mountpoint).map_err(DiskError::OsError)?;
        
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(300),
            tokio::process::Command::new("mount")
                .arg(device_path)
                .arg(&mountpoint)
                .output()
        ).await
        .map_err(|_| DiskError::OsError(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "mount timed out after 5 minutes",
        )))?
        .map_err(DiskError::OsError)?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let _ = std::fs::remove_dir(&mountpoint);
            return Err(DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("mount failed for {}: {}", device_path, err_msg),
            )));
        }

        Ok(Self { mountpoint, needs_unmount: true })
    }
}

impl Drop for MountGuard {
    fn drop(&mut self) {
        if self.needs_unmount {
            let _ = std::process::Command::new("umount")
                .args(["-l", &self.mountpoint])
                .output();
            let _ = std::fs::remove_dir(&self.mountpoint);
        }
    }
}

pub struct LinuxDiskManager {
    debug_mode: bool,
}

impl LinuxDiskManager {
    pub fn new(debug_mode: bool) -> Self {
        Self { debug_mode }
    }

    fn get_stable_id(device: &BlockDevice) -> Option<String> {
        device.wwn.clone()
            .or_else(|| device.serial.clone())
            .or_else(|| device.uuid.clone())
    }
}

#[async_trait]
impl DiskManager for LinuxDiskManager {
    async fn get_disks(&self) -> Result<Vec<Disk>, DiskError> {
        let block_devices = tokio::task::spawn_blocking(|| {
            get_devices().map_err(|e| DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )))
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let mut disks = Vec::new();

        for device in block_devices.iter() {
            let is_allowed_type = device.is_disk() || (self.debug_mode && device.device_type == DeviceType::Loop);
            
            if !is_allowed_type {
                continue;
            }

            let stable_id = match Self::get_stable_id(device) {
                Some(id) => id,
                None if self.debug_mode && device.device_type == DeviceType::Loop => device.name.clone(),
                None => continue, // Skip devices without a stable ID
            };

            disks.push(Disk {
                stable_id,
                name: device.name.clone(),
                total_gb: (device.size / 1024 / 1024 / 1024) as u32,
                free_gb: 0, // Placeholder
                is_system_drive: device.is_system(),
            });
        }

        Ok(disks)
    }

    async fn get_partitions(&self, disk_id: &str) -> Result<Vec<Partition>, DiskError> {
        let disk_id_owned = disk_id.to_string();
        
        let block_devices = tokio::task::spawn_blocking(|| {
            get_devices().map_err(|e| DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )))
        }).await.map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        let target_device = block_devices.iter_all()
            .filter(|d| d.is_disk() || (self.debug_mode && d.device_type == DeviceType::Loop))
            .find(|d| {
                let id = Self::get_stable_id(d).unwrap_or_else(|| {
                    if d.device_type == DeviceType::Loop {
                        d.name.clone()
                    } else {
                        "".to_string()
                    }
                });
                id == disk_id_owned
            })
            .ok_or_else(|| DiskError::DiskNotFound(disk_id_owned))?;

        let mut partitions = Vec::new();

        for child in target_device.children_iter() {
            if !child.is_partition() {
                continue;
            }

            let uuid = match &child.uuid {
                Some(u) => u.clone(),
                None => continue, // Skip partitions without UUID
            };

            partitions.push(Partition {
                id: uuid,
                drive_letter: child.active_mountpoints().first().map(|s| s.to_string()),
                size_gb: (child.size / 1024 / 1024 / 1024) as u32,
                file_system: child.fstype.clone().unwrap_or_else(|| "Unknown".to_string()),
            });
        }

        Ok(partitions)
    }

    async fn shrink_partition(&self, disk_id: &str, partition_id: &str, target_size_gb: u32) -> Result<(), DiskError> {
        let d_id = disk_id.to_string();
        let p_id = partition_id.to_string();
        let debug_mode = self.debug_mode;

        let p_id_for_closure = p_id.clone();
        // 1. The Blocking Phase: Identify Parent Disk, Child Device, & Start Sector
        let (device_path, parent_disk, fstype, active_mounts, start_sector, logical_sector_size, partn) = 
            tokio::task::spawn_blocking(move || -> Result<_, DiskError> {
                let devices = get_devices().map_err(|e| DiskError::OsError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))?;

                let mut parent_name = None;
                let mut child_name = None;
                let mut partn = None;
                let mut start_sector = None;
                let mut logical_sector_size = 512;
                let mut fstype = None;
                let mut active_mounts: Vec<String> = Vec::new();

                for disk in devices.iter_all() {
                    let id = Self::get_stable_id(disk).unwrap_or_else(|| {
                        if debug_mode && disk.device_type == DeviceType::Loop {
                            disk.name.clone()
                        } else {
                            "".to_string()
                        }
                    });
                    
                    if id != d_id {
                        continue;
                    }

                    if let Some(children) = &disk.children {
                        for child in children {
                            if child.uuid.as_deref() == Some(&p_id_for_closure) {
                                parent_name = Some(disk.name.clone());
                                child_name = Some(child.name.clone());
                                partn = child.partn;
                                start_sector = child.start;
                                logical_sector_size = disk.log_sec.filter(|&s| s > 0).unwrap_or(512);
                                fstype = child.fstype.clone();
                                active_mounts = child.active_mountpoints().into_iter().map(|s| s.to_string()).collect();
                                break;
                            }
                        }
                    }
                    if parent_name.is_some() {
                        break;
                    }
                }

                let pkname = parent_name.ok_or_else(|| {
                    DiskError::PartitionNotFound(p_id_for_closure.clone(), format!("Could not find partition {} on disk {}", p_id_for_closure, d_id))
                })?;
                let cname = child_name.ok_or_else(|| {
                    DiskError::DataValidation("Child partition name is missing".to_string())
                })?;
                let partn = partn.ok_or_else(|| {
                    DiskError::DataValidation(format!("Partition {} has no partn value", p_id_for_closure))
                })?;
                let start_sector = start_sector.ok_or_else(|| {
                    DiskError::DataValidation(format!("Partition {} has no start sector", p_id_for_closure))
                })?;
                let fstype = fstype.ok_or_else(|| {
                    DiskError::DataValidation(format!("Partition {} has no fstype value", p_id_for_closure))
                })?;

                Ok((
                    format!("/dev/{}", cname),
                    format!("/dev/{}", pkname),
                    fstype,
                    active_mounts,
                    start_sector,
                    logical_sector_size,
                    partn
                ))
            })
            .await
            .map_err(|e| DiskError::DataValidation(format!("Thread Pool crashed: {}", e)))??;

        // 2 & 3. The Async Phase: Filesystem Shrink using Strategy Pattern
        let strategy = get_shrink_strategy(&fstype);
        match strategy {
            ShrinkStrategy::Unsupported(reason) => {
                return Err(DiskError::DataValidation(reason));
            }
            ShrinkStrategy::Offline { check_args, resize_args } => {
                execute_strategy_cmd(&check_args, &device_path, target_size_gb, "").await?;
                execute_strategy_cmd(&resize_args, &device_path, target_size_gb, "").await?;
            }
            ShrinkStrategy::Online { resize_args } => {
                if let Some(mount) = active_mounts.first() {
                    execute_strategy_cmd(&resize_args, &device_path, target_size_gb, mount).await?;
                } else {
                    let guard = MountGuard::new(&device_path, &p_id).await?;
                    execute_strategy_cmd(&resize_args, &device_path, target_size_gb, &guard.mountpoint).await?;
                }
            }
        }

        // 4. Calculate the Safe End Sector
        let target_sectors = (target_size_gb as u64) * 1024 * 1024 * 1024 / logical_sector_size;
        let safety_buffer_sectors = (100 * 1024 * 1024) / logical_sector_size; // 100 MiB safety buffer
        let end_sector = start_sector + target_sectors + safety_buffer_sectors;

        // 5. Shrink the Boundary
        use tokio::io::AsyncWriteExt;
        let mut parted_cmd = tokio::process::Command::new("parted");
        parted_cmd.arg("---pretend-input-tty")
            .arg(&parent_disk)
            .arg("resizepart")
            .arg(partn.to_string())
            .arg(format!("{}s", end_sector))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = parted_cmd.spawn().map_err(DiskError::OsError)?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"yes\n").await;
        }

        let parted_output = tokio::time::timeout(std::time::Duration::from_secs(300), child.wait_with_output())
            .await
            .map_err(|_| DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "parted timed out after 5 minutes",
            )))?
            .map_err(DiskError::OsError)?;

        if !parted_output.status.success() {
            let err_msg = String::from_utf8_lossy(&parted_output.stderr).trim().to_string();
            return Err(DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("parted failed for {}: {}", parent_disk, err_msg),
            )));
        }

        // 6. Flush to Kernel
        let partprobe_output = tokio::time::timeout(
            std::time::Duration::from_secs(300),
            tokio::process::Command::new("partprobe")
                .arg(&parent_disk)
                .output()
        ).await
        .map_err(|_| DiskError::OsError(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "partprobe timed out after 5 minutes",
        )))?
        .map_err(DiskError::OsError)?;

        if !partprobe_output.status.success() {
            let err_msg = String::from_utf8_lossy(&partprobe_output.stderr).trim().to_string();
            return Err(DiskError::OsError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("partprobe failed for {}: {}", parent_disk, err_msg),
            )));
        }

        Ok(())
    }
}
