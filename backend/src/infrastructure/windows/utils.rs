use tokio::time::Instant;
use crate::domain::models::DiskType;

pub struct PartitionUtils;

impl PartitionUtils {
    pub fn determine_partition_label(drive_letter: Option<&str>, gpt_type: Option<&str>, mbr_type: Option<u16>) -> String {
        if let Some(dl) = drive_letter {
            let trimmed = dl.trim_matches('\0').trim();
            if trimmed == "C" {
                return "C: (Windows/NTFS)".to_string();
            } else if !trimmed.is_empty() {
                return format!("{}: drive", trimmed)
            }
        }

        if let Some(gpt) = gpt_type {
            let gpt_lower = gpt.to_lowercase();
            if gpt_lower.contains("de94bba4") {
                return "Recovery".to_string();
            } else if gpt_lower.contains("e3c9e316") {
                return "Microsoft Reversed".to_string();
            } else if gpt_lower.contains("c12a7328") {
                return "EFI System (ESP)".to_string();
            }
        }

        if let Some(pt) = mbr_type {
            if pt == 4 || pt == 39 {
                return "Recovery".to_string();
            }
        }

        "Partition".to_string()
    }
}

pub struct ProgressDebouncer {
    reported_bytes: f64,
    target_bytes: u64,
    guaranteed_bytes: u64,
    last_tick: Instant
}

impl ProgressDebouncer {
    pub fn new() -> Self {
        Self {
            reported_bytes: 0.0,
            target_bytes: 0,
            guaranteed_bytes: 0,
            last_tick: Instant::now()
        }
    }

    pub fn calculate(&mut self, raw_bytes: u64, has_locked_eof: bool, disk_type: &DiskType) -> u64 {
        if raw_bytes > self.target_bytes {
            self.guaranteed_bytes = self.target_bytes;
            self.target_bytes = raw_bytes;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        if has_locked_eof {
            if (self.reported_bytes as u64) < self.guaranteed_bytes {
                self.reported_bytes = self.guaranteed_bytes as f64;
            }

            let speed_bps = match disk_type {
                DiskType::NVME => 500.0 * 1024.0 * 1024.0,
                DiskType::SSD => 250.0 * 1024.0 * 1024.0,
                DiskType::HDD => 60.0 * 1024.0 * 1024.0,
                DiskType::USB => 20.0 * 1024.0 * 1024.0,
                DiskType::Unknown => 50.0 * 1024.0 * 1024.0,
            };

            self.reported_bytes += (speed_bps / 2.0) * dt;

            if self.reported_bytes > self.target_bytes as f64 {
                self.reported_bytes = self.target_bytes as f64;
            }
        } else {
            self.reported_bytes = raw_bytes as f64;
        }

        self.reported_bytes as u64
    }
}