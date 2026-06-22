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
    /// List all available physical disks
    List,
    /// List partitions for a specific disk
    Parts {
        /// The Hardware ID of the disk
        disk_id: String,
    },
    
    /// Shrink an existing NTFS partition and install the ISO alongside it
    ShrinkAndInstall {
        /// Target Disk Hardware ID
        #[arg(long)]
        disk_id: String,
        /// Target Partition ID to Shrink (e.g. your C: drive partition)
        #[arg(long)]
        partition_id: String,
        /// Absolute path to the payload ISO
        #[arg(long)]
        iso_path: String,
    },

    /// Exit the CLI
    Exit,
    /// Exit the CLI
    Quit,
}
