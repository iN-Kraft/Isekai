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
        let tokens: Vec<&str> = line_until_cursor.split(' ').collect();

        if tokens.len() == 1 {
            let cmds = ["check", "list", "parts", "shrink", "exit", "quit", "help"];
            let word = tokens[0];
            for cmd in cmds {
                if cmd.starts_with(word) {
                    candidates.push(Pair { display: cmd.to_string(), replacement: cmd.to_string() });
                }
            }
            return Ok((pos - word.len(), candidates));
        }

        if tokens.len() == 2 && (tokens[0] == "parts" || tokens[0] == "shrink") {
            let word = tokens[1];
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let dm = self.disk_manager.clone();
                // block_on safely executes the async future on the current thread
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

        if tokens.len() == 3 && tokens[0] == "shrink" {
            let disk_id = tokens[1];
            let word = tokens[2];
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let dm = self.disk_manager.clone();
                if let Ok(parts) = handle.block_on(async { dm.get_partitions(disk_id).await }) {
                    for part in parts {
                        if part.id.starts_with(word) {
                            candidates.push(Pair {
                                display: format!("{} ({}GB)", part.id, part.size_gb),
                                replacement: part.id.clone(),
                            });
                        }
                    }
                }
            }
            return Ok((pos - word.len(), candidates));
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
