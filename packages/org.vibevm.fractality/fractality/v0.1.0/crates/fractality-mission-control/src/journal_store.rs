//! The append-only JSONL journal on disk (plan D9).
//!
//! One envelope per line, flushed per append — the threat model is
//! daemon death, not OS crash, so `flush` (not fsync) is the durability
//! point. Rotation by size renames the live file to
//! `events-<ms, zero-padded>.jsonl`; zero-padding keeps lexicographic
//! order equal to time order, and `-` sorting before `.` keeps rotated
//! files ahead of the live `events.jsonl` in a name sort — replay is one
//! sorted directory walk.
//!
//! Replay tolerance: a torn **final** line (death mid-write) is expected
//! and skipped with a warning; a malformed line in the middle means real
//! damage — it is skipped, counted, and reported, never silently
//! absorbed.

use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use fractality_core::journal::Envelope;
use fractality_core::time::now_ms;

specmark::scope!("spec://fractality/PROP-001#architecture");

const LIVE_FILE: &str = "events.jsonl";
const DEFAULT_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Appending writer over the live journal file.
pub struct JournalWriter {
    dir: Utf8PathBuf,
    file: std::fs::File,
    bytes: u64,
    max_bytes: u64,
}

impl JournalWriter {
    pub fn open(dir: &Utf8Path) -> Result<Self, String> {
        Self::open_with_max(dir, DEFAULT_MAX_BYTES)
    }

    /// Test seam: a tiny `max_bytes` exercises rotation cheaply.
    pub fn open_with_max(dir: &Utf8Path, max_bytes: u64) -> Result<Self, String> {
        std::fs::create_dir_all(dir.as_std_path()).map_err(|e| format!("creating `{dir}`: {e}"))?;
        let path = dir.join(LIVE_FILE);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_std_path())
            .map_err(|e| format!("opening `{path}`: {e}"))?;
        let bytes = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(Self {
            dir: dir.to_owned(),
            file,
            bytes,
            max_bytes,
        })
    }

    /// Appends one envelope and flushes. Rotates first when the live file
    /// is already over budget.
    pub fn append(&mut self, envelope: &Envelope) -> Result<(), String> {
        if self.bytes >= self.max_bytes {
            self.rotate()?;
        }
        let mut line =
            serde_json::to_string(envelope).map_err(|e| format!("encoding journal line: {e}"))?;
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .and_then(|()| self.file.flush())
            .map_err(|e| format!("appending to journal: {e}"))?;
        self.bytes += line.len() as u64;
        Ok(())
    }

    fn rotate(&mut self) -> Result<(), String> {
        let live = self.dir.join(LIVE_FILE);
        // Two rotations inside one millisecond must not collide: a
        // sequence suffix keeps names unique AND lexicographically
        // time-ordered within the same stamp.
        let ms = now_ms();
        let mut rotated = None;
        for n in 0u32..=9999 {
            let candidate = self.dir.join(format!("events-{ms:020}-{n:04}.jsonl"));
            if !candidate.as_std_path().exists() {
                rotated = Some(candidate);
                break;
            }
        }
        let Some(rotated) = rotated else {
            return Err(format!(
                "no free rotation slot: 10000 rotations inside millisecond {ms} \
                 (the size budget is broken; raise max_bytes)"
            ));
        };
        std::fs::rename(live.as_std_path(), rotated.as_std_path())
            .map_err(|e| format!("rotating journal to `{rotated}`: {e}"))?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(LIVE_FILE).as_std_path())
            .map_err(|e| format!("reopening live journal: {e}"))?;
        self.file = file;
        self.bytes = 0;
        tracing::info!(%rotated, "journal rotated");
        Ok(())
    }
}

/// What replay saw besides the good lines.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReplayReport {
    pub lines: u64,
    /// Malformed lines mid-stream (journal damage).
    pub skipped: u64,
    /// A torn final line (death mid-write; expected, benign).
    pub torn_tail: bool,
}

