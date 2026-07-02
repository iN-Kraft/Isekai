use std::cmp::min;
use std::ffi::c_void;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::process::Stdio;
use std::ptr::null_mut;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::{interval, Instant};
use tracing::{debug, info, warn};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_SHARE_DELETE, FILE_SHARE_NONE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Memory::CreateFileMappingW;
use crate::application::spawn_blocking_with_context;
use crate::domain::errors::DiskError;
use crate::infrastructure::CommandExt;
use crate::telemetry;
use std::os::windows::ffi::OsStrExt;
use crate::domain::models::DiskType;
use crate::infrastructure::windows::utils::ProgressDebouncer;

const FSCTL_QUERY_FILE_REGIONS: u32 = 0x00090284;
const FILE_REGION_USAGE_VALID_CACHED_DATA: u32 = 1;

#[repr(C)]
#[derive(Default)]
struct FILE_REGION_INPUT {
    pub file_offset: i64,
    pub length: i64,
    pub desired_usage: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Default)]
struct FILE_REGION_OUTPUT {
    pub flags: u32,
    pub total_region_entry_count: u32,
    pub region_entry_count: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Default)]
struct FILE_REGION_INFO {
    pub file_offset: i64,
    pub length: i64,
    pub usage: u32,
    pub reserved: u32,
}

pub struct PayloadManager;

impl PayloadManager {
    pub async fn copy_payload(
        source_drive_letter: &str,
        target_drive_letter: &str,
        disk_type: &DiskType,
        mut on_progress: Option<impl FnMut(u64, u64)>
    ) -> Result<(), DiskError> {
        let source = format!("{}:\\", source_drive_letter.trim_end_matches(':'));
        let target = format!("{}:\\", target_drive_letter.trim_end_matches(':'));
        let mt_flag = if *disk_type == DiskType::HDD { "/MT:1" } else { "/MT:8" };

        info!("Calculating payload size for {}...", source);

        let source_path = source.clone();
        let total_bytes = spawn_blocking_with_context(move || Self::get_dir_size(source_path))
            .await
            .unwrap_or(0);

        if total_bytes == 0 {
            return Err(DiskError::DataValidation("Source ISO appears to be empty!".into()));
        }

        info!("Starting high-speed payload copy (Mode: {})", mt_flag);

        let mut child = Command::new("robocopy")
            .kill_on_drop(true)
            .no_window()
            .args([
                &source,
                &target,
                "/E", // Copy subdirectories, including empty ones
                "/R:3", // 3 retries on locked/failed files
                "/W:2", // wait 2 seconds between retries
                "/NP", // no progress
                "/NFL", // no file list
                "/NDL", // no directory list
                mt_flag // multi-threading (8 threads)
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(DiskError::OsError)?;

        let mut ticker = interval(Duration::from_millis(500));
        let mut last_percent = 0u8;
        let mut debouncer = ProgressDebouncer::new();
        let mut vdl_failed = false;

        if let Some(ref mut cb) = on_progress {
            cb(0, total_bytes);
        }

        let exit_status = loop {
            tokio::select! {
                status_result = child.wait() => {
                    break status_result.map_err(DiskError::OsError)?;
                }

                _ = ticker.tick() => {
                    let target_path = target.clone();
                    let raw_bytes = if vdl_failed {
                        spawn_blocking_with_context(move || Self::get_dir_size(target_path))
                        .await
                        .unwrap_or(0)
                    } else {
                        let (bytes, failure) = spawn_blocking_with_context(move || Self::get_dir_size_vdl(target_path))
                        .await
                        .unwrap_or((0, false));

                        vdl_failed |= failure;
                        bytes
                    };

                    let current_bytes = debouncer.calculate(raw_bytes, vdl_failed, disk_type);
                    let raw_percent = (current_bytes as f64 / total_bytes as f64 * 100.0).clamp(0.0, 100.0) as u8;
                    if raw_percent > last_percent {
                        if let Some(ref mut cb) = on_progress {
                            cb(current_bytes, total_bytes);
                        }
                        last_percent = raw_percent
                    }
                }
            }
        };

        let exit_code = exit_status.code().unwrap_or(-1);

        // Microsoft Robocopy exit codes are non-standard:
        // 0 = No files copied (source and dest match)
        // 1 = Files copied successfully
        // 2-7 = Various success states with extra/mismatched files ignored
        // 8+ = Hard failure
        if exit_code >= 8 {
            return Err(DiskError::OsError(Error::new(
                ErrorKind::Other,
                format!("Robocopy failed with code {}.", exit_code,)
            )));
        }

        info!("Payload copy completed successfully!");
        Self::strip_readonly_attributes(&target).await?;

        Ok(())
    }

    async fn strip_readonly_attributes(target_path: &str) -> Result<(), DiskError> {
        info!("Stripping read-only attributes from copied files...");

        let target_glob = format!("{}*", target_path);
        let status = Command::new("attrib")
            .kill_on_drop(true)
            .no_window()
            .args(["-R", &target_glob, "/S", "/D"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(DiskError::OsError)?;

        if !status.success() {
            warn!("Note: Some systems files denied attribute changes (this is normal).");
        } else {
            info!("Read-only attributes removed.");
        }

        Ok(())
    }

    pub fn get_dir_size(path: impl AsRef<Path>) -> u64 {
        let mut size = 0;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        size += Self::get_dir_size(entry.path());
                    } else {
                        size += metadata.len();
                    }
                }
            }
        }
        size
    }

