//! Existing terminal report/reconciliation projections, moved without changing rules.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");
use crate::{PlannedExecution, RitualPlan};
use anyhow::Result;
use vibe_lifecycle::{LifecycleRun, REMOVED_DECLARATION, UNKNOWN_PROVENANCE};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::lifecycle_state_v2::e2::state::{
    ExecutionRecordScope, ExecutionRecordStatus,
};

/// Reconcile live PHASE-scoped parks against the COMPLETE current phase plan.
///
/// Same-id adoption deliberately retains delegated rows — that is how a resume
/// finds its own work. But if the declaration that parked one has since been
/// removed, the current plan never visits its key, so the row would sit live
/// forever while every later invocation reported a clean completion. That is
/// the one thing this must never do.
///
/// The POLICY is cancellation, chosen once and applied deterministically: the
/// row is removed by exact state-owned cleanup — recompute the `(run, key)`
/// task path, remove only that file, drop only that record, prune only a
/// proven-empty run directory — and the run continues, reporting the
/// cancellation as a contribution row rather than swallowing it. Refusing
/// instead would strand the operator on a declaration they already deleted.
///
/// Scope comes from the typed tag the engine recorded, never from parsing the
/// execution key or a task filename: a `slot`-scoped row belongs to the slot
/// plan and is invisible to this phase plan, so it is left alone here.
pub(super) fn reconcile_removed_parks(
    run: &mut LifecycleRun,
    plan: &RitualPlan,
    reports: &mut Vec<LifecycleContributionReport>,
) -> Result<()> {
    let planned: std::collections::BTreeSet<String> = plan
        .executions
        .iter()
        .map(|execution| execution.row.key().to_string())
        .collect();
    let project_root = std::path::PathBuf::from(&plan.project.root);
    for (key, record) in run.delegated_rows() {
        if record.scope != Some(ExecutionRecordScope::Phase) || planned.contains(&key) {
            continue;
        }
        let Some(message) = run.cancel_delegated(&key, &project_root)? else {
            continue;
        };
        // Report ONLY what survives the declaration's removal. The row's own
        // persisted phase is authoritative, so the point is exact; everything
        // else — who provided it, at which tier, under what reference — died
        // with the declaration, and a host row that vanished never had a
        // `dependency` tier to begin with. Naming a sentinel is honest; naming
        // the first surviving execution's phase, or guessing `dependency`,
        // invents provenance the cancelled row cannot corroborate.
        reports.push(LifecycleContributionReport {
            flagged: None,
            handler: "agent".into(),
            key,
            message: Some(message),
            stderr: None,
            stderr_truncated: None,
            stdout: None,
            stdout_truncated: None,
            point: format!("phase:{}", record.phase),
            phase: record.phase,
            provider: REMOVED_DECLARATION.into(),
            reference: Some(REMOVED_DECLARATION.into()),
            slot_target: None,
            status: "cancelled".into(),
            tier: UNKNOWN_PROVENANCE.into(),
            version: None,
        });
    }
    Ok(())
}

pub(super) fn state_status(status: &ExecutionRecordStatus) -> &'static str {
    match status {
        ExecutionRecordStatus::Ok => "ok",
        ExecutionRecordStatus::Skip => "skip",
        ExecutionRecordStatus::Fresh => "fresh",
        ExecutionRecordStatus::Fail => "fail",
        ExecutionRecordStatus::Delegated => "delegated",
    }
}

pub(super) fn contribution_status_report(
    execution: &PlannedExecution,
    status: &str,
    message: Option<String>,
    streams: Option<&vibe_lifecycle::handlers::HandlerStreams>,
) -> LifecycleContributionReport {
    let row = &execution.row;
    let (provider, version) = crate::plan::provider_and_version(row.provider());
    LifecycleContributionReport {
        flagged: None,
        handler: row.declaration().handler.kind().to_string(),
        key: row.key().to_string(),
        message,
        stderr: streams
            .and_then(|streams| (!streams.stderr.is_empty()).then(|| streams.stderr.clone())),
        stderr_truncated: streams.and_then(|streams| streams.stderr_truncated.then_some(true)),
        stdout: streams
            .and_then(|streams| (!streams.stdout.is_empty()).then(|| streams.stdout.clone())),
        stdout_truncated: streams.and_then(|streams| streams.stdout_truncated.then_some(true)),
        phase: execution.phase.clone(),
        point: row.declaration().point.to_string(),
        provider,
        reference: None,
        slot_target: None,
        status: status.to_string(),
        tier: crate::plan::tier_name(row.effective_tier()).to_string(),
        version,
    }
}
