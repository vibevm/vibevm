//! One-contribution lifecycle execution with per-transition checkpoints.

use anyhow::Result;
use std::sync::Arc;
use vibe_lifecycle::handlers::{HandlerRuntime, PackageBindingBackend};
use vibe_lifecycle::process::{StreamMode, SystemProcessRunner};

use vibe_lifecycle::{
    Delegation, ExecutionReuse, HandlerExecution, LifecycleLease, LifecycleRun, LifecycleRunHandle,
    RunMetadata,
};
use vibe_lifecycle::{REMOVED_DECLARATION, UNKNOWN_PROVENANCE};
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleReport, LifecycleStepReport,
};
use vibe_wire::generated::lifecycle_state::{ExecutionRecordScope, ExecutionRecordStatus};

use crate::commands::compile_trace::{RegisteredReportDraft, carry, carry_measured};
use crate::output;

use backends::{ProjectPackageBindingBackend, WorkspaceBinaryBackend};

use super::agent::CliAgentBackend;
use super::draft::LifecycleDraft;
use super::world;

#[path = "dispatch/backends.rs"]
mod backends;

#[cfg(test)]
#[path = "dispatch/inject.rs"]
pub(super) mod inject;

/// The UNTRACKED clean epoch's failure document, unchanged and still internal.
///
/// The clean lifecycle keeps no state record and its wipe destroys the tree a
/// trace would live in, so it never opens a session and has no outer funnel to
/// hand a draft to. Moving this one outward would mean inventing a boundary
/// for a command that deliberately has none.
fn emit_untracked_failure_outcome(
    ctx: &output::Context,
    metadata: &RunMetadata,
    phase: &str,
    contributions: &[LifecycleContributionReport],
) -> Result<()> {
    if !ctx.is_json() {
        return Ok(());
    }
    // A failing run still shows the plan it was executing: the deferral only
    // ever holds documents back until the outcome is known, and a failure is
    // an outcome.
    ctx.flush_json_plans()?;
    ctx.emit_json(&LifecycleReport {
        chain: metadata.chain.clone(),
        command: "lifecycle".into(),
        contributions: contributions.to_vec(),
        notices: Vec::new(),
        ok: false,
        requested: metadata.requested.clone(),
        steps: vec![LifecycleStepReport {
            phase: phase.into(),
            status: "fail".into(),
        }],
        delegation: None,
        trace: None,
    })
}

/// What one dispatch pass produced: the contribution rows it reported and,
/// when a hosted agent row parked, the typed handoff plus the phase the chain
/// stopped at. A park is NOT a failure — it travels as a value so the caller
/// can truncate its step list and render one handoff.
#[derive(Debug, Default)]
pub(super) struct DispatchOutcome {
    pub reports: Vec<LifecycleContributionReport>,
    pub parked: Option<(String, Delegation)>,
}

