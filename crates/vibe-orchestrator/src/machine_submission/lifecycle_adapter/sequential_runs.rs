//! Fixed-owner transition only; generic reserved-run inspection stays strict.
use super::*;
use vibe_wire::generated::lifecycle_state_v2::e2::state::ExecutionRecordStatus;

/// A different header can be replaced only after independent C1 correlation
/// and the outcome owner's exact marker/report validation under the same lease.
/// Freshness rows alone never prove completion and remain available to begin.
pub(super) fn inspect_previous_terminal(
    lease: &LifecycleLease,
    reserved_run_id: &str,
) -> Result<()> {
    let state = LifecycleStateStore::peek_with_lease(lease)?
        .context("previous lifecycle header disappeared")?;
    let previous = state
        .run
        .run_id
        .as_deref()
        .context("unattributed previous header")?;
    ensure!(
        vibe_lifecycle::process::is_valid_run_id(previous)
            && vibe_lifecycle::process::is_valid_run_id(reserved_run_id)
            && previous != reserved_run_id,
        "invalid previous/new run identity"
    );
    ensure!(
        state.run.slot_continuation.is_none()
            && state
                .execution
                .values()
                .all(|row| row.status != ExecutionRecordStatus::Delegated && row.tasks.is_empty()),
        "previous run still owes parked work"
    );
    let correlation = SubmissionReservations::inspect_existing_run(lease, previous)?
        .context("previous run has no C1 reservation")?;
    ensure!(
        correlation.journal_admission == JournalAdmission::Ready,
        "previous C1 journal is not ready"
    );
    let reference = correlation
        .reservation_ref
        .context("previous C1 correlation absent")?;
    match inspect_retained_lifecycle_outcome(lease, previous, &reference)? {
        LifecycleOutcomeInspection::Validated {
            marker,
            report_bytes: Some(bytes),
        } if matches!(
            marker.outcome,
            MarkerOutcome::Completed | MarkerOutcome::Failed
        ) =>
        {
            let report: vibe_wire::generated::lifecycle_report::LifecycleReport =
                serde_json::from_slice(&bytes)?;
            ensure!(
                state.run.requested == report.requested
                    && state.run.chain == report.chain
                    && state.run.selected.as_deref() == Some(".")
                    && !state.run.compile_trace,
                "previous header differs from terminal report"
            );
            Ok(())
        }
        other => bail!("previous run has no validated terminal report: {other:?}"),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
