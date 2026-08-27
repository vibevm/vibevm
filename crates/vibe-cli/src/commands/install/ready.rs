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

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_core::user_config::SlotIntegrity;
use vibe_install::PlannedInstall;
use vibe_lifecycle::process::StreamMode;
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;

use crate::cli::InstallArgs;
use crate::commands::compile_trace::{RegisteredReportDraft, carry};
use crate::exit_code::InstallError;
use crate::output;

use super::report;
use super::{
    InstallDisposition, InstallDraft, InstallResolver, InstallRun, InstallRunContext,
    LifecycleSlotObserver, WorldCallbackOutcome, emit_closure_diff, lane_sizes, resume,
    resume_slot_continuation,
};

/// Everything the ready branch borrows from the command body.
pub(super) struct ReadyApply<'a> {
    pub(super) args: &'a InstallArgs,
    pub(super) project_root: &'a Path,
    pub(super) manifest: &'a Manifest,
    pub(super) workspace: &'a Workspace,
    pub(super) resolver: &'a InstallResolver,
    pub(super) planned: PlannedInstall,
    pub(super) slot_integrity: SlotIntegrity,
    pub(super) spec_format: SpecFormat,
    pub(super) lockfile_path: &'a Path,
    pub(super) lockfile_snapshot: &'a Lockfile,
    pub(super) lanes_before: &'a [(String, Option<u64>)],
    pub(super) run_metadata: &'a vibe_lifecycle::RunMetadata,
    pub(super) lifecycle_output: Option<&'a output::Context>,
    pub(super) trace: Option<&'a TraceRun>,
}

pub(super) fn apply(
    ctx: &output::Context,
    inputs: ReadyApply<'_>,
    mut lifecycle_run: InstallRunContext,
    after: impl FnOnce(
        &Path,
        InstallDisposition,
        InstallRunContext,
        &Workspace,
    ) -> Result<WorldCallbackOutcome>,
) -> Result<InstallRun> {
    let ReadyApply {
        args,
        project_root,
        manifest,
        workspace,
        resolver,
        planned,
        slot_integrity,
        spec_format,
        lockfile_path,
        lockfile_snapshot,
        lanes_before,
        run_metadata,
        lifecycle_output,
        trace,
    } = inputs;
    // Show the plan: the packages to materialise.
    report::present_resolution(ctx, &planned.resolution);
    // Counted here, from the solved graph itself, before the plan is
    // consumed by the apply.
    let packages_resolved = planned.resolution.len();

    // Confirm (unless --assume-yes or --json or not a TTY).
    let approved = if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        true
    } else if !console::user_attended() {
        // No TTY → refuse to apply without explicit --assume-yes.
        // This matches the book's "ask a human" discipline for any
        // destructive action.
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively"
        );
    } else {
        Confirm::new()
            .with_prompt(format!(
                "Materialise {} package{} into vibedeps/ and regenerate boot artifacts?",
                planned.resolution.len(),
                if planned.resolution.len() == 1 {
                    ""
                } else {
                    "s"
                },
            ))
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    // PROP-054 ##INSTALL-IS-CONSENT: `[hooks]` is translated to
    // `slot:` contributions and runs through the lifecycle handler
    // engine. The install confirmation above is the sole trust
    // decision; there is no hook-specific prompt or allow flag.
    let observer = LifecycleSlotObserver::new(
        lifecycle_output.unwrap_or(ctx),
        lifecycle_run.metadata.clone(),
    );
    let applied = vibe_install::apply_with_spec_format_and_lifecycle_observed_traced_prepared(
        resolver,
        planned,
        slot_integrity,
        spec_format,
        lifecycle_run.metadata.clone(),
        if ctx.is_json() {
            StreamMode::Capture
        } else if ctx.suppresses_output() {
            StreamMode::Null
        } else {
            StreamMode::Inherit
        },
        // `agent` is legal at `slot:` points too, so the same
        // `vibe-llm` adapter the create phase uses is injected here;
        // an install-time agent contribution must not silently degrade
        // to the refusing default just because it ran at the barrier.
        vibe_install::SlotLifecycleSeams {
            observer: std::sync::Arc::new(observer),
            agent: std::sync::Arc::new(crate::commands::lifecycle::install_agent_backend(
                &workspace.root,
                manifest,
            )),
        },
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
            crate::commands::lifecycle::check_delegation(&delegation)?;
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
            return Err(carry(
                RegisteredReportDraft::Install(Box::new(InstallDraft::failed(
                    project_root,
                    *progress,
                    reports,
                ))),
                anyhow::Error::new(*source),
                // The policy this site has always had, exactly: the
                // failed root is a MACHINE document — emitted only in
                // JSON mode, and only by an unsuppressed context. A
                // direct `vibe install --json` narrates it; the same
                // failure under a phase verb's suppressed child does
                // not, and neither does any human/quiet run.
                ctx.is_json() && !ctx.suppresses_output(),
            ));
        }
        Err(error) => return Err(error.into()),
    };
    let post_workspace = applied.workspace;
    let applied = applied.report;
    lifecycle_run.lifecycle_run = applied.lifecycle_run.clone();
    lifecycle_run.lifecycle_reports = applied.lifecycle_reports.clone();
    let tail = ReadyTail {
        ctx,
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
    // The APPLY's own tree and the manifest THAT tree carries — step 7 rewrote
    // `[requires]` with the finalised roots, so the pre-apply pair this
    // function was handed describes a world one write out of date. Pairing the
    // post-apply tree with the pre-apply manifest would be a world nobody ever
    // observed, so its absence is an invariant failure and not a fallback.
    let resumed_manifest = super::selected_node_manifest(&post_workspace, project_root)
        .ok_or_else(|| SelectedNodeMissing {
            root: post_workspace.root.display().to_string(),
            selected: project_root.display().to_string(),
        })?;
    if let Some(resumed) = resume_slot_continuation(
        ctx,
        resume::ResumeRequest {
            project_root,
            workspace: &post_workspace,
            manifest: resumed_manifest,
            metadata: run_metadata,
            spec_format,
            disposition: InstallDisposition::Applied,
            progress: applied.progress.clone(),
            packages_resolved,
        },
    )? {
        return complete_ready_resume(
            resumed,
            project_root,
            &post_workspace,
            &applied.lifecycle_reports,
            tail,
            after,
        );
    }
    // The apply's OWN post-durability workspace, not the pre-apply one this
    // function was handed: step 7 of the apply rewrites `[requires]` with the
    // finalised roots, so a world planned from the pre-apply snapshot would
    // not see the dependency this command just installed.
    let world = after(
        project_root,
        InstallDisposition::Applied,
        lifecycle_run,
        &post_workspace,
    )?;
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

/// A post-apply tree that does not contain the node the command selected.
///
/// Typed rather than an `unwrap_or` back to the pre-apply manifest: the two
/// describe different worlds, and silently pairing one tree with the other's
/// manifest is the kind of mismatch that surfaces much later as a phantom
/// dependency.
#[derive(Debug, thiserror::Error)]
#[error(
    "internal: the post-apply workspace rooted at `{root}` does not contain the selected \
     node `{selected}`, so there is no manifest describing what this apply just wrote"
)]
struct SelectedNodeMissing {
    root: String,
    selected: String,
}

/// What every completed Ready apply finishes with, resumed or not.
struct ReadyTail<'a> {
    ctx: &'a output::Context,
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
        emit_closure_diff(
            tail.ctx,
            "install",
            tail.lockfile_snapshot,
            &new_lock,
            tail.lanes_before,
            &lane_sizes(tail.lane_root),
        );
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
/// Merge, callback, shared tail — in that order and nowhere else. A second
/// completion site could merge differently from this one and nothing would
/// notice, so there is exactly one, and the unit that freezes the merge drives
/// the same [`prefix_applied_rows`] this calls.
fn complete_ready_resume(
    mut resumed: resume::ResumedInstall,
    project_root: &Path,
    post_workspace: &Workspace,
    applied_rows: &[vibe_install::SlotLifecycleReport],
    tail: ReadyTail<'_>,
    after: impl FnOnce(
        &Path,
        InstallDisposition,
        InstallRunContext,
        &Workspace,
    ) -> Result<WorldCallbackOutcome>,
) -> Result<InstallRun> {
    // Chronology: THIS apply's own slot rows happened before the older
    // continuation this resume just serviced, which happened before the phase
    // callback below.
    prefix_applied_rows(&mut resumed, applied_rows);
    let run = resume::finish_resumed(resumed, project_root, post_workspace, after)?;
    Ok(finish_ready(tail, run))
}

/// Put this apply's own slot rows in front of the resumed continuation's, in
/// both carriers, exactly once.
///
/// BOTH, because the two are read by different consumers and a run described
/// one way to the document and another way to the callback is a run nobody can
/// reconcile: the install report joins `slot_reports`, and the post-durability
/// callback counts the context's copy in its summary.
fn prefix_applied_rows(
    resumed: &mut resume::ResumedInstall,
    applied_rows: &[vibe_install::SlotLifecycleReport],
) {
    if applied_rows.is_empty() {
        return;
    }
    for rows in [
        &mut resumed.run.slot_reports,
        &mut resumed.context.lifecycle_reports,
    ] {
        let mut prefixed = applied_rows.to_vec();
        prefixed.append(rows);
        *rows = prefixed;
    }
}

#[cfg(test)]
mod tests;
