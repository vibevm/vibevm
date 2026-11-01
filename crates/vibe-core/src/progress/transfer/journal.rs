//! The journal of transfers under the settings root (`PROP-068#journal`):
//! `transfers/active/<pid>-<n>.json` rewritten at every sample,
//! `transfers/history.jsonl` appended at every end and trimmed to seven
//! days or 500 entries, and `transfers/revision` bumped at every change of
//! the active set — the device `PROP-059#launcher-runtime` uses, so a
//! watcher polls one small file.
//!
//! The journal is ancillary to the download it describes: every write is
//! best effort and a failure to write it never fails a transfer. Nothing
//! here creates the directory until a transfer begins, so a machine that
//! downloaded nothing has no journal at all.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#journal");

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::TransferRecord;
use super::clock::{parse_millis, unix_millis};
use super::sampler::{Bucket, Sample};

/// History older than this is trimmed at the next append.
const HISTORY_DAYS: u64 = 7;
/// …and so is everything beyond this many entries.
const HISTORY_ENTRIES: usize = 500;

/// How a transfer ended.
///
/// ```
/// use vibe_core::progress::transfer::Outcome;
///
/// assert_eq!(serde_json::to_string(&Outcome::Interrupted).unwrap(), "\"interrupted\"");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Finished,
    Failed,
    Interrupted,
}

/// One transfer as the journal keeps it: an active file (the record, the
/// bytes and peak so far, the samples of the last five minutes) or a
/// history line (the record, the outcome, the totals and the per-second
/// series). The listing adds the last sample of an active one as `last`.
///
/// ```
/// use vibe_core::progress::transfer::{Destination, Purpose, TransferEntry, TransferRecord};
///
/// let record = TransferRecord::http(Purpose::IndexFile, "https://index.example/repomd.json", Destination::Store);
/// let entry = TransferEntry::active(record);
/// assert_eq!(entry.bytes, 0);
/// assert!(entry.outcome.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferEntry {
    #[serde(flatten)]
    pub record: TransferRecord,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub peak_bps: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<Sample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<Sample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_bps: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub series: Vec<Bucket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket_seconds: Option<u64>,
}

impl TransferEntry {
    /// The entry of a transfer that has only begun.
    pub fn active(record: TransferRecord) -> Self {
        Self {
            record,
            bytes: 0,
            peak_bps: 0,
            samples: Vec::new(),
            last: None,
            outcome: None,
            message: None,
            elapsed_ms: None,
            average_bps: None,
            series: Vec::new(),
            bucket_seconds: None,
        }
    }

    /// The history entry of an active file whose process is gone: what its
    /// samples show, the series rebuilt from them (the bytes before the
    /// five-minute window fall in the window's first second), and the
    /// command's resume line.
    fn orphaned(mut self) -> Self {
        let started = parse_millis(&self.record.started).unwrap_or(0);
        let last = self.samples.last().cloned();
        let ended = last
            .as_ref()
            .and_then(|sample| parse_millis(&sample.at))
            .unwrap_or(started);
        let elapsed = ended.saturating_sub(started);
        let mut seconds: Vec<u64> = vec![0; (elapsed / 1000) as usize + 1];
        let mut previous = 0;
        for sample in &self.samples {
            let at = parse_millis(&sample.at).unwrap_or(started);
            let second = (at.saturating_sub(started) / 1000) as usize;
            if let Some(slot) = seconds.get_mut(second) {
                *slot += sample.bytes.saturating_sub(previous);
            }
            previous = previous.max(sample.bytes);
        }
        self.bytes = last.map_or(self.bytes, |sample| sample.bytes);
        self.message = Some(format!(
            "the process {} ended before the transfer did",
            self.record.pid
        ));
        self.close(Outcome::Interrupted, elapsed, &seconds_to_series(seconds));
        self
    }

