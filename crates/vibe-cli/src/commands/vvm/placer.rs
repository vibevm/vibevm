//! Placing a built distribution into a new immutable instance by diff-copy
//! (PROP-019 §2.15): hardlink unchanged files from the previous instance,
//! copy only what changed. Essential binaries are always content-hashed;
//! future large optional assets may use the bounded size/mtime policy.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#instances");

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;

use super::model::VersionId;
use super::store::{BINARY_NAME, INDEX_BINARY_NAME, VersionStore, open_regular_no_follow};

#[path = "placer_integrity.rs"]
mod integrity;
#[cfg(test)]
use integrity::manifest_shape_valid;
use integrity::{actual_matches, safe_reuse};
pub(crate) use integrity::{installed_files_match, matches_on_disk};

/// The placement layer's failure surface (PROP-019 §2.15): statting a built
/// file, copying it into the new instance, or preparing/publishing the
/// instance layout.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#instances")]
pub(crate) enum PlaceError {
    #[error(
        "statting distribution file `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
          fix: ensure the freshly-built distribution is readable)"
    )]
    Stat {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "copying `{from}` → `{to}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
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
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
          fix: ensure the instance root is writable)"
    )]
    Layout {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "serialising the instance manifest failed: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
          fix: report this — the manifest is malformed)"
    )]
    Serialise { detail: String },

    #[error(
        "refusing to overwrite immutable local instance `{path}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
          fix: allocate a fresh terminal #N instance)"
    )]
    InstanceExists { path: PathBuf },
}

/// Non-essential files at or below this size are content-hashed; larger
/// optional files may use `(size, mtime)`. Essential binaries always hash.
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

    pub(crate) fn content_hash_for(&self, rel: &str) -> Option<&str> {
        self.get(rel)?.hash.as_deref()
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

fn content_hash(path: &Path) -> io::Result<String> {
    let (mut file, _) = open_regular_no_follow(path)?;
    let mut h = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        h.update(&buffer[..read]);
    }
    Ok(format!("{:x}", h.finalize()))
}

fn is_essential_binary(rel: &str) -> bool {
    Path::new(rel)
        .file_name()
        .is_some_and(|name| name == BINARY_NAME || name == INDEX_BINARY_NAME)
}

fn should_hash(rel: &str, size: u64) -> bool {
    is_essential_binary(rel) || size <= SMALL_FILE_MAX
}

