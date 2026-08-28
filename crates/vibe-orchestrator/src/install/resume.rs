//! Finishing a slot run that parked AFTER the lockfile barrier.
//!
//! A post-install park writes the lock first, so its resume arrives on the
//! FRESH fast path — the one branch that would otherwise never rebuild the
//! slot run and would report a clean completion over a live delegated row.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME");

use specmark::spec;

use std::path::Path;

use anyhow::{Context, Result};
use vibe_core::manifest::Lockfile;
use vibe_lifecycle::{AgentBackend, LifecycleLease, RunMetadata};
use vibe_wire::generated::lifecycle_state::ExecutionRecordScope;
use vibe_workspace::Workspace;

use crate::failure::{MeasuredFailure, Measurement};
use crate::ports::InstallObserver;

use super::{InstallDisposition, InstallRun, InstallRunContext};

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
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
pub struct ResumeRequest<'a> {
    /// The one canonical selection of this command's project root.
    pub project_root: &'a Path,
    /// The tree this continuation runs over. The selected node's manifest is
    /// DERIVED from it below rather than passed beside it: a manifest handed in
    /// separately could be the pre-apply copy while the tree is the post-apply
    /// one, and the slot projection would then be built from two different
    /// moments of the same node.
    pub workspace: &'a Workspace,
    /// The invocation's durable identity.
    pub metadata: &'a RunMetadata,
    /// The command's mutation lease: the resumed slot run is rebuilt on the
    /// caller's ONE acquisition — a resume never reacquires.
    pub lease: &'a std::sync::Arc<LifecycleLease>,
    /// The resolved-package count of the run being serviced, carried through
    /// unchanged: a continuation finishes that run, it does not resolve again.
    pub packages_resolved: usize,
    /// What the CALLER's own pass did, so a serviced continuation reports the
    /// caller's progress rather than a hard-coded "fresh".
    pub disposition: InstallDisposition,
    /// What the caller's own pass made durable.
    pub progress: vibe_install::InstallProgress,
}

/// A serviced continuation, plus what the SAME lifecycle run needs to carry on.
///
/// The context is not a courtesy: a satisfied slot resume still owes its
/// caller the post-durability phase work (an authored `phase:install`
/// contribution, a lifecycle prerequisite's later phases). That work must join
/// the run this resume just finished — its real handle, its metadata, and the
/// rows it produced — rather than begin a second run beside it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
pub struct ResumedInstall {
    /// The install run the continuation finished.
    pub run: InstallRun,
    /// What the SAME lifecycle run needs to carry on.
    pub context: InstallRunContext,
}

/// What servicing a continuation produced — as a VALUE, including the failure.
///
/// A resume builds its OWN [`vibe_install::InstallSlotLifecycle`], and that is
/// the whole reason this is a sum rather than a `Result<Option<_>>`. A `?` out
/// of the row-owning region drops that lifecycle, and with it every row the
/// resumed run had already produced: an earlier contribution that succeeded,
/// the delegated row it satisfied, the point it reached. No caller can recover
/// them afterwards — the outer command's own lifecycle is a DIFFERENT run and
/// never saw them — so a failed resume used to report an empty run over work
/// that really happened.
///
/// The error travels as the ORIGINAL object. Nothing here formats or re-wraps
/// it: the exit code is read by downcasting through its chain.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
pub enum ResumeOutcome {
    /// Nothing was owed, so the ordinary path proceeds untouched.
    Nothing,
    /// A continuation was owed and was serviced.
    Completed(Box<ResumedInstall>),
    /// It failed after its lifecycle existed; the measurement travels neutral.
    Failed(MeasuredFailure),
}

