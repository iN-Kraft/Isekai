use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct IsekaiCli {
    /// Enable debug mode (shows virtual drives/loops)
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Enter interactive REPL mode
    #[arg(short, long)]
    pub cli: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check if required system packages are available
    Check,
    /// List all available physical disks
    List,
    /// List partitions for a specific disk
    Parts {
        /// The Hardware ID of the disk
        disk_id: String,
    },
    /// Shrink a partition on a disk
    Shrink {
        /// The Hardware ID of the disk
        disk_id: String,
        /// The ID of the partition
        partition_id: String,
        /// Target size in GB
        target_size_gb: u32,
    },
    /// Exit the CLI
    Exit,
    /// Exit the CLI
    Quit,
}
