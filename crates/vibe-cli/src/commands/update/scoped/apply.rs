//! The measured half of a scoped `vibe update`: prune, materialise, regenerate
//! boot under the borrowed recorder, write the lock, and service whatever the
//! run still owes.
//!
//! Everything here runs with a live [`InstallSlotLifecycle`], which is exactly
//! why it is its own cell: from the first line of it a failure owes the
//! operator the progress and rows the run really produced. The caller wraps
//! this call in one `carry_measured`, so no path out of here can return a bare
//! error that would report an empty run over a tree already pruned and
//! re-materialised.
//!
//! The prune is HERE, after the lifecycle exists, rather than in the staging
//! region. That ordering is deliberate: the run then owns its own removals —
//! `record_pruned` transfers the prefix into it exactly once, before the
//! materialise pass can park — instead of being handed a list measured by
//! somebody else. Nothing between the two reads a pruned slot: the provisional
//! world takes its replacements from this resolution, and every slot removed
//! here belongs to a package that is in it.
//!
//! Nothing here emits a document or disposes of a deferred plan preview. Both
//! are the funnel's job, once, at the command boundary — a park emits ONE
//! document, and an inner flush would print a preview beside it.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::Path;

use anyhow::{Context, Result};
use vibe_core::manifest::{Lockfile, SpecFormat};
use vibe_install::{InstallProgress, InstallSlotLifecycle};
use vibe_lifecycle::RunMetadata;
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;
use vibe_workspace::install::{
    ResolvedDep, SlotLifecycleMode, materialise_subtree_with_spec_format_and_slot_lifecycle,
    regenerate_boot_traced, run_post_install_slot_lifecycle,
};

use crate::commands::compile_trace::{RegisteredReportDraft, carry_measured};
use crate::commands::install::{
    ResumeOutcome, ResumeRequest, emit_closure_diff, lane_sizes, resume_slot_continuation,
};
use crate::output;

use super::super::draft::{UpdateDraft, UpdateIdentity};
use super::super::inputs::locked_package;
use super::measured::Measured;
use super::{Resolved, SCOPED_INTEGRITY, SourceHashes, prune_superseded};

/// Everything the measured half borrows from the scoped body.
pub(super) struct ScopedApply<'a> {
    pub(super) lifecycle: &'a InstallSlotLifecycle,
    pub(super) measured: &'a mut Measured,
    pub(super) identity: &'a UpdateIdentity,
    pub(super) project_root: &'a Path,
    pub(super) workspace: &'a Workspace,
    pub(super) metadata: &'a RunMetadata,
    pub(super) spec_format: SpecFormat,
    pub(super) trace: Option<&'a TraceRun>,
    /// The command's mutation lease, threaded into the continuation this
    /// apply may service — the ONE acquisition, never reacquired.
    pub(super) lease: &'a std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    pub(super) resolved: usize,
    pub(super) resolution: &'a [ResolvedDep],
    pub(super) source_hashes: &'a SourceHashes,
    pub(super) updated: &'a [Resolved],
    pub(super) lockfile: &'a mut Lockfile,
    pub(super) old_lock: &'a Lockfile,
    pub(super) lanes_before: &'a [(String, Option<u64>)],
    /// The command's ONE agent backend.
    pub(super) agent: &'a std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
}

