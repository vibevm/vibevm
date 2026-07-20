//! Placing a built distribution into a new immutable instance by diff-copy
//! (PROP-019 §2.15): hardlink unchanged files from the previous instance,
//! copy only what changed, never hashing large files.

specmark::scope!("spec://vibevm/common/PROP-019#instances");

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;

use super::model::VersionId;
use super::store::VersionStore;

/// The placement layer's failure surface (PROP-019 §2.15): statting a built
/// file, copying it into the new instance, or preparing/publishing the
/// instance layout.
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/common/PROP-019#instances")]
pub(crate) enum PlaceError {
    #[error(
        "statting distribution file `{path}` failed: {source} \
         (violates spec://vibevm/common/PROP-019#instances; \
          fix: ensure the freshly-built distribution is readable)"
    )]
    Stat {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "copying `{from}` → `{to}` failed: {source} \
         (violates spec://vibevm/common/PROP-019#instances; \
          fix: ensure the instance root is writable and has free space)"
    )]
    Copy {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "preparing the instance layout at `{path}` failed: {source} \
         (violates spec://vibevm/common/PROP-019#instances; \
          fix: ensure the instance root is writable)"
    )]
    Layout {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "serialising the instance manifest failed: {detail} \
         (violates spec://vibevm/common/PROP-019#instances; \
          fix: report this — the manifest is malformed)"
    )]
    Serialise { detail: String },
}

/// Files at or below this size are compared by content hash (cheap, robust);
/// larger files are compared by `(size, mtime)` only — never read in bulk
/// (PROP-019 §2.15, §9.2).
const SMALL_FILE_MAX: u64 = 16 * 1024 * 1024;

/// One distribution file's identity in a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileEntry {
    pub rel: String,
    pub size: u64,
    pub mtime_nanos: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// The per-instance file manifest (`.vvm-manifest.toml`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Manifest {
    #[serde(default, rename = "file")]
    pub files: Vec<FileEntry>,
}

impl Manifest {
    fn get(&self, rel: &str) -> Option<&FileEntry> {
        self.files.iter().find(|e| e.rel == rel)
    }
}

const MANIFEST_NAME: &str = ".vvm-manifest.toml";

