//! The READY apply: a solved plan, confirmed, materialised under one borrowed
//! recorder, and turned into an outcome.
//!
//! Split from the command body so that body reads as the DECISION it makes —
//! empty world, fresh lock, or ready plan — rather than as the largest of the
//! three branches. Everything here belongs to that one branch: the interactive
//! confirmation, the traced apply, the typed slot-failure carrier, and the
//! post-apply closure diff.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use anyhow::Result;
use vibe_core::manifest::{Lockfile, SpecFormat};
use vibe_core::user_config::SlotIntegrity;
use vibe_install::PlannedInstall;
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;

use crate::failure::{MeasuredFailure, Measurement, carry};
use crate::ports::{
    AfterDurableWorld, ConfirmGate, InstallNarration, InstallObserver, PackageSource,
};

use super::{
    InstallDisposition, InstallRun, InstallRunContext, ResumeOutcome, resume,
    resume_slot_continuation,
};

/// Everything the ready branch borrows from the command body.
pub(super) struct ReadyApply<'a> {
    pub(super) project_root: &'a Path,
    pub(super) workspace: &'a Workspace,
    pub(super) resolver: &'a dyn PackageSource,
    pub(super) planned: PlannedInstall,
    pub(super) slot_integrity: SlotIntegrity,
    pub(super) spec_format: SpecFormat,
    pub(super) lockfile_path: &'a Path,
    pub(super) lockfile_snapshot: &'a Lockfile,
    pub(super) lanes_before: &'a [(String, Option<u64>)],
    pub(super) run_metadata: &'a vibe_lifecycle::RunMetadata,
    pub(super) confirm_gate: &'a dyn ConfirmGate,
    pub(super) observer: &'a dyn InstallObserver,
    pub(super) agent: &'a std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
    pub(super) trace: Option<&'a TraceRun>,
}

pub(super) fn apply(
    inputs: ReadyApply<'_>,
    mut lifecycle_run: InstallRunContext,
    after: &mut dyn AfterDurableWorld,
) -> Result<InstallRun> {
    let ReadyApply {
        project_root,
        workspace,
        resolver,
        planned,
        slot_integrity,
        spec_format,
        lockfile_path,
        lockfile_snapshot,
        lanes_before,
        run_metadata,
        confirm_gate,
        observer,
        agent,
        trace,
    } = inputs;
    // Show the plan: the packages to materialise.
    observer.narrate(InstallNarration::Resolution(&planned.resolution));
    // Counted here, from the solved graph itself, before the plan is
    // consumed by the apply.
    let packages_resolved = planned.resolution.len();

    confirm_gate.confirm_install(planned.resolution.len())?;

    // PROP-054 ##INSTALL-IS-CONSENT: `[hooks]` is translated to
    // `slot:` contributions and runs through the lifecycle handler
    // engine. The install confirmation above is the sole trust
    // decision; there is no hook-specific prompt or allow flag.
    let slot_observer = observer.slot_observer(&lifecycle_run.metadata);
    let applied = vibe_install::apply_with_spec_format_and_lifecycle_observed_traced_prepared(
        resolver,
        planned,
        slot_integrity,
        spec_format,
        lifecycle_run.metadata.clone(),
        // The CHILD stream formula, unchanged: json captures, a SUPPRESSED
        // context nulls, everything else inherits. It is deliberately not the
        // phase observer's quiet-based formula.
        observer.stream_mode(),
        // `agent` is legal at `slot:` points too, so the SAME backend the
        // phase dispatch uses is injected here; an install-time agent
        // contribution must not silently degrade to the refusing default just
        // because it ran at the barrier.
        vibe_install::SlotLifecycleSeams {
            observer: slot_observer,
            agent: agent.clone(),
        },
        // The command's ONE acquisition, shared by Arc — the apply builds
        // its slot run on the caller's lease and never reacquires.
        lifecycle_run.lease.clone(),
        trace,
    );
    // A parked slot row is a durable handoff, not an install failure:
    // the chain stopped at that row's point, whatever preceded it is
    // already durable and measured in `progress`, and nothing was paid
    // for. It travels OUT as a value — this layer renders nothing, so
    // the outermost command owns the single document.
    let applied = match applied {
        Ok(applied) => applied,
        Err(vibe_install::Error::Delegated {
            delegation,
            reports,
            progress,
        }) => {
            crate::values::check_delegation(&delegation)?;
            let mut parked =
                InstallRun::new(project_root.to_path_buf(), InstallDisposition::Parked);
            parked.packages_resolved = packages_resolved;
            parked.progress = *progress;
            parked.slot_reports = reports;
            parked.parked = Some(*delegation);
            return Ok(parked);
        }
        // A slot row FAILED. The rows and progress are measured HERE
        // and nowhere else, so the report family and its historical
        // emission policy are frozen here too, in a typed carrier the
        // command boundary unwraps. Nothing downstream re-derives
        // either from the error.
        Err(vibe_install::Error::SlotFailed {
            source,
            reports,
            progress,
        }) => {
            return Err(carry(MeasuredFailure {
                original: anyhow::Error::new(*source),
                evidence: Measurement::InstallBarrier {
                    progress,
                    reports,
                    packages_resolved,
                },
                // The policy this site has always had, exactly: the
                // failed root is a MACHINE document — emitted only in
                // JSON mode, and only by an unsuppressed context. A
                // direct `vibe install --json` narrates it; the same
                // failure under a phase verb's suppressed child does
                // not, and neither does any human/quiet run. That is the
                // CHILD observer's answer, never the phase observer's.
                emit_machine_failure: observer.emit_machine_failure(),
            }));
        }
        Err(error) => return Err(error.into()),
    };
    let post_workspace = applied.workspace;
    let applied = applied.report;
    lifecycle_run.lifecycle_run = applied.lifecycle_run.clone();
    lifecycle_run.lifecycle_reports = applied.lifecycle_reports.clone();
    let tail = ReadyTail {
        observer,
        lockfile_path,
        lockfile_snapshot,
        lanes_before,
        lane_root: &workspace.root,
        applied: &applied,
        packages_resolved,
    };
    // An apply can finish without visiting a live slot-scoped park:
    // an unchanged slot produces no payload event, so the post-install
    // plan is empty and the delegated row is never revisited. The
    // persisted continuation is exactly the mechanism for that case —
    // consume it before anything reports a completed run.
    //
    // ONE join for all three arms. The two lifecycles are different objects and
    // only this site holds both, so the chronology is fixed here — before the
    // match — rather than separately in each branch.
    match join_applied_rows(
        resume_slot_continuation(
            observer,
            agent,
            resume::ResumeRequest {
                project_root,
                // The APPLY's own tree — step 7 rewrote `[requires]` with the
                // finalised roots, so the pre-apply pair this function was
                // handed describes a world one write out of date. The resume
                // derives the selected node's manifest from THIS tree; there is
                // no second manifest to mispair it with.
                workspace: &post_workspace,
                metadata: run_metadata,
                lease: &lifecycle_run.lease,
                disposition: InstallDisposition::Applied,
                progress: applied.progress.clone(),
                packages_resolved,
            },
        )?,
        &applied.lifecycle_reports,
    ) {
        ResumeOutcome::Completed(resumed) => {
            return complete_ready_resume(*resumed, project_root, &post_workspace, tail, after);
        }
        // TRANSPORTED neutrally — the family is the outer command's to choose.
        ResumeOutcome::Failed(failure) => return Err(carry(failure)),
        ResumeOutcome::Nothing => {}
    }
    // The apply's OWN post-durability workspace, not the pre-apply one this
    // function was handed: step 7 of the apply rewrites `[requires]` with the
    // finalised roots, so a world planned from the pre-apply snapshot would
    // not see the dependency this command just installed.
    let world = after.after(project_root, lifecycle_run, &post_workspace)?;
    let mut run = InstallRun::new(
        project_root.to_path_buf(),
        if world.parked.is_some() {
            InstallDisposition::Parked
        } else {
            InstallDisposition::Applied
        },
    );
    run.slot_reports = applied.lifecycle_reports.clone();
    // ONLY the phase-ritual rows: the slot rows live on
    // `slot_reports`, and the document joins the two exactly once.
    // Carrying them in both places double-counted every slot row.
    run.contributions = world.contributions;
    run.notices = world.notices;
    run.parked = world.parked;
    run.world_summary = world.summary;
    Ok(finish_ready(tail, run))
}

