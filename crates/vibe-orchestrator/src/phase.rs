//! The executed region of a default-lifecycle verb: everything between the
//! surface opening its trace and handing one typed outcome back to its funnel.
//!
//! [`run_phases`] returns a VALUE, never a bare `Result`. That is the whole
//! shape of this cell: on the CLI an open recorder holds the project's
//! cooperative lock and leaves a `running` index on disk, so a `?` escaping to
//! the caller would release the lock by dropping a handle and leave a run that
//! claims forever to be in progress. The inner body keeps its ordinary
//! `Result` ergonomics, and its failure is handed outward exactly once against
//! an accumulator holding whatever rows had really been measured when it
//! stopped.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use specmark::spec;

use std::sync::Arc;

use anyhow::{Context, Result};
use vibe_lifecycle::{AgentBackend, LifecycleLease, Phase, RunMetadata};
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleDelegation, LifecycleStepReport,
};
use vibe_wire::generated::shared::{Timestamp, VerificationEvidence};
use vibe_workspace::compile_trace::TraceRun;

use crate::dispatch::{DeployCarriage, MechanismTargets, lower_binaries};
use crate::failure::{MeasuredFailure, Measurement, prepend_rows, take};
use crate::install::{
    InstallDisposition, InstallExecution, InstallInputs, InstallPolicy, PreparedSelection,
    ProvenSelection,
};
use crate::phase::prerequisite::PrerequisiteInstall;
use crate::plan::surface_plan;
use crate::ports::{
    ConfirmGate, InstallManifestMutation, InstallObserver, PackageSourceFactory,
    RegistryEnvironment, RunObserver,
};
use crate::values::{
    LifecycleValues, StepStatus, contribution_report, delegation_member, step_report,
};
use crate::{dispatch, world};

/// The prerequisite install's collector, and the law over its captured tree.
mod prerequisite;

/// The prepared inputs one phase run owns, plus whatever the clean epoch
/// already contributed.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct PhaseRun<'a> {
    /// The requested phase.
    pub requested: Phase,
    /// The default phases this run executes, in chain order.
    pub phases: Vec<Phase>,
    /// The complete requested chain, including a clean prefix.
    pub chain: Vec<String>,
    /// The invocation's durable identity and effective posture.
    pub metadata: RunMetadata,
    /// The surface-neutral install inputs of the prerequisite install.
    pub install_args: InstallInputs,
    /// The narrow execution policy of the prerequisite install.
    pub policy: InstallPolicy,
    /// The command's mutation lease, from the prelude epoch. The prerequisite
    /// install and the phase dispatch both consume this one acquisition; the
    /// command boundary holds the owner through the finalised report.
    pub lease: Arc<LifecycleLease>,
    /// The command's ONE selected-world provenance bundle, from the prelude
    /// epoch (or the single post-clean reload): the canonical root, the manifest
    /// snapshot taken at it, and the tree built from THAT snapshot. Proven at
    /// the TOP of the executed region below — including on a validate-only
    /// chain, where dropping it would let a stored parse error be replaced by
    /// whatever a later workspace read happened to notice.
    pub selection: PreparedSelection,
    /// Step rows the clean epoch already produced.
    pub steps: Vec<LifecycleStepReport>,
    /// Contribution rows the clean epoch already produced.
    pub contributions: Vec<LifecycleContributionReport>,
    /// Notices the clean epoch already produced.
    pub notices: Vec<String>,
    /// The OUTER phase observation policy.
    pub observer: &'a dyn RunObserver,
    /// The CHILD install observation policy — never the same object.
    pub install_observer: &'a dyn InstallObserver,
    /// The surface's confirmation policy for the prerequisite install.
    pub confirm_gate: &'a dyn ConfirmGate,
    /// The surface's package-source composition root.
    pub sources: &'a dyn PackageSourceFactory,
    /// Where this run's registry environment is seeded and loaded — once.
    pub environment: &'a dyn RegistryEnvironment,
    /// The surface's own manifest mutation for the prerequisite install. A
    /// phase verb admits no manifest-mutating flag, so it injects the named
    /// no-op — stated in the call graph rather than assumed.
    pub manifest_mutation: &'a dyn InstallManifestMutation,
    /// The ONE agent backend every agent row of this run is served by.
    pub agent: Arc<dyn AgentBackend>,
    /// The surface's compile-trace recorder, borrowed.
    pub trace: Option<&'a TraceRun>,
    /// Everything the COMMAND LAYER resolved for this run's deploy half,
    /// when it resolved any (§7.0.5: "Profile resolution happens ONCE, in
    /// the command layer that owns flags, and travels as data"; §6.3.0.6:
    /// "Home and executable authority are injected"). `None` is a run that
    /// carries no deploy half — a chain that never reaches deploy, or a
    /// project that declares no deploy section — and the deploy fence then
    /// arms nothing at all.
    pub deploy: Option<crate::DeployAuthority>,
    /// The surface's injected instant for the verify comparison.
    ///
    /// This entry point owns the COMPLETE derived chain, so handing it down is
    /// also the permission for the engine-owned verify boundary to fire at all
    /// — the partial install callback deliberately withholds it. Nothing below
    /// a surface reads a clock, so the instant a member records is the one the
    /// command was given.
    pub observed_at: Timestamp,
}

