//! Durable commit/rollback intent publication.

use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::{NamedTempFile, TempPath};

use crate::WorkspaceError;

use super::journal::{
    Journal, JournalMode, StageRole, ensure_journal_matches, read_journal, stage_name,
    validate_journal, validate_owned_stage_name, validate_twins,
};
use super::{
    io_error, journal_path, remove_regular_if_exists, rollback_journal_path, sync_directory,
};

pub(super) fn persist_new_journal(parent: &Path, journal: &Journal) -> Result<(), WorkspaceError> {
    validate_journal(journal, &journal_path(parent), JournalMode::Commit)?;
    let temp = stage_journal(parent, journal, StageRole::CommitIntent)?;
    let target = journal_path(parent);
    let file = persist_noclobber_owned(temp, &target)?;
    file.sync_all().map_err(|error| io_error(&target, error))?;
    drop(file);
    sync_directory(parent)?;
    let actual = read_journal(&target, JournalMode::Commit)?
        .ok_or_else(|| io_error(&target, "new commit intent disappeared"))?;
    if actual == *journal {
        Ok(())
    } else {
        Err(io_error(
            &target,
            "new commit intent changed after publication",
        ))
    }
}

pub(super) fn arm_rollback(
    parent: &Path,
    commit: &Journal,
    rollback_journal: &Journal,
) -> Result<(), WorkspaceError> {
    ensure_journal_matches(parent, commit, JournalMode::Commit)?;
    validate_journal(
        rollback_journal,
        &rollback_journal_path(parent),
        JournalMode::Rollback,
    )?;
    validate_twins(commit, rollback_journal, &rollback_journal_path(parent))?;
    let temp = stage_journal(parent, rollback_journal, StageRole::RollbackIntent)?;
    let rollback = rollback_journal_path(parent);
    let file = persist_noclobber_owned(temp, &rollback)?;
    file.sync_all()
        .map_err(|error| io_error(&rollback, error))?;
    drop(file);
    sync_directory(parent)?;
    let actual = read_journal(&rollback, JournalMode::Rollback)?
        .ok_or_else(|| io_error(&rollback, "new rollback intent disappeared"))?;
    validate_twins(commit, &actual, &rollback)?;
    ensure_journal_matches(parent, commit, JournalMode::Commit)?;
    remove_regular_if_exists(&journal_path(parent))?;
    sync_directory(parent)
}

fn stage_journal(
    parent: &Path,
    journal: &Journal,
    role: StageRole,
) -> Result<NamedTempFile, WorkspaceError> {
    let bytes = toml::to_string(journal)
        .map_err(|error| io_error(&journal_path(parent), error))?
        .into_bytes();
    let name = stage_name(&journal.transaction, role);
    let path = parent.join(&name);
    validate_owned_stage_name(&name, Some(&journal.transaction), &path)?;
    let mut file =
        crate::safe_file::create_new_read_write(&path).map_err(|error| io_error(&path, error))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| io_error(&path, error))?;
    let temp_path = TempPath::try_from_path(path).map_err(|error| io_error(parent, error))?;
    Ok(NamedTempFile::from_parts(file, temp_path))
}

fn persist_noclobber_owned(temp: NamedTempFile, target: &Path) -> Result<fs::File, WorkspaceError> {
    match temp.persist_noclobber(target) {
        Ok(file) => Ok(file),
        Err(error) => {
            let primary = error.error.to_string();
            let staged = error.file.path().to_path_buf();
            let keep = error.file.keep().err().map(|error| error.error.to_string());
            let reason = match keep {
                Some(keep) => {
                    format!("intent publish failed: {primary}; preserving stage failed: {keep}")
                }
                None => format!(
                    "intent publish failed: {primary}; stage retained at `{}`",
                    staged.display()
                ),
            };
            Err(io_error(target, reason))
        }
    }
}