    /// Turn an active entry into its history line.
    pub(crate) fn close(&mut self, outcome: Outcome, elapsed_ms: u64, series: &(Vec<Bucket>, u64)) {
        self.outcome = Some(outcome);
        self.elapsed_ms = Some(elapsed_ms);
        self.average_bps = Some(self.bytes.saturating_mul(1000) / elapsed_ms.max(1));
        self.samples.clear();
        self.last = None;
        self.series = series.0.clone();
        self.bucket_seconds = Some(series.1);
        if outcome != Outcome::Interrupted {
            self.record.resume = None;
        }
    }
}

fn seconds_to_series(seconds: Vec<u64>) -> (Vec<Bucket>, u64) {
    let width = seconds.len().div_ceil(super::sampler::SERIES_CAP).max(1);
    let buckets = seconds
        .chunks(width)
        .enumerate()
        .map(|(index, chunk)| Bucket {
            second: (index * width) as u64,
            bytes: chunk.iter().sum(),
        })
        .collect();
    (buckets, width as u64)
}

/// The journal under one settings root. Clones share one lock, which keeps
/// two transfers of this process from interleaving a revision bump or a
/// history rewrite; across processes a lost bump only delays a watcher by
/// a tick, since the counter is a change hint.
///
/// ```
/// use vibe_core::progress::transfer::TransferJournal;
///
/// let home = tempfile::tempdir().unwrap();
/// let journal = TransferJournal::new(home.path());
/// assert_eq!(journal.revision(), 0);
/// assert!(journal.active().is_empty());
/// assert!(!home.path().join("transfers").exists(), "reading creates nothing");
/// ```
#[derive(Debug, Clone)]
pub struct TransferJournal {
    root: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl TransferJournal {
    /// The journal of the settings root `settings` (`<settings>/transfers`).
    pub fn new(settings: &Path) -> Self {
        Self {
            root: settings.join("transfers"),
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn active_dir(&self) -> PathBuf {
        self.root.join("active")
    }

    fn history_path(&self) -> PathBuf {
        self.root.join("history.jsonl")
    }

    fn active_path(&self, record: &TransferRecord) -> PathBuf {
        self.active_dir()
            .join(format!("{}.json", record.file_stem()))
    }

    /// The change counter of the active set; 0 before anything changed.
    pub fn revision(&self) -> u64 {
        fs::read_to_string(self.root.join("revision"))
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(0)
    }

    fn bump(&self) {
        let next = self.revision().saturating_add(1);
        let _ = replace(&self.root.join("revision"), format!("{next}\n").as_bytes());
    }

    /// A transfer began: its active file appears and the revision moves.
    pub(crate) fn opened(&self, entry: &TransferEntry) {
        let _guard = self.lock.lock();
        if fs::create_dir_all(self.active_dir()).is_ok() {
            self.write_active(entry);
            self.bump();
        }
    }

    /// A sample was taken: the active file is rewritten in place.
    pub(crate) fn sampled(&self, entry: &TransferEntry) {
        self.write_active(entry);
    }

    fn write_active(&self, entry: &TransferEntry) {
        if let Ok(bytes) = serde_json::to_vec(entry) {
            let _ = replace(&self.active_path(&entry.record), &bytes);
        }
    }

    /// A transfer ended: its history line is appended **before** its active
    /// file goes, so a reader that sees the file gone finds the line.
    pub(crate) fn ended(&self, entry: &TransferEntry) {
        let _guard = self.lock.lock();
        self.append(entry);
        let _ = fs::remove_file(self.active_path(&entry.record));
        self.bump();
    }

    fn append(&self, entry: &TransferEntry) {
        let Ok(mut line) = serde_json::to_vec(entry) else {
            return;
        };
        line.push(b'\n');
        let path = self.history_path();
        let _ = fs::create_dir_all(&self.root);
        // One write per line: an append-mode write of one buffer lands
        // whole, so a reader never sees half a line followed by another.
        if let Ok(mut file) = private(OpenOptions::new().create(true).append(true)).open(&path) {
            let _ = file.write_all(&line);
        }
        self.trim();
    }

    /// Keep the last seven days or the last 500 entries, whichever is fewer.
    fn trim(&self) {
        let entries = self.history();
        let horizon = unix_millis().saturating_sub(HISTORY_DAYS * 86_400_000);
        let recent = |entry: &TransferEntry| {
            parse_millis(&entry.record.started).is_none_or(|started| started >= horizon)
        };
        let kept: Vec<&TransferEntry> = entries.iter().filter(|e| recent(e)).collect();
        let kept = &kept[kept.len().saturating_sub(HISTORY_ENTRIES)..];
        if kept.len() == entries.len() {
            return;
        }
        let mut text = Vec::new();
        for entry in kept {
            if let Ok(line) = serde_json::to_vec(entry) {
                text.extend(line);
                text.push(b'\n');
            }
        }
        let _ = replace(&self.history_path(), &text);
    }

    /// Every active file as written, unordered liveness aside.
    pub fn active(&self) -> Vec<TransferEntry> {
        let Ok(entries) = fs::read_dir(self.active_dir()) else {
            return Vec::new();
        };
        let mut active: Vec<TransferEntry> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|entry| fs::read(entry.path()).ok())
            .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
            .collect();
        active.sort_by(|a, b| a.record.started.cmp(&b.record.started));
        active
    }

    /// The active transfers whose process is alive; an active file whose
    /// process is gone becomes an `interrupted` history entry
    /// (`PROP-068#journal`), with the command's resume line.
    pub fn sweep(&self, alive: &dyn Fn(u32) -> bool) -> Vec<TransferEntry> {
        let (live, dead): (Vec<_>, Vec<_>) = self
            .active()
            .into_iter()
            .partition(|entry| entry.record.pid == std::process::id() || alive(entry.record.pid));
        for entry in dead {
            self.ended(&entry.orphaned());
        }
        self.sweep_siblings(alive);
        live
    }

    /// A process killed while it rewrote its active file leaves the
    /// sibling (`.<pid>-<n>.json.<pid>.tmp`) behind; it goes with its
    /// process.
    fn sweep_siblings(&self, alive: &dyn Fn(u32) -> bool) {
        let Ok(entries) = fs::read_dir(self.active_dir()) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let writer = name
                .strip_suffix(".tmp")
                .and_then(|rest| rest.rsplit('.').next())
                .and_then(|pid| pid.parse::<u32>().ok());
            if let Some(pid) = writer
                && pid != std::process::id()
                && !alive(pid)
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    /// The history, oldest first; a damaged line is skipped.
    pub fn history(&self) -> Vec<TransferEntry> {
        fs::read(self.history_path())
            .map(|bytes| {
                bytes
                    .split(|byte| *byte == b'\n')
                    .filter(|line| !line.is_empty())
                    .filter_map(|line| serde_json::from_slice(line).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Empty the history; answers how many entries it held.
    pub fn clear_history(&self) -> io::Result<usize> {
        let _guard = self.lock.lock();
        let count = self.history().len();
        match fs::remove_file(self.history_path()) {
            Ok(()) => Ok(count),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error),
        }
    }

    /// The interrupted transfer a new one continues: the latest history
    /// entry of the same package and URL, when it was interrupted.
    pub(crate) fn predecessor(&self, record: &TransferRecord) -> Option<String> {
        let history = self.history();
        let previous = history.iter().rev().find(|entry| {
            entry.record.url == record.url && entry.record.package == record.package
        })?;
        (previous.outcome == Some(Outcome::Interrupted)).then(|| previous.record.id.clone())
    }
}

/// `0600` on Unix: a journal names what a user downloaded.
fn private(options: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

/// Write a sibling and rename it over `path`. `fs::rename` replaces an
/// existing file on Windows too (`MoveFileExW` with
/// `MOVEFILE_REPLACE_EXISTING`). No `sync_all`: the active file is a live
/// view rewritten four times a second, not a record anything recovers from.
fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    // The process id keeps two processes bumping one revision apart.
    let sibling = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut file: File =
        private(OpenOptions::new().create(true).write(true).truncate(true)).open(&sibling)?;
    file.write_all(bytes)?;
    drop(file);
    fs::rename(&sibling, path).inspect_err(|_| {
        let _ = fs::remove_file(&sibling);
    })
}
