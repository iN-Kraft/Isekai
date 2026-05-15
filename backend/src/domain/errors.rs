use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiskError {
    #[error("Drive {0} is locked by BitLocker or LUKS")]
    DriveEncrypted(String),

    #[error("Insufficient space: need {required}GB, has {available}GB")]
    InsufficientSpace { required: u32, available: u32 },

    #[error("OS Permission denied. Are we running as Admin/Root?")]
    PermissionDenied,

    #[error("Disk {0} was not found")]
    DiskNotFound(String),

    #[error("Underlying OS error: {0}")]
    OsError(#[from] std::io::Error),
}