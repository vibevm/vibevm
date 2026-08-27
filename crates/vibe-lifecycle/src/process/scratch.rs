//! Collision-resistant scratch allocation and unpublished create-new JSON files.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;

pub const CONTEXT_CAP: usize = 8 * 1024 * 1024;

#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub struct PendingReply {
    path: PathBuf,
    file: Option<File>,
    scratch: PathBuf,
}

impl PendingReply {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read_capped(&mut self, cap: usize) -> Result<Vec<u8>, ScratchError> {
        verify_regular_in(&self.path, &self.scratch)?;
        let mut file = open_nofollow(&self.path)?;
        let metadata = file.metadata().map_err(|source| ScratchError::Io {
            path: self.path.clone(),
            source,
        })?;
        if !metadata.is_file() || is_link_or_reparse(&metadata) {
            return Err(ScratchError::Unsafe {
                path: self.path.clone(),
                reason: "opened reply is not a regular non-reparse file".into(),
            });
        }
        file.seek(SeekFrom::Start(0))
            .map_err(|source| ScratchError::Io {
                path: self.path.clone(),
                source,
            })?;
        let mut bytes = Vec::with_capacity(cap + 1);
        (&mut file)
            .take((cap + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|source| ScratchError::Io {
                path: self.path.clone(),
                source,
            })?;
        Ok(bytes)
    }

    pub fn publish(&mut self) {
        drop(self.file.take());
    }

    pub fn consume(self) -> Result<(), ScratchError> {
        let path = self.path.clone();
        drop(self.file);
        fs::remove_file(&path).map_err(|source| ScratchError::Io { path, source })
    }
}

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub enum ScratchError {
    #[error(
        "lifecycle scratch under `{path}` is unsafe: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW; \
          fix: restore a canonical link-free project .vibe directory and rerun)"
    )]
    Unsafe { path: PathBuf, reason: String },
    #[error(
        "creating lifecycle scratch `{path}` failed: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW; \
          fix: make the selected project writable and rerun)"
    )]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "encoding lifecycle context failed: {0} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW; \
          fix: report this generated-wire serialization failure)"
    )]
    Encode(String),
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
pub fn allocate_run_id(project_root: &Path) -> Result<String, ScratchError> {
    let base = safe_base(project_root)?;
    for _ in 0..16 {
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|error| ScratchError::Unsafe {
            path: base.clone(),
            reason: format!("OS CSPRNG unavailable: {error}"),
        })?;
        let id = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let run = base.join(&id);
        match fs::create_dir(&run) {
            Ok(()) => return Ok(id),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(ScratchError::Io { path: run, source }),
        }
    }
    Err(ScratchError::Unsafe {
        path: base,
        reason: "16 CSPRNG run-id collisions".into(),
    })
}