pub fn resume_slot_continuation(
    observer: &dyn InstallObserver,
    agent: &std::sync::Arc<dyn AgentBackend>,
    request: ResumeRequest<'_>,
) -> Result<ResumeOutcome> {
    let ResumeRequest {
        project_root,
        workspace,
        metadata,
        lease,
        disposition,
        progress,
        packages_resolved,
    } = request;
    // ---- the agreement gate, before ANY state or handler work ------------
    //
    // A resume rebuilds a slot lifecycle over the caller's tree and then runs
    // handlers against it. The lease is the state root and the metadata names
    // the selected node, so a tree that agrees with neither would service
    // another command's parked run. Both refusals are the lease's own typed
    // gates, checked before the state store is even peeked.
    lease.ensure_root(&workspace.root, "at slot resume")?;
    let observed_selected = workspace
        .node_rel_of(project_root)
        .map(|rel| rel.as_str().to_string());
    lease.ensure_selected(
        &metadata.selected,
        observed_selected.as_deref(),
        "at slot resume",
    )?;
    // The selected node's manifest, from the tree ALREADY in hand — never a
    // value passed beside it. A node the tree does not carry is a refusal, not
    // a fallback to the workspace root's manifest.
    let manifest =
        crate::install::selected_node_manifest(workspace, project_root).ok_or_else(|| {
            SelectedNodeMissing {
                root: workspace.root.display().to_string(),
                selected: project_root.display().to_string(),
            }
        })?;
    // READ-ONLY: asking what this run owes must not rewrite the run header.
    // `begin` would replace the persisted chain with this caller's own, which
    // silently rewrote a clean-composed run's recorded chain. The read goes
    // through the LEASE's pinned capability — this seam runs inside a leased
    // command, and a second `Project::open` of the same root would be a
    // second capability over a tree the lease already owns.
    let Some(state) = vibe_lifecycle::LifecycleStateStore::peek_with_lease(lease)? else {
        return Ok(ResumeOutcome::Nothing);
    };
    // A slot continuation is a capability owned by the EXACT lifecycle run
    // that parked it. Identity selection has already decided whether this
    // invocation adopted that run or displaced it: adoption preserves the id,
    // displacement mints a new one. Letting the fresh id service the old
    // continuation would resurrect cancelled work under a different owner and
    // can re-park the new command on a task its state just superseded.
    if !owns_continuation(state.run.run_id.as_deref(), &metadata.run_id) {
        return Ok(ResumeOutcome::Nothing);
    }
    let Some(continuation) = state.run.slot_continuation.clone() else {
        return Ok(ResumeOutcome::Nothing);
    };
    let owes_slot_work = state
        .execution
        .values()
        .any(|record| record.scope == Some(ExecutionRecordScope::Slot));
    if !owes_slot_work || continuation.targets.is_empty() {
        return Ok(ResumeOutcome::Nothing);
    }
    drop(state);

    let lockfile = Lockfile::read(workspace.lockfile_path())
        .context("reading the lockfile to rebuild a parked slot run")?;
    let world = crate::install::provisional_world(workspace, &lockfile, &[])?;
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
    let slot_observer = observer.slot_observer(metadata);
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
        // The CHILD stream formula, unchanged.
        observer.stream_mode(),
        vibe_install::SlotLifecycleSeams {
            observer: slot_observer,
            // The command's ONE injected backend, shared: a resume finishes the
            // run somebody else began and must serve its agent rows from the
            // same seam that run was started with.
            agent: agent.clone(),
        },
        lease.clone(),
    )?;
    // ---- everything past this line owns rows ------------------------------
    //
    // The lifecycle now exists, so a bare `?` here would drop it and every row
    // it has produced. The region is a function returning a `Result`, and its
    // failure is captured with ONE `take_reports()` and the caller's durable
    // progress.
    let outcome = serviced(Serviced {
        lifecycle: &lifecycle,
        workspace,
        world: &world,
        targets: &targets,
        selected: &selected,
        metadata,
        lease,
        project_root,
        disposition,
        packages_resolved,
        progress: progress.clone(),
    });
    Ok(capture(
        outcome,
        progress,
        packages_resolved,
        // LAZY: the take is destructive, and it belongs to the failure arm
        // alone. Calling it eagerly would empty the lifecycle on a successful
        // resume, whose own rows the completed arm still has to read.
        || lifecycle.take_reports().unwrap_or_default(),
    ))
}

/// Whether the current lifecycle identity owns the persisted continuation.
///
/// Kept pure so the adoption/displacement boundary is pinned without building
/// a live state store: missing and foreign identities both own nothing.
fn owns_continuation(state_run_id: Option<&str>, current_run_id: &str) -> bool {
    state_run_id == Some(current_run_id)
}

