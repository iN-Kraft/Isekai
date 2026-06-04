use crate::domain::models::{InstallPlan, Partition};

const MSR_RESERVE_BYTES: u64 = 16 * 1024 * 1024;
const PARTITION_ALIGNMENT_BYTES: u64 = 1024 * 1024;
const TOTAL_PLACEMENT_OVERHEAD_BYTES: u64 = MSR_RESERVE_BYTES + PARTITION_ALIGNMENT_BYTES;

struct Gap {
    start: u64,
    end: u64,
    size: u64
}

pub struct PartitionPlanner;

impl PartitionPlanner {
    pub fn calculate_required_shrink_bytes(linux_size_mb: u32, boot_size_mb: u32) -> u64 {
        let mb_to_bytes = 1024_u64 * 1024;
        let linux_bytes = (linux_size_mb as u64) * mb_to_bytes;
        let boot_bytes = (boot_size_mb as u64) * mb_to_bytes;

        linux_bytes + boot_bytes + TOTAL_PLACEMENT_OVERHEAD_BYTES
    }

    pub fn get_contiguous_install_plan(
        disk_size_bytes: u64,
        partitions: &[Partition],
        anchor_end_bytes: u64,
        boot_size_mb: u32,
        linux_size_mb: u32
    ) -> InstallPlan {
        let mut gaps = Vec::new();
        let mut prev_end: u64 = 0;

        for part in partitions {
            if part.offset_bytes > prev_end {
                let gap_size = part.offset_bytes - prev_end;
                if gap_size > PARTITION_ALIGNMENT_BYTES {
                    gaps.push(Gap {
                        start: prev_end,
                        end: part.offset_bytes,
                        size: gap_size
                    });
                }
            }
            prev_end = part.offset_bytes + part.size_bytes;
        }

        if disk_size_bytes > prev_end {
            let trailing_gap = disk_size_bytes - prev_end;
            if trailing_gap > PARTITION_ALIGNMENT_BYTES {
                gaps.push(Gap {
                    start: prev_end,
                    end: disk_size_bytes,
                    size: trailing_gap
                });
            }
        }

        let boot_size_bytes = (boot_size_mb as u64) * 1024 * 1024;
        let min_gap_required = boot_size_bytes + TOTAL_PLACEMENT_OVERHEAD_BYTES;
        let usable_gaps: Vec<&Gap> = gaps.iter().filter(|g| g.size >= min_gap_required).collect();
        let mut result = InstallPlan {
            has_boot_space: false,
            has_requested_linux_space: false,
            boot_partition_offset_bytes: 0,
            linux_space_bytes: 0
        };

        if usable_gaps.is_empty() {
            return result;
        }

        let chosen_gap = usable_gaps.iter().find(|&&g| {
            let lower_bound = anchor_end_bytes.saturating_sub(PARTITION_ALIGNMENT_BYTES);
            let upper_bound = anchor_end_bytes.saturating_add(PARTITION_ALIGNMENT_BYTES);
            g.start >= lower_bound && g.start <= upper_bound
        }).copied().or_else(|| {
            usable_gaps.iter()
                .filter(|&&g| g.start >= anchor_end_bytes)
                .max_by_key(|&&g| g.size)
                .copied()
        });

        let chosen_gap = match chosen_gap {
            Some(g) => g,
            None => return result
        };

        let boot_end = chosen_gap.end.saturating_sub(MSR_RESERVE_BYTES);
        let raw_boot_offset = boot_end.saturating_sub(boot_size_bytes);
        let boot_partition_offset = (raw_boot_offset / PARTITION_ALIGNMENT_BYTES) * PARTITION_ALIGNMENT_BYTES;

        if boot_partition_offset < (chosen_gap.start + PARTITION_ALIGNMENT_BYTES) {
            return result;
        }

        let linux_space = boot_partition_offset - chosen_gap.start;
        let requested_linux_bytes = (linux_size_mb as u64) * 1024 * 1024;

        result.has_boot_space = true;
        result.has_requested_linux_space = linux_space >= requested_linux_bytes;
        result.boot_partition_offset_bytes = boot_partition_offset;
        result.linux_space_bytes = linux_space;

        result
    }
}