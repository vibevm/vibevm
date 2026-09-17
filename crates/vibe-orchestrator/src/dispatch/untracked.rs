//! The state-blind clean epoch's contribution dispatch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use anyhow::Result;
use std::sync::Arc;
use vibe_lifecycle::{
    AgentBackend, ExecutionReuse, HandlerExecution, LifecycleLease, LifecycleRun, RunMetadata,
};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

use super::{
    ProjectPackageBindingBackend, contribution_status_report, native_backend, runtime, state_status,
};
use crate::RitualPlan;
use crate::ports::RunObserver;

/// Dispatch the UNTRACKED clean epoch.
///
/// The clean lifecycle keeps no state record and its wipe destroys the tree a
/// trace would live in, so it never opens a session and has no outer funnel to
/// hand a measurement to. A failed transition therefore reports its rows to the
/// observer and the ordinary error travels on.
///
/// It also owes no verify boundary: a clean epoch is state-blind, so there is
/// no durable half to compare against and no member to publish.
pub fn dispatch_plan_untracked(
    observer: &dyn RunObserver,
    plan: &RitualPlan,
    lease: &Arc<LifecycleLease>,
    agent: &Arc<dyn AgentBackend>,
    metadata: RunMetadata,
) -> Result<Vec<LifecycleContributionReport>> {
    lease.ensure_root(&plan.workspace_root, "at untracked phase dispatch")?;
    let mut run = LifecycleRun::untracked(
        lease.clone(),
        plan.project.clone(),
        plan.world.clone(),
        metadata.clone(),
    );
    let mut reports = Vec::with_capacity(plan.executions.len());
    let package_binding = ProjectPackageBindingBackend::new(plan);
    let native_candidates = plan.native_candidates.iter().collect::<Vec<_>>();
    let native = native_backend(plan, &metadata, &native_candidates)?;
    let runtime = runtime(observer, &package_binding, &native, agent.as_ref());
    let mut active_phase: Option<String> = None;
    for execution in plan.executions.iter() {
        if active_phase.as_deref() != Some(execution.phase.as_str())
            && let Some(phase) = active_phase.replace(execution.phase.clone())
        {
            observer.observe_phase_finished(&phase, phase_status(&reports, &phase));
        }
        observer.observe_contribution_started(&execution.phase, &execution.row.key().to_string());
        let handler = HandlerExecution::from_row(&execution.row);
        let outcome =
            match run.execute_one(&handler, &execution.phase, ExecutionReuse::Always, &runtime) {
                Ok(outcome) => outcome,
                Err(error) => {
                    if let Some(failed) = error.failed_transition() {
                        let report = contribution_status_report(
                            execution,
                            "fail",
                            Some(failed.message.clone()),
                            Some(&failed.streams),
                        );
                        observer.observe_contribution_terminal(&report);
                        reports.push(report);
                        observer.observe_phase_finished(&execution.phase, "fail");
                        observer.observe_untracked_failure(
                            &metadata,
                            &execution.phase,
                            &reports,
                        )?;
                    } else {
                        observer.observe_phase_finished(&execution.phase, "fail");
                    }
                    return Err(error.into());
                }
            };
        let report = contribution_status_report(
            execution,
            state_status(&outcome.status),
            outcome.message,
            Some(&outcome.streams),
        );
        observer.observe_contribution_terminal(&report);
        observer.observe_contribution(&report);
        reports.push(report);
    }
    if let Some(phase) = active_phase {
        observer.observe_phase_finished(&phase, phase_status(&reports, &phase));
    }
    Ok(reports)
}

fn phase_status(reports: &[LifecycleContributionReport], phase: &str) -> &'static str {
    if reports
        .iter()
        .filter(|report| report.phase == phase)
        .all(|report| report.status == "fresh")
    {
        "fresh"
    } else {
        "ok"
    }
}
