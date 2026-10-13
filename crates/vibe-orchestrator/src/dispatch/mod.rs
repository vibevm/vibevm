//! One-contribution lifecycle execution with per-transition checkpoints.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");
use anyhow::Result;
use std::sync::Arc;
use vibe_lifecycle::process::ProcessRunner;
use vibe_workspace::hooks::InterpreterProbe;

use vibe_lifecycle::{
    AgentBackend, ExecutionReuse, HandlerExecution, LifecycleLease, LifecycleRun,
    LifecycleRunHandle, RunMetadata, ScriptSourceStage,
};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::shared::Timestamp;

use crate::failure::{MeasuredFailure, Measurement, carry, carry_once};
use crate::ports::RunObserver;
use crate::{RitualPlan, world};

use backends::{ProjectPackageBindingBackend, native_backend};

pub use mechanism::DeployAuthority;
pub(crate) use mechanism::{DeployCarriage, MechanismTargets, lower_binaries};
mod backends;
mod mechanism;
mod native_mechanism;
mod observation;
mod reports;
mod runtime;
use reports::{contribution_status_report, reconcile_removed_parks, state_status};
use runtime::runtime;
mod outcome;
mod untracked;
mod verify;

use observation::{empty_phase_boundary, observe_empty_fence};
pub(crate) use outcome::DispatchOutcome;
use outcome::MeasuredDispatch;
pub use untracked::dispatch_plan_untracked;

/// A single already-published source stage and the actual process boundary.
/// Only the internal protected fixture supplies this; normal dispatch uses
/// the incumbent system runtime with no source stage.
#[derive(Clone, Copy)]
pub(crate) struct ProtectedScriptExecution<'a> {
    pub stage: &'a ScriptSourceStage,
    pub process: &'a dyn ProcessRunner,
    pub probe: &'a dyn InterpreterProbe,
}

