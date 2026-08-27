//! Finishing a slot run that parked AFTER the lockfile barrier.
//!
//! A post-install park writes the lock first, so its resume arrives on the
//! FRESH fast path — the one branch that would otherwise never rebuild the
//! slot run and would report a clean completion over a live delegated row.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME");

use std::path::Path;

use anyhow::{Context, Result};
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_wire::generated::lifecycle_state::ExecutionRecordScope;
use vibe_workspace::Workspace;

use crate::output;

use super::{InstallDisposition, InstallRun, InstallRunContext, LifecycleSlotObserver};

/// Finish a slot run that parked AFTER the lockfile barrier.
///
/// The lock is fresh, so no materialisation pass will produce a post-install
/// plan. The persisted continuation names the exact ordered payload-event
/// targets the original pass selected; this rebuilds the locked world from the
/// lock plus its exact slots, selects precisely those targets through the
/// checked workspace constructor, and runs the same post-install continuation.
/// Earlier successful rows reuse, the delegated row probes its current
/// contract, and later rows run in their original order.
///
/// `Ok(None)` when there is nothing owed — no continuation, or no live
/// slot-scoped park — so the ordinary fresh path proceeds untouched.
/// Everything the continuation needs about the run that is servicing it.
/// Grouped so the seam reads as one request rather than a positional list.
pub(crate) struct ResumeRequest<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) workspace: &'a Workspace,
    pub(crate) manifest: &'a Manifest,
    pub(crate) metadata: &'a RunMetadata,
    /// The resolved-package count of the run being serviced, carried through
    /// unchanged: a continuation finishes that run, it does not resolve again.
    pub(crate) packages_resolved: usize,
    pub(crate) spec_format: SpecFormat,
    /// What the CALLER's own pass did, so a serviced continuation reports the
    /// caller's progress rather than a hard-coded "fresh".
    pub(crate) disposition: InstallDisposition,
    pub(crate) progress: vibe_install::InstallProgress,
}

/// A serviced continuation, plus what the SAME lifecycle run needs to carry on.
///
/// The context is not a courtesy: a satisfied slot resume still owes its
/// caller the post-durability phase work (an authored `phase:install`
/// contribution, a lifecycle prerequisite's later phases). That work must join
/// the run this resume just finished — its real handle, its metadata, and the
/// rows it produced — rather than begin a second run beside it.
pub(crate) struct ResumedInstall {
    pub(crate) run: InstallRun,
    pub(crate) context: InstallRunContext,
}

