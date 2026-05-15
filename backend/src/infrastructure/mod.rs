#[cfg(target_os = "windows")]
pub mod windows_disk;

#[cfg(target_os = "linux")]
pub mod linux_disk;

#[cfg(target_os = "linux")]
pub use linux_disk::LinuxDiskManager as NativeDiskManager;