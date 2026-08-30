//! One-contribution lifecycle execution with per-transition checkpoints.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use specmark::spec;

use anyhow::Result;
use std::sync::Arc;
use vibe_lifecycle::handlers::{HandlerRuntime, PackageBindingBackend};
use vibe_lifecycle::process::SystemProcessRunner;

use vibe_lifecycle::{
    AgentBackend, Delegation, ExecutionReuse, HandlerExecution, LifecycleLease, LifecycleRun,
    LifecycleRunHandle, RunMetadata,
};
use vibe_lifecycle::{REMOVED_DECLARATION, UNKNOWN_PROVENANCE};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordScope, ExecutionRecordStatus};
use vibe_wire::generated::shared::{Timestamp, VerificationEvidence};

use crate::failure::{MeasuredFailure, Measurement, carry, carry_once};
use crate::ports::RunObserver;
use crate::{PlannedExecution, RitualPlan, world};

use backends::{ProjectPackageBindingBackend, WorkspaceBinaryBackend};

pub use mechanism::DeployAuthority;
pub(crate) use mechanism::{DeployCarriage, MechanismTargets, lower_binaries};

mod backends;
mod mechanism;
mod verify;

#[cfg(test)]
#[path = "inject.rs"]
pub(crate) mod inject;

/// What one dispatch pass produced: the contribution rows it reported and,
/// when a hosted agent row parked, the typed handoff plus the phase the chain
/// stopped at. A park is NOT a failure — it travels as a value so the caller
/// can truncate its step list and render one handoff.
#[derive(Debug, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub(crate) struct DispatchOutcome {
    /// The contribution rows this pass reported, in order.
    pub(crate) reports: Vec<LifecycleContributionReport>,
    /// The phase the chain stopped at and its typed handoff, when a hosted
    /// agent row parked.
    pub(crate) parked: Option<(String, Delegation)>,
    /// The ONE verification-evidence member, present exactly when this pass
    /// reached the engine-owned verify boundary — a `matched` or `unavailable`
    /// comparison it then continued past. A stop travels on the failure
    /// carrier instead, never here.
    pub(crate) verification: Option<VerificationEvidence>,
}

/// What the dispatch had measured when it stopped: the rows, and the member
/// the verify boundary had already reconciled.
///
/// One value rather than two out-parameters, because the two are refreshed at
/// the same instants and a generic post-row failure must pick up BOTH. An
/// accumulator that carried only rows is exactly how a stale comparison
/// reaches the surface as a run that reconciled nothing.
#[derive(Debug, Default)]
struct MeasuredDispatch {
    rows: Vec<LifecycleContributionReport>,
    verification: Option<VerificationEvidence>,
}

/// Dispatch the UNTRACKED clean epoch.
///
/// The clean lifecycle keeps no state record and its wipe destroys the tree a
/// trace would live in, so it never opens a session and has no outer funnel to
/// hand a measurement to. A failed transition therefore reports its rows to the
/// observer and the ordinary error travels on.
///
/// It also owes no verify boundary: a clean epoch is state-blind, so there is
/// no durable half to compare against and no member to publish.
///
/// ```no_run
/// use vibe_orchestrator::dispatch_plan_untracked;
/// # fn call(
/// #     observer: &dyn vibe_orchestrator::ports::RunObserver,
/// #     plan: &vibe_orchestrator::RitualPlan,
/// #     lease: &std::sync::Arc<vibe_lifecycle::LifecycleLease>,
/// #     agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
/// #     metadata: vibe_lifecycle::RunMetadata,
/// # ) -> anyhow::Result<()> {
/// let rows = dispatch_plan_untracked(observer, plan, lease, &agent, metadata)?;
/// assert!(rows.is_empty() || !rows.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn dispatch_plan_untracked(
    observer: &dyn RunObserver,
    plan: &RitualPlan,
    lease: &Arc<LifecycleLease>,
    agent: &Arc<dyn AgentBackend>,
    metadata: RunMetadata,
) -> Result<Vec<LifecycleContributionReport>> {
    // The untracked clean epoch retains the same lease proof a tracked run
    // does: it mutates the tree, so it owns the workspace for its life — and
    // it owes the same root gate its tracked twin owes. A plan whose workspace
    // root disagrees with the lease would run contributions against a tree this
    // command never leased, and the clean point runs handlers over the very
    // tree it is about to wipe. Checked FIRST, before the run is constructed
    // and before any row is observed or executed, so a refusal costs nothing.
    lease.ensure_root(&plan.workspace_root, "at untracked phase dispatch")?;
    let mut run = LifecycleRun::untracked(
        lease.clone(),
        plan.project.clone(),
        plan.world.clone(),
        metadata.clone(),
    );
    let mut reports = Vec::with_capacity(plan.executions.len());
    let package_binding = ProjectPackageBindingBackend::new(plan);
    let runtime = runtime(observer, &package_binding, agent.as_ref());
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
                        observer.observe_untracked_failure(
                            &metadata,
                            &execution.phase,
                            &reports,
                        )?;
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
        observer.observe_contribution(&report);
        reports.push(report);
    }
    Ok(reports)
}

/// Begin a tracked run at the leased root and dispatch its plan.
///
/// Crate-private: a tracked dispatch begins a run at the LEASED root and writes
/// state there, so it is reachable only through the entry points that hold the
/// lease and have already proven their world — `run_phases` and the
/// post-durability stage. Exporting it let any caller with a plan and a lease
/// start a run beside them.
///
/// `verification_observed_at` is the verify boundary's permission and its
/// injected instant in one value — see [`verify`] for why the complete epoch
/// has to say so explicitly rather than let this cell read `metadata.chain`.
#[allow(
    clippy::too_many_arguments,
    reason = "one dispatch's named inputs, none of them bundleable"
)]
pub(crate) fn dispatch_plan(
    observer: &dyn RunObserver,
    plan: &RitualPlan,
    lease: Arc<LifecycleLease>,
    agent: &Arc<dyn AgentBackend>,
    metadata: RunMetadata,
    state_chain: Vec<String>,
    verification_observed_at: Option<Timestamp>,
    targets: Option<&MechanismTargets<'_>>,
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
    dispatch_plan_with_run(
        observer,
        plan,
        &run,
        agent,
        &metadata,
        verification_observed_at,
        targets,
    )
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
///
/// Crate-private for the same reason as [`dispatch_plan`]: it continues into a
/// run somebody else began, and only a holder of that run may do so.
pub(crate) fn dispatch_plan_with_run(
    observer: &dyn RunObserver,
    plan: &RitualPlan,
    run: &LifecycleRunHandle,
    agent: &Arc<dyn AgentBackend>,
    metadata: &RunMetadata,
    verification_observed_at: Option<Timestamp>,
    targets: Option<&MechanismTargets<'_>>,
) -> Result<DispatchOutcome> {
    let mut measured = MeasuredDispatch::default();
    dispatch_measured(
        observer,
        plan,
        run,
        agent,
        metadata,
        &mut measured,
        verification_observed_at,
        targets,
    )
    .map_err(|error| {
        // Idempotent: a stale stop and a failed handler already froze their
        // own evidence AND their own emission policy, and this must not
        // replace either with the generic stage's historical silence.
        carry_once(error, || Measurement::Lifecycle {
            rows: measured.rows,
            stopped_phase: metadata.requested.clone(),
            requested: metadata.requested.clone(),
            chain: metadata.chain.clone(),
            verification: measured.verification.map(Box::new),
        })
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "one dispatch's named inputs, none of them bundleable"
)]
fn dispatch_measured(
    observer: &dyn RunObserver,
    plan: &RitualPlan,
    run: &LifecycleRunHandle,
    agent: &Arc<dyn AgentBackend>,
    metadata: &RunMetadata,
    measured: &mut MeasuredDispatch,
    verification_observed_at: Option<Timestamp>,
    targets: Option<&MechanismTargets<'_>>,
) -> Result<DispatchOutcome> {
    let package_binding = ProjectPackageBindingBackend::new(plan);
    let runtime = runtime(observer, &package_binding, agent.as_ref());
    let mut outcome = DispatchOutcome {
        reports: Vec::with_capacity(plan.executions.len()),
        parked: None,
        verification: None,
    };
    // The permission and the instant, resolved ONCE before the first row: a
    // partial epoch (`None`) owes no boundary whatever its chain says, and a
    // complete one owes it only when the chain really contains verify.
    let mut boundary = verification_observed_at
        .and_then(|at| verify::boundary(plan, &metadata.chain).map(|end| (end, at)));
    // The three mechanism fences, armed from the same plan and the same
    // chain the verify boundary reads. They straddle that boundary exactly
    // as §2's phase line does — build, then verify, then package, then
    // deploy.
    let mut fences = mechanism::Fences::arm(targets, plan, &metadata.chain);
    let gate = verify::Gate {
        plan,
        agent,
        metadata,
        emit_machine_failure: observer.emit_machine_failure(),
    };
    let mut run = run
        .lock()
        .map_err(|_| anyhow::anyhow!("lifecycle run lock was poisoned"))?;
    run.rebind_world(plan.project.clone(), plan.world.clone())?;
    // Reconciliation can both PRODUCE rows and then fail, so the accumulator
    // is refreshed either way — a cancellation the operator needs to see must
    // not disappear because the next step went wrong.
    let reconciled = reconcile_removed_parks(&mut run, plan, &mut outcome.reports);
    measured.rows.clone_from(&outcome.reports);
    reconciled?;
    for (index, execution) in plan.executions.iter().enumerate() {
        // BEFORE the first build-or-later row, and therefore before any
        // build contribution is dispatched.
        if let Some(fences) = fences.as_mut() {
            fences.fire_build(index)?;
        }
        // BEFORE the first verify-or-later row, and therefore before any
        // verify contribution is dispatched.
        if let Some((end, at)) = boundary
            && end == index
        {
            boundary = None;
            gate.fire(&mut run, end, at, &mut outcome, measured)?;
        }
        // BEFORE the first package-or-later row, and after the boundary the
        // phase line puts between them.
        if let Some(fences) = fences.as_mut() {
            fences.fire_package(index)?;
        }
        // And BEFORE the first deploy row — the last member of the phase
        // line, so the last fence.
        if let Some(fences) = fences.as_mut() {
            fences.fire_deploy(index)?;
        }
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
                    measured.rows.clone_from(&outcome.reports);
                    return Err(carry(MeasuredFailure {
                        original,
                        evidence: Measurement::Lifecycle {
                            rows: outcome.reports.clone(),
                            stopped_phase: execution.phase.clone(),
                            requested: metadata.requested.clone(),
                            chain: metadata.chain.clone(),
                            // A verify handler that fails AFTER a matched or
                            // unavailable comparison keeps that comparison
                            // exactly: the two are independent axes, and this
                            // failure rewrites neither.
                            verification: outcome.verification.clone().map(Box::new),
                        },
                        // The policy this site has always had: the failed root
                        // is a machine document, emitted only in JSON mode and
                        // only by an unsuppressed context.
                        emit_machine_failure: observer.emit_machine_failure(),
                    }));
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
        observer.observe_contribution(&report);
        outcome.reports.push(report);
        measured.rows.clone_from(&outcome.reports);
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
    // The plan held no row at or after one of these phases: each fence and the
    // boundary are then armed at the end of the prefix, so they fire HERE —
    // after everything that could contribute, and still in phase-line order.
    // An empty build, verify or package phase therefore cannot bypass its own
    // engine-owned work.
    let end_of_plan = plan.executions.len();
    if let Some(fences) = fences.as_mut() {
        fences.fire_build(end_of_plan)?;
    }
    if let Some((end, at)) = boundary {
        gate.fire(&mut run, end, at, &mut outcome, measured)?;
    }
    if let Some(fences) = fences.as_mut() {
        fences.fire_package(end_of_plan)?;
    }
    if let Some(fences) = fences.as_mut() {
        fences.fire_deploy(end_of_plan)?;
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
            run.retain_execution_prefix(
                vibe_agent_projection::pkgskill::PROJECT_SKILL_PREFIX,
                &keep,
            )?;
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

fn runtime<'a>(
    observer: &dyn RunObserver,
    package_binding: &'a dyn PackageBindingBackend,
    agent: &'a dyn AgentBackend,
) -> HandlerRuntime<'a> {
    static PROCESS: SystemProcessRunner = SystemProcessRunner;
    static BINARY_INHERIT: WorkspaceBinaryBackend = WorkspaceBinaryBackend { quiet: false };
    static BINARY_QUIET: WorkspaceBinaryBackend = WorkspaceBinaryBackend { quiet: true };
    static PROBE: vibe_workspace::hooks::SystemProbe = vibe_workspace::hooks::SystemProbe;
    HandlerRuntime {
        process: &PROCESS,
        binary: if observer.binary_quiet() {
            &BINARY_QUIET
        } else {
            &BINARY_INHERIT
        },
        package_binding,
        agent,
        probe: &PROBE,
        streams: observer.stream_mode(),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// The `[[binary]]` lowering's reds — §7.0.7's law at the assembly that
/// arms the fences, in their own cell for the same budget reason.
#[cfg(test)]
#[path = "mechanism_tests.rs"]
mod mechanism_tests;

/// The verify-boundary reds live in their own cell: they need a declared-input
/// fixture and a mutating observer the row-accumulator reds have no use for,
/// and both files stay under the budget this way.
#[cfg(test)]
#[path = "verification_tests.rs"]
mod verification_tests;