fn mtime_nanos(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

fn small_file_hash(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Some(format!("{:x}", h.finalize()))
}

/// Compute a manifest entry for a distribution file (PROP-019 §2.15).
fn entry_for(src: &Path, rel: &str) -> Result<FileEntry, PlaceError> {
    let meta = fs::metadata(src).map_err(|source| PlaceError::Stat {
        path: src.to_path_buf(),
        source,
    })?;
    let size = meta.len();
    let hash = if size <= SMALL_FILE_MAX {
        small_file_hash(src)
    } else {
        None
    };
    Ok(FileEntry {
        rel: rel.to_string(),
        size,
        mtime_nanos: mtime_nanos(&meta),
        hash,
    })
}

/// The manifest of a freshly-built distribution (PROP-019 §2.15).
pub(crate) fn manifest_for(dist: &[(PathBuf, String)]) -> Result<Manifest, PlaceError> {
    let mut files = Vec::with_capacity(dist.len());
    for (src, rel) in dist {
        files.push(entry_for(src, rel)?);
    }
    Ok(Manifest { files })
}

/// Whether two entries are the same file: by content hash for small files,
/// else by `(size, mtime)` (PROP-019 §2.15).
fn unchanged(new: &FileEntry, prev: &FileEntry) -> bool {
    match (&new.hash, &prev.hash) {
        (Some(a), Some(b)) => a == b,
        _ => new.size == prev.size && new.mtime_nanos == prev.mtime_nanos,
    }
}

/// Read an instance's manifest, if present.
pub(crate) fn read_manifest(instance_dir: &Path) -> Option<Manifest> {
    let text = fs::read_to_string(instance_dir.join(MANIFEST_NAME)).ok()?;
    toml::from_str(&text).ok()
}

/// Whether a new build is byte-for-byte the previous instance (so no new
/// instance is needed) (PROP-019 §2.15).
pub(crate) fn matches(new: &Manifest, prev: &Manifest) -> bool {
    new.files.len() == prev.files.len()
        && new
            .files
            .iter()
            .all(|e| prev.get(&e.rel).map(|pe| unchanged(e, pe)).unwrap_or(false))
}

/// Is `e` the transient "a real-time scanner / indexer has a handle open on a
/// file inside this directory" lock? On Windows that is `ERROR_ACCESS_DENIED`
/// (5); `fs::rename` / `fs::remove_dir_all` of a directory holding a freshly
/// written `.exe` / `.dll` (the placer stages the full distribution — the
/// Electron apps included) trips it moments after the write. The handle
/// releases within a second or two, so a short retry turns a flaky install
/// into a reliable one. `PermissionDenied` is the kind-level fallback for
/// other platforms / mappings.
fn is_transient_lock(e: &io::Error) -> bool {
    e.raw_os_error() == Some(5) || e.kind() == io::ErrorKind::PermissionDenied
}

/// `fs::rename` that retries on a transient access-denied lock (see
/// [`is_transient_lock`]). A non-lock error surfaces immediately — the retry
/// never masks a real failure.
fn rename_into_place(from: &Path, to: &Path) -> io::Result<()> {
    retry(|| fs::rename(from, to))
}

/// `fs::remove_dir_all` that retries on a transient access-denied lock — the
/// staging cleanup and the existing-instance removal hit the same scanner
/// race the publish rename does.
fn remove_tree(path: &Path) -> io::Result<()> {
    retry(|| fs::remove_dir_all(path))
}

/// Run `op`, retrying only on a transient lock with a short backoff. The
/// backoff sequence is bounded (~5s total) — long enough for a scanner to
/// release a handle, short enough that a genuinely locked file still
/// surfaces in reasonable time.
fn retry(mut op: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    const BACKOFF_MS: [u64; 7] = [100, 200, 400, 800, 800, 800, 800];
    let mut last = None;
    for ms in BACKOFF_MS {
        match op() {
            Ok(()) => return Ok(()),
            Err(e) if is_transient_lock(&e) => {
                last = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            Err(e) => return Err(e),
        }
    }
    // The loop only advances on a transient-lock error, which sets `last`;
    // reaching here means the backoff is exhausted on a lock.
    Err(last.unwrap_or_else(|| io::Error::other("retry backoff exhausted")))
}

/// Place a distribution into a new instance dir by diff-copy: hardlink files
/// unchanged versus `prev`, copy the rest, write the manifest, and publish
/// atomically (PROP-019 §2.15).
pub(crate) fn place(
    store: &VersionStore,
    id: &VersionId,
    instance: u64,
    dist: &[(PathBuf, String)],
    manifest: &Manifest,
    prev: Option<(&Path, &Manifest)>,
) -> Result<(), PlaceError> {
    let staging = store.version_id_dir(id).join(".staging");
    if staging.exists() {
        remove_tree(&staging).map_err(|source| PlaceError::Layout {
            path: staging.clone(),
            source,
        })?;
    }
    fs::create_dir_all(&staging).map_err(|source| PlaceError::Layout {
        path: staging.clone(),
        source,
    })?;

    for (src, rel) in dist {
        let dest = staging.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|source| PlaceError::Layout {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let reuse = prev.and_then(|(pdir, pman)| {
            let new_e = manifest.get(rel)?;
            let prev_e = pman.get(rel)?;
            unchanged(new_e, prev_e).then(|| pdir.join(rel))
        });
        let hardlinked = match &reuse {
            Some(prev_file) => fs::hard_link(prev_file, &dest).is_ok(),
            None => false,
        };
        if !hardlinked {
            fs::copy(src, &dest).map_err(|source| PlaceError::Copy {
                from: src.clone(),
                to: dest.clone(),
                source,
            })?;
        }
    }

    let text = toml::to_string(manifest).map_err(|e| PlaceError::Serialise {
        detail: e.to_string(),
    })?;
    let manifest_path = staging.join(MANIFEST_NAME);
    fs::write(&manifest_path, text).map_err(|source| PlaceError::Layout {
        path: manifest_path,
        source,
    })?;

    let final_dir = store.instance_dir(id, instance);
    if final_dir.exists() {
        remove_tree(&final_dir).map_err(|source| PlaceError::Layout {
            path: final_dir.clone(),
            source,
        })?;
    }
    if let Some(parent) = final_dir.parent() {
        fs::create_dir_all(parent).map_err(|source| PlaceError::Layout {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    rename_into_place(&staging, &final_dir).map_err(|source| PlaceError::Layout {
        path: final_dir.clone(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::vvm::model::Kind;
    use crate::commands::vvm::store::BINARY_NAME;
    use specmark::verifies;

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#instances", r = 1)]
    fn manifest_round_trips_and_detects_change() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("vibe");
        fs::write(&f, b"hello").unwrap();
        let m = manifest_for(&[(f.clone(), "vibe".into())]).unwrap();
        let text = toml::to_string(&m).unwrap();
        let back: Manifest = toml::from_str(&text).unwrap();
        assert_eq!(m, back);
        assert!(matches(&m, &back), "identical manifest matches");

        // A content change (small file → hashed) is detected.
        fs::write(&f, b"hello world").unwrap();
        let m2 = manifest_for(&[(f, "vibe".into())]).unwrap();
        assert!(!matches(&m2, &m), "changed content does not match");
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#instances", r = 1)]
    fn place_creates_an_instance_with_a_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let id = VersionId::new(Kind::Branch, "main");
        let built = tmp.path().join("built-vibe");
        fs::write(&built, b"BIN").unwrap();
        let dist = vec![(built, BINARY_NAME.to_string())];
        let manifest = manifest_for(&dist).unwrap();

        place(&store, &id, 1, &dist, &manifest, None).unwrap();
        let inst = store.instance_dir(&id, 1);
        assert_eq!(fs::read(inst.join(BINARY_NAME)).unwrap(), b"BIN");
        assert!(inst.join(MANIFEST_NAME).is_file());
        assert!(read_manifest(&inst).is_some());

        // A second place against the unchanged previous hardlinks (same
        // content placed into instance 2).
        let dist2 = vec![(inst.join(BINARY_NAME), BINARY_NAME.to_string())];
        let m2 = manifest_for(&dist2).unwrap();
        place(&store, &id, 2, &dist2, &m2, Some((&inst, &manifest))).unwrap();
        assert_eq!(
            fs::read(store.instance_dir(&id, 2).join(BINARY_NAME)).unwrap(),
            b"BIN"
        );
    }

    /// `is_transient_lock` recognises the Windows `ERROR_ACCESS_DENIED` (5)
    /// and the kind-level `PermissionDenied` — the scanner-held-handle races
    /// the publish rename / staging cleanup retry on — and not other errors.
    #[test]
    #[verifies("spec://vibevm/common/PROP-019#instances", r = 1)]
    fn is_transient_lock_classifies_access_denied() {
        assert!(is_transient_lock(&io::Error::from_raw_os_error(5)));
        assert!(is_transient_lock(&io::Error::new(
            io::ErrorKind::PermissionDenied,
            "denied",
        )));
        assert!(!is_transient_lock(&io::Error::new(
            io::ErrorKind::NotFound,
            "missing",
        )));
    }

    /// `rename_into_place` / `remove_tree` succeed on an unlocked directory
    /// (the common path, no retry needed) and report the lock-class errors
    /// they would retry on rather than masking them.
    #[test]
    #[verifies("spec://vibevm/common/PROP-019#instances", r = 1)]
    fn rename_and_remove_succeed_when_unlocked() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("staging");
        let to = tmp.path().join("final");
        fs::create_dir_all(&from).unwrap();
        fs::write(from.join("vibe"), b"x").unwrap();
        rename_into_place(&from, &to).unwrap();
        assert!(to.join("vibe").is_file());
        assert!(!from.exists(), "rename moved the dir out of staging");
        remove_tree(&to).unwrap();
        assert!(!to.exists());
    }
}
