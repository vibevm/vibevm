//! Crash-recoverable INDEX/STATIC publication with a durable roll-forward intent.

use std::fs;
use std::path::{Path, PathBuf};

use crate::WorkspaceError;
use tempfile::{NamedTempFile, TempPath};

mod durable_io;
use durable_io::*;
mod journal;
use journal::*;
mod lock;
use lock::BootArtifactLock;
mod prepare;
use prepare::prepare_journal;
mod intent;
use intent::{arm_rollback, persist_new_journal};
mod selector;
pub(super) use selector::replace_selector;

const JOURNAL_NAME: &str = ".vibe-boot-artifacts.transaction.toml";
const ROLLBACK_JOURNAL_NAME: &str = ".vibe-boot-artifacts.rollback.toml";
const JOURNAL_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WritePoint {
    IndexWrite,
    StaticWrite,
    PostStagePreReplace,
    StaticReplace,
    PostStaticPreIndex,
    IndexReplace,
    PostIndexPreStaleCleanup,
    StaleRemove,
    RollbackStart,
    RollbackRestore,
    PostRollbackRestore,
}

pub(super) trait FaultInjector {
    fn check(&self, point: WritePoint, path: &Path) -> Result<(), WorkspaceError>;
}

pub(super) struct NoFault;

impl FaultInjector for NoFault {
    fn check(&self, _point: WritePoint, _path: &Path) -> Result<(), WorkspaceError> {
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(super) struct ArtifactWrite<'a> {
    pub(super) index_path: &'a Path,
    pub(super) index_bytes: &'a [u8],
    pub(super) static_path: &'a Path,
    pub(super) static_bytes: Option<&'a [u8]>,
    pub(super) stale_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentState {
    present: bool,
    digest: Option<String>,
}

fn absent_state() -> RecordedState {
    RecordedState {
        present: false,
        digest: None,
        staged: None,
    }
}

fn recorded_current(state: &RecordedState) -> CurrentState {
    CurrentState {
        present: state.present,
        digest: state.digest.clone(),
    }
}

#[cfg(test)]
pub(super) fn write_production(write: ArtifactWrite<'_>) -> Result<(), WorkspaceError> {
    write_production_with_selectors(write, |_| Ok(()))
}

pub(super) fn write_production_with_selectors<T>(
    write: ArtifactWrite<'_>,
    selectors: impl FnOnce(&str) -> Result<T, WorkspaceError>,
) -> Result<T, WorkspaceError> {
    write_with_faults_and_selectors(write, &NoFault, selectors)
}

pub(super) fn preflight_artifact_roles(write: ArtifactWrite<'_>) -> Result<(), WorkspaceError> {
    let _ = common_parent(&write)?;
    validate_artifact_roles(write.index_path, write.static_path, write.stale_path)
}

pub(super) fn with_boot_lock<T>(
    parent: &Path,
    action: impl FnOnce() -> Result<T, WorkspaceError>,
) -> Result<T, WorkspaceError> {
    fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    let lock = BootArtifactLock::acquire(parent)?;
    lock.assert_current()?;
    action()
}

#[cfg(test)]
pub(super) fn write_with_faults(
    write: ArtifactWrite<'_>,
    faults: &impl FaultInjector,
) -> Result<(), WorkspaceError> {
    write_with_faults_and_selectors(write, faults, |_| Ok(()))
}

fn write_with_faults_and_selectors<T>(
    write: ArtifactWrite<'_>,
    faults: &impl FaultInjector,
    selectors: impl FnOnce(&str) -> Result<T, WorkspaceError>,
) -> Result<T, WorkspaceError> {
    let parent = common_parent(&write)?;
    validate_artifact_roles(write.index_path, write.static_path, write.stale_path)?;
    fs::create_dir_all(parent).map_err(|error| io_error(write.index_path, error))?;
    let lock = BootArtifactLock::acquire(parent)?;
    lock.assert_current()?;
    write_with_faults_locked(write, parent, faults, &lock, selectors)
}

fn write_with_faults_locked<T>(
    write: ArtifactWrite<'_>,
    parent: &Path,
    faults: &impl FaultInjector,
    lock: &BootArtifactLock,
    selectors: impl FnOnce(&str) -> Result<T, WorkspaceError>,
) -> Result<T, WorkspaceError> {
    recover_pending_locked(parent, lock)?;
    let mut journal = prepare_journal(write, parent, faults)?;
    preflight_journal_paths(parent, &journal)?;
    if let Err(primary) = persist_new_journal(parent, &journal) {
        let cleanup = discard_assets(parent, &journal);
        return if cleanup.is_empty() {
            Err(primary)
        } else {
            Err(aggregate_error(
                parent,
                primary,
                cleanup,
                "unpublished-stage cleanup",
            ))
        };
    }

    if let Err(primary) = roll_forward_core(parent, &journal, faults) {
        let commit = journal.clone();
        journal.mode = JournalMode::Rollback;
        if let Err(update) = arm_rollback(parent, &commit, &journal) {
            return Err(aggregate_error(
                parent,
                primary,
                vec![update],
                "arming rollback",
            ));
        }
        if let Err(start) = faults.check(WritePoint::RollbackStart, &journal_path(parent)) {
            return Err(aggregate_error(
                parent,
                primary,
                vec![start],
                "starting rollback",
            ));
        }
        let rollback = rollback_all(parent, &journal, faults);
        if rollback.is_empty() {
            cleanup(parent, &journal)?;
            return Err(primary);
        } else {
            return Err(aggregate_error(parent, primary, rollback, "rollback"));
        }
    }
    let output = selectors(&journal.transaction)?;
    faults.check(
        WritePoint::PostIndexPreStaleCleanup,
        &target_path(parent, &journal.index)?,
    )?;
    faults.check(
        WritePoint::StaleRemove,
        &target_path(parent, &journal.stale)?,
    )?;
    apply_forward(parent, &journal.stale)?;
    cleanup(parent, &journal)?;
    Ok(output)
}

fn common_parent<'a>(write: &'a ArtifactWrite<'_>) -> Result<&'a Path, WorkspaceError> {
    let parent = write
        .index_path
        .parent()
        .ok_or_else(|| io_error(write.index_path, "INDEX has no parent directory"))?;
    for path in [write.static_path, write.stale_path] {
        if path.parent() != Some(parent) {
            return Err(io_error(
                path,
                "boot artifact paths do not share one directory",
            ));
        }
    }
    if write.index_path == write.static_path
        || write.index_path == write.stale_path
        || write.static_path == write.stale_path
    {
        return Err(io_error(
            write.index_path,
            "INDEX, selected STATIC, and stale STATIC must be distinct paths",
        ));
    }
    Ok(parent)
}

fn roll_forward_core(
    parent: &Path,
    journal: &Journal,
    faults: &impl FaultInjector,
) -> Result<(), WorkspaceError> {
    faults.check(WritePoint::PostStagePreReplace, &journal_path(parent))?;
    faults.check(
        WritePoint::StaticReplace,
        &target_path(parent, &journal.selected)?,
    )?;
    apply_forward(parent, &journal.selected)?;
    faults.check(
        WritePoint::PostStaticPreIndex,
        &target_path(parent, &journal.selected)?,
    )?;
    faults.check(
        WritePoint::IndexReplace,
        &target_path(parent, &journal.index)?,
    )?;
    apply_forward(parent, &journal.index)
}

fn apply_forward(parent: &Path, entry: &JournalEntry) -> Result<(), WorkspaceError> {
    let target = target_path(parent, entry)?;
    let current = current_state(&target)?;
    if current == recorded_current(&entry.after) {
        return Ok(());
    }
    if current != recorded_current(&entry.before) {
        return Err(third_state_error(
            &target,
            &entry.before,
            &entry.after,
            &current,
        ));
    }
    apply_recorded(parent, &target, &entry.after).map(|_| ())
}

fn rollback_all(
    parent: &Path,
    journal: &Journal,
    faults: &impl FaultInjector,
) -> Vec<WorkspaceError> {
    let mut errors = Vec::new();
    let static_entries = [&journal.stale, &journal.selected];
    let order = static_entries
        .iter()
        .copied()
        .filter(|entry| entry.before.present)
        .chain(std::iter::once(&journal.index))
        .chain(
            static_entries
                .iter()
                .copied()
                .filter(|entry| !entry.before.present),
        );
    for entry in order {
        let target = match target_path(parent, entry) {
            Ok(path) => path,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        if let Err(error) = faults.check(WritePoint::RollbackRestore, &target) {
            errors.push(error);
            continue;
        }
        match apply_rollback(parent, entry, &target) {
            Ok(mutated) => {
                if mutated
                    && let Err(error) = faults.check(WritePoint::PostRollbackRestore, &target)
                {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
    errors
}

fn apply_rollback(
    parent: &Path,
    entry: &JournalEntry,
    target: &Path,
) -> Result<bool, WorkspaceError> {
    let current = current_state(target)?;
    if current == recorded_current(&entry.before) {
        return Ok(false);
    }
    if current != recorded_current(&entry.after) {
        return Err(third_state_error(
            target,
            &entry.before,
            &entry.after,
            &current,
        ));
    }
    apply_recorded(parent, target, &entry.before)
}

fn apply_recorded(
    parent: &Path,
    target: &Path,
    desired: &RecordedState,
) -> Result<bool, WorkspaceError> {
    if !desired.present {
        let removed = remove_regular_if_exists(target)?;
        sync_directory(parent)?;
        return Ok(removed);
    }
    let stage_name = desired
        .staged
        .as_deref()
        .ok_or_else(|| io_error(target, "recorded present state has no staged payload"))?;
    let staged = parent.join(stage_name);
    validate_owned_stage_name(stage_name, None, &staged)?;
    let bytes = read_regular_optional(&staged)?
        .ok_or_else(|| io_error(&staged, "recorded stage is missing"))?;
    if desired.digest.as_deref() != Some(bytes_digest(&bytes).as_str()) {
        return Err(io_error(&staged, "staged payload digest mismatch"));
    }
    let file = crate::safe_file::open_existing_read_write(&staged)
        .map_err(|error| io_error(&staged, error))?;
    let identity = crate::safe_file::identity(&file).map_err(|error| io_error(&staged, error))?;
    crate::safe_file::assert_path_identity(&staged, identity)
        .map_err(|error| io_error(&staged, error))?;
    let temp_path = TempPath::try_from_path(staged).map_err(|error| io_error(target, error))?;
    let temp = NamedTempFile::from_parts(file, temp_path);
    persist_temp(temp, target)?;
    let current = current_state(target)?;
    if current != recorded_current(desired) {
        return Err(io_error(
            target,
            "replacement target digest changed after persist",
        ));
    }
    sync_directory(parent)?;
    Ok(true)
}

fn persist_temp(temp: NamedTempFile, target: &Path) -> Result<(), WorkspaceError> {
    match temp.persist(target) {
        Ok(file) => file.sync_all().map_err(|error| io_error(target, error)),
        Err(error) => {
            let primary = error.error.to_string();
            let staged = error.file.path().to_path_buf();
            let keep = error.file.keep().err().map(|keep| keep.error.to_string());
            let reason = match keep {
                Some(keep) => format!(
                    "atomic replace failed: {primary}; preserving `{}` also failed: {keep}",
                    staged.display()
                ),
                None => format!(
                    "atomic replace failed: {primary}; staged payload retained at `{}`",
                    staged.display()
                ),
            };
            Err(io_error(target, reason))
        }
    }
}

#[cfg(test)]
pub(super) fn recover_pending(parent: &Path) -> Result<(), WorkspaceError> {
    fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    let lock = BootArtifactLock::acquire(parent)?;
    recover_pending_locked(parent, &lock)
}

fn recover_pending_locked(parent: &Path, lock: &BootArtifactLock) -> Result<(), WorkspaceError> {
    lock.assert_current()?;
    let commit_path = journal_path(parent);
    let rollback_path = rollback_journal_path(parent);
    let commit = read_journal(&commit_path, JournalMode::Commit)?;
    let rollback = read_journal(&rollback_path, JournalMode::Rollback)?;
    let mut current = std::collections::BTreeSet::new();
    if let Some(journal) = commit.as_ref() {
        current.insert(journal.transaction.clone());
    }
    if let Some(journal) = rollback.as_ref() {
        current.insert(journal.transaction.clone());
    }
    let journal = match (commit, rollback) {
        (None, None) => {
            sweep_orphan_stages(parent, &current)?;
            return Ok(());
        }
        (Some(commit), None) => {
            sweep_orphan_stages(parent, &current)?;
            preflight_journal_paths(parent, &commit)?;
            roll_forward_core(parent, &commit, &NoFault)?;
            commit
        }
        (commit, Some(rollback)) => {
            if let Some(commit) = commit.as_ref() {
                validate_twins(commit, &rollback, &rollback_path)?;
            }
            sweep_orphan_stages(parent, &current)?;
            preflight_journal_paths(parent, &rollback)?;
            let errors = rollback_all(parent, &rollback, &NoFault);
            if !errors.is_empty() {
                return Err(aggregate_error(
                    parent,
                    io_error(&rollback_path, "recovering durable rollback intent"),
                    errors,
                    "rollback recovery",
                ));
            }
            rollback
        }
    };
    cleanup(parent, &journal)
}

fn cleanup(parent: &Path, journal: &Journal) -> Result<(), WorkspaceError> {
    ensure_journal_matches(parent, journal, journal.mode.clone())?;
    let errors = cleanup_transaction_stages(parent, &journal.transaction);
    if errors.is_empty() {
        remove_regular_if_exists(&journal_path(parent))?;
        remove_regular_if_exists(&rollback_journal_path(parent))?;
        sync_directory(parent)
    } else {
        Err(aggregate_error(
            parent,
            io_error(&journal_path(parent), "committed transaction cleanup"),
            errors,
            "cleanup",
        ))
    }
}

fn target_path(parent: &Path, entry: &JournalEntry) -> Result<PathBuf, WorkspaceError> {
    Ok(parent.join(&entry.target))
}

fn current_state(path: &Path) -> Result<CurrentState, WorkspaceError> {
    match read_regular_optional(path)? {
        Some(bytes) => Ok(CurrentState {
            present: true,
            digest: Some(bytes_digest(&bytes)),
        }),
        None => Ok(CurrentState {
            present: false,
            digest: None,
        }),
    }
}

fn third_state_error(
    target: &Path,
    before: &RecordedState,
    after: &RecordedState,
    current: &CurrentState,
) -> WorkspaceError {
    io_error(
        target,
        format!(
            "concurrent/third state detected (before={:?}, after={:?}, current={current:?}); refusing recovery",
            recorded_current(before),
            recorded_current(after),
        ),
    )
}

#[cfg(test)]
#[path = "transaction/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "transaction/safety_tests.rs"]
mod safety_tests;

#[cfg(test)]
#[path = "transaction/lock_tests.rs"]
mod lock_tests;
