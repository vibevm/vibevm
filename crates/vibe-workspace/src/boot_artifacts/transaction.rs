//! Crash-recoverable INDEX/STATIC publication with a durable roll-forward intent.

use std::fs;
use std::path::{Path, PathBuf};

use crate::WorkspaceError;

mod apply;
use apply::{apply_forward, roll_forward_core, rollback_all};
mod durable_io;
use durable_io::*;
mod control;
mod detailed;
/// The closed role algebra and the one enumeration of a journal's entries.
mod roles;
pub(super) use control::write_controlled_with_selectors;
pub(crate) use detailed::{DetailedTransactionFailure, TransactionFailureDisposition};
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
pub(crate) enum WritePoint {
    EntryRecovery,
    IndexWrite,
    StaticWrite,
    PromptStage,
    PostIntentPreRollForward,
    PostStagePreReplace,
    /// Just before one generated prompt artifact's bytes replace what is on
    /// disk — the ignore file first, then the three artifacts it covers.
    PromptWrite,
    /// Just before one generated prompt artifact is removed — the artifacts
    /// first, the ignore file last.
    PromptRemove,
    StaticReplace,
    PostStaticPreIndex,
    IndexReplace,
    PostIndexPreStaleCleanup,
    StaleRemove,
    RollbackStart,
    RollbackRestore,
    PostRollbackRestore,
    PostRollbackPreCleanup,
}

pub(crate) trait FaultInjector {
    fn check(&self, point: WritePoint, path: &Path) -> Result<(), WorkspaceError>;
}

pub(super) struct NoFault;

impl FaultInjector for NoFault {
    fn check(&self, _point: WritePoint, _path: &Path) -> Result<(), WorkspaceError> {
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ArtifactWrite<'a> {
    pub(crate) index_path: &'a Path,
    pub(crate) index_bytes: &'a [u8],
    pub(crate) static_path: &'a Path,
    pub(crate) static_bytes: Option<&'a [u8]>,
    pub(crate) stale_path: &'a Path,
    /// The generated system-prompt artifacts of this same generation, or
    /// `None` when this publication has nothing to say about them.
    ///
    /// `None` is not "remove": it means the four prompt members are absent
    /// from the journal and no prompt path is read, staged, replaced or
    /// removed — which is every project that has not opted in and every
    /// publication with no node identity to read an opt-in from. Removal is
    /// [`PromptWrite::REMOVE`], which is a different statement and a
    /// different journal (`PROP-076#SYS-OPT-IN`,
    /// `#SYS-REMOVAL-AND-CONFLICT`).
    pub(crate) prompt: Option<PromptWrite<'a>>,
}

/// One generation's system-prompt payload, inside the same commit as
/// `INDEX.md` and the selected `STATIC` rather than beside it: a failure must
/// never leave a new prefix paired with a sidecar of another generation, nor
/// damage the previous usable set (`PROP-076#SYS-TRANSACTION-ROLES`).
///
/// Each member is `Some(bytes)` to publish those bytes and `None` for "this
/// target must end absent". The paths are not carried: they are the four
/// reserved names in the boot directory `index_path` already fixes, so the
/// transaction's one-directory invariant holds trivially and no caller can
/// point a prompt role at a file of its own choosing.
#[derive(Clone, Copy, Default)]
pub(crate) struct PromptWrite<'a> {
    /// The ignore file vibe writes into the boot directory it owns. The
    /// repository root's `.gitignore` is never touched — that file is the
    /// developer's (`PROP-076#SYS-ARTIFACTS-NOT-TRACKED`).
    pub(crate) gitignore: Option<&'a [u8]>,
    /// `SYSTEM.md` — the cache-stable prefix.
    pub(crate) system: Option<&'a [u8]>,
    /// `SYSTEM.meta.json` — its witness.
    pub(crate) meta: Option<&'a [u8]>,
    /// `AGENT-LANE.md` — the agent-mode lane.
    pub(crate) lane: Option<&'a [u8]>,
}

