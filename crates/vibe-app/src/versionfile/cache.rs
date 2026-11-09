//! The resolution cache of the version-file search,
//! `<home>/cache/version-files.json` (PROP-075 `#VERSION-FILE-FORMATS`:
//! "cached by the modification time and size of every file consulted").
//!
//! A hit needs the same directory, the same project root, the same declared
//! set, every file that existed still unchanged and every file that was
//! absent still absent — which is what makes a repeated `cd` between two
//! project directories two cache hits (PROP-075 `#ACTIVATION-HOOK`).
//!
//! It is a pure cache: a corrupt or unwritable document is ignored, never an
//! error, because a prompt hook must not fail.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_wire::generated::application_version_file_cache as wire;

use super::declared::Declared;
use super::formats::VersionRequest;
use super::search::{Consulted, Found, Search, search};
use crate::settings::SettingsRoot;

/// The protocol tag of the cache document.
pub const CACHE_PROTOCOL: &str = "vibe-app-version-file-cache/1";

/// How many directories the cache remembers; the least recently used entry
/// is dropped first, so a long-lived shell cannot grow it without bound.
const CAPACITY: usize = 64;

/// `<home>/cache/version-files.json`.
pub fn path(settings: &SettingsRoot) -> PathBuf {
    settings.version_files_cache_path()
}

/// Resolve the version files for `from`, reading the cache when it still
/// describes the filesystem and refreshing it otherwise.
pub fn resolve(
    settings: &SettingsRoot,
    from: &Path,
    stop_at: Option<&Path>,
    declared: &Declared,
) -> Search {
    let fingerprint = declared.fingerprint();
    let mut document = load(settings);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let position = document.entries.iter().position(|entry| {
        entry.directory == from.to_string_lossy()
            && entry.stop.as_deref() == stop_at.map(|stop| stop.to_string_lossy()).as_deref()
            && entry.declared == fingerprint
    });
    if let Some(position) = position
        && let Some(hit) = from_wire(&document.entries[position])
        && hit.consulted.iter().all(Consulted::unchanged)
    {
        document.entries[position].used = now;
        store(settings, &document);
        return hit;
    }
    let fresh = search(from, stop_at, declared);
    let mut entry = to_wire(from, stop_at, &fingerprint, &fresh);
    entry.used = now;
    match position {
        Some(position) => document.entries[position] = entry,
        None => document.entries.push(entry),
    }
    if document.entries.len() > CAPACITY {
        document.entries.sort_by(|a, b| a.used.cmp(&b.used));
        let excess = document.entries.len() - CAPACITY;
        document.entries.drain(..excess);
    }
    store(settings, &document);
    fresh
}

/// Forget every cached resolution of this home, for a `vibe cache`-style
/// sweep and for a test that wants a cold read.
pub fn clear(settings: &SettingsRoot) {
    let _ = fs::remove_file(path(settings));
}

fn load(settings: &SettingsRoot) -> wire::ApplicationVersionFileCache {
    let empty = || wire::ApplicationVersionFileCache {
        protocol: CACHE_PROTOCOL.to_owned(),
        entries: Vec::new(),
    };
    let Ok(bytes) = fs::read(path(settings)) else {
        return empty();
    };
    match serde_json::from_slice::<wire::ApplicationVersionFileCache>(&bytes) {
        Ok(document) if document.protocol == CACHE_PROTOCOL => document,
        _ => empty(),
    }
}

/// Best-effort write: a cache that cannot be written costs a re-read, never
/// a failure.
fn store(settings: &SettingsRoot, document: &wire::ApplicationVersionFileCache) {
    let file = path(settings);
    let Some(parent) = file.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(mut bytes) = serde_json::to_vec_pretty(document) else {
        return;
    };
    bytes.push(b'\n');
    let sibling = parent.join(format!(".version-files.{}.tmp", std::process::id()));
    if fs::write(&sibling, &bytes).is_ok() && fs::rename(&sibling, &file).is_err() {
        let _ = fs::remove_file(&sibling);
    }
}

fn to_wire(from: &Path, stop_at: Option<&Path>, fingerprint: &str, search: &Search) -> wire::Entry {
    wire::Entry {
        directory: from.to_string_lossy().into_owned(),
        stop: stop_at.map(|stop| stop.to_string_lossy().into_owned()),
        declared: fingerprint.to_owned(),
        consulted: search
            .consulted
            .iter()
            .map(|consulted| wire::Consulted {
                path: consulted.path.to_string_lossy().into_owned(),
                modified: consulted
                    .stamp
                    .map(|(modified, _)| modified)
                    .map(|modified| modified.to_string()),
                size: consulted.stamp.map(|(_, size)| size.to_string()),
            })
            .collect(),
        found: search
            .found
            .iter()
            .map(|found| wire::Selection {
                slot: found.slot.clone(),
                path: found.path.to_string_lossy().into_owned(),
                authored: found.request.authored.clone(),
                unused: found.unused.clone(),
            })
            .collect(),
        faults: search.faults.clone(),
        used: String::new(),
    }
}

/// A cached entry read back. `None` when any recorded field no longer parses
/// — a cache is never a reason to refuse.
fn from_wire(entry: &wire::Entry) -> Option<Search> {
    let mut found = Vec::new();
    for selection in &entry.found {
        found.push(Found {
            slot: selection.slot.clone(),
            path: PathBuf::from(&selection.path),
            request: VersionRequest::parse(&selection.authored).ok()?,
            unused: selection.unused.clone(),
        });
    }
    let mut consulted = Vec::new();
    for row in &entry.consulted {
        let stamp = match (&row.modified, &row.size) {
            (Some(modified), Some(size)) => Some((modified.parse().ok()?, size.parse().ok()?)),
            (None, None) => None,
            _ => return None,
        };
        consulted.push(Consulted {
            path: PathBuf::from(&row.path),
            stamp,
        });
    }
    Some(Search {
        found,
        consulted,
        faults: entry.faults.clone(),
    })
}