/// The shape every durable run identity has: 32 lowercase hex characters.
/// Owned here because this module mints run ids; the state store, the outbox
/// path and adoption all judge identity through this one predicate.
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
pub fn is_valid_run_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
pub fn execution_scratch(
    project_root: &Path,
    run_id: &str,
    key: &str,
) -> Result<PathBuf, ScratchError> {
    if !is_valid_run_id(run_id) {
        return Err(ScratchError::Unsafe {
            path: project_root.into(),
            reason: "run id must be 32 lowercase hex characters".into(),
        });
    }
    let base = safe_base(project_root)?;
    let run = base.join(run_id);
    verify_component(&run)?;
    if !run.exists() {
        fs::create_dir(&run).map_err(|source| ScratchError::Io {
            path: run.clone(),
            source,
        })?;
    }
    let component = format!("{:x}", Sha256::digest(key.as_bytes()));
    let execution = run.join(component);
    if !execution.exists() {
        fs::create_dir(&execution).map_err(|source| ScratchError::Io {
            path: execution.clone(),
            source,
        })?;
    }
    verify_component(&execution)?;
    let canonical = execution
        .canonicalize()
        .map_err(|source| ScratchError::Io {
            path: execution.clone(),
            source,
        })?;
    let root = project_root
        .canonicalize()
        .map_err(|source| ScratchError::Io {
            path: project_root.into(),
            source,
        })?;
    if !canonical.starts_with(&root) {
        return Err(ScratchError::Unsafe {
            path: canonical,
            reason: "scratch escaped selected project".into(),
        });
    }
    Ok(canonical)
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
pub fn write_atomic_json<T: Serialize>(
    scratch: &Path,
    name: &str,
    value: &T,
) -> Result<PathBuf, ScratchError> {
    validate_genre(scratch, name)?;
    let bytes =
        serde_json::to_vec(value).map_err(|error| ScratchError::Encode(error.to_string()))?;
    if bytes.len() > CONTEXT_CAP {
        return Err(ScratchError::Unsafe {
            path: scratch.into(),
            reason: "context exceeds 8 MiB".into(),
        });
    }
    let scratch = verified_scratch_dir(scratch)?;
    let (final_path, mut file) = create_unique_file(&scratch, name)?;
    if let Err(source) = file
        .write_all(&bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
    {
        drop(file);
        let _ = fs::remove_file(&final_path);
        return Err(ScratchError::Io {
            path: final_path,
            source,
        });
    }
    drop(file);
    verify_regular_in(&final_path, &scratch)?;
    Ok(final_path)
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub fn allocate_pending_reply(scratch: &Path) -> Result<PendingReply, ScratchError> {
    let scratch = verified_scratch_dir(scratch)?;
    let (path, file) = create_unique_file(&scratch, "reply-pending")?;
    Ok(PendingReply {
        path,
        file: Some(file),
        scratch,
    })
}

fn open_nofollow(path: &Path) -> Result<File, ScratchError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options.open(path).map_err(|source| ScratchError::Io {
        path: path.into(),
        source,
    })
}

pub(super) fn create_unique_file(
    scratch: &Path,
    genre: &str,
) -> Result<(PathBuf, File), ScratchError> {
    validate_genre(scratch, genre)?;
    let scratch = verified_scratch_dir(scratch)?;
    for _ in 0..16 {
        let path = scratch.join(format!("{genre}-{}.tmp", random_hex(&scratch)?));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => {
                verify_regular_in(&path, &scratch)?;
                return Ok((path, file));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(ScratchError::Io { path, source }),
        }
    }
    Err(ScratchError::Unsafe {
        path: scratch,
        reason: "16 CSPRNG scratch-file collisions".into(),
    })
}

fn validate_genre(scratch: &Path, genre: &str) -> Result<(), ScratchError> {
    let mut components = Path::new(genre).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(ScratchError::Unsafe {
            path: scratch.join(genre),
            reason: "scratch file genre must be one Normal path component".into(),
        });
    }
    Ok(())
}

fn random_hex(path: &Path) -> Result<String, ScratchError> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|error| ScratchError::Unsafe {
        path: path.into(),
        reason: format!("OS CSPRNG unavailable: {error}"),
    })?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn verified_scratch_dir(scratch: &Path) -> Result<PathBuf, ScratchError> {
    verify_component(scratch)?;
    let canonical = scratch.canonicalize().map_err(|source| ScratchError::Io {
        path: scratch.into(),
        source,
    })?;
    let metadata = fs::symlink_metadata(&canonical).map_err(|source| ScratchError::Io {
        path: canonical.clone(),
        source,
    })?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err(ScratchError::Unsafe {
            path: canonical,
            reason: "scratch is not a regular link-free directory".into(),
        });
    }
    Ok(canonical)
}

fn verify_regular_in(path: &Path, scratch: &Path) -> Result<(), ScratchError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| ScratchError::Io {
        path: path.into(),
        source,
    })?;
    if !metadata.is_file() || is_link_or_reparse(&metadata) {
        return Err(ScratchError::Unsafe {
            path: path.into(),
            reason: "JSON path is not a regular non-reparse file".into(),
        });
    }
    if path.parent() != Some(scratch) {
        return Err(ScratchError::Unsafe {
            path: path.into(),
            reason: "JSON parent is not the verified scratch directory".into(),
        });
    }
    Ok(())
}

fn safe_base(project_root: &Path) -> Result<PathBuf, ScratchError> {
    let root = project_root
        .canonicalize()
        .map_err(|source| ScratchError::Io {
            path: project_root.into(),
            source,
        })?;
    let vibe = root.join(".vibe");
    if !vibe.exists() {
        fs::create_dir(&vibe).map_err(|source| ScratchError::Io {
            path: vibe.clone(),
            source,
        })?;
    }
    verify_component(&vibe)?;
    let base = vibe.join("lifecycle");
    if !base.exists() {
        fs::create_dir(&base).map_err(|source| ScratchError::Io {
            path: base.clone(),
            source,
        })?;
    }
    verify_component(&base)?;
    Ok(base)
}

fn verify_component(path: &Path) -> Result<(), ScratchError> {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && is_link_or_reparse(&metadata)
    {
        return Err(ScratchError::Unsafe {
            path: path.into(),
            reason: "symlink/reparse component".into(),
        });
    }
    Ok(())
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
}
