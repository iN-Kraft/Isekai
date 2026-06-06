pub const BOOT_X64_EFI: &[u8] = include_bytes!("../assets/bootx64.efi");
pub const NTFS_X64_EFI: &[u8] = include_bytes!("../assets/ntfs_x64.efi");
pub const EXFAT_X64_EFI: &[u8] = include_bytes!("../assets/exfat_x64.efi");

pub const GRLDR_MBR: &[u8] = include_bytes!("../assets/grldr.mbr");
pub const GRLDR: &[u8] = include_bytes!("../assets/grldr");

pub const COMMAND_NO_WINDOW: u32 = 0x08000000;