pub(super) fn apply(ctx: &output::Context, inputs: ScopedApply<'_>) -> Result<UpdateDraft> {
    let ScopedApply {
        lifecycle,
        measured,
        identity,
        project_root,
        workspace,
        metadata,
        spec_format,
        trace,
        lease,
        resolved,
        resolution,
        source_hashes,
        updated,
        lockfile,
        old_lock,
        lanes_before,
        agent,
    } = inputs;

    // Remove every superseded slot, recording each real removal as it happens.
    // The transfer into the run is unconditional and comes BEFORE the `?`: a
    // prune that fails half-way has still deleted whatever it deleted, and the
    // run — which every later `progress()` is read from — must already own that
    // list when the error propagates.
    let pruned = prune_superseded(workspace, lockfile, updated, measured);
    lifecycle.record_pruned(measured.pruned().to_vec());
    pruned?;

    // Materialise the subtree (copy / hardlink / in-place move) and run each
    // freshly-placed slot's pre-install hook (PROP-020 §2.1) — no prune, no
    // boot here; boot is regenerated below from the whole tree.
    let materialised = materialise_subtree_with_spec_format_and_slot_lifecycle(
        &workspace.root,
        resolution,
        SCOPED_INTEGRITY,
        spec_format,
        Some(source_hashes),
        lifecycle,
    );
    // A hosted row parked. That is a durable handoff, not a failure — and it
    // belongs to THIS command: `vibe update` reports `update`, its own run,
    // and `resume: vibe update`. It never impersonates install.
    if let Some(delegation) = lifecycle.parked() {
        crate::commands::lifecycle::check_delegation(&delegation)?;
        return Ok(parked_draft(
            identity,
            resolved,
            measured,
            lifecycle,
            &delegation,
        ));
    }
    let mut subtree = materialised.context("re-materialising the updated subtree")?;

    // Regenerate every node's boot from the new `vibedeps/` state — under the
    // command's ONE borrowed recorder, so these compiles join the same run as
    // everything else this invocation did.
    let nodes_regenerated = regenerate_boot_traced(workspace, spec_format, trace)
        .context("regenerating boot artifacts")?;

    // The scoped update's own complete record, assembled from what each step
    // really returned: the subtree pass's slot lists, the removals measured
    // above, and the nodes boot regeneration actually rewrote. Recorded on the
    // lifecycle BEFORE the post-install callbacks, so a row that parks there
    // reports a finished materialisation instead of the partial snapshot the
    // materialise boundary left behind.
    lifecycle.record_complete(InstallProgress {
        complete: true,
        fresh: false,
        materialised: subtree.materialised.clone(),
        skipped: subtree.skipped.clone(),
        pruned: measured.pruned().to_vec(),
        nodes_regenerated,
    });

    // Replace each subtree package's lockfile entry, carrying the
    // install-scoped metadata (features / language) the version bump does
    // not change.
    for (cached, deps, _) in updated {
        let old = lockfile.find(&cached.resolved.group, &cached.resolved.name);
        let entry = locked_package(cached, deps, old);
        match lockfile
            .packages
            .iter()
            .position(|p| p.group == entry.group && p.name == entry.name)
        {
            Some(i) => lockfile.packages[i] = entry,
            None => lockfile.packages.push(entry),
        }
    }
    lockfile.meta.generated_at = crate::commands::init::current_timestamp_utc();
    lockfile.write(workspace.lockfile_path())?;

    // PROP-050 ##VERIFY-LOCK-DIFF — the closure diff after the apply is
    // durable (lock written, boot regenerated): entering/leaving members,
    // version moves, and the lane byte delta against the pre-apply
    // snapshot. Ahead of the post-install hooks and the report, mirroring
    // the `vibe install` emit order.
    emit_closure_diff(
        ctx,
        "update",
        old_lock,
        lockfile,
        lanes_before,
        &lane_sizes(&workspace.root),
    );

    if let Some(plan) = subtree.take_post_install_plan() {
        let ran = run_post_install_slot_lifecycle(plan, SlotLifecycleMode::Callback(lifecycle));
        if let Some(delegation) = lifecycle.parked() {
            crate::commands::lifecycle::check_delegation(&delegation)?;
            return Ok(parked_draft(
                identity,
                resolved,
                measured,
                lifecycle,
                &delegation,
            ));
        }
        ran.context("running post-install lifecycle")?;
    }
    // A scoped update whose slot is already materialised raises no payload
    // event, so its post-install pass never revisits a live park. The
    // persisted continuation is exactly the mechanism for that: service it
    // before anything reports a completed update.
    if let Some(done) = service_continuation(
        ctx,
        Continuation {
            lifecycle,
            measured,
            identity,
            project_root,
            workspace,
            metadata,
            resolved,
            lease,
            agent,
        },
    )? {
        return Ok(done);
    }
    lifecycle.clear_continuation().map_err(anyhow::Error::msg)?;
    let rows = lifecycle.take_reports()?;
    // SUCCESS reports the run's own progress, unjoined. The accumulator is a
    // truthfulness floor for failures; a completed command's bytes are
    // characterised, and widening them here would change what an old consumer
    // reads from a green run.
    Ok(UpdateDraft::completed(
        identity,
        resolved,
        measured.bumps().to_vec(),
        lifecycle.progress(),
        rows,
        None,
    ))
}

/// The park draft, built the same way at both park sites.
///
/// `take_reports` tolerates its own failure here on purpose: a park is a
/// SUCCESSFUL outcome the operator must be told about, and losing the whole
/// handoff because a row list could not be taken would strand the run.
fn parked_draft(
    identity: &UpdateIdentity,
    resolved: usize,
    measured: &Measured,
    lifecycle: &InstallSlotLifecycle,
    delegation: &vibe_lifecycle::Delegation,
) -> UpdateDraft {
    UpdateDraft::completed(
        identity,
        resolved,
        measured.bumps().to_vec(),
        lifecycle.progress(),
        lifecycle.take_reports().unwrap_or_default(),
        Some(delegation),
    )
}

struct Continuation<'a> {
    lifecycle: &'a InstallSlotLifecycle,
    /// Read-only here: the accumulator's in-place prefix predates the
    /// lifecycle, so a failed resume's progress has to be joined with it.
    measured: &'a Measured,
    identity: &'a UpdateIdentity,
    project_root: &'a Path,
    /// The tree this continuation runs over. Its selected node's manifest is
    /// DERIVED from it by the resume itself, never passed beside it.
    workspace: &'a Workspace,
    metadata: &'a RunMetadata,
    resolved: usize,
    /// The command's mutation lease: the resumed slot run is rebuilt on the
    /// ONE acquisition the update boundary made — a resume never reacquires.
    lease: &'a std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    /// The command's ONE agent backend, shared with the pass this continuation
    /// finishes.
    agent: &'a std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
}