    pub fn get_dir_size_vdl(path: impl AsRef<Path>) -> (u64, bool) {
        let mut size = 0;
        let mut vdl_failed = false;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        let (sub_size, sub_failed) = Self::get_dir_size_vdl(entry.path());
                        size += sub_size;
                        vdl_failed |= sub_failed;
                    } else {
                        let eof_size = metadata.len();
                        if eof_size > 50 * 1024 * 1024 {
                            if let Some(vdl) = Self::get_file_vdl(&entry.path(), eof_size) {
                                size += vdl;
                            } else {
                                size += eof_size;
                                vdl_failed = true;
                            }
                        } else {
                            size += eof_size;
                        }
                    }
                }
            }
        }
        (size, vdl_failed)
    }

    fn get_file_vdl(path: &Path, eof_size: u64) -> Option<u64> {
        let mut path_u16: Vec<u16> = path.as_os_str().encode_wide().collect();
        path_u16.push(0);

        unsafe {
            let handle = CreateFileW(
                path_u16.as_ptr(),
                FILE_READ_ATTRIBUTES | FILE_READ_DATA,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null_mut(),
                OPEN_EXISTING,
                0,
                null_mut()
            );

            if handle == INVALID_HANDLE_VALUE {
                debug!("VDL Probe: CreateFileW failed for {:?}. Win32 Error: {}", path, GetLastError());
                return None;
            }

            let input = FILE_REGION_INPUT {
                file_offset: 0,
                length: eof_size as i64,
                desired_usage: FILE_REGION_USAGE_VALID_CACHED_DATA,
                _padding: 0,
            };

            let output_size = size_of::<FILE_REGION_OUTPUT>();
            let info_size = size_of::<FILE_REGION_INFO>();

            let mut buffer = vec![0u8; output_size + (info_size * 16)];
            let mut bytes_returned = 0;

            let success = DeviceIoControl(
                handle,
                FSCTL_QUERY_FILE_REGIONS,
                &input as *const _ as *const c_void,
                size_of::<FILE_REGION_INPUT>() as u32,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut bytes_returned,
                null_mut()
            );

            CloseHandle(handle);

            if success == 0 || (bytes_returned as usize) < output_size {
                debug!("VDL Probe: DeviceIoControl returned malformed data (Bytes returned: {})", bytes_returned);
                return None;
            }

            let output = &*(buffer.as_ptr() as *const FILE_REGION_OUTPUT);
            let max_safe_entries = ((bytes_returned as usize) - output_size) / info_size;
            let safe_entry_count = min(output.region_entry_count as usize, max_safe_entries);

            let mut vdl = 0u64;
            let info_ptr = buffer.as_ptr().add(output_size) as *const FILE_REGION_INFO;

            for i in 0..safe_entry_count {
                let info = &*info_ptr.add(i);
                vdl += info.length as u64;
            }

            Some(vdl)
        }
    }
}