/// What every completed Ready apply finishes with, resumed or not.
struct ReadyTail<'a> {
    observer: &'a dyn InstallObserver,
    lockfile_path: &'a Path,
    lockfile_snapshot: &'a Lockfile,
    lanes_before: &'a [(String, Option<u64>)],
    lane_root: &'a Path,
    applied: &'a vibe_install::ApplyReport,
    packages_resolved: usize,
}

/// The common Ready tail.
///
/// Both completion paths run this and neither returns before it. Factored
/// because they used to diverge: a resumed Ready returned its run directly and
/// so emitted no closure diff, attached no hook reports, and reported the
/// resume's counts instead of the apply's — three silent differences between
/// two runs that did the same durable work.
fn finish_ready(tail: ReadyTail<'_>, mut run: InstallRun) -> InstallRun {
    // PROP-050 ##VERIFY-LOCK-DIFF — after a successful apply, print
    // the closure diff (the pre-apply lock snapshot vs the freshly
    // written one, lane bytes before/after): a mid-graph re-export
    // widening is a reviewed event, not a silent seep. Emitted ahead
    // of the final report so the `--json` stream keeps the report as
    // its last document. A read failure of the just-written lock
    // skips the diff rather than failing the completed install.
    if let Ok(new_lock) = Lockfile::read(tail.lockfile_path) {
        let lanes_after = tail.observer.lane_sizes(tail.lane_root);
        tail.observer.narrate(InstallNarration::ClosureDiff {
            old: tail.lockfile_snapshot,
            new: &new_lock,
            lanes_before: tail.lanes_before,
            lanes_after: &lanes_after,
        });
    }
    run.packages_resolved = tail.packages_resolved;
    // The APPLY's progress and hooks, on both paths: what this invocation made
    // durable does not change because it also finished someone else's park.
    run.progress = tail.applied.progress.clone();
    run.hooks = tail
        .applied
        .outcome
        .hook_reports
        .iter()
        .chain(&tail.applied.post_install_reports)
        .cloned()
        .collect();
    run
}

/// The WHOLE Ready-resume completion, in one place.
///
/// Callback, shared tail — in that order and nowhere else. The row merge has
/// already happened, once, in [`join_applied_rows`].
fn complete_ready_resume(
    resumed: resume::ResumedInstall,
    project_root: &Path,
    post_workspace: &Workspace,
    tail: ReadyTail<'_>,
    after: &mut dyn AfterDurableWorld,
) -> Result<InstallRun> {
    let run = resume::finish_resumed(resumed, project_root, post_workspace, after)?;
    Ok(finish_ready(tail, run))
}

/// Put THIS apply's own slot rows in front of whatever the resume produced —
/// once, on whichever arm carries rows, and never on `Nothing`.
///
/// One seam for all three arms, because they are the same fact: the apply ran
/// before the older continuation it serviced. Splitting it across the match —
/// a helper for the completed arm and an inline `append` for the failed one —
/// is how the two drifted, and how a test of the helper could stay green while
/// the failed arm reported a run with no apply in it.
///
/// The COMPLETED arm merges BOTH carriers. They are read by different
/// consumers, and a run described one way to the document and another way to
/// the callback is a run nobody can reconcile: the install report joins
/// `slot_reports`, and the post-durability callback counts the context's copy
/// in its summary.
fn join_applied_rows(
    outcome: ResumeOutcome,
    applied_rows: &[vibe_install::SlotLifecycleReport],
) -> ResumeOutcome {
    match outcome {
        // No rows, nothing to put them in front of, and nothing fabricated.
        ResumeOutcome::Nothing => ResumeOutcome::Nothing,
        ResumeOutcome::Completed(mut resumed) => {
            if !applied_rows.is_empty() {
                for rows in [
                    &mut resumed.run.slot_reports,
                    &mut resumed.context.lifecycle_reports,
                ] {
                    *rows = super::prefixed(applied_rows.to_vec(), rows.split_off(0));
                }
            }
            ResumeOutcome::Completed(resumed)
        }
        ResumeOutcome::Failed(mut failure) => {
            if let Measurement::Slot { reports, .. } | Measurement::InstallBarrier { reports, .. } =
                &mut failure.evidence
            {
                *reports = super::prefixed(applied_rows.to_vec(), std::mem::take(reports));
            }
            ResumeOutcome::Failed(failure)
        }
    }
}

#[cfg(test)]
mod tests;
