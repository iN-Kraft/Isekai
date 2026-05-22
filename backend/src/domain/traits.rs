use async_trait::async_trait;
use mockall::automock;
use crate::domain::errors::DiskError;
use crate::domain::models::{Disk, Partition};

#[automock]
#[async_trait]
pub trait DiskManager: Send + Sync {
    async fn get_disks(&self) -> Result<Vec<Disk>, DiskError>;
    async fn get_partitions(&self, disk_id: &str) -> Result<Vec<Partition>, DiskError>;
    async fn shrink_partition(&self, disk_id: &str, partition_id: &str, target_size_gb: u32) -> Result<(), DiskError>;
}