/// Turn the row-owning region's `Result` into the typed outcome.
///
/// The whole capture is here, in one function with its destructive dependency
/// INJECTED, so that the three things it must get right can be driven directly:
/// the rows are taken exactly once and only on the failure arm, every measured
/// field crosses unchanged, and the error is moved rather than formatted.
///
/// `take` is `FnOnce` and lazy on purpose. `take_reports` empties the
/// lifecycle, so an eager call would strip a SUCCESSFUL resume of the rows its
/// completed arm is about to report — a failure mode no assertion on the
/// failure arm could ever see.
fn capture(
    outcome: Result<ResumeOutcome>,
    progress: vibe_install::InstallProgress,
    packages_resolved: usize,
    take: impl FnOnce() -> Vec<vibe_install::SlotLifecycleReport>,
) -> ResumeOutcome {
    match outcome {
        Ok(outcome) => outcome,
        Err(original) => ResumeOutcome::Failed(MeasuredFailure {
            // MOVED, never formatted: the exit code is read by downcasting
            // through this object's chain.
            original,
            measurement: Measurement::Slot {
                progress: Box::new(progress),
                reports: take(),
                packages_resolved,
            },
            // Historically silent: a resume failure never emitted a machine
            // root of its own with tracing off.
            emit_machine_failure: false,
        }),
    }
}

/// Own the ordering of a resume that has a CURRENT pass in front of it.
///
/// Two lifecycles exist on the scoped-update and forced-reinstall paths: the
/// command's own, and the one the resume builds. This is the single place that
/// joins them, and the ordering it fixes is the whole contract:
///
/// ```text
/// resume (lazy)  ->  match  ->  take the current rows (lazy)  ->  prefix
/// ```
///
/// Both dependencies are injected and lazy so the ordering is provable. The
/// take must not run before the resume result exists (an early take would empty
/// the current pass while the resume then reports "nothing was owed"), and it
/// must not run on `Nothing` or on an ordinary `Err` — those paths still owe
/// their rows to the caller's own failure fallback, which takes them itself.
pub fn own_resume(
    resume: impl FnOnce() -> Result<ResumeOutcome>,
    take_current: impl FnOnce() -> Vec<vibe_install::SlotLifecycleReport>,
) -> Result<ResumeOutcome> {
    Ok(match resume()? {
        ResumeOutcome::Nothing => ResumeOutcome::Nothing,
        ResumeOutcome::Completed(mut resumed) => {
            // VALIDATE FIRST. A malformed handoff is a failed command, and the
            // check is the last fallible thing on this arm — so it must happen
            // while both halves are still recoverable. Taking first and
            // validating afterwards loses BOTH: the joined outcome is dropped
            // on the error path, and the outer fallback then reads a lifecycle
            // this seam has already emptied, reporting a run that did nothing
            // over one that did several things.
            //
            // Root-neutral: the exact validation error is returned, and the
            // caller's own fallback still chooses the family.
            if let Some(delegation) = resumed.run.parked.as_ref() {
                crate::values::check_delegation(delegation)?;
            }
            resumed.run.slot_reports = prefixed(take_current(), resumed.run.slot_reports);
            ResumeOutcome::Completed(resumed)
        }
        ResumeOutcome::Failed(mut failure) => {
            if let Measurement::Slot { reports, .. } | Measurement::InstallBarrier { reports, .. } =
                &mut failure.measurement
            {
                *reports = prefixed(take_current(), std::mem::take(reports));
            }
            ResumeOutcome::Failed(failure)
        }
    })
}

/// `current` rows, then `resumed` ones. Pure, so the order is provable alone.
pub fn prefixed(
    current: Vec<vibe_install::SlotLifecycleReport>,
    resumed: Vec<vibe_install::SlotLifecycleReport>,
) -> Vec<vibe_install::SlotLifecycleReport> {
    let mut rows = current;
    rows.extend(resumed);
    rows
}

/// The borrowed inputs of the row-owning region.
struct Serviced<'a> {
    lifecycle: &'a vibe_install::InstallSlotLifecycle,
    workspace: &'a Workspace,
    world: &'a [vibe_workspace::install::ResolvedDep],
    targets: &'a [(String, String, String)],
    selected: &'a [vibe_workspace::install::ResolvedDep],
    metadata: &'a RunMetadata,
    lease: &'a std::sync::Arc<LifecycleLease>,
    project_root: &'a Path,
    disposition: InstallDisposition,
    packages_resolved: usize,
    progress: vibe_install::InstallProgress,
}