pub(super) fn dispatch_plan_untracked(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    lease: &Arc<LifecycleLease>,
    metadata: RunMetadata,
) -> Result<Vec<LifecycleContributionReport>> {
    // The untracked clean epoch retains the same lease proof a tracked run
    // does: it mutates the tree, so it owns the workspace for its life.
    let mut run = LifecycleRun::untracked(
        lease.clone(),
        plan.project.clone(),
        plan.world.clone(),
        metadata.clone(),
    );
    let mut reports = Vec::with_capacity(plan.executions.len());
    let package_binding = ProjectPackageBindingBackend::new(plan);
    let agent = CliAgentBackend::for_plan(plan);
    let runtime = runtime(ctx, &package_binding, &agent);
    for execution in plan.executions.iter() {
        let handler = HandlerExecution::from_row(&execution.row);
        let outcome =
            match run.execute_one(&handler, &execution.phase, ExecutionReuse::Always, &runtime) {
                Ok(outcome) => outcome,
                Err(error) => {
                    if let Some(failed) = error.failed_transition() {
                        reports.push(contribution_status_report(
                            execution,
                            "fail",
                            Some(failed.message.clone()),
                            Some(&failed.streams),
                        ));
                        emit_untracked_failure_outcome(ctx, &metadata, &execution.phase, &reports)?;
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
        render_outcome(ctx, &report);
        reports.push(report);
    }
    Ok(reports)
}

pub(super) fn dispatch_plan(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    lease: Arc<LifecycleLease>,
    metadata: RunMetadata,
    state_chain: Vec<String>,
) -> Result<DispatchOutcome> {
    // The lease root IS the state root: `begin` derives its store path from
    // the lease, and a plan whose workspace root disagrees refuses here
    // rather than write state beside another process's lock — through the
    // lease's one typed gate.
    lease.ensure_root(&plan.workspace_root, "at phase dispatch")?;
    let run = LifecycleRun::begin(
        lease,
        plan.project.clone(),
        plan.world.clone(),
        metadata.clone(),
        state_chain,
    )?
    .shared();
    dispatch_plan_with_run(ctx, plan, &run, &metadata)
}

/// Dispatch one phase plan, carrying whatever rows were measured out with ANY
/// failure.
///
/// The inner pass freezes rows only for the failure it can name — a handler
/// that reported a failed transition. Every other way the pass can stop (the
/// state write behind a checkpoint, a park reconciliation, the execution-prefix
/// retention) happens AFTER rows already exist, and returning a bare error
/// there would report a run that had done several things successfully as one
/// that did nothing.
///
/// So the rows live in an accumulator this function owns, and any uncarried
/// error leaving the inner pass picks them up on the way out. The original
/// error object is untouched — `carry_measured` moves it, never reformats it.
pub(super) fn dispatch_plan_with_run(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    run: &LifecycleRunHandle,
    metadata: &RunMetadata,
) -> Result<DispatchOutcome> {
    let mut measured: Vec<LifecycleContributionReport> = Vec::new();
    dispatch_measured(ctx, plan, run, metadata, &mut measured).map_err(|error| {
        carry_measured(error, || {
            RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
                &metadata.requested,
                metadata.chain.clone(),
                &metadata.requested,
                measured,
            )))
        })
    })
}

fn dispatch_measured(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    run: &LifecycleRunHandle,
    metadata: &RunMetadata,
    measured: &mut Vec<LifecycleContributionReport>,
) -> Result<DispatchOutcome> {
    let package_binding = ProjectPackageBindingBackend::new(plan);
    let agent = CliAgentBackend::for_plan(plan);
    let runtime = runtime(ctx, &package_binding, &agent);
    let mut outcome = DispatchOutcome {
        reports: Vec::with_capacity(plan.executions.len()),
        parked: None,
    };
    let mut run = run
        .lock()
        .map_err(|_| anyhow::anyhow!("lifecycle run lock was poisoned"))?;
    run.rebind_world(plan.project.clone(), plan.world.clone())?;
    // Reconciliation can both PRODUCE rows and then fail, so the accumulator
    // is refreshed either way — a cancellation the operator needs to see must
    // not disappear because the next step went wrong.
    let reconciled = reconcile_removed_parks(&mut run, plan, &mut outcome.reports);
    measured.clone_from(&outcome.reports);
    reconciled?;
    for execution in plan.executions.iter() {
        let handler = HandlerExecution::from_row(&execution.row);
        let transition = match run.execute_one(
            &handler,
            &execution.phase,
            ExecutionReuse::FreshnessAware,
            &runtime,
        ) {
            Ok(transition) => transition,
            Err(error) => {
                // The rows this handler produced are measured HERE, and this
                // is the only place they exist. Copy what the report needs
                // before the error is moved into the contextual anyhow the
                // caller has always seen.
                let failed_row = error
                    .failed_transition()
                    .map(|failed| (failed.message.clone(), failed.streams.clone()));
                if let Some((message, streams)) = failed_row {
                    outcome.reports.push(contribution_status_report(
                        execution,
                        "fail",
                        Some(message),
                        Some(&streams),
                    ));
                    // Constructed exactly once, exactly where it always was —
                    // `HandlerError` plus the `phase … stopped …` context —
                    // and then MOVED into the carrier. Nothing strips or
                    // re-adds context after this point, so stderr, the chain
                    // and every downcast stay identical with tracing off.
                    let original = anyhow::Error::new(error).context(format!(
                        "phase `{}` stopped before any later lifecycle contribution",
                        execution.phase
                    ));
                    measured.clone_from(&outcome.reports);
                    return Err(carry(
                        RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
                            &metadata.requested,
                            metadata.chain.clone(),
                            &execution.phase,
                            outcome.reports.clone(),
                        ))),
                        original,
                        // The policy this site has always had: the failed root
                        // is a machine document, emitted only in JSON mode and
                        // only by an unsuppressed context.
                        ctx.is_json() && !ctx.suppresses_output(),
                    ));
                }
                return Err(anyhow::Error::new(error).context(format!(
                    "phase `{}` stopped before any later lifecycle contribution",
                    execution.phase
                )));
            }
        };
        let status = state_status(&transition.status);
        let fresh = transition.is_fresh();
        let parked = transition.delegation.clone();
        let report = contribution_status_report(
            execution,
            status,
            transition.message,
            (!fresh).then_some(&transition.streams),
        );
        render_outcome(ctx, &report);
        outcome.reports.push(report);
        measured.clone_from(&outcome.reports);
        // A deterministic stand-in for the generic post-row failures this
        // boundary really has — a state write, a checkpoint, a reconciliation.
        // Compiled out entirely outside `cfg(test)`; see `inject`.
        #[cfg(test)]
        if inject::armed_at(outcome.reports.len()) {
            return Err(
                anyhow::anyhow!("injected state fault").context("writing the execution checkpoint")
            );
        }
        // The first unsatisfied hosted row wins: every later contribution AND
        // every later phase of this plan is skipped. The rows are already in
        // chain order, so returning here stops both.
        if let Some(delegation) = parked {
            outcome.parked = Some((execution.phase.clone(), delegation));
            return Ok(outcome);
        }
    }
    if plan.package_phase_planned {
        let reconciled = outcome.reports.iter().any(|report| {
            report.key == world::PACKAGE_SKILL_RECONCILE_KEY
                && matches!(report.status.as_str(), "ok" | "fresh")
        });
        if reconciled {
            let mut keep = plan.package_desired_keys.clone();
            keep.insert(world::PACKAGE_SKILL_RECONCILE_KEY.to_string());
            keep.insert(world::PACKAGE_SKILL_RECOVER_KEY.to_string());
            run.retain_execution_prefix(vibe_mcp::pkgskill::PROJECT_SKILL_PREFIX, &keep)?;
        }
    }
    Ok(outcome)
}

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
fn reconcile_removed_parks(
    run: &mut LifecycleRun,
    plan: &world::RitualPlan,
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

fn state_status(status: &ExecutionRecordStatus) -> &'static str {
    match status {
        ExecutionRecordStatus::Ok => "ok",
        ExecutionRecordStatus::Skip => "skip",
        ExecutionRecordStatus::Fresh => "fresh",
        ExecutionRecordStatus::Fail => "fail",
        ExecutionRecordStatus::Delegated => "delegated",
    }
}

fn contribution_status_report(
    execution: &world::PlannedExecution,
    status: &str,
    message: Option<String>,
    streams: Option<&vibe_lifecycle::handlers::HandlerStreams>,
) -> LifecycleContributionReport {
    let row = &execution.row;
    let (provider, version) = super::provider_and_version(row.provider());
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
        tier: super::tier_name(row.effective_tier()).to_string(),
        version,
    }
}

fn runtime<'a>(
    ctx: &output::Context,
    package_binding: &'a dyn PackageBindingBackend,
    agent: &'a dyn vibe_lifecycle::AgentBackend,
) -> HandlerRuntime<'a> {
    static PROCESS: SystemProcessRunner = SystemProcessRunner;
    static BINARY_INHERIT: WorkspaceBinaryBackend = WorkspaceBinaryBackend { quiet: false };
    static BINARY_QUIET: WorkspaceBinaryBackend = WorkspaceBinaryBackend { quiet: true };
    static PROBE: vibe_workspace::hooks::SystemProbe = vibe_workspace::hooks::SystemProbe;
    HandlerRuntime {
        process: &PROCESS,
        binary: if ctx.is_json() || ctx.is_quiet() {
            &BINARY_QUIET
        } else {
            &BINARY_INHERIT
        },
        package_binding,
        agent,
        probe: &PROBE,
        streams: if ctx.is_json() {
            StreamMode::Capture
        } else if ctx.is_quiet() {
            StreamMode::Null
        } else {
            StreamMode::Inherit
        },
    }
}

fn render_outcome(ctx: &output::Context, report: &LifecycleContributionReport) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    if report.status == "fresh" {
        ctx.step(&format!(
            "fresh `{}` — provider={}",
            report.key, report.provider
        ));
    } else if let Some(message) = &report.message {
        if report.key.starts_with("@vibe/package/skill/") {
            ctx.step(&format!("package binding [{}]: {message}", report.provider));
        } else {
            ctx.step(&format!("log [{}]: {message}", report.provider));
        }
    }
}

#[cfg(test)]
#[path = "dispatch/tests.rs"]
mod tests;