/// Reads every journal file in order and parses line by line.
pub fn replay(dir: &Utf8Path) -> Result<(Vec<Envelope>, ReplayReport), String> {
    let mut names: Vec<String> = Vec::new();
    match std::fs::read_dir(dir.as_std_path()) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|e| format!("listing `{dir}`: {e}"))?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("events") && name.ends_with(".jsonl") {
                    names.push(name);
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), ReplayReport::default()));
        }
        Err(e) => return Err(format!("listing `{dir}`: {e}")),
    }
    names.sort();

    let mut envelopes = Vec::new();
    let mut report = ReplayReport::default();
    let last_file = names.len().saturating_sub(1);
    for (fi, name) in names.iter().enumerate() {
        let path = dir.join(name);
        let text = std::fs::read_to_string(path.as_std_path())
            .map_err(|e| format!("reading `{path}`: {e}"))?;
        let lines: Vec<&str> = text.lines().collect();
        let last_line = lines.len().saturating_sub(1);
        for (li, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            report.lines += 1;
            match serde_json::from_str::<Envelope>(line) {
                Ok(env) => envelopes.push(env),
                Err(e) => {
                    if fi == last_file && li == last_line {
                        report.torn_tail = true;
                        tracing::warn!(%path, error = %e, "torn final journal line skipped");
                    } else {
                        report.skipped += 1;
                        tracing::warn!(%path, line = li + 1, error = %e, "malformed journal line skipped");
                    }
                }
            }
        }
    }
    Ok((envelopes, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractality_core::ids::RunId;
    use fractality_core::journal::Event;

    fn scratch(tag: &str) -> Utf8PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fractality-journal-{tag}-{}", ulid::Ulid::new()));
        Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
    }

    fn spawned(ts: u64) -> Envelope {
        let run_id: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("ulid");
        Envelope {
            ts_ms: ts,
            event: Event::Spawned {
                run_id,
                worker_pid: 1,
            },
        }
    }

    #[test]
    fn append_then_replay_round_trips() {
        let dir = scratch("rt");
        let mut w = JournalWriter::open(&dir).expect("opens");
        for ts in 1..=3 {
            w.append(&spawned(ts)).expect("appends");
        }
        let (envs, report) = replay(&dir).expect("replays");
        assert_eq!(envs.len(), 3);
        assert_eq!(report.lines, 3);
        assert_eq!(report.skipped, 0);
        assert!(!report.torn_tail);
        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }

    #[test]
    fn torn_tail_is_tolerated_and_reported() {
        let dir = scratch("torn");
        let mut w = JournalWriter::open(&dir).expect("opens");
        w.append(&spawned(1)).expect("appends");
        // Simulate death mid-write: a half line with no newline.
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(dir.join(LIVE_FILE).as_std_path())
                .expect("open live");
            f.write_all(b"{\"ts_ms\":2,\"event\":\"spaw")
                .expect("write torn");
        }
        let (envs, report) = replay(&dir).expect("replays");
        assert_eq!(envs.len(), 1);
        assert!(report.torn_tail);
        assert_eq!(report.skipped, 0);
        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }

    #[test]
    fn midstream_damage_is_skipped_and_counted() {
        let dir = scratch("damage");
        let mut w = JournalWriter::open(&dir).expect("opens");
        w.append(&spawned(1)).expect("appends");
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(dir.join(LIVE_FILE).as_std_path())
                .expect("open live");
            f.write_all(b"garbage-line\n").expect("write garbage");
        }
        w.append(&spawned(3)).expect("appends after damage");
        let (envs, report) = replay(&dir).expect("replays");
        assert_eq!(envs.len(), 2);
        assert_eq!(report.skipped, 1);
        assert!(!report.torn_tail);
        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }

    #[test]
    fn rotation_keeps_replay_order() {
        let dir = scratch("rotate");
        // Tiny budget: every append rotates.
        let mut w = JournalWriter::open_with_max(&dir, 10).expect("opens");
        for ts in 1..=5 {
            w.append(&spawned(ts)).expect("appends");
        }
        let (envs, report) = replay(&dir).expect("replays");
        assert_eq!(envs.len(), 5);
        assert_eq!(report.skipped, 0);
        let stamps: Vec<u64> = envs.iter().map(|e| e.ts_ms).collect();
        assert_eq!(stamps, vec![1, 2, 3, 4, 5], "order survives rotation");
        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }
}