#[cfg(test)]
#[path = "inject.rs"]
pub(crate) mod inject;

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
    targets: Option<MechanismTargets<'_>>,
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_plan_protected(
    observer: &dyn RunObserver,
    plan: &RitualPlan,
    lease: Arc<LifecycleLease>,
    agent: &Arc<dyn AgentBackend>,
    metadata: RunMetadata,
    state_chain: Vec<String>,
    verification_observed_at: Option<Timestamp>,
    targets: Option<MechanismTargets<'_>>,
    protected: ProtectedScriptExecution<'_>,
) -> Result<DispatchOutcome> {
    lease.ensure_root(&plan.workspace_root, "at protected phase dispatch")?;
    let run = LifecycleRun::begin(
        lease,
        plan.project.clone(),
        plan.world.clone(),
        metadata.clone(),
        state_chain,
    )?
    .shared();
    dispatch_plan_with_run_protected(
        observer,
        plan,
        &run,
        agent,
        &metadata,
        verification_observed_at,
        targets,
        Some(protected),
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
    targets: Option<MechanismTargets<'_>>,
) -> Result<DispatchOutcome> {
    dispatch_plan_with_run_protected(
        observer,
        plan,
        run,
        agent,
        metadata,
        verification_observed_at,
        targets,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn dispatch_plan_with_run_protected(
    observer: &dyn RunObserver,
    plan: &RitualPlan,
    run: &LifecycleRunHandle,
    agent: &Arc<dyn AgentBackend>,
    metadata: &RunMetadata,
    verification_observed_at: Option<Timestamp>,
    targets: Option<MechanismTargets<'_>>,
    protected: Option<ProtectedScriptExecution<'_>>,
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
        protected,
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
    targets: Option<MechanismTargets<'_>>,
    protected: Option<ProtectedScriptExecution<'_>>,
) -> Result<DispatchOutcome> {
    let package_binding = ProjectPackageBindingBackend::new(plan);
    let native_candidates = plan.native_candidates.iter().collect::<Vec<_>>();
    let native = native_backend(plan, metadata, &native_candidates)?;
    let runtime = runtime(
        observer,
        &package_binding,
        &native,
        agent.as_ref(),
        protected,
        vibe_lifecycle::InvocationIntent::from_requested(&metadata.requested)
            .is_some_and(|intent| intent.admits_build_artifact()),
    );
    let mut outcome = DispatchOutcome {
        reports: Vec::with_capacity(plan.executions.len()),
        parked: None,
        verification: None,
        phase_terminals: Default::default(),
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
    let build_work = targets
        .as_ref()
        .is_some_and(|targets| !targets.build.is_empty());
    let package_work = targets
        .as_ref()
        .is_some_and(|targets| !targets.package.is_empty());
    let deploy_work = targets
        .as_ref()
        .is_some_and(|targets| targets.deploy.is_some() && !targets.deploy_targets.is_empty());
    let mut fences = mechanism::Fences::arm(targets, plan, &metadata.chain);
    let empty_build_at = empty_phase_boundary(plan, &metadata.chain, vibe_lifecycle::Phase::Build);
    let empty_package_at =
        empty_phase_boundary(plan, &metadata.chain, vibe_lifecycle::Phase::Package);
    let empty_deploy_at =
        empty_phase_boundary(plan, &metadata.chain, vibe_lifecycle::Phase::Deploy);
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
            if empty_build_at == Some(index) {
                observe_empty_fence(
                    observer,
                    "build",
                    if build_work { "ok" } else { "no-op" },
                    || fences.fire_build(index),
                )?;
                outcome.phase_terminals.insert(
                    "build".into(),
                    if build_work { "ok" } else { "no-op" }.into(),
                );
            } else {
                fences.fire_build(index)?;
            }
        }
        // BEFORE the first verify-or-later row, and therefore before any
        // verify contribution is dispatched.
        if let Some((end, at)) = boundary
            && end == index
        {
            boundary = None;
            if plan.count_for(vibe_lifecycle::Phase::Verify) == 0 {
                observe_empty_fence(observer, "verify", "ok", || {
                    gate.fire(&mut run, end, at, &mut outcome, measured)
                })?;
                outcome.phase_terminals.insert("verify".into(), "ok".into());
            } else {
                gate.fire(&mut run, end, at, &mut outcome, measured)?;
            }
        }
        // BEFORE the first package-or-later row, and after the boundary the
        // phase line puts between them.
        if let Some(fences) = fences.as_mut() {
            if empty_package_at == Some(index) {
                observe_empty_fence(
                    observer,
                    "package",
                    if package_work { "ok" } else { "no-op" },
                    || fences.fire_package(index),
                )?;
                outcome.phase_terminals.insert(
                    "package".into(),
                    if package_work { "ok" } else { "no-op" }.into(),
                );
            } else {
                fences.fire_package(index)?;
            }
        }
        // And BEFORE the first deploy row — the last member of the phase
        // line, so the last fence.
        if let Some(fences) = fences.as_mut() {
            if empty_deploy_at == Some(index) {
                observe_empty_fence(
                    observer,
                    "deploy",
                    if deploy_work { "ok" } else { "no-op" },
                    || fences.fire_deploy(index),
                )?;
                outcome.phase_terminals.insert(
                    "deploy".into(),
                    if deploy_work { "ok" } else { "no-op" }.into(),
                );
            } else {
                fences.fire_deploy(index)?;
            }
        }
        observer.observe_contribution_started(&execution.phase, &execution.row.key().to_string());
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
                    let report = contribution_status_report(
                        execution,
                        "fail",
                        Some(message),
                        Some(&streams),
                    );
                    observer.observe_contribution_terminal(&report);
                    observer.observe_phase_finished(&execution.phase, "fail");
                    outcome.reports.push(report);
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
        observer.observe_contribution_terminal(&report);
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
        if empty_build_at == Some(end_of_plan) {
            observe_empty_fence(
                observer,
                "build",
                if build_work { "ok" } else { "no-op" },
                || fences.fire_build(end_of_plan),
            )?;
            outcome.phase_terminals.insert(
                "build".into(),
                if build_work { "ok" } else { "no-op" }.into(),
            );
        } else {
            fences.fire_build(end_of_plan)?;
        }
    }
    if let Some((end, at)) = boundary {
        if plan.count_for(vibe_lifecycle::Phase::Verify) == 0 {
            observe_empty_fence(observer, "verify", "ok", || {
                gate.fire(&mut run, end, at, &mut outcome, measured)
            })?;
            outcome.phase_terminals.insert("verify".into(), "ok".into());
        } else {
            gate.fire(&mut run, end, at, &mut outcome, measured)?;
        }
    }
    if let Some(fences) = fences.as_mut() {
        if empty_package_at == Some(end_of_plan) {
            observe_empty_fence(
                observer,
                "package",
                if package_work { "ok" } else { "no-op" },
                || fences.fire_package(end_of_plan),
            )?;
            outcome.phase_terminals.insert(
                "package".into(),
                if package_work { "ok" } else { "no-op" }.into(),
            );
        } else {
            fences.fire_package(end_of_plan)?;
        }
    }
    if let Some(fences) = fences.as_mut() {
        if empty_deploy_at == Some(end_of_plan) {
            observe_empty_fence(
                observer,
                "deploy",
                if deploy_work { "ok" } else { "no-op" },
                || fences.fire_deploy(end_of_plan),
            )?;
            outcome.phase_terminals.insert(
                "deploy".into(),
                if deploy_work { "ok" } else { "no-op" }.into(),
            );
        } else {
            fences.fire_deploy(end_of_plan)?;
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
            run.retain_execution_prefix(
                vibe_agent_projection::pkgskill::PROJECT_SKILL_PREFIX,
                &keep,
            )?;
        }
    }
    Ok(outcome)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// The `[[binary]]` lowering's reds — §7.0.7's law at the assembly that
/// arms the fences, in their own cell for the same budget reason.
#[cfg(test)]
#[path = "mechanism_tests.rs"]
mod mechanism_tests;
#[cfg(test)]
#[path = "native_all_owner_tests.rs"]
mod native_all_owner_tests;
#[cfg(test)]
mod native_mechanism_tests;

/// The verify-boundary reds live in their own cell: they need a declared-input
/// fixture and a mutating observer the row-accumulator reds have no use for,
/// and both files stay under the budget this way.
#[cfg(test)]
#[path = "verification_tests.rs"]
mod verification_tests;
