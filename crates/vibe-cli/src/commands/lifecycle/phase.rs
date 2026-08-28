//! The executed region of a default-lifecycle verb: everything between the
//! command opening its trace and handing one typed exit back to the funnel.
//!
//! `execute_after_open` returns a VALUE, never a `Result`. That is the whole
//! shape of this cell: an open recorder holds the project's cooperative lock
//! and leaves a `running` index on disk, so a `?` escaping to the caller would
//! release the lock by dropping a handle and leave a run that claims forever
//! to be in progress. The inner body keeps its ordinary `Result` ergonomics and
//! is classified exactly once, against an accumulator holding whatever rows had
//! really been measured when it stopped.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::PathBuf;

use anyhow::{Context, Result};
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{LifecycleLease, Phase, RunMetadata};
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleDelegation, LifecycleStepReport,
};
use vibe_workspace::compile_trace::TraceRun;

use crate::cli::InstallArgs;
use crate::commands::compile_trace::{
    CommandExit, RegisteredReportDraft, classify, prepend_lifecycle_rows,
};
use crate::commands::install::{InstallExecution, PreparedWorkspace, SelectedManifest};
use crate::output;

use super::draft::LifecycleDraft;
use super::{
    InstallDisposition, RunObserver, StepStatus, dispatch, report, slot, step_report, surface_plan,
    world,
};

/// The prepared inputs one phase run owns, plus whatever the clean epoch
/// already contributed.
pub(super) struct PhaseInputs {
    pub(super) requested: Phase,
    pub(super) phases: Vec<Phase>,
    pub(super) chain: Vec<String>,
    pub(super) metadata: RunMetadata,
    pub(super) install_args: InstallArgs,
    pub(super) root_offline: bool,
    /// The command's mutation lease, from the prelude epoch. The prerequisite
    /// install and the phase dispatch both consume this one acquisition; the
    /// command boundary holds the owner through the finalised report.
    pub(super) lease: std::sync::Arc<LifecycleLease>,
    /// The ONE canonical selection of this command's project root, from the
    /// prelude epoch (or the single post-clean epoch). Nothing below
    /// re-resolves a path.
    pub(super) project_root: PathBuf,
    /// The command's ONE `UserConfig` load, carried into the prerequisite
    /// install rather than repeated there.
    pub(super) user_config: UserConfig,
    /// The command's ONE selected-manifest snapshot. Consumed at the TOP of
    /// the executed region below — including on a validate-only chain, where
    /// dropping it would let a stored parse error be replaced by whatever a
    /// later workspace read happened to notice.
    pub(super) manifest: SelectedManifest,
    /// What the ONE workspace load produced — carried, never retried.
    pub(super) workspace: PreparedWorkspace,
    pub(super) steps: Vec<LifecycleStepReport>,
    pub(super) contributions: Vec<LifecycleContributionReport>,
    pub(super) notices: Vec<String>,
}

/// Rows measured so far, so a failure reports what really ran.
#[derive(Default)]
struct Measured {
    contributions: Vec<LifecycleContributionReport>,
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
    match crate::commands::install::take_resume_failure(error) {
        Ok(failure) => {
            measured
                .contributions
                .extend(failure.reports.into_iter().map(slot::contribution_report));
            failure.original
        }
        Err(error) => error,
    }
}

/// The one boundary. No `?` escapes it.
pub(super) fn execute_after_open(
    ctx: &output::Context,
    child: &output::Context,
    observer: &dyn RunObserver,
    inputs: PhaseInputs,
    prepare_install: impl FnOnce() -> Option<PathBuf>,
    trace: Option<&TraceRun>,
) -> CommandExit<RegisteredReportDraft> {
    let requested = inputs.requested;
    let chain = inputs.chain.clone();
    let mut measured = Measured {
        contributions: inputs.contributions.clone(),
    };
    match run(
        ctx,
        child,
        observer,
        inputs,
        prepare_install,
        trace,
        &mut measured,
    ) {
        Ok(Outcome::Completed(draft)) => {
            CommandExit::Success(RegisteredReportDraft::Lifecycle(Box::new(draft)))
        }
        Ok(Outcome::Parked(draft)) => {
            CommandExit::Parked(RegisteredReportDraft::Lifecycle(Box::new(draft)))
        }
        // A handler failure arrives CARRIED, with the rows and emission policy
        // its own site froze. Everything else is a generic stage failure:
        // boot, publication, planning, continuation, world, surface or a
        // malformed handoff — all of which were historically silent, and all
        // of which report this command's own registered family.
        Err(error) => classify(error, || {
            RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
                requested.as_str(),
                chain,
                requested.as_str(),
                measured.contributions,
            )))
        }),
    }
}

enum Outcome {
    Completed(LifecycleDraft),
    Parked(LifecycleDraft),
}

