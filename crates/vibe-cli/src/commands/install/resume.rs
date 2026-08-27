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

use super::{InstallDisposition, InstallRun, LifecycleSlotObserver};

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

pub(crate) fn resume_slot_continuation(
    ctx: &output::Context,
    request: ResumeRequest<'_>,
) -> Result<Option<InstallRun>> {
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
    let lifecycle = vibe_install::InstallSlotLifecycle::from_projection_observed(
        project_root,
        manifest,
        &world,
        &selected,
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
            agent: std::sync::Arc::new(crate::commands::lifecycle::install_agent_backend(
                project_root,
            )?),
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
    if let Some(delegation) = lifecycle.parked() {
        crate::commands::lifecycle::check_delegation(&delegation)?;
        run.disposition = InstallDisposition::Parked;
        run.parked = Some(delegation);
        run.slot_reports = lifecycle.take_reports()?;
        return Ok(Some(run));
    }
    ran.context("finishing the parked slot run")?;
    // Nothing is owed any more: the continuation goes before anything reports
    // a completed run.
    lifecycle.clear_continuation().map_err(anyhow::Error::msg)?;
    run.slot_reports = lifecycle.take_reports()?;
    Ok(Some(run))
}