/// What one phase run produced.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub enum PhaseOutcome {
    /// The chain ran to completion.
    Completed(LifecycleValues),
    /// The chain parked at a hosted agent row.
    Parked(LifecycleValues),
    /// The chain failed; the surface owns the report family and the exit code.
    Failed {
        /// The rows and phase this run had really measured.
        measurement: Measurement,
        /// The caller's error, unchanged.
        original: anyhow::Error,
        /// Whether the failing site emitted a machine document with tracing off.
        emit_machine_failure: bool,
    },
}

/// Rows measured so far, so a failure reports what really ran — and the
/// verification member the verify boundary had already reconciled, so a
/// failure AFTER it reports the comparison rather than none.
#[derive(Default)]
struct Measured {
    contributions: Vec<LifecycleContributionReport>,
    verification: Option<VerificationEvidence>,
}

/// Absorb a NEUTRAL resume failure from the prerequisite install into THIS
/// command's accumulator, then let the error travel on uncarried.
///
/// The substrate transports the measurement without naming a family, because
/// the same code is also `vibe install`'s body and `vibe update --all`'s
/// delegate. Here the family is the lifecycle one, and this command already has
/// a mechanism for choosing it — the fallback in `execute_after_open` — so the
/// only thing missing is the rows. They are the resumed run's ONLY copy: the
/// resume built its own lifecycle, and nothing in this function ever saw it.
///
/// Appended in chronology, after whatever the clean epoch and the install
/// callback already recorded, and the ORIGINAL error is returned so the
/// fallback keeps its historical silence and its exact downcast identity.
fn absorb_resume_failure(error: anyhow::Error, measured: &mut Measured) -> anyhow::Error {
    match take(error) {
        // The NEUTRAL resume transport, and only it. An
        // `InstallBarrier` measurement was frozen by the substrate as
        // install-shaped, and a phase verb has always reported the prerequisite
        // install's slot failure in that family — so it is re-carried, not
        // absorbed.
        Ok(MeasuredFailure {
            original,
            evidence: Measurement::Slot { reports, .. },
            ..
        }) => {
            measured
                .contributions
                .extend(reports.into_iter().map(contribution_report));
            original
        }
        Ok(other) => crate::failure::carry(other),
        Err(error) => error,
    }
}

/// The one boundary. No `?` escapes it.
///
/// ```no_run
/// use vibe_orchestrator::{PhaseOutcome, PhaseRun, run_phases};
/// # fn call(inputs: PhaseRun<'_>) {
/// match run_phases(inputs) {
///     PhaseOutcome::Completed(values) => assert!(values.ok),
///     PhaseOutcome::Parked(values) => assert!(values.delegation.is_some()),
///     PhaseOutcome::Failed { .. } => {}
/// }
/// # }
/// ```
pub fn run_phases(inputs: PhaseRun<'_>) -> PhaseOutcome {
    let requested = inputs.requested;
    let chain = inputs.chain.clone();
    let mut measured = Measured {
        contributions: inputs.contributions.clone(),
        verification: None,
    };
    match run(inputs, &mut measured) {
        Ok(Outcome::Completed(values)) => PhaseOutcome::Completed(values),
        Ok(Outcome::Parked(values)) => PhaseOutcome::Parked(values),
        // A handler failure arrives CARRIED, with the rows and emission policy
        // its own site froze. Everything else is a generic stage failure:
        // boot, publication, planning, continuation, world, surface or a
        // malformed handoff — all of which were historically silent.
        Err(error) => match take(error) {
            Ok(MeasuredFailure {
                original,
                evidence,
                emit_machine_failure,
            }) => PhaseOutcome::Failed {
                measurement: evidence,
                original,
                emit_machine_failure,
            },
            Err(original) => PhaseOutcome::Failed {
                measurement: Measurement::Lifecycle {
                    rows: measured.contributions,
                    stopped_phase: requested.as_str().to_string(),
                    requested: requested.as_str().to_string(),
                    chain,
                    // A malformed handoff on a verify-phase agent row is the
                    // real shape here: it arrives uncarried, AFTER the
                    // boundary already reconciled, so the accumulator is the
                    // only thing left holding that comparison.
                    verification: measured.verification.map(Box::new),
                },
                original,
                emit_machine_failure: false,
            },
        },
    }
}

