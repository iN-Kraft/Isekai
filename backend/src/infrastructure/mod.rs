pub mod windows;

pub use windows::WindowsDiskManager as NativeDiskManager;

pub mod windows_validator;

pub use windows_validator::WindowsValidator as NativeValidator;

pub mod assets;

pub trait CommandExt {
    fn no_window(&mut self) -> &mut Self;
}

#[cfg(windows)]
impl CommandExt for tokio::process::Command {
    fn no_window(&mut self) -> &mut Self {
        self.creation_flags(0x08000000_u32)
    }
}

#[cfg(not(windows))]
impl CommandExt for tokio::process::Command {
    fn no_window(&mut self) -> &mut Self {
        self
    }
}