pub(crate) fn resume_slot_continuation(
    ctx: &output::Context,
    request: ResumeRequest<'_>,
) -> Result<Option<ResumedInstall>> {
    let ResumeRequest {
        project_root,
        workspace,
        manifest,
        metadata,
        spec_format,
        disposition,
        progress,
        packages_resolved,
    } = request;
    let _ = spec_format;
    // READ-ONLY: asking what this run owes must not rewrite the run header.
    // `begin` would replace the persisted chain with this caller's own, which
    // silently rewrote a clean-composed run's recorded chain.
    let Some(state) = vibe_lifecycle::LifecycleStateStore::peek(&workspace.root)? else {
        return Ok(None);
    };
    let Some(continuation) = state.run.slot_continuation.clone() else {
        return Ok(None);
    };
    let owes_slot_work = state
        .execution
        .values()
        .any(|record| record.scope == Some(ExecutionRecordScope::Slot));
    if !owes_slot_work || continuation.targets.is_empty() {
        return Ok(None);
    }
    drop(state);

    let lockfile = Lockfile::read(workspace.lockfile_path())
        .context("reading the lockfile to rebuild a parked slot run")?;
    let world = crate::commands::update::lifecycle::provisional_world(workspace, &lockfile, &[])?;
    let targets: Vec<(String, String, String)> = continuation
        .targets
        .iter()
        .map(|target| {
            (
                target.group.clone(),
                target.name.clone(),
                target.version.clone(),
            )
        })
        .collect();
    let selected: Vec<vibe_workspace::install::ResolvedDep> = targets
        .iter()
        .filter_map(|(group, name, version)| {
            world.iter().find(|dep| {
                dep.group.as_str() == group
                    && dep.name == *name
                    && dep.version.to_string() == *version
            })
        })
        .cloned()
        .collect();
    let observer = LifecycleSlotObserver::new(ctx, metadata.clone());
    // The PREPARED constructor: this continuation already carries the exact
    // tree its caller owns — for an applied resume, the post-apply one — so
    // the wrapper's discovery would replace a known value with a fresh read of
    // a tree the same command just rewrote.
    let lifecycle = vibe_install::InstallSlotLifecycle::from_projection_observed_prepared(
        project_root,
        manifest,
        &world,
        &selected,
        workspace,
        metadata.clone(),
        if ctx.is_json() {
            StreamMode::Capture
        } else if ctx.suppresses_output() {
            StreamMode::Null
        } else {
            StreamMode::Inherit
        },
        vibe_install::SlotLifecycleSeams {
            observer: std::sync::Arc::new(observer),
            // Built from the values this continuation already carries — the
            // prepared workspace root and the selected manifest — so the
            // backend serves the same world the run does.
            agent: std::sync::Arc::new(crate::commands::lifecycle::install_agent_backend(
                &workspace.root,
                manifest,
            )),
        },
    )?;
    // This slot plan's exact ordered target set is the persisted continuation
    // itself. Announcing it is what lets a LATER declared row park once the
    // one being resumed is satisfied — between those two writes the run owes
    // nothing, so the durable continuation is correctly dropped.
    lifecycle
        .stage_resumed_targets(&selected)
        .map_err(anyhow::Error::msg)?;
    let Some(plan) = vibe_workspace::install::PostInstallPlan::resume_for_targets(
        &workspace.root,
        &world,
        &targets,
    )?
    else {
        return Ok(None);
    };
    let ran = vibe_workspace::install::run_post_install_slot_lifecycle(
        plan,
        vibe_workspace::install::SlotLifecycleMode::Callback(&lifecycle),
    );
    // The continuation is serviced on BOTH the fresh fast path and after a
    // completed apply, so the progress this returns is the caller's, not a
    // hard-coded "fresh": saying an applied run moved no slot would be the
    // same class of lie the honest progress model exists to prevent.
    let mut run = InstallRun::new(project_root.to_path_buf(), disposition);
    run.packages_resolved = packages_resolved;
    run.progress = progress;
    // The ONE handle to the run this resume is servicing, taken before the
    // reports so both halves below describe the same run.
    let mut context = InstallRunContext {
        metadata: metadata.clone(),
        lifecycle_run: Some(lifecycle.run_handle()),
        lifecycle_reports: Vec::new(),
    };
    if let Some(delegation) = lifecycle.parked() {
        crate::commands::lifecycle::check_delegation(&delegation)?;
        run.disposition = InstallDisposition::Parked;
        run.parked = Some(delegation);
        run.slot_reports = lifecycle.take_reports()?;
        // Still parked: the caller returns it untouched, so the context
        // carries no rows it would fold anywhere.
        return Ok(Some(ResumedInstall { run, context }));
    }
    ran.context("finishing the parked slot run")?;
    // Nothing is owed any more: the continuation goes before anything reports
    // a completed run.
    lifecycle.clear_continuation().map_err(anyhow::Error::msg)?;
    // ONE take. The install report joins these rows to its document, and the
    // post-durability callback counts them in its summary the same way the
    // ordinary applied path does — hence the clone, not a second take.
    let reports = lifecycle.take_reports()?;
    context.lifecycle_reports = reports.clone();
    run.slot_reports = reports;
    Ok(Some(ResumedInstall { run, context }))
}

/// Finish a serviced continuation: run the post-durability callback and fold
/// what it produced into the resumed run.
///
/// Both resume sites used to `return Ok(resumed)` here, which skipped the
/// callback entirely — so an authored `phase:install` contribution never ran
/// once a park had been satisfied, and a lifecycle prerequisite never saw the
/// resumed rows. The fold below is deliberately the SAME one the ordinary
/// completed branch performs, including the rule that a callback park is what
/// turns a completed disposition into `Parked`.
pub(crate) fn finish_resumed(
    resumed: ResumedInstall,
    project_root: &Path,
    workspace: &Workspace,
    after: impl FnOnce(
        &Path,
        InstallDisposition,
        InstallRunContext,
        &Workspace,
    ) -> Result<super::WorldCallbackOutcome>,
) -> Result<InstallRun> {
    let ResumedInstall { mut run, context } = resumed;
    // A run that is STILL parked owes the caller nothing further: its chain
    // stopped at the delegated row, and post-install phase work belongs after
    // that row, not around it.
    if run.parked.is_some() {
        return Ok(run);
    }
    let world = after(project_root, run.disposition, context, workspace)?;
    run.contributions = world.contributions;
    run.notices = world.notices;
    run.world_summary = world.summary;
    if world.parked.is_some() {
        run.disposition = InstallDisposition::Parked;
        run.parked = world.parked;
    }
    Ok(run)
}