enum Outcome {
    Completed(LifecycleValues),
    Parked(LifecycleValues),
}

#[allow(
    clippy::too_many_lines,
    reason = "one phase run, in its canonical order"
)]
fn run(inputs: PhaseRun<'_>, measured: &mut Measured) -> Result<Outcome> {
    let PhaseRun {
        requested,
        phases,
        chain,
        metadata,
        install_args,
        policy,
        lease,
        selection,
        mut steps,
        contributions,
        mut notices,
        observer,
        install_observer,
        confirm_gate,
        sources,
        environment,
        manifest_mutation,
        agent,
        trace,
        deploy,
        observed_at,
    } = inputs;
    // The ONE proof of the bundle, before validate and before install.
    //
    // A malformed selected manifest is THIS command's error, with the words its
    // own read produced — not a later, vaguer one from whichever workspace read
    // happened to run next. And the tree in the same breath: a validate-only
    // chain has no install phase, so nothing downstream used to consume it, the
    // state was simply dropped, validate reported OK, and `world::plan_default`
    // then discovered the tree AGAIN. A sibling repaired between the two reads
    // would turn the first failure into a success — the exact retry the bundle
    // exists to forbid. So it is proven here, before any phase runs.
    let (project_root, manifest, workspace) = selection.prove()?.into_parts();
    // The declared artifact graph and the host's mechanism routes, taken off
    // the ONE proven manifest before it is rebound into the prerequisite
    // install's bundle below. They are the whole manifest surface the mechanism
    // executors need — target rows and provider pins, never configuration — so
    // reading them here keeps the boundary's no-manifest-below-the-surface rule
    // exactly as it was.
    let artifacts = manifest.artifacts.clone();
    // The legacy `[[binary]]` rows and the declared deploy targets, taken
    // off the SAME proven manifest and in the same breath as the artifact
    // graph — §7.0.7's lowering call site and the deploy fence's target
    // set both read them, and a second manifest read would be a second
    // answer.
    let binaries = manifest.binaries.clone();
    let deploy_targets = manifest
        .deploy
        .as_ref()
        .map(|section| section.targets.clone())
        .unwrap_or_default();
    // The deploy carriage: the command layer's resolved selection plus the
    // engine's own state home and identity. Assembled once, here, from the
    // proven manifest — never re-derived below.
    let deploy_carriage = match deploy {
        Some(authority) => Some(DeployCarriage::assemble(authority, &manifest)?),
        None => None,
    };
    // §7.0.7's lowering, at the assembly that arms the fences: a legacy
    // `[[binary]]` IS a build target after this call, so the executor has
    // no legacy case.
    let build_targets = lower_binaries(artifacts.as_ref(), &binaries)?;
    // ---- the agreement gate, before validate and before any state work ---
    //
    // `run_phases` is a PUBLIC entry point, and a validate-only chain reaches
    // state and outbox writes without ever entering the install core's own
    // gate. The lease is the state root and the metadata names the selected
    // node, so a bundle agreeing with neither would validate — and then write —
    // against a tree this command never leased. Both refusals are the lease's
    // own typed gates.
    lease.ensure_root(&workspace.root, "at phase execution")?;
    let observed_selected = workspace
        .node_rel_of(&project_root)
        .map(|rel| rel.as_str().to_string());
    lease.ensure_selected(
        &metadata.selected,
        observed_selected.as_deref(),
        "at phase execution",
    )?;
    let prelude_workspace = workspace.clone();
    // Rebound into the bundle for the prerequisite install: the pair this run
    // just proved, never two fields the install could be handed separately.
    let mut install_inputs = Some(PreparedSelection::proven(ProvenSelection::from_parts(
        project_root.clone(),
        manifest,
        workspace,
    )));
    // The named collector this run's prerequisite install reports back through.
    // It is NOT a closure: what the phase loop needs out of the stage — the
    // shared slot run, its rows, and the exact post-install tree — is a stated
    // shape, and a `FnOnce` would let a surface capture anything at all here.
    let mut collector = PrerequisiteInstall::default();
    // The workspace the LATER phases plan against. It starts as the prelude's
    // load and is replaced by the one the prerequisite install ended with —
    // the only copy carrying that install's in-memory `--git` delta. A
    // rediscovery here would collect a world this command did not produce.
    let mut validate_status = None;
    let mut install_status = None;

    for phase in &phases {
        match phase {
            Phase::Validate => {
                // The manifest parsed and the tree LOADED — both consumed
                // above, and a failure of either already returned. Validation
                // is therefore proven by the reads this command made, not by
                // repeating them, and only a `Loaded` state can reach here.
                validate_status = Some(StepStatus::Ok);
            }
            Phase::Install => {
                let selection = install_inputs
                    .take()
                    .context("internal: the prepared install inputs were consumed twice")?;
                let install_run = crate::install::execute_prepared(
                    InstallExecution {
                        args: install_args.clone(),
                        environment,
                        policy,
                        lease: lease.clone(),
                        manifest_mutation,
                        // The pair this run already proved, rebound: the install's
                        // own boundary keeps its shape without a second read, and
                        // there are no separate pieces to mispair.
                        selection,
                        metadata: metadata.clone(),
                        sources,
                        confirm_gate,
                        observer: install_observer,
                        agent: agent.clone(),
                        // The command owner's recorder, borrowed: the
                        // prerequisite install's compiles belong to THIS run's
                        // trace, not to one of its own.
                        trace,
                    },
                    &mut collector,
                )
                .map_err(|error| absorb_resume_failure(error, measured))?;
                // A parked prerequisite install stops the whole chain — and
                // THIS command renders the one document, because it is the
                // outermost one. The step list is the prefix that really ran:
                // whatever preceded install, then `install: delegated`, and
                // nothing after it.
                if let Some(delegation) = install_run.parked {
                    let mut prefix = steps;
                    if validate_status.is_some() {
                        prefix.push(step_report(
                            Phase::Validate.as_str(),
                            validate_status.unwrap_or(StepStatus::Ok),
                        ));
                    }
                    prefix.push(step_report(Phase::Install.as_str(), StepStatus::Delegated));
                    let mut rows = collector.take_rows();
                    rows.extend(
                        install_run
                            .slot_reports
                            .into_iter()
                            .map(contribution_report),
                    );
                    measured.contributions.clone_from(&rows);
                    // Fallible, and therefore BEFORE the park is decided: a
                    // malformed handoff is a failed command, never a finalised
                    // park whose renderer later refuses.
                    let member = delegation_member(delegation)?;
                    return Ok(Outcome::Parked(LifecycleValues::completed(
                        requested.as_str(),
                        chain,
                        prefix,
                        rows,
                        notices,
                        Some(member),
                    )));
                }
                install_status = Some(match install_run.disposition {
                    InstallDisposition::Fresh => StepStatus::Fresh,
                    InstallDisposition::Applied => StepStatus::Ok,
                    InstallDisposition::Parked => unreachable!("returned above"),
                });
            }
            _ => {}
        }
    }

    // The rows this command has ALREADY measured, in the order they HAPPENED:
    // the clean epoch's contributions, then the prerequisite install's slot
    // rows. Frozen HERE — before planning and before surfacing, both of which
    // are fallible — so any later failure reports the work already done rather
    // than an empty run.
    let mut prefix = contributions.clone();
    prefix.extend(collector.rows().iter().cloned());
    measured.contributions.clone_from(&prefix);

    // The tree the remaining phases plan against — proven, not guessed. A
    // skipped stage is an internal error here, never a silent fall back to the
    // pre-install world.
    let planning_workspace =
        collector.planning_workspace(&prelude_workspace, phases.contains(&Phase::Install))?;
    let ritual = world::plan_default_prepared(&project_root, planning_workspace, &phases)?;
    notices.extend(ritual.notices.clone());
    surface_plan(observer, &ritual, &metadata, true)?;
    // ---- the ONE mechanism wiring (§6.0.2) ---------------------------
    //
    // The declared artifact graph travels INTO the contribution dispatch, which
    // is the only walk in this engine that visits phases in the requested
    // chain's order. That is what makes §2's phase line hold: every
    // `phase:generate` contribution is dispatched before the build fence fires,
    // and the package fence fires after the verify boundary. A manifest that
    // declares neither family reaches an empty executor, and the run is
    // byte-identical to the historical one.
    let created_at = observed_at.to_rfc3339();
    let targets = MechanismTargets {
        project_root: &project_root,
        build: &build_targets,
        package: artifacts.as_ref().map_or(
            &[] as &[vibe_core::manifest::ArtifactPackageTarget],
            |section| &section.package,
        ),
        deploy_targets: &deploy_targets,
        registry: &ritual.mechanisms,
        routes: &ritual.mechanism_routes,
        native_candidates: &ritual.native_candidates,
        offline: metadata.offline,
        created_at: &created_at,
        deploy: deploy_carriage.as_ref(),
    };
    let state_chain = phases.iter().map(ToString::to_string).collect();
    // `Some(...)` here, and ONLY here on the tracked path: this is the
    // complete derived chain, so the engine-owned verify boundary is allowed
    // to fire — see `dispatch::verify` for the partial epoch it is not.
    let dispatched = if let Some(shared) = collector.into_lifecycle_run() {
        dispatch::dispatch_plan_with_run(
            observer,
            &ritual,
            &shared,
            &agent,
            &metadata,
            Some(observed_at),
            Some(&targets),
        )
    } else {
        dispatch::dispatch_plan(
            observer,
            &ritual,
            lease,
            &agent,
            metadata,
            state_chain,
            Some(observed_at),
            Some(&targets),
        )
    };
    let outcome = match dispatched {
        Ok(outcome) => outcome,
        // A carried failure keeps its own draft — its error, its root family,
        // its emission policy — and gains the prefix it could not have known
        // about. An uncarried one is left alone so the outer fallback builds
        // from the same accumulator.
        Err(error) => return Err(prepend_rows(error, prefix)),
    };
    let parked = outcome.parked;
    // Frozen into the accumulator BEFORE the fallible handoff validation
    // below: that refusal travels uncarried, and the fallback in `run_phases`
    // is then the only carrier left for a comparison this run really made.
    let verification = outcome.verification;
    measured.verification.clone_from(&verification);
    // The one canonical order, reused: clean rows, then the prerequisite
    // install's slot rows, then this phase's own. The prerequisite rows belong
    // to THIS report in every mode — they used to be excluded from JSON
    // because each was echoed as its own document, and that echo is gone.
    let mut contributions = prefix;
    contributions.extend(outcome.reports);
    measured.contributions.clone_from(&contributions);
    for phase in phases {
        let status = match phase {
            Phase::Validate => validate_status.unwrap_or(StepStatus::Ok),
            Phase::Install => install_status.unwrap_or(StepStatus::Ok),
            _ if ritual.count_for(phase) == 0 => StepStatus::NoOp,
            _ if contributions
                .iter()
                .filter(|row| row.phase == phase.as_str())
                .all(|row| row.status == "fresh") =>
            {
                StepStatus::Fresh
            }
            _ => StepStatus::Ok,
        };
        // Steps end AT the parked phase: later phases did not run, so they
        // are not reported as if they had.
        let parked_here = parked
            .as_ref()
            .is_some_and(|(stopped, _)| stopped == phase.as_str());
        steps.push(step_report(
            phase.as_str(),
            if parked_here {
                StepStatus::Delegated
            } else {
                status
            },
        ));
        if parked_here {
            break;
        }
    }

    let member: Option<LifecycleDelegation> = parked
        .map(|(_, delegation)| delegation_member(delegation))
        .transpose()?;
    let parked = member.is_some();
    let draft = LifecycleValues::completed_with_verification(
        requested.as_str(),
        chain,
        steps,
        contributions,
        notices,
        member,
        verification,
    );
    Ok(if parked {
        Outcome::Parked(draft)
    } else {
        Outcome::Completed(draft)
    })
}

#[cfg(test)]
#[path = "phase/tests.rs"]
mod tests;
