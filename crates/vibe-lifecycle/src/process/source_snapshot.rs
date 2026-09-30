//! Read-only, bounded witness of the selected project's manifest and lock.
//!
//! This binds only these two files. It does not witness handler executables,
//! the installed world, or provide an atomic snapshot against hostile writers.

use std::path::Path;

use thiserror::Error;
use vibe_safefs::Project;

const SOURCE_FILE_CAP: usize = 8 * 1024 * 1024;

/// Exact bytes and their independently inspectable identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedFileWitness {
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub size: u64,
}

/// The selected project's `vibe.toml` and optionally present `vibe.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedManifestLockWitness {
    pub manifest: SelectedFileWitness,
    pub lock: Option<SelectedFileWitness>,
}

#[derive(Debug, Error)]
pub enum SourceSnapshotError {
    #[error("opening the selected project for a manifest/lock witness failed: {0}")]
    Open(String),
    #[error("reading selected `{name}` for a manifest/lock witness failed: {reason}")]
    Read { name: &'static str, reason: String },
    #[error("selected `vibe.toml` is absent")]
    MissingManifest,
    #[error("selected `{name}` changed since its manifest/lock witness")]
    Drift { name: &'static str },
}

/// Capture both selected files without creating a directory or state file.
pub fn capture_selected_manifest_lock(
    selected_root: &Path,
) -> Result<SelectedManifestLockWitness, SourceSnapshotError> {
    let project = Project::open(selected_root)
        .map_err(|error| SourceSnapshotError::Open(error.to_string()))?;
    let manifest = read(&project, "vibe.toml")?.ok_or(SourceSnapshotError::MissingManifest)?;
    let lock = read(&project, "vibe.lock")?;
    Ok(SelectedManifestLockWitness { manifest, lock })
}

/// Detect changes before a later effect through the same bounded read path.
///
/// This is a cooperating-writer check, not a content CAS across the interval
/// between this check and a later effect.
pub fn recheck_selected_manifest_lock(
    selected_root: &Path,
    expected: &SelectedManifestLockWitness,
) -> Result<(), SourceSnapshotError> {
    let observed = capture_selected_manifest_lock(selected_root)?;
    if observed.manifest != expected.manifest {
        return Err(SourceSnapshotError::Drift { name: "vibe.toml" });
    }
    if observed.lock != expected.lock {
        return Err(SourceSnapshotError::Drift { name: "vibe.lock" });
    }
    Ok(())
}

fn read(
    project: &Project,
    name: &'static str,
) -> Result<Option<SelectedFileWitness>, SourceSnapshotError> {
    project
        .read_file_snapshot_bounded(name, SOURCE_FILE_CAP)
        .map(|snapshot| {
            snapshot.map(|snapshot| SelectedFileWitness {
                bytes: snapshot.bytes,
                sha256: format!("sha256:{}", snapshot.sha256),
                size: snapshot.size,
            })
        })
        .map_err(|source| SourceSnapshotError::Read {
            name,
            reason: source.to_string(),
        })
}
