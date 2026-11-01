//! The events journal of a home (`PROP-059#launcher-runtime`):
//! `<home>/applications/events.jsonl`, one `vibe-app-events/1` line per
//! change, appended by the same calls that bump the revision and read by
//! `vibe app watch` from wherever it started.
//!
//! The journal is a change feed, not a record anything depends on: a
//! client that missed lines refetches the catalog, which is why it is
//! bounded by rotation rather than kept, and why a damaged line is skipped
//! rather than refused.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_wire::generated::application_events as wire;

use crate::error::StateError;
use crate::settings::{SettingsRoot, io};

pub use wire::{ApplicationEvent, EventKind};

/// The protocol tag of every journal line and every `watch --json` line.
pub const EVENTS_PROTOCOL: &str = "vibe-app-events/1";

/// The size at which the journal is rotated before the next append. A
/// watcher polls four times a second, so it has read the old file long
/// before this much has been written behind it.
const ROTATE_AT: u64 = 256 * 1024;

/// One change of a home, before it is given its revision and time.
///
/// ```
/// use vibe_app::{EventKind, HomeEvent};
///
/// let event = HomeEvent::about(EventKind::Started, "org.example/x@1.0.0#main").with_pid(42);
/// assert_eq!(event.kind, EventKind::Started);
/// assert_eq!(event.key.as_deref(), Some("org.example/x@1.0.0#main"));
/// assert_eq!(event.pid, Some(42));
/// assert_eq!(HomeEvent::new(EventKind::CollectionsChanged).key, None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub struct HomeEvent {
    pub kind: EventKind,
    /// A package key for `installed`, `updated` and `removed`, a launcher
    /// key for `started`, `exited` and an override of one launcher.
    pub key: Option<String>,
    pub pid: Option<u64>,
}

impl HomeEvent {
    pub fn new(kind: EventKind) -> Self {
        Self {
            kind,
            key: None,
            pid: None,
        }
    }

    pub fn about(kind: EventKind, key: impl Into<String>) -> Self {
        Self {
            key: Some(key.into()),
            ..Self::new(kind)
        }
    }

    pub fn with_pid(self, pid: u64) -> Self {
        Self {
            pid: Some(pid),
            ..self
        }
    }

    fn line(&self, revision: u64) -> ApplicationEvent {
        ApplicationEvent {
            protocol: EVENTS_PROTOCOL.into(),
            event: self.kind.clone(),
            revision,
            key: self.key.clone(),
            pid: self.pid,
            at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }
    }
}

fn rotated(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".1");
    PathBuf::from(name)
}

/// Append one event at `revision`; the caller holds the home's mutation
/// lock, which is what keeps two appends from interleaving.
pub(crate) fn append(
    settings: &SettingsRoot,
    event: &HomeEvent,
    revision: u64,
) -> Result<(), StateError> {
    append_bounded(settings, event, revision, ROTATE_AT)
}

fn append_bounded(
    settings: &SettingsRoot,
    event: &HomeEvent,
    revision: u64,
    bound: u64,
) -> Result<(), StateError> {
    let path = settings.events_path();
    if fs::metadata(&path).is_ok_and(|meta| meta.len() >= bound) {
        let old = rotated(&path);
        fs::rename(&path, &old).map_err(|source| io("rotating", &path, source))?;
    }
    let mut line = serde_json::to_vec(&event.line(revision)).map_err(|error| StateError::Io {
        action: "encoding",
        path: path.display().to_string(),
        source: std::io::Error::other(error),
    })?;
    line.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| io("appending to", &path, source))?;
    // One write per line: an append-mode write of a short buffer lands
    // whole, so a reader never sees half of a line followed by another.
    file.write_all(&line)
        .map_err(|source| io("appending to", &path, source))
}

/// Append one event at the current revision, for a change that is not part
/// of the catalog (a launcher that exited). The caller holds the home's
/// mutation lock.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub fn record_event(settings: &SettingsRoot, event: &HomeEvent) -> Result<(), StateError> {
    append(settings, event, super::revision(settings))
}

