//! Read-only, bounded witness of the selected project's manifest and lock.
//!
//! This binds only these two files. It does not witness handler executables,
//! the installed world, or provide an atomic snapshot against hostile writers.

use std::path::Path;

use sha2::{Digest, Sha256};
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

impl SelectedFileWitness {
    /// Build a self-consistent witness from caller-owned bytes without I/O.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            sha256: format!("sha256:{:x}", Sha256::digest(&bytes)),
            size: bytes.len() as u64,
            bytes,
        }
    }

    /// Check that public witness fields still describe these exact bytes.
    pub fn validate(&self) -> Result<(), SourceSnapshotError> {
        self.validate_named("file")
    }

    fn validate_named(&self, name: &'static str) -> Result<(), SourceSnapshotError> {
        let invalid = |reason: &str| SourceSnapshotError::InvalidWitness {
            name,
            reason: reason.to_owned(),
        };
        if self.bytes.len() > SOURCE_FILE_CAP || self.size != self.bytes.len() as u64 {
            return Err(invalid("size does not match bounded bytes"));
        }
        let Some(digest) = self.sha256.strip_prefix("sha256:") else {
            return Err(invalid("digest must use sha256 prefix"));
        };
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(invalid("digest must be 64 lowercase hex characters"));
        }
        if digest != format!("{:x}", Sha256::digest(&self.bytes)) {
            return Err(invalid("digest does not match bytes"));
        }
        Ok(())
    }
}

/// The selected project's `vibe.toml` and optionally present `vibe.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedManifestLockWitness {
    pub manifest: SelectedFileWitness,
    pub lock: Option<SelectedFileWitness>,
}

impl SelectedManifestLockWitness {
    /// Validate both present file witnesses without consulting the filesystem.
    pub fn validate(&self) -> Result<(), SourceSnapshotError> {
        self.manifest.validate_named("vibe.toml")?;
        if let Some(lock) = &self.lock {
            lock.validate_named("vibe.lock")?;
        }
        Ok(())
    }
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
    #[error("selected `{name}` has an invalid manifest/lock witness: {reason}")]
    InvalidWitness { name: &'static str, reason: String },
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
