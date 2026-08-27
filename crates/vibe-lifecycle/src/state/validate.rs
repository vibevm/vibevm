//! The one semantic gate over persisted lifecycle state.
//!
//! Structure is the schema's job; this is about MEANING — the handshake laws a
//! well-formed TOML file can still violate. It is pure: it reads a decoded
//! state and returns why it may not be trusted, never touching the filesystem.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME");

use vibe_wire::generated::lifecycle_state::{
    ExecutionRecordScope, ExecutionRecordStatus, LifecycleState,
};

use crate::process::is_valid_run_id;

/// The semantic invariant, in one place rather than in constructors by
/// convention. Any run id a header carries must be a real identity. A
/// `delegated` row is a durable handoff, so it must live under such an
/// identity, name exactly ONE outbox task, await a non-empty set of planned
/// output rows, and — the part a constructor cannot promise — name the task
/// that `(run id, execution key)` deterministically owns, so a row can never
/// point at another run's or another execution's file. Every other status
/// carries no task files. A pre-R7.3 file — no run id, no delegated row —
/// satisfies all of this vacuously.
pub(super) fn validate_state(state: &LifecycleState) -> Result<(), String> {
    if let Some(run_id) = state.run.run_id.as_deref()
        && !is_valid_run_id(run_id)
    {
        return Err(format!(
            "the run header carries `{run_id}`, which is not a valid 32-hex run id",
        ));
    }
    let mut slot_debt = false;
    for (key, record) in &state.execution {
        match record.status {
            ExecutionRecordStatus::Delegated => {
                let Some(run_id) = state.run.run_id.as_deref().filter(|id| is_valid_run_id(id))
                else {
                    return Err(format!(
                        "execution `{key}` is delegated, but the run header carries no valid \
                         32-hex run id",
                    ));
                };
                // The scope is the ENGINE's record of which plan owns this
                // park. Reconciliation reads it rather than parsing the key or
                // a task filename, so a delegated row without one is
                // unreconcilable and must never be adopted.
                let Some(scope) = record.scope.as_ref() else {
                    return Err(format!(
                        "execution `{key}` is delegated but carries no typed scope; \
                         reconciliation would have to guess which plan owns it",
                    ));
                };
                if *scope == ExecutionRecordScope::Slot {
                    slot_debt = true;
                }
                let [task] = record.tasks.as_slice() else {
                    return Err(format!(
                        "execution `{key}` is delegated with {} outbox task file(s); a delegated \
                         row publishes exactly one",
                        record.tasks.len(),
                    ));
                };
                if record.artifacts.is_empty() {
                    return Err(format!(
                        "execution `{key}` is delegated but awaits no planned output row",
                    ));
                }
                let expected = crate::delegation::outbox_task_path(run_id, key)
                    .map_err(|error| error.to_string())?;
                if *task != expected {
                    return Err(format!(
                        "execution `{key}` is delegated to `{task}`, but run `{run_id}` owns \
                         `{expected}` for that execution",
                    ));
                }
            }
            _ => {
                if !record.tasks.is_empty() {
                    return Err(format!(
                        "execution `{key}` has status `{}`, which may not carry outbox task \
                         files",
                        status_name(&record.status),
                    ));
                }
                if record.scope.is_some() {
                    return Err(format!(
                        "execution `{key}` has status `{}`, which may not carry a \
                         delegation scope",
                        status_name(&record.status),
                    ));
                }
            }
        }
    }
    // The continuation and the slot debt are two halves of one fact: the slot
    // run this state still owes. Either without the other is a state no run
    // can honestly resume from — a continuation nobody needs, or slot debt
    // with no record of which targets to rebuild.
    match (&state.run.slot_continuation, slot_debt) {
        (Some(continuation), true) if continuation.targets.is_empty() => Err(
            "the run owes slot-scoped work but its continuation names no payload-event target"
                .to_string(),
        ),
        (Some(_), false) => Err(
            "the run carries a slot continuation but no delegated row is slot-scoped; \
             nothing would ever consume it"
                .to_string(),
        ),
        (None, true) => Err(
            "a delegated row is slot-scoped but the run records no continuation; the slot \
             run it belongs to could never be rebuilt"
                .to_string(),
        ),
        _ => Ok(()),
    }
}

fn status_name(status: &ExecutionRecordStatus) -> &'static str {
    match status {
        ExecutionRecordStatus::Ok => "ok",
        ExecutionRecordStatus::Fail => "fail",
        ExecutionRecordStatus::Skip => "skip",
        ExecutionRecordStatus::Fresh => "fresh",
        ExecutionRecordStatus::Delegated => "delegated",
    }
}