impl PromptWrite<'_> {
    /// Opting out: every generated prompt artifact and the ignore file that
    /// covered them end absent. Distinct from `prompt: None`, which touches
    /// nothing at all.
    pub(crate) const REMOVE: Self = Self {
        gitignore: None,
        system: None,
        meta: None,
        lane: None,
    };
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
    write_with_faults_and_selectors_detailed(write, &NoFault, selectors)
        .map_err(DetailedTransactionFailure::into_source)
}

/// The detailed entry a replay publication takes, with the node selectors of
/// the ordinary write path.
///
/// A replay that republishes a changed `STATIC.*` must also rewrite the
/// managed block whose expected lane digest names it, and must publish the
/// prefix whose entrypoint witness is that very block — otherwise a replay
/// leaves a project telling a session to re-read a lane that is current, and
/// a prefix describing one that is not (PROP-076
/// `#SYS-PROSPECTIVE-ENTRYPOINT`).
pub(crate) fn write_production_detailed_with_selectors(
    write: ArtifactWrite<'_>,
    selectors: NodeSelectors<'_>,
) -> Result<(), DetailedTransactionFailure> {
    write_with_faults_and_selectors_detailed(write, &NoFault, selectors)
}

/// The managed-block selector a publication runs inside its own
/// transaction, as one name rather than as a type spelled at four call
/// sites. The transaction id is the argument, because a selector writes
/// through the same journal rather than beside it.
pub(crate) type NodeSelectors<'a> = &'a dyn Fn(&str) -> Result<(), WorkspaceError>;

/// The selector of a publication that has no managed block to rewrite — a
/// package unit slot, and every test that does not exercise one.
pub(crate) fn no_selectors(_transaction: &str) -> Result<(), WorkspaceError> {
    Ok(())
}

pub(super) fn preflight_artifact_roles(write: ArtifactWrite<'_>) -> Result<(), WorkspaceError> {
    let _ = common_parent(&write)?;
    validate_artifact_roles(write.index_path, write.static_path, write.stale_path)
}

/// The transaction journal still pending in `parent`, if any: a directory
/// whose recovery has not run is that journal's to settle, not a caller's.
pub(super) fn pending_journal(parent: &Path) -> Option<PathBuf> {
    [journal_path(parent), rollback_journal_path(parent)]
        .into_iter()
        .find(|path| fs::symlink_metadata(path).is_ok())
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
    write_with_faults_and_selectors_detailed(write, faults, |_| Ok(()))
        .map_err(DetailedTransactionFailure::into_source)
}

#[cfg(test)]
pub(crate) fn write_with_faults_detailed(
    write: ArtifactWrite<'_>,
    faults: &impl FaultInjector,
) -> Result<(), DetailedTransactionFailure> {
    write_with_faults_and_selectors_detailed(write, faults, no_selectors)
}

#[cfg(test)]
pub(crate) fn write_with_faults_and_selectors(
    write: ArtifactWrite<'_>,
    faults: &impl FaultInjector,
    selectors: NodeSelectors<'_>,
) -> Result<(), DetailedTransactionFailure> {
    write_with_faults_and_selectors_detailed(write, faults, selectors)
}

fn write_with_faults_and_selectors_detailed<T>(
    write: ArtifactWrite<'_>,
    faults: &impl FaultInjector,
    selectors: impl FnOnce(&str) -> Result<T, WorkspaceError>,
) -> Result<T, DetailedTransactionFailure> {
    let uncommitted = |source| {
        DetailedTransactionFailure::new(source, TransactionFailureDisposition::Uncommitted)
    };
    let parent = common_parent(&write).map_err(uncommitted)?;
    validate_artifact_roles(write.index_path, write.static_path, write.stale_path)
        .map_err(uncommitted)?;
    fs::create_dir_all(parent)
        .map_err(|error| io_error(write.index_path, error))
        .map_err(uncommitted)?;
    let lock = BootArtifactLock::acquire(parent).map_err(uncommitted)?;
    lock.assert_current().map_err(uncommitted)?;
    write_with_faults_locked(write, parent, faults, &lock, selectors)
}

