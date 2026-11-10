//! Roll-forward and rollback over a journal's entries.
//!
//! Both directions are **data-driven over journal entries**, which is why a
//! new production role needs no new arm here (spike S2 of the sys-prompt
//! campaign confirmed that before the roles were added). What a new role does
//! need is a place in the two orders below, and those orders are the whole
//! reason this cell exists as prose rather than as a loop:
//!
//! - **Forward.** The ignore file is written before the artifacts it covers,
//!   then the selected `STATIC`, then `INDEX.md`; removals come last, with
//!   the ignore file removed **after** the artifacts it covered. An artifact
//!   that exists before its ignore line is an artifact a `git add -A`
//!   commits, and an interrupted removal that dropped the ignore line first
//!   would leave exactly that state (`PROP-076#SYS-ARTIFACTS-NOT-TRACKED`).
//! - **Backward.** Present-before entries are restored first and
//!   absent-before entries removed last, so an interrupted undo never leaves
//!   an `INDEX.md` naming a `STATIC` lane that is already gone, nor a prompt
//!   artifact whose ignore line this operation created and has just removed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-TRANSACTION-ROLES");

use std::path::Path;

use tempfile::{NamedTempFile, TempPath};

use crate::WorkspaceError;

use super::journal::{Journal, JournalEntry, RecordedState, validate_owned_stage_name};
use super::roles::{prompt_entries, removals, writes};
use super::{
    FaultInjector, WritePoint, bytes_digest, current_state, io_error, journal_path,
    read_regular_optional, recorded_current, remove_regular_if_exists, sync_directory, target_path,
    third_state_error,
};

pub(super) fn roll_forward_core(
    parent: &Path,
    journal: &Journal,
    faults: &impl FaultInjector,
) -> Result<(), WorkspaceError> {
    faults.check(WritePoint::PostStagePreReplace, &journal_path(parent))?;
    // The ignore line, then the artifacts it covers.
    for entry in writes(journal) {
        faults.check(WritePoint::PromptWrite, &target_path(parent, entry)?)?;
        apply_forward(parent, entry)?;
    }
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
    apply_forward(parent, &journal.index)?;
    // The artifacts, then the ignore line that covered them.
    for entry in removals(journal) {
        faults.check(WritePoint::PromptRemove, &target_path(parent, entry)?)?;
        apply_forward(parent, entry)?;
    }
    Ok(())
}

pub(super) fn apply_forward(parent: &Path, entry: &JournalEntry) -> Result<(), WorkspaceError> {
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

pub(super) fn rollback_all(
    parent: &Path,
    journal: &Journal,
    faults: &impl FaultInjector,
) -> Vec<WorkspaceError> {
    let mut errors = Vec::new();
    let static_entries = [&journal.stale, &journal.selected];
    let prompt: Vec<&JournalEntry> = prompt_entries(journal).collect();
    let order = prompt
        .iter()
        .copied()
        .filter(|entry| entry.before.present)
        .chain(
            static_entries
                .iter()
                .copied()
                .filter(|entry| entry.before.present),
        )
        .chain(std::iter::once(&journal.index))
        .chain(
            static_entries
                .iter()
                .copied()
                .filter(|entry| !entry.before.present),
        )
        .chain(
            prompt
                .iter()
                .copied()
                .filter(|entry| !entry.before.present)
                .collect::<Vec<_>>()
                .into_iter()
                .rev(),
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