/// Compute a manifest entry for a distribution file (PROP-019 §2.15).
fn entry_for(src: &Path, rel: &str) -> Result<FileEntry, PlaceError> {
    let meta = fs::metadata(src).map_err(|source| PlaceError::Stat {
        path: src.to_path_buf(),
        source,
    })?;
    let size = meta.len();
    let hash = if should_hash(rel, size) {
        Some(content_hash(src).map_err(|source| PlaceError::Stat {
            path: src.to_path_buf(),
            source,
        })?)
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
    let path = instance_dir.join(MANIFEST_NAME);
    let (mut file, metadata) = open_regular_no_follow(&path).ok()?;
    if metadata.len() > 4 * 1024 * 1024 {
        return None;
    }
    let mut text = String::with_capacity(metadata.len() as usize);
    file.read_to_string(&mut text).ok()?;
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

fn guard(store: &VersionStore, path: &Path) -> Result<(), PlaceError> {
    store
        .guard_mutation_path(path)
        .map_err(|error| PlaceError::Layout {
            path: path.to_path_buf(),
            source: io::Error::other(error),
        })
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
    let final_dir = store.instance_dir(id, instance);
    guard(store, &final_dir)?;
    if final_dir.exists() {
        return Err(PlaceError::InstanceExists { path: final_dir });
    }
    let staging = store.version_id_dir(id).join(".staging");
    store
        .guard_mutation_tree(&staging)
        .map_err(|error| PlaceError::Layout {
            path: staging.clone(),
            source: io::Error::other(error),
        })?;
    if staging.exists() {
        remove_tree(&staging).map_err(|source| PlaceError::Layout {
            path: staging.clone(),
            source,
        })?;
    }
    let staging_parent = staging.parent().expect("version staging has a parent");
    guard(store, staging_parent)?;
    fs::create_dir_all(staging_parent).map_err(|source| PlaceError::Layout {
        path: staging_parent.to_path_buf(),
        source,
    })?;
    fs::create_dir(&staging).map_err(|source| PlaceError::Layout {
        path: staging.clone(),
        source,
    })?;
    if let Some((previous, _)) = prev {
        store
            .guard_mutation_tree(previous)
            .map_err(|error| PlaceError::Layout {
                path: previous.to_path_buf(),
                source: io::Error::other(error),
            })?;
    }

    for (src, rel) in dist {
        let dest = staging.join(rel);
        if let Some(parent) = dest.parent() {
            guard(store, parent)?;
            fs::create_dir_all(parent).map_err(|source| PlaceError::Layout {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let reuse = prev.and_then(|(pdir, pman)| {
            let new_e = manifest.get(rel)?;
            let prev_e = pman.get(rel)?;
            let prev_file = pdir.join(rel);
            safe_reuse(new_e, prev_e, &prev_file).then_some(prev_file)
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
        let expected = manifest
            .get(rel)
            .expect("distribution manifest covers every file");
        if !actual_matches(expected, &dest) {
            return Err(PlaceError::Copy {
                from: src.clone(),
                to: dest,
                source: io::Error::new(
                    io::ErrorKind::InvalidData,
                    "placed bytes do not match the pre-copy manifest",
                ),
            });
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

    if let Some(parent) = final_dir.parent() {
        guard(store, parent)?;
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

/// Publish a fully verified bundle staging directory as one immutable local
/// instance. Verification/extraction belongs to the bundle consumer; this
/// seam provides the same no-overwrite and transient-lock-safe rename as the
/// source-build placer.
pub(crate) fn publish_staged_instance(
    store: &VersionStore,
    id: &VersionId,
    instance: u64,
    staging: &Path,
) -> Result<PathBuf, PlaceError> {
    let final_dir = store.instance_dir(id, instance);
    store
        .guard_mutation_tree(staging)
        .map_err(|error| PlaceError::Layout {
            path: staging.to_path_buf(),
            source: io::Error::other(error),
        })?;
    guard(store, &final_dir)?;
    if final_dir.exists() {
        return Err(PlaceError::InstanceExists { path: final_dir });
    }
    if let Some(parent) = final_dir.parent() {
        guard(store, parent)?;
        fs::create_dir_all(parent).map_err(|source| PlaceError::Layout {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    rename_into_place(staging, &final_dir).map_err(|source| PlaceError::Layout {
        path: final_dir.clone(),
        source,
    })?;
    Ok(final_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::vvm::model::Kind;
    use crate::commands::vvm::store::BINARY_NAME;
    use specmark::verifies;

    fn write_test_binary(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).unwrap();
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
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
        assert!(should_hash(
            &format!("bin/{BINARY_NAME}"),
            SMALL_FILE_MAX + 1
        ));
        assert!(should_hash(
            &format!("bin/{INDEX_BINARY_NAME}"),
            SMALL_FILE_MAX + 1
        ));
        assert!(!should_hash("assets/large.bin", SMALL_FILE_MAX + 1));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
    fn place_creates_an_instance_with_a_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let id = VersionId::new(Kind::Branch, "main");
        let built = tmp.path().join("built-vibe");
        write_test_binary(&built, b"BIN");
        let dist = vec![(built.clone(), BINARY_NAME.to_string())];
        let manifest = manifest_for(&dist).unwrap();

        place(&store, &id, 1, &dist, &manifest, None).unwrap();
        let inst = store.instance_dir(&id, 1);
        assert_eq!(fs::read(inst.join(BINARY_NAME)).unwrap(), b"BIN");
        assert!(inst.join(MANIFEST_NAME).is_file());
        assert!(read_manifest(&inst).is_some());

        // Corrupting the previous bytes must not propagate through a hardlink.
        fs::write(inst.join(BINARY_NAME), b"CORRUPT").unwrap();
        let fresh = tmp.path().join("fresh-vibe");
        write_test_binary(&fresh, b"BIN");
        let dist2 = vec![(fresh, BINARY_NAME.to_string())];
        let m2 = manifest_for(&dist2).unwrap();
        place(&store, &id, 2, &dist2, &m2, Some((&inst, &manifest))).unwrap();
        assert_eq!(
            fs::read(store.instance_dir(&id, 2).join(BINARY_NAME)).unwrap(),
            b"BIN"
        );
        assert_eq!(fs::read(inst.join(BINARY_NAME)).unwrap(), b"CORRUPT");
        assert!(!matches_on_disk(&store, &m2, &manifest, &inst));

        // A reused terminal #N is immutable even if state bookkeeping was
        // damaged; placement never deletes or rewrites the old payload.
        write_test_binary(&built, b"REPLACEMENT");
        let replacement = vec![(built, BINARY_NAME.to_string())];
        let replacement_manifest = manifest_for(&replacement).unwrap();
        let error = place(&store, &id, 1, &replacement, &replacement_manifest, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("immutable local instance"));
        assert_eq!(fs::read(inst.join(BINARY_NAME)).unwrap(), b"CORRUPT");
    }

    /// `is_transient_lock` recognises the Windows `ERROR_ACCESS_DENIED` (5)
    /// and the kind-level `PermissionDenied` — the scanner-held-handle races
    /// the publish rename / staging cleanup retry on — and not other errors.
    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
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
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
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

    #[test]
    fn installed_manifest_shape_rejects_traversal_duplicates_and_extras() {
        let entry = |rel: &str| FileEntry {
            rel: rel.into(),
            size: 1,
            mtime_nanos: 0,
            hash: Some("a".repeat(64)),
        };
        let valid = Manifest {
            files: vec![entry(BINARY_NAME)],
        };
        assert!(manifest_shape_valid(
            super::super::model::Origin::Binary,
            &valid
        ));
        for files in [
            vec![entry("../vibe")],
            vec![entry(BINARY_NAME), entry("extra")],
            vec![entry(BINARY_NAME), entry(BINARY_NAME)],
        ] {
            assert!(!manifest_shape_valid(
                super::super::model::Origin::Binary,
                &Manifest { files }
            ));
        }
    }

    #[test]
    fn source_change_between_manifest_and_copy_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(temp.path().join("opt"));
        let source = temp.path().join("vibe");
        write_test_binary(&source, b"first");
        let dist = vec![(source.clone(), BINARY_NAME.to_string())];
        let manifest = manifest_for(&dist).unwrap();
        write_test_binary(&source, b"second");
        let id = VersionId::new(Kind::Tag, "1.0.0");
        let error = place(&store, &id, 1, &dist, &manifest, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("placed bytes do not match"), "{error}");
        assert!(!store.instance_dir(&id, 1).exists());
    }
}