#[allow(
    clippy::too_many_lines,
    reason = "one phase run, in its canonical order"
)]
fn run(
    ctx: &output::Context,
    child: &output::Context,
    observer: &dyn RunObserver,
    inputs: PhaseInputs,
    prepare_install: impl FnOnce() -> Option<PathBuf>,
    trace: Option<&TraceRun>,
    measured: &mut Measured,
) -> Result<Outcome> {
    let PhaseInputs {
        requested,
        phases,
        chain,
        metadata,
        mut install_args,
        root_offline,
        lease,
        project_root,
        user_config,
        manifest,
        workspace,
        mut steps,
        contributions,
        mut notices,
    } = inputs;
    let mut prepare_install = Some(prepare_install);
    // The ONE consumption of the snapshot, before validate and before
    // install. A malformed selected manifest is THIS command's error, with the
    // words its own read produced — not a later, vaguer one from whichever
    // workspace read happened to run next.
    let manifest = manifest.into_manifest()?;
    // And the ONE consumption of the prepared workspace, in the same breath.
    //
    // A validate-only chain has no install phase, so nothing downstream used
    // to consume this: the state was simply dropped, validate reported OK, and
    // `world::plan_default` then discovered the tree AGAIN. A sibling repaired
    // between the two reads would turn the first failure into a success — the
    // exact retry the typed state exists to forbid. So it is refused here,
    // before any phase runs, and only `Loaded` reaches the rest of this run.
    let workspace = match workspace {
        PreparedWorkspace::Loaded(workspace) => *workspace,
        // The FIRST answer, with the same context the install path gives it.
        PreparedWorkspace::DiscoveryFailed(error) => {
            return Err(anyhow::Error::new(*error)
                .context("discovering the workspace enclosing the project"));
        }
        // Unreachable: the manifest error above returns first. Named rather
        // than merged so a future caller that rewraps a parsed manifest beside
        // this arm is a compile-time question, not a silent success.
        PreparedWorkspace::SelectedManifestInvalid => {
            anyhow::bail!(
                "internal: the selected manifest was reported invalid but its error was                  already consumed"
            );
        }
        // A lifecycle command ALWAYS has a prelude; `DiscoverHere` is the
        // compatibility arm for callers that do not.
        PreparedWorkspace::DiscoverHere => {
            anyhow::bail!(
                "internal: a lifecycle run reached execution without a prepared workspace"
            );
        }
    };
    let prelude_workspace = workspace.clone();
    let mut install_inputs = Some((
        user_config,
        manifest,
        PreparedWorkspace::Loaded(Box::new(workspace)),
    ));
    let mut install_lifecycle_run = None;
    let mut install_contribution_reports = Vec::new();
    // The workspace the LATER phases plan against. It starts as the prelude's
    // load and is replaced by the one the prerequisite install ended with —
    // the only copy carrying that install's in-memory `--git` delta. A
    // rediscovery here would collect a world this command did not produce.
    let mut current_workspace = None;
    let mut validate_status = None;
    let mut install_status = None;

    for phase in &phases {
        match phase {
            Phase::Validate => {
                // The manifest parsed and the tree LOADED — both consumed
                // above, and a failure of either already returned. Validation
                // is therefore proven by the reads this command made, not by
                // repeating them, and only a `Loaded` state can reach here.
                install_args.path = project_root.clone();
                validate_status = Some(StepStatus::Ok);
            }
            Phase::Install => {
                let prepare = prepare_install
                    .take()
                    .context("internal: install inputs prepared more than once")?;
                let (user_config, manifest, workspace) = install_inputs
                    .take()
                    .context("internal: the prepared install inputs were consumed twice")?;
                let install_args = install_args.clone();
                let confirm_gate =
                    crate::commands::install::CliConfirmGate::new(child, install_args.assume_yes);
                let install_run = crate::commands::install::execute_prepared(
                    child,
                    InstallExecution {
                        args: install_args,
                        embedded_root: prepare(),
                        root_offline,
                        lease: lease.clone(),
                        project_root: project_root.clone(),
                        user_config,
                        // Already parsed above; rewrapped so the install's own
                        // boundary keeps its shape without a second read.
                        manifest: SelectedManifest::parsed(manifest),
                        workspace,
                        metadata: metadata.clone(),
                        resolver_factory: &crate::commands::install::CliResolverFactory,
                        confirm_gate: &confirm_gate,
                        lifecycle_output: Some(ctx),
                        // The command owner's recorder, borrowed: the
                        // prerequisite install's compiles belong to THIS run's
                        // trace, not to one of its own.
                        trace,
                    },
                    |_, _, run, workspace| {
                        install_lifecycle_run = run.lifecycle_run;
                        install_contribution_reports = run
                            .lifecycle_reports
                            .into_iter()
                            .map(slot::contribution_report)
                            .collect();
                        // Captured, not re-read: this is the exact tree the
                        // install finished with.
                        current_workspace = Some(workspace.clone());
                        Ok(crate::commands::install::WorldCallbackOutcome::default())
                    },
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
                    let mut rows = install_contribution_reports;
                    rows.extend(
                        install_run
                            .slot_reports
                            .into_iter()
                            .map(slot::contribution_report),
                    );
                    measured.contributions.clone_from(&rows);
                    // Fallible, and therefore BEFORE the park is decided: a
                    // malformed handoff is a failed command, never a finalised
                    // park whose renderer later refuses.
                    let member = report::delegation_member(delegation)?;
                    return Ok(Outcome::Parked(LifecycleDraft::completed(
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
    prefix.extend(install_contribution_reports.iter().cloned());
    measured.contributions.clone_from(&prefix);

    let ritual = world::plan_default_prepared(
        &project_root,
        current_workspace.as_ref().unwrap_or(&prelude_workspace),
        &phases,
    )?;
    notices.extend(ritual.notices.clone());
    surface_plan(observer, &ritual, &metadata, true)?;
    let state_chain = phases.iter().map(ToString::to_string).collect();
    let dispatched = if let Some(shared) = install_lifecycle_run {
        dispatch::dispatch_plan_with_run(observer, &ritual, &shared, &metadata)
    } else {
        dispatch::dispatch_plan(observer, &ritual, lease, metadata, state_chain)
    };
    let outcome = match dispatched {
        Ok(outcome) => outcome,
        // A carried failure keeps its own draft — its error, its root family,
        // its emission policy — and gains the prefix it could not have known
        // about. An uncarried one is left alone so the outer fallback builds
        // from the same accumulator.
        Err(error) => return Err(prepend_lifecycle_rows(error, prefix)),
    };
    let parked = outcome.parked;
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
        .map(|(_, delegation)| report::delegation_member(delegation))
        .transpose()?;
    let parked = member.is_some();
    let draft = LifecycleDraft::completed(
        requested.as_str(),
        chain,
        steps,
        contributions,
        notices,
        member,
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
