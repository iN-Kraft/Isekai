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