fn write_with_faults_locked<T>(
    write: ArtifactWrite<'_>,
    parent: &Path,
    faults: &impl FaultInjector,
    lock: &BootArtifactLock,
    selectors: impl FnOnce(&str) -> Result<T, WorkspaceError>,
) -> Result<T, DetailedTransactionFailure> {
    let failure = DetailedTransactionFailure::new;
    faults
        .check(WritePoint::EntryRecovery, parent)
        .map_err(|error| failure(error, TransactionFailureDisposition::EntryRecoveryFailed))?;
    recover_pending_locked(parent, lock)
        .map_err(|error| failure(error, TransactionFailureDisposition::EntryRecoveryFailed))?;
    let mut journal = prepare_journal(write, parent, faults)
        .map_err(|error| failure(error, TransactionFailureDisposition::Uncommitted))?;
    preflight_journal_paths(parent, &journal)
        .map_err(|error| failure(error, TransactionFailureDisposition::Uncommitted))?;
    if let Err(primary) = persist_new_journal(parent, &journal) {
        let cleanup = discard_assets(parent, &journal);
        let source = if cleanup.is_empty() {
            primary
        } else {
            aggregate_error(parent, primary, cleanup, "unpublished-stage cleanup")
        };
        return Err(failure(
            source,
            TransactionFailureDisposition::Indeterminate,
        ));
    }
    faults
        .check(WritePoint::PostIntentPreRollForward, &journal_path(parent))
        .map_err(|error| failure(error, TransactionFailureDisposition::Indeterminate))?;

    if let Err(primary) = roll_forward_core(parent, &journal, faults) {
        let commit = journal.clone();
        journal.mode = JournalMode::Rollback;
        if let Err(update) = arm_rollback(parent, &commit, &journal) {
            return Err(failure(
                aggregate_error(parent, primary, vec![update], "arming rollback"),
                TransactionFailureDisposition::Indeterminate,
            ));
        }
        if let Err(start) = faults.check(WritePoint::RollbackStart, &journal_path(parent)) {
            return Err(failure(
                aggregate_error(parent, primary, vec![start], "starting rollback"),
                TransactionFailureDisposition::RollbackRecoveryIntent,
            ));
        }
        let rollback = rollback_all(parent, &journal, faults);
        if rollback.is_empty() {
            if let Err(post_restore) = faults.check(
                WritePoint::PostRollbackPreCleanup,
                &rollback_journal_path(parent),
            ) {
                return Err(failure(
                    aggregate_error(parent, primary, vec![post_restore], "post-rollback cleanup"),
                    TransactionFailureDisposition::RollbackRecoveryIntent,
                ));
            }
            return match cleanup(parent, &journal) {
                Ok(()) => Err(failure(
                    primary,
                    TransactionFailureDisposition::RestoredBefore,
                )),
                Err(error) => Err(failure(
                    error,
                    TransactionFailureDisposition::RollbackRecoveryIntent,
                )),
            };
        } else {
            return Err(failure(
                aggregate_error(parent, primary, rollback, "rollback"),
                TransactionFailureDisposition::RollbackRecoveryIntent,
            ));
        }
    }
    let committed = |source| failure(source, TransactionFailureDisposition::CommitRecoveryIntent);
    let output = selectors(&journal.transaction).map_err(committed)?;
    let index_target = target_path(parent, &journal.index).map_err(committed)?;
    faults
        .check(WritePoint::PostIndexPreStaleCleanup, &index_target)
        .map_err(committed)?;
    let stale_target = target_path(parent, &journal.stale).map_err(committed)?;
    faults
        .check(WritePoint::StaleRemove, &stale_target)
        .map_err(committed)?;
    apply_forward(parent, &journal.stale).map_err(committed)?;
    cleanup(parent, &journal).map_err(committed)?;
    Ok(output)
}

/// The one directory every target of this write shares.
///
/// The prompt roles need no clause of their own: their paths are not the
/// caller's to choose but the four reserved names in *this* parent, so they
/// share it by construction and are distinct from `INDEX.md` and from both
/// `STATIC` spellings by construction too — which is the whole of what
/// `PROP-076#SYS-TRANSACTION-ROLES` means by "the `common_parent` invariant
/// holds trivially". The static fact behind "by construction" is pinned by
/// `prompt_targets_are_distinct_from_the_lane_targets`.
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

#[cfg(test)]
#[path = "transaction/detailed_tests.rs"]
mod detailed_tests;
