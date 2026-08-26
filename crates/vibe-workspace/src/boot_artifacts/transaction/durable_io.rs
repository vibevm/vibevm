//! Durable no-follow file primitives and bounded owned-stage reclamation.

use std::collections::BTreeSet;
use std::fs;
#[cfg(unix)]
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::WorkspaceError;
use crate::safe_file;

use super::journal::{
    Journal, STAGE_PREFIX, STAGE_SUFFIX, owned_stage_transaction, validate_owned_stage_name,
};
use super::{JOURNAL_NAME, ROLLBACK_JOURNAL_NAME};

const ORPHAN_MIN_AGE: Duration = Duration::from_secs(60 * 60);

pub(super) fn bytes_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(64);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

pub(super) fn journal_path(parent: &Path) -> PathBuf {
    parent.join(JOURNAL_NAME)
}

pub(super) fn rollback_journal_path(parent: &Path) -> PathBuf {
    parent.join(ROLLBACK_JOURNAL_NAME)
}

#[cfg(unix)]
pub(super) fn sync_directory(path: &Path) -> Result<(), WorkspaceError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| io_error(path, error))
}

#[cfg(not(unix))]
pub(super) fn sync_directory(_path: &Path) -> Result<(), WorkspaceError> {
    Ok(())
}

pub(super) fn read_regular_optional(path: &Path) -> Result<Option<Vec<u8>>, WorkspaceError> {
    safe_file::read_optional(path).map_err(|error| io_error(path, error))
}

pub(super) fn remove_regular_if_exists(path: &Path) -> Result<bool, WorkspaceError> {
    safe_file::preflight_absent_or_regular(path).map_err(|error| io_error(path, error))?;
    let file = match safe_file::open_existing_read(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(io_error(path, error)),
    };
    let identity = safe_file::identity(&file).map_err(|error| io_error(path, error))?;
    safe_file::assert_path_identity(path, identity).map_err(|error| io_error(path, error))?;
    fs::remove_file(path).map_err(|error| io_error(path, error))?;
    Ok(true)
}

pub(super) fn remove_owned_stage(
    parent: &Path,
    name: &str,
    transaction: Option<&str>,
) -> Result<bool, WorkspaceError> {
    let path = parent.join(name);
    validate_owned_stage_name(name, transaction, &path)?;
    remove_regular_if_exists(&path)
}

pub(super) fn discard_assets(parent: &Path, journal: &Journal) -> Vec<WorkspaceError> {
    let mut errors = Vec::new();
    for state in [
        &journal.index.before,
        &journal.index.after,
        &journal.selected.before,
        &journal.selected.after,
        &journal.stale.before,
    ] {
        if let Some(name) = &state.staged
            && let Err(error) = remove_owned_stage(parent, name, Some(&journal.transaction))
        {
            errors.push(error);
        }
    }
    errors
}

pub(super) fn preflight_journal_paths(
    parent: &Path,
    journal: &Journal,
) -> Result<(), WorkspaceError> {
    for target in [
        &journal.index.target,
        &journal.selected.target,
        &journal.stale.target,
    ] {
        preflight_optional_file(&parent.join(target))?;
    }
    for state in [
        &journal.index.before,
        &journal.index.after,
        &journal.selected.before,
        &journal.selected.after,
        &journal.stale.before,
    ] {
        if let Some(name) = &state.staged {
            preflight_optional_file(&parent.join(name))?;
        }
    }
    Ok(())
}

fn preflight_optional_file(path: &Path) -> Result<(), WorkspaceError> {
    safe_file::preflight_absent_or_regular(path).map_err(|error| io_error(path, error))?;
    match safe_file::open_existing_read(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(path, error)),
    }
}

pub(super) fn cleanup_transaction_stages(parent: &Path, transaction: &str) -> Vec<WorkspaceError> {
    let mut errors = Vec::new();
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) => return vec![io_error(parent, error)],
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(io_error(parent, error));
                continue;
            }
        };
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if owned_stage_transaction(&name) == Some(transaction)
            && let Err(error) = remove_owned_stage(parent, &name, Some(transaction))
        {
            errors.push(error);
        }
    }
    errors
}

pub(super) fn sweep_orphan_stages(
    parent: &Path,
    current_transactions: &BTreeSet<String>,
) -> Result<(), WorkspaceError> {
    sweep_orphan_stages_at(parent, current_transactions, SystemTime::now())
}

pub(super) fn sweep_orphan_stages_at(
    parent: &Path,
    current_transactions: &BTreeSet<String>,
    now: SystemTime,
) -> Result<(), WorkspaceError> {
    for entry in fs::read_dir(parent).map_err(|error| io_error(parent, error))? {
        let entry = entry.map_err(|error| io_error(parent, error))?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !name.starts_with(STAGE_PREFIX) || !name.ends_with(STAGE_SUFFIX) {
            continue;
        }
        let Some(transaction) = owned_stage_transaction(&name) else {
            continue;
        };
        if current_transactions.contains(transaction) {
            continue;
        }
        let path = parent.join(&name);
        validate_owned_stage_name(&name, None, &path)?;
        safe_file::preflight_absent_or_regular(&path).map_err(|error| io_error(&path, error))?;
        let file = safe_file::open_existing_read(&path).map_err(|error| io_error(&path, error))?;
        let modified = file
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map_err(|error| io_error(&path, error))?;
        let age = now.duration_since(modified).unwrap_or(Duration::ZERO);
        if age >= ORPHAN_MIN_AGE {
            remove_owned_stage(parent, &name, None)?;
        }
    }
    Ok(())
}

pub(super) fn aggregate_error(
    parent: &Path,
    primary: WorkspaceError,
    secondary: Vec<WorkspaceError>,
    phase: &str,
) -> WorkspaceError {
    let listed = secondary
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" | ");
    io_error(
        &journal_path(parent),
        format!("primary failure: {primary}; {phase} failures: {listed}"),
    )
}

pub(super) fn io_error(path: &Path, reason: impl ToString) -> WorkspaceError {
    WorkspaceError::Io {
        path: PathBuf::from(path),
        reason: reason.to_string(),
    }
}
