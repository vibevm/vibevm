//! What one lifecycle run still OWES, and how a run cancels work no plan
//! will ever visit again.
//!
//! The slot continuation, the live delegated rows, and the state-first
//! cancellation that removes one of them. Split out of the transition cell
//! because it is a different question: `execute_one` asks what happens to a
//! row now, this asks what a run is still carrying from before.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME");

use std::path::Path;

use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordScope, SlotContinuation, SlotTargetRecord,
};

use super::{LifecycleRun, LifecycleRunError};
use crate::LifecycleStateStore;

/// What a cancellation report names in place of a provider or a reference.
///
/// A cancelled row's declaration is GONE — that is why it was cancelled — so
/// its provider, reference and point-detail cannot be re-derived from
/// anything. The persisted row carries a status, a phase and a typed scope,
/// and nothing else about where it came from. These sentinels say exactly
/// that, instead of borrowing a neighbouring execution's provenance.
pub const REMOVED_DECLARATION: &str = "<removed-declaration>";

/// The tier of a cancelled row. A host declaration and a dependency
/// declaration both park identically, and the state does not record which,
/// so claiming `dependency` would be false for every removed host row.
pub const UNKNOWN_PROVENANCE: &str = "<unknown>";

/// The point of a cancelled SLOT row. `slot:pre-install` and
/// `slot:post-install` are different facts and the state records neither, so
/// this never pretends to know which one the removed declaration used.
pub const REMOVED_SLOT_DECLARATION: &str = "<removed-slot-declaration>";

impl LifecycleRun {
    /// Record the exact ordered payload-event target set this slot run
    /// selected, before any pre-install callback can stop it. Untracked runs
    /// keep no state, so there is nothing to record and nothing to resume.
    ///
    /// EVERY slot-plan construction records its set, including one servicing
    /// an adopted park. The store owns what that means: an empty selection
    /// retains the adopted set, a matching one is retained for later rows in
    /// the same invocation, and a genuinely different non-empty one refuses
    /// rather than overwriting what the parked run needs to rebuild itself.
    /// Declining to record here — the shape this had — is what left a resume
    /// that satisfied its last row unable to park the NEXT one: the set it
    /// would have named had never reached the new store.
    pub fn record_slot_continuation(
        &mut self,
        targets: Vec<SlotTargetRecord>,
    ) -> Result<(), LifecycleRunError> {
        let Some(state) = self.state.as_mut() else {
            return Ok(());
        };
        state
            .record_slot_continuation(SlotContinuation { targets })
            .map_err(LifecycleRunError::from)
    }

    /// The slot continuation this run still owes.
    #[must_use]
    pub fn slot_continuation(&self) -> Option<&SlotContinuation> {
        self.state
            .as_ref()
            .and_then(LifecycleStateStore::slot_continuation)
    }

    /// Drop what the slot run owed — but ONLY once nothing is owed.
    ///
    /// A pass can finish without visiting a live slot-scoped park: an
    /// unchanged slot produces no payload event, so the post-install plan is
    /// empty and the delegated row is never revisited. Clearing the
    /// continuation there would strand that row forever behind a run that
    /// reported completion. So the guard is the state itself, not the
    /// caller's belief about it.
    pub fn clear_slot_continuation(&mut self) -> Result<(), LifecycleRunError> {
        if self.owes_slot_work() {
            return Ok(());
        }
        let Some(state) = self.state.as_mut() else {
            return Ok(());
        };
        state
            .clear_slot_continuation()
            .map_err(LifecycleRunError::from)
    }

    /// Whether a slot-scoped park is still live in this run's state.
    #[must_use]
    pub fn owes_slot_work(&self) -> bool {
        self.delegated_rows()
            .iter()
            .any(|(_, record)| record.scope == Some(ExecutionRecordScope::Slot))
    }

    /// Every delegated row still live, with the typed scope the engine gave it.
    #[must_use]
    pub fn delegated_rows(&self) -> Vec<(String, ExecutionRecord)> {
        self.state
            .as_ref()
            .map(LifecycleStateStore::delegated_rows)
            .unwrap_or_default()
    }

    /// Cancel one delegated row: durably drop the state row first, then clean
    /// exactly the task `(run, key)` owns. A cleanup failure leaves a named
    /// orphan, never a live record pointing at an absent task. Used when the
    /// declaration that parked it no longer exists, so nothing in the current
    /// plan will ever visit the key.
    pub fn cancel_delegated(
        &mut self,
        key: &str,
        project_root: &Path,
    ) -> Result<Option<String>, LifecycleRunError> {
        let Some(record) = self
            .state
            .as_ref()
            .and_then(|state| state.prior(key))
            .cloned()
        else {
            return Ok(None);
        };
        let run_id = self.run_id.clone();
        // STATE FIRST. The durable row is the thing that keeps a run from
        // completing over stranded work, so it goes before the file it names.
        // Removing the task first and then failing to forget the row would
        // leave a live delegated record pointing at a task that no longer
        // exists — the one state the whole handshake must never reach. The
        // reverse gap is safe and self-describing: an orphaned task file.
        self.state
            .as_mut()
            .ok_or(LifecycleRunError::UntrackedCheckpoint)?
            .forget(key)
            .map_err(LifecycleRunError::from)?;
        let notice = record.tasks.first().and_then(|task| {
            crate::delegation::cleanup_task(project_root, &run_id, key, task).err()
        });
        Ok(Some(notice.map_or_else(
            || {
                format!(
                    "cancelled the parked execution `{key}`: its declaration is absent \
                     from the current plan, so the run would otherwise never visit it again"
                )
            },
            |orphan| {
                // Cleanup failure is a NOTICE, never a reason to put the row
                // back: the record is already durably gone, and reinserting it
                // would resurrect work no plan will ever visit.
                format!(
                    "cancelled the parked execution `{key}`; its state row is durably \
                     gone, but its task file remains as a named orphan: {orphan}"
                )
            },
        )))
    }
}
