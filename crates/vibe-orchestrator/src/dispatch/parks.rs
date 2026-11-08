//! Reconcile removed parked executions without inventing provider history.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");
use super::*;

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