/// Finish a slot run this project still owes, as a VALUE.
///
/// `Ok(None)` when nothing is owed, so the ordinary path proceeds untouched.
/// Nothing here selects an identity, re-reads a manifest or emits a document:
/// the outer session's metadata and prepared tree are handed in, and the
/// outcome travels back as a draft the one funnel renders.
/// ## Two lifecycles, one chronology
///
/// The resume builds a SECOND `InstallSlotLifecycle`. This command's own rows —
/// everything the materialise pass and the post-install callbacks ran — live in
/// the first one, and the resumed rows live in the second. Whichever draft this
/// returns is FINAL for the command, so the outer `carry_measured` can no
/// longer repair it: that helper is idempotent by design, and correctly so.
fn service_continuation(
    ctx: &output::Context,
    inputs: Continuation<'_>,
) -> Result<Option<UpdateDraft>> {
    if !inputs.lifecycle.owes_slot_work() {
        return Ok(None);
    }
    let lifecycle = inputs.lifecycle;
    let observer = crate::commands::install::CliInstallObserver::new(ctx, None);
    let request = ResumeRequest {
        project_root: inputs.project_root,
        workspace: inputs.workspace,
        metadata: inputs.metadata,
        lease: inputs.lease,
        disposition: crate::commands::install::InstallDisposition::Fresh,
        progress: lifecycle.progress(),
        packages_resolved: inputs.resolved,
    };
    // The REAL closures. Everything the ordering law says lives in
    // `owned_continuation_values`, which the reds drive with injected ones.
    owned_continuation_values(
        inputs.measured,
        inputs.identity,
        inputs.resolved,
        || resume_slot_continuation(&observer, inputs.agent, request),
        || lifecycle.take_reports().unwrap_or_default(),
    )
}

/// The ownership seam: resume, then match, then take — in that order, with both
/// dependencies injected so the order is provable.
///
/// `Ok(None)` when nothing is owed, so the ordinary path proceeds untouched.
/// Nothing here selects an identity, re-reads a manifest or emits a document:
/// the outer session's values are handed in, and the outcome travels back as a
/// draft the one funnel renders.
///
/// The take is DESTRUCTIVE and therefore lazy. It runs on the two arms that own
/// a draft and nowhere else: on `Nothing`, and on an ordinary `Err` before a
/// typed outcome exists, the rows stay with the lifecycle because the outer
/// failure closure is what will take them, and a take here would hand that
/// closure an empty list.
fn owned_continuation_values(
    measured: &Measured,
    identity: &UpdateIdentity,
    resolved: usize,
    resume: impl FnOnce() -> Result<ResumeOutcome>,
    take_current: impl FnOnce() -> Vec<vibe_install::SlotLifecycleReport>,
) -> Result<Option<UpdateDraft>> {
    let resumed = match crate::commands::install::own_resume(resume, take_current)? {
        // Nothing was owed after all: the rows stay where the outer fallback
        // can still take them.
        ResumeOutcome::Nothing => return Ok(None),
        ResumeOutcome::Completed(resumed) => *resumed,
        // A resume builds its OWN lifecycle, so its rows exist nowhere else,
        // and the in-place prefix predates BOTH lifecycles — hence the join.
        ResumeOutcome::Failed(failure) => {
            let (own_progress, rows) = match failure.measurement {
                crate::commands::install::Measurement::Slot {
                    progress, reports, ..
                }
                | crate::commands::install::Measurement::InstallBarrier {
                    progress, reports, ..
                } => (*progress, reports),
                crate::commands::install::Measurement::Lifecycle { .. } => {
                    (vibe_install::InstallProgress::default(), Vec::new())
                }
            };
            let progress = measured.joined(own_progress);
            let bumps = measured.bumps().to_vec();
            return Err(carry_measured(failure.original, || {
                RegisteredReportDraft::Update(Box::new(UpdateDraft::failed(
                    identity, resolved, bumps, progress, rows,
                )))
            }));
        }
    };
    // The handoff was validated by `own_resume`, BEFORE it took the current
    // rows — see that seam for why the order is load-bearing. There is nothing
    // fallible left on this arm.
    let run = resumed.run;
    // SUCCESS/park progress stays the characterised one — the resume's own view
    // of what was durable. Only the failure arm above widens it, because only a
    // failure had nothing truthful to say before.
    Ok(Some(UpdateDraft::completed(
        identity,
        resolved,
        measured.bumps().to_vec(),
        run.progress,
        run.slot_reports,
        run.parked.as_ref(),
    )))
}

#[cfg(test)]
#[path = "apply/tests.rs"]
mod tests;
