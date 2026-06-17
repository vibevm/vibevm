//! Placing a built distribution into a new immutable instance by diff-copy
//! (PROP-019 §2.15): hardlink unchanged files from the previous instance,
//! copy only what changed, never hashing large files.

specmark::scope!("spec://vibevm/common/PROP-019#instances");

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::model::VersionId;
use super::store::VersionStore;

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
fn entry_for(src: &Path, rel: &str) -> Result<FileEntry> {
    let meta = fs::metadata(src).with_context(|| format!("statting `{}`", src.display()))?;
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
pub(crate) fn manifest_for(dist: &[(PathBuf, String)]) -> Result<Manifest> {
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
) -> Result<()> {
    let staging = store.version_id_dir(id).join(".staging");
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .with_context(|| format!("clearing `{}`", staging.display()))?;
    }
    fs::create_dir_all(&staging).with_context(|| format!("creating `{}`", staging.display()))?;

    for (src, rel) in dist {
        let dest = staging.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
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
            fs::copy(src, &dest)
                .with_context(|| format!("copying `{}` → `{}`", src.display(), dest.display()))?;
        }
    }

    let text = toml::to_string(manifest).context("serialising the instance manifest")?;
    fs::write(staging.join(MANIFEST_NAME), text).context("writing the instance manifest")?;

    let final_dir = store.instance_dir(id, instance);
    if final_dir.exists() {
        fs::remove_dir_all(&final_dir)
            .with_context(|| format!("clearing `{}`", final_dir.display()))?;
    }
    if let Some(parent) = final_dir.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&staging, &final_dir)
        .with_context(|| format!("publishing instance at `{}`", final_dir.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::man::model::Kind;
    use crate::commands::man::store::BINARY_NAME;
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
}