/// A reader of the journal from a position, following one rotation between
/// two reads.
///
/// ```
/// use vibe_app::{EventKind, EventTail, HomeEvent, SettingsRoot};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// let mut tail = EventTail::at_end(&settings);
/// assert!(tail.read(&settings).is_empty());
/// let _lock = settings.lock().unwrap();
/// vibe_app::record_event(&settings, &HomeEvent::new(EventKind::CollectionsChanged)).unwrap();
/// let lines = tail.read(&settings);
/// assert_eq!(lines.len(), 1);
/// assert_eq!(lines[0].protocol, "vibe-app-events/1");
/// assert!(tail.read(&settings).is_empty(), "a line is read once");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub struct EventTail {
    offset: u64,
    /// The first line of the file being read. Every line carries its
    /// revision and a millisecond time, so a file that starts differently
    /// is a new file: the journal was rotated. Comparing lengths would not
    /// tell, since the new file may already be longer than the position.
    first: Option<Vec<u8>>,
}

impl EventTail {
    /// A reader that sees only what is appended from now on.
    pub fn at_end(settings: &SettingsRoot) -> Self {
        let path = settings.events_path();
        Self {
            offset: fs::metadata(&path).map_or(0, |meta| meta.len()),
            first: first_line(&path),
        }
    }

    /// Every whole line appended since the last read. After a rotation the
    /// rest of the old file comes first, then the new one from its start.
    pub fn read(&mut self, settings: &SettingsRoot) -> Vec<ApplicationEvent> {
        let path = settings.events_path();
        let first = first_line(&path);
        let mut events = Vec::new();
        if self.first.is_some() && first != self.first {
            let (old, _) = read_from(&rotated(&path), self.offset);
            events.extend(old);
            self.offset = 0;
        }
        self.first = first;
        let (new, consumed) = read_from(&path, self.offset);
        self.offset += consumed;
        if self.first.is_none() && consumed > 0 {
            self.first = first_line(&path);
        }
        events.extend(new);
        events
    }
}

/// The first whole line of `path`, or `None` while it has none.
fn first_line(path: &Path) -> Option<Vec<u8>> {
    let mut head = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(64 * 1024)
        .read_to_end(&mut head)
        .ok()?;
    let end = head.iter().position(|byte| *byte == b'\n')?;
    head.truncate(end);
    Some(head)
}

/// The whole lines of `path` from `offset`, and how many bytes they took; a
/// trailing partial line is left for the next read.
fn read_from(path: &Path, offset: u64) -> (Vec<ApplicationEvent>, u64) {
    let Ok(mut file) = fs::File::open(path) else {
        return (Vec::new(), 0);
    };
    let mut bytes = Vec::new();
    if file.seek(SeekFrom::Start(offset)).is_err() || file.read_to_end(&mut bytes).is_err() {
        return (Vec::new(), 0);
    }
    let whole = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |last| last + 1);
    let events = bytes[..whole]
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect();
    (events, whole as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_journal_rotates_at_its_bound_and_a_tail_follows_the_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        fs::create_dir_all(settings.applications_dir()).unwrap();
        let mut tail = EventTail::at_end(&settings);
        let event = |n: u64| HomeEvent::about(EventKind::Started, format!("k#{n}"));
        let size = serde_json::to_vec(&event(0).line(0)).unwrap().len() as u64 + 1;
        let bound = size * 4;
        for n in 0..3 {
            append_bounded(&settings, &event(n), n, bound).unwrap();
        }
        let first = tail.read(&settings);
        assert_eq!(first.len(), 3);
        // The next appends pass the bound: the file rotates under the tail,
        // and the tail still sees every line once, in order.
        for n in 3..8 {
            append_bounded(&settings, &event(n), n, bound).unwrap();
        }
        assert!(rotated(&settings.events_path()).is_file(), "it rotated");
        assert!(fs::metadata(settings.events_path()).unwrap().len() <= bound);
        let rest: Vec<u64> = tail.read(&settings).iter().map(|e| e.revision).collect();
        assert_eq!(rest, vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn a_partial_line_waits_and_a_damaged_line_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        fs::create_dir_all(settings.applications_dir()).unwrap();
        let mut tail = EventTail::at_end(&settings);
        let whole = serde_json::to_string(&HomeEvent::new(EventKind::Removed).line(9)).unwrap();
        fs::write(
            settings.events_path(),
            format!("{{damaged\n{whole}\n{}", &whole[..10]),
        )
        .unwrap();
        let read = tail.read(&settings);
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].revision, 9);
        let mut file = OpenOptions::new()
            .append(true)
            .open(settings.events_path())
            .unwrap();
        writeln!(file, "{}", &whole[10..]).unwrap();
        assert_eq!(tail.read(&settings).len(), 1, "the finished line arrives");
    }
}
