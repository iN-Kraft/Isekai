#[cfg(target_os = "windows")]
pub mod windows_disk;

#[cfg(target_os = "windows")]
pub use windows_disk::WindowsDiskManager as NativeDiskManager;

#[cfg(target_os = "linux")]
pub mod linux_disk;

#[cfg(target_os = "linux")]
mod blockdev;

#[cfg(target_os = "linux")]
pub use linux_disk::LinuxDiskManager as NativeDiskManager;

#[cfg(target_os = "windows")]
pub mod windows_validator;

#[cfg(target_os = "windows")]
pub use windows_validator::WindowsValidator as NativeValidator;

#[cfg(target_os = "linux")]
pub mod linux_validator;
pub mod iso_manager;
pub mod assets;
pub mod boot_manager;

#[cfg(target_os = "linux")]
pub use linux_validator::LinuxValidator as NativeValidator;