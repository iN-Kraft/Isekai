use std::sync::Arc;
use rustyline::completion::{Completer, Pair};
use rustyline::{Context, Helper};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use crate::domain::traits::DiskManager;

pub struct IsekaiHelper {
    pub disk_manager: Arc<dyn DiskManager>
}

impl Completer for IsekaiHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let mut candidates = Vec::new();
        let line_until_cursor = &line[..pos];
        let tokens: Vec<&str> = line_until_cursor.split_whitespace().collect();
        
        let is_space = line_until_cursor.ends_with(' ');
        let word = if is_space { "" } else { tokens.last().unwrap_or(&"") };
        let tokens_len_actual = if is_space { tokens.len() + 1 } else { tokens.len() };

        if tokens_len_actual == 1 {
            let cmds = ["list", "parts", "shrink-and-install", "exit", "quit", "help"];
            for cmd in cmds {
                if cmd.starts_with(word) {
                    candidates.push(Pair { display: cmd.to_string(), replacement: cmd.to_string() });
                }
            }
            return Ok((pos - word.len(), candidates));
        }

        let cmd = tokens[0];

        // Autocomplete positional args for `parts`
        if cmd == "parts" && tokens_len_actual == 2 {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let dm = self.disk_manager.clone();
                if let Ok(disks) = handle.block_on(async { dm.get_disks().await }) {
                    for disk in disks {
                        if disk.stable_id.starts_with(word) {
                            candidates.push(Pair {
                                display: format!("{} ({})", disk.stable_id, disk.name),
                                replacement: disk.stable_id.clone(),
                            });
                        }
                    }
                }
            }
            return Ok((pos - word.len(), candidates));
        }

        // Advanced completion for commands with flags
        if cmd == "shrink-and-install" {
            let prev_word = if tokens_len_actual >= 2 && !is_space { 
                tokens.get(tokens.len() - 2).unwrap_or(&"") 
            } else { 
                tokens.last().unwrap_or(&"") 
            };

            // 1. Completing flag values
            if prev_word == &"--disk-id" {
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    let dm = self.disk_manager.clone();
                    if let Ok(disks) = handle.block_on(async { dm.get_disks().await }) {
                        for disk in disks {
                            if disk.stable_id.starts_with(word) {
                                candidates.push(Pair {
                                    display: format!("{} ({})", disk.stable_id, disk.name),
                                    replacement: disk.stable_id.clone(),
                                });
                            }
                        }
                    }
                }
                return Ok((pos - word.len(), candidates));
            }

            if prev_word == &"--partition-id" {
                // Find disk-id in previous tokens to query partitions accurately
                let mut disk_id = None;
                for i in 0..tokens.len() {
                    if tokens[i] == "--disk-id" && i + 1 < tokens.len() {
                        disk_id = Some(tokens[i + 1]);
                    }
                }
                
                if let Some(did) = disk_id {
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        let dm = self.disk_manager.clone();
                        if let Ok(parts) = handle.block_on(async { dm.get_partitions(did).await }) {
                            for part in parts {
                                if part.id.starts_with(word) {
                                    candidates.push(Pair {
                                        display: format!("{} ({}MB {})", part.id, part.size_bytes / 1024 / 1024, part.file_system),
                                        replacement: part.id.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
                return Ok((pos - word.len(), candidates));
            }

            // 2. Completing flags themselves
            if word.starts_with("--") || is_space {
                let flags = vec!["--disk-id", "--iso-path", "--partition-id"];

                for flag in flags {
                    if flag.starts_with(word) {
                        candidates.push(Pair { display: flag.to_string(), replacement: flag.to_string() });
                    }
                }
                return Ok((pos - word.len(), candidates));
            }
        }

        Ok((pos, candidates))
    }
}

impl Helper for IsekaiHelper { }
impl Hinter for IsekaiHelper {
    type Hint = String;
}
impl Highlighter for IsekaiHelper { }
impl Validator for IsekaiHelper { }
