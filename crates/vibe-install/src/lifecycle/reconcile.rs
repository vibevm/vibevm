//! Reconciling a SLOT-scoped park against the plan a new run just built.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME");

use std::path::Path;

use vibe_lifecycle::{
    LifecycleRunError, LifecycleRunHandle, REMOVED_DECLARATION, REMOVED_SLOT_DECLARATION,
    UNKNOWN_PROVENANCE,
};
use vibe_wire::generated::lifecycle_state::ExecutionRecordScope;

use super::{SlotLifecyclePlan, SlotLifecycleReport};
use crate::error::{Error, Result};

/// Cancel every SLOT-scoped park whose declaration is gone from the plan this
/// run just built, and drop a continuation nothing owes any more.
///
/// The mirror image of the phase plan's own reconciliation, and it exists for
/// the same reason: a delegated row nothing will ever visit again keeps the
/// run from completing forever. Three things keep it honest.
///
/// Scope comes from the TYPED tag the engine recorded — a `phase`-scoped row
/// belongs to the phase plan and is invisible here, exactly as a `slot`-scoped
/// row is invisible there. Membership is decided against the plan's own typed
/// keys, never by parsing an execution key or a task filename. And the removal
/// itself goes through the one state-first primitive, so a failed durable
/// write leaves the row and its task both intact.
///
/// The continuation is cleared only AFTER the cancellations, and only when the
/// run genuinely owes no slot work: a continuation naming targets no delegated
/// row still needs is the invariant `validate_state` refuses to read back.
pub(super) fn reconcile_removed_slot_parks(
    run: &LifecycleRunHandle,
    plan: &SlotLifecyclePlan,
    project_root: &Path,
) -> Result<Vec<SlotLifecycleReport>> {
    let planned: std::collections::BTreeSet<&str> = plan
        .entries
        .iter()
        .map(|entry| entry.key.as_str())
        .collect();
    let mut run = run
        .lock()
        .map_err(|_| Error::Lifecycle("slot lifecycle run lock was poisoned".into()))?;
    let mut cancelled = Vec::new();
    for (key, record) in run.delegated_rows() {
        if record.scope != Some(ExecutionRecordScope::Slot) || planned.contains(key.as_str()) {
            continue;
        }
        let Some(message) = run
            .cancel_delegated(&key, project_root)
            .map_err(|error: LifecycleRunError| Error::Lifecycle(error.to_string()))?
        else {
            continue;
        };
        // The declaration is gone, so its provenance is gone with it. The
        // state records a slot SCOPE, never which slot point the row used, so
        // the point is a sentinel rather than a guess between pre and post;
        // provider, reference and tier are sentinels for the same reason. A
        // removed HOST row never had a `dependency` tier at all.
        cancelled.push(SlotLifecycleReport {
            key,
            reference: REMOVED_DECLARATION.into(),
            slot_target: None,
            point: REMOVED_SLOT_DECLARATION.into(),
            provider: REMOVED_DECLARATION.into(),
            handler: "agent".into(),
            tier: UNKNOWN_PROVENANCE.into(),
            version: None,
            status: "cancelled".into(),
            flagged: false,
            message: Some(message),
            stdout: None,
            stderr: None,
            stdout_truncated: false,
            stderr_truncated: false,
        });
    }
    if !cancelled.is_empty() && !run.owes_slot_work() {
        run.clear_slot_continuation()
            .map_err(|error| Error::Lifecycle(error.to_string()))?;
    }
    Ok(cancelled)
}