fn serviced(inputs: Serviced<'_>) -> Result<ResumeOutcome> {
    let Serviced {
        lifecycle,
        workspace,
        world,
        targets,
        selected,
        metadata,
        lease,
        project_root,
        disposition,
        packages_resolved,
        progress,
    } = inputs;
    // This slot plan's exact ordered target set is the persisted continuation
    // itself. Announcing it is what lets a LATER declared row park once the
    // one being resumed is satisfied — between those two writes the run owes
    // nothing, so the durable continuation is correctly dropped.
    lifecycle
        .stage_resumed_targets(selected)
        .map_err(anyhow::Error::msg)?;
    let Some(plan) = vibe_workspace::install::PostInstallPlan::resume_for_targets(
        &workspace.root,
        world,
        targets,
    )?
    else {
        return Ok(ResumeOutcome::Nothing);
    };
    let ran = vibe_workspace::install::run_post_install_slot_lifecycle(
        plan,
        vibe_workspace::install::SlotLifecycleMode::Callback(lifecycle),
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
        lease: lease.clone(),
        lifecycle_run: Some(lifecycle.run_handle()),
        lifecycle_reports: Vec::new(),
    };
    if let Some(delegation) = lifecycle.parked() {
        crate::values::check_delegation(&delegation)?;
        run.disposition = InstallDisposition::Parked;
        run.parked = Some(delegation);
        run.slot_reports = lifecycle.take_reports()?;
        // Still parked: the caller returns it untouched, so the context
        // carries no rows it would fold anywhere.
        return Ok(ResumeOutcome::Completed(Box::new(ResumedInstall {
            run,
            context,
        })));
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
    Ok(ResumeOutcome::Completed(Box::new(ResumedInstall {
        run,
        context,
    })))
}

#[cfg(test)]
#[path = "resume/tests.rs"]
mod tests;

/// Finish a serviced continuation: run the post-durability callback and fold
/// what it produced into the resumed run.
///
/// Both resume sites used to `return Ok(resumed)` here, which skipped the
/// callback entirely — so an authored `phase:install` contribution never ran
/// once a park had been satisfied, and a lifecycle prerequisite never saw the
/// resumed rows. The fold below is deliberately the SAME one the ordinary
/// completed branch performs, including the rule that a callback park is what
/// turns a completed disposition into `Parked`.
pub fn finish_resumed(
    resumed: ResumedInstall,
    project_root: &Path,
    workspace: &Workspace,
    after: &mut dyn crate::ports::AfterDurableWorld,
) -> Result<InstallRun> {
    let ResumedInstall { mut run, context } = resumed;
    // A run that is STILL parked owes the caller nothing further: its chain
    // stopped at the delegated row, and post-install phase work belongs after
    // that row, not around it.
    if run.parked.is_some() {
        return Ok(run);
    }
    let world = after.after(project_root, context, workspace)?;
    run.contributions = world.contributions;
    run.notices = world.notices;
    run.world_summary = world.summary;
    if world.parked.is_some() {
        run.disposition = InstallDisposition::Parked;
        run.parked = world.parked;
    }
    Ok(run)
}

/// A tree that does not contain the node the command selected.
///
/// Typed rather than an `unwrap_or` back to the workspace root's manifest: the
/// two describe different nodes, and silently pairing one tree with the other's
/// manifest is the kind of mismatch that surfaces much later as a phantom
/// dependency. It is also why the manifest is DERIVED here instead of passed
/// beside the tree — a caller could only supply the pre-apply copy.
///
/// Hand-implemented rather than derived: this crate's normal dependency set is
/// an asserted fence, and one internal invariant error is not worth widening it.
#[derive(Debug)]
struct SelectedNodeMissing {
    root: String,
    selected: String,
}

impl std::fmt::Display for SelectedNodeMissing {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "internal: the workspace rooted at `{}` does not contain the selected node `{}`, \
             so there is no manifest describing the world this run acts on",
            self.root, self.selected,
        )
    }
}

impl std::error::Error for SelectedNodeMissing {}
