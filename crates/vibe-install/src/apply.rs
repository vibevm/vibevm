//! The apply phase — runs only after the caller confirmed the plan:
//! merge the requested roots into `vibe.toml`, materialise the
//! resolution into `vibedeps/`, and rebuild `vibe.lock` wholesale.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use vibe_core::PackageRef;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::user_config::SlotIntegrity;
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{LifecycleLease, LifecycleRunHandle, RunMetadata};
use vibe_resolver::{DepProviderError, ResolvedNode};
use vibe_workspace::Workspace;
use vibe_workspace::hooks::{HookOutput, HookPolicy, HookReport};
use vibe_workspace::install::{
    InstallOutcome, ResolvedDep, SlotLifecycleMode,
    apply_resolution_with_spec_format_and_slot_lifecycle_traced_placements,
    run_post_install_slot_lifecycle,
};
use vibe_workspace::vibedeps;

use crate::InstallSource;
use crate::error::{Error, Result};
use crate::fetched::Fetched;
use crate::lifecycle::{
    InstallProgress, InstallSlotLifecycle, NoSlotLifecycleObserver, SlotLifecycleObserver,
    SlotLifecycleReport,
};
use crate::plan::PlannedInstall;
use crate::record::{
    exact_pinned_pkgref, finalize_pkgref_for_manifest, merge_manifest_requires,
    record_app_requirements, replace_root_dependencies,
};
use crate::slot_verify::RegistrySlotVerifier;

mod deferred;
mod file_guard;
mod lock;
mod lock_publication;
pub use lock_publication::LockPublication;
mod project_records;
pub use project_records::{ProjectRecordCapture, ProjectRecordPublication};
mod metadata;
mod native;
#[cfg(all(test, unix))]
mod native_materialisation_tests;
use deferred::materialise_deferred_in_place;
use file_guard::MetadataFiles;
mod publication;
use metadata::apply_with_spec_format_and_slot_lifecycle;
use native::NativeApplyPreparation;
pub use publication::{publish_admitted_manifest, publish_empty_legacy_lock};

/// What [`apply`] did — the caller renders it.
#[derive(Debug)]
pub struct ApplyReport {
    /// Materialised / skipped / pruned slots, the regenerated nodes, and
    /// the `pre-install` hook reports — straight from the workspace
    /// orchestrator.
    pub outcome: InstallOutcome,
    /// `post-install` hook reports (PROP-020 §2.1), gathered after the
    /// lockfile was written. Empty when no payload-changing package declares a
    /// `post-install` hook; the CLI renders them with the pre-install
    /// reports carried on `outcome.hook_reports`.
    pub post_install_reports: Vec<HookReport>,
    /// Legacy `[hooks]` translated to `slot:` contributions and executed by
    /// the lifecycle handler engine. Empty only on compatibility entry points.
    pub lifecycle_reports: Vec<SlotLifecycleReport>,
    /// The same canonical run used by slot callbacks; the caller rebinds it
    /// to the durable world before dispatching normal phase contributions.
    pub lifecycle_run: Option<LifecycleRunHandle>,
    /// What this apply changed, in the one slot-level shape every install-family
    /// report renders — so no caller re-derives it from `outcome` and no report
    /// invents a per-file census the engine never measured.
    pub progress: InstallProgress,
}

/// [`ApplyReport`] plus the workspace AS OF the end of this apply.
///
/// A separate type on purpose. `ApplyReport` is public 1.0 surface, and a new
/// `pub` field on it breaks every downstream struct literal and exhaustive
/// pattern — a semver-major change for what is additive information. So the
/// prepared sibling returns this, the existing entry points return only
/// `.report`, and nothing outside this crate has to change.
///
/// Step 7 of the apply rewrites `[requires]` with the finalised roots and
/// rebuilds the tree so the boot computation reads them. A caller that plans a
/// post-durability world needs exactly that snapshot: the pre-apply one it
/// passed in does not yet contain the requirements this apply just recorded.
pub struct PreparedApplyReport {
    pub report: ApplyReport,
    pub workspace: Workspace,
    /// Present only on the native-aware production sibling.
    pub native: Option<vibe_workspace::install::NativeInstallCarriage>,
}

/// Apply a confirmed plan. `slot_integrity` selects the PROP-011 §2.3
/// materialise-diff strategy (the caller reads it from the user
/// config, so a malformed config fails before resolution, not here):
/// `trust-presence` skips a present slot outright, `verify` first
/// spot-checks its `content_hash` through [`RegistrySlotVerifier`].
/// `source` is the same install source the plan ran against: the apply
/// phase needs it to perform the deferred incremental `in-place` updates
/// the plan held back from re-cloning (PROP-022 §2.4) — every other
/// package is already fetched into `planned`.
pub fn apply<S: InstallSource + ?Sized>(
    source: &S,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    hooks: &HookPolicy,
) -> Result<ApplyReport> {
    apply_with_spec_format(source, planned, slot_integrity, SpecFormat::Mixed, hooks)
}

/// Apply a confirmed plan using the effective PROP-045 representation.
pub fn apply_with_spec_format<S: InstallSource + ?Sized>(
    source: &S,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    hooks: &HookPolicy,
) -> Result<ApplyReport> {
    apply_with_spec_format_and_hook_output(
        source,
        planned,
        slot_integrity,
        spec_format,
        hooks,
        HookOutput::Inherit,
    )
}

/// Apply with an explicit hook-subprocess stream policy. The existing public
/// entry points remain byte-compatible and inherit hook streams.
pub fn apply_with_spec_format_and_hook_output<S: InstallSource + ?Sized>(
    source: &S,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    hooks: &HookPolicy,
    hook_output: HookOutput,
) -> Result<ApplyReport> {
    apply_with_spec_format_and_slot_lifecycle(
        source,
        planned,
        slot_integrity,
        spec_format,
        SlotLifecycleMode::LegacyHooks {
            policy: hooks,
            output: hook_output,
        },
        None,
        None,
        None,
    )
    // The legacy entry points predate the post-apply tree and have no use for
    // it; dropping it here keeps their signatures byte-identical.
    .map(|prepared| prepared.report)
}

/// Apply through the canonical lifecycle handler engine. `[hooks]` is sugar
/// for one `slot:` script contribution per declared phase; install itself is
/// consent, so this path has no [`HookPolicy`] or prompt surface.
/// The caller-supplied seams the slot lifecycle runs through: narration, and
/// the agent backend a slot-scoped `agent` contribution executes on.
///
/// There is deliberately **no `Default`**. `agent` is legal at slot points, so
/// a defaulted seam is a silent downgrade of a selected contribution to the
/// refusing backend — the exact failure that looks like nothing happening. A
/// caller with genuinely nothing to inject writes
/// [`SlotLifecycleSeams::refusing`] and says so at the call site.
pub struct SlotLifecycleSeams {
    pub observer: Arc<dyn SlotLifecycleObserver>,
    pub agent: Arc<dyn vibe_lifecycle::AgentBackend>,
}

impl SlotLifecycleSeams {
    /// Narrate nothing and refuse every agent row with remediation. Named, not
    /// defaulted, so choosing it is visible in the diff.
    #[must_use]
    pub fn refusing() -> Self {
        Self {
            observer: Arc::new(NoSlotLifecycleObserver),
            agent: Arc::new(vibe_lifecycle::NoAgentBackend),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_with_spec_format_and_lifecycle_observed<S: InstallSource + ?Sized>(
    source: &S,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    run: RunMetadata,
    streams: StreamMode,
    seams: SlotLifecycleSeams,
    lease: std::sync::Arc<LifecycleLease>,
) -> Result<ApplyReport> {
    apply_with_spec_format_and_lifecycle_observed_traced(
        source,
        planned,
        slot_integrity,
        spec_format,
        run,
        streams,
        seams,
        lease,
        None,
    )
}

/// The traced sibling of [`apply_with_spec_format_and_lifecycle_observed`]:
/// one borrowed compile-trace run, carried through the workspace apply this
/// phase performs — package-unit emission first, then every node. The
/// recorder's lifetime is the COMMAND's, not this layer's: apply borrows it
/// and never opens, finishes or clones it into an outcome. A pre-install
/// park/failure still aborts before any boot compilation, exactly as
/// untraced, so a parked run leaves a partial but honest trace.
#[allow(clippy::too_many_arguments)]
pub fn apply_with_spec_format_and_lifecycle_observed_traced<S: InstallSource + ?Sized>(
    source: &S,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    run: RunMetadata,
    streams: StreamMode,
    seams: SlotLifecycleSeams,
    lease: std::sync::Arc<LifecycleLease>,
    trace: Option<&vibe_workspace::compile_trace::TraceRun>,
) -> Result<ApplyReport> {
    apply_with_spec_format_and_lifecycle_observed_traced_prepared(
        source,
        planned,
        slot_integrity,
        spec_format,
        run,
        streams,
        seams,
        lease,
        trace,
    )
    .map(|prepared| prepared.report)
}

/// The same apply, returning the post-apply workspace beside the report.
///
/// This is where the algorithm lives; the entry point above is the
/// compatibility shape that discards the extra value. A caller that must plan
/// a post-durability world — the CLI's Ready path — takes this one, because
/// the tree step 7 rebuilt is the only one that carries the requirements this
/// apply just recorded, and rediscovering it would be a second byte snapshot
/// of a tree the command is midway through changing.
#[allow(clippy::too_many_arguments)]
pub fn apply_with_spec_format_and_lifecycle_observed_traced_prepared<S: InstallSource + ?Sized>(
    source: &S,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    run: RunMetadata,
    streams: StreamMode,
    seams: SlotLifecycleSeams,
    lease: std::sync::Arc<LifecycleLease>,
    trace: Option<&vibe_workspace::compile_trace::TraceRun>,
) -> Result<PreparedApplyReport> {
    apply_with_spec_format_and_lifecycle_observed_traced_prepared_inner(
        source,
        planned,
        slot_integrity,
        spec_format,
        run,
        streams,
        seams,
        lease,
        trace,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_with_spec_format_and_lifecycle_observed_traced_prepared_inner<
    S: InstallSource + ?Sized,
>(
    source: &S,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    run: RunMetadata,
    streams: StreamMode,
    seams: SlotLifecycleSeams,
    lease: std::sync::Arc<LifecycleLease>,
    trace: Option<&vibe_workspace::compile_trace::TraceRun>,
    native: Option<NativeApplyPreparation<'_>>,
) -> Result<PreparedApplyReport> {
    crate::configuration_admission::planned(&planned)?;
    crate::source_admission::planned(&planned, spec_format)?;
    let lifecycle = InstallSlotLifecycle::from_plan_observed(&planned, run, streams, seams, lease)?;
    let lifecycle_run = lifecycle.run_handle();
    let applied = apply_with_spec_format_and_slot_lifecycle(
        source,
        planned,
        slot_integrity,
        spec_format,
        SlotLifecycleMode::Callback(&lifecycle),
        Some(&lifecycle),
        trace,
        native,
    );
    // A parked hosted row halts the orchestrator through the seam's only
    // stopping channel. Retype the sentinel as a durable handoff only when
    // rollback succeeded; any rollback conflict stays a reported failure,
    // with the parked task and run state retained for inspection.
    let mut prepared = match applied {
        Ok(prepared) => prepared,
        Err(error) => {
            let reports = lifecycle.take_reports().unwrap_or_default();
            let progress = Box::new(lifecycle.progress());
            return Err(match (lifecycle.parked(), error) {
                // Parking is a clean handoff only when undo succeeded. Keep
                // the task/state on disk, but surface both errors instead of
                // converting a rollback conflict into a successful park.
                (Some(delegation), Error::RollbackFailed { primary, reason }) => {
                    Error::SlotFailed {
                        source: Box::new(Error::RollbackFailed {
                            primary,
                            reason: format!(
                                "{reason}; lifecycle run `{}` remains parked; retained task(s): {}; resolve the rollback conflict before resuming",
                                delegation.run_id,
                                delegation.tasks.join(", ")
                            ),
                        }),
                        reports,
                        progress,
                    }
                }
                (Some(delegation), _) => Error::Delegated {
                    delegation: Box::new(delegation),
                    reports,
                    progress,
                },
                (None, error) => Error::SlotFailed {
                    source: Box::new(error),
                    reports,
                    progress,
                },
            });
        }
    };
    prepared.report.lifecycle_reports = lifecycle.take_reports()?;
    prepared.report.lifecycle_run = Some(lifecycle_run);
    prepared.report.progress = InstallProgress::complete(&prepared.report.outcome);
    // The slot run reached its end: nothing is owed, so the durable
    // continuation goes. A resume must never rebuild a run that finished.
    lifecycle.clear_continuation().map_err(Error::Lifecycle)?;
    Ok(prepared)
}

#[allow(clippy::too_many_arguments)]
fn apply_metadata_checked<S: InstallSource + ?Sized>(
    source: &S,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    lifecycle: SlotLifecycleMode<'_>,
    // The slot lifecycle, when the caller owns one, so the COMPLETE apply
    // outcome replaces the partial materialise-boundary snapshot the moment
    // it exists. A post-install park then reports the whole apply rather than
    // the pre-install view of it.
    observer: Option<&InstallSlotLifecycle>,
    // The borrowed compile-trace run, when the command owns one; carried into
    // the workspace apply's boot regeneration and no further.
    trace: Option<&vibe_workspace::compile_trace::TraceRun>,
    native: Option<NativeApplyPreparation<'_>>,
    workspace: Workspace,
    files: &mut MetadataFiles,
) -> Result<PreparedApplyReport> {
    let PlannedInstall {
        project_root,
        request: _,
        manifest,
        mut lockfile,
        language_chain,
        mut fetched,
        mut resolution,
        // The plan's pre-apply tree. Step 7 below replaces it with the
        // post-write epoch, so it is deliberately not read between here and
        // there — a value that is about to be stale must not be consulted.
        workspace: _pre_apply_workspace,
        progress,
        source_candidates: _,
    } = planned;

    progress.checkpoint()?;
    let native_placements = deferred::stage_deferred_native(&fetched, &mut resolution)?;
    let placements: Vec<&dyn vibe_workspace::install::PreparedSlotPlacement> = native_placements
        .iter()
        .map(|p| p as &dyn vibe_workspace::install::PreparedSlotPlacement)
        .collect();

    for placement in &placements {
        placement.validate().map_err(|reason| {
            vibe_workspace::WorkspaceError::SpecMaterialization {
                path: placement.destination().into(),
                reason,
            }
        })?;
    }

    progress.checkpoint()?;
    progress.cancellation().publishing();
    // Everything from this write to the lock write is undone together: the
    // guard puts the manifest's earlier bytes back unless the apply finishes.
    if let Some(guard) = &mut files.manifest {
        let rendered = manifest.render_with_comments(guard.previous_text()?)?;
        guard.publish(Some(&rendered))?;
    }

    // 7. Rebuild the workspace so the boot computation reads the just-updated
    //    `[requires]`. This is a legitimate NEW epoch — the manifest above was
    //    rewritten, so the pre-apply tree is genuinely stale — but not a
    //    licence to re-read the selected file: the bytes it would parse are
    //    the ones this function just wrote, and re-reading them would let a
    //    concurrent writer decide what this command installed. The preloaded
    //    seam takes the in-memory value for the selected node and discovers
    //    the rest of the tree around it.
    // The selected-manifest projection and siblings were admitted before publication.
    crate::configuration_admission::workspace_inputs_for_install(
        &workspace,
        &project_root,
        &manifest,
    )?;
    // Check the siblings this discovery actually read before slot effects.
    // Deferred in-place refresh below preserves the solver's version/kind;
    // it only refreshes the source manifest and materialization provenance.
    let roots = crate::record::workspace_root_snapshot(&workspace, &resolution)?;

    // Legacy deferred refresh keeps its incumbent owner. Native candidates were
    // staged above; their slot publication joins the workspace rollback below.
    materialise_deferred_in_place(
        source,
        &workspace,
        &mut fetched,
        &mut resolution,
        &native_placements,
    )?;

    // 8. Apply: materialise each package into vibedeps/, run each freshly
    //    populated slot's pre-install hook (PROP-020 §2.1), and regenerate
    //    every node's boot artifacts. Under `slot_integrity = verify` a
    //    present slot is spot-checked through the registry-hash seam
    //    (PROP-011 §2.3/§5.2): a hash match accepts it without the copy,
    //    a divergence re-materialises it and warns — the fetched set the
    //    verifier reads is post-deferral, so an incrementally-updated
    //    in-place slot (which never consults it) still records fresh.
    let slot_verifier = RegistrySlotVerifier::from_fetched(&fetched);
    let apply_task = progress.task("Applying resolved packages");
    apply_task.set_progress(0, Some(resolution.len() as u64), "packages");
    let (mut outcome, native) = match native {
        Some(native) => {
            let NativeApplyPreparation {
                prepare: prepare_lowering,
                run,
                platform,
            } = native;
            let mut prepare = |workspace: &Workspace, resolution: &[ResolvedDep]| {
                prepare_lowering(workspace, resolution).map_err(|reason| {
                    vibe_workspace::WorkspaceError::NativeCompileProvider {
                        owner: "<install-lowering>".to_owned(),
                        reason,
                    }
                })
            };
            let mut make_provider = |policies| {
                Ok(
                    vibe_lifecycle::native::ArtifactCompilerNativeProvider::new(platform, policies)
                        .with_cancellation(progress.cancellation().clone()),
                )
            };
            let prepared =
                vibe_workspace::install::apply_resolution_with_spec_format_and_slot_lifecycle_traced_native_progress_placements(
                    &workspace,
                    &resolution,
                    slot_integrity,
                    spec_format,
                    Some(&slot_verifier),
                    lifecycle,
                    trace,
                    &mut prepare,
                    run,
                    &mut make_provider,
                    &progress,
                    &placements,
                ).map_err(deferred::workspace_error)?;
            (prepared.outcome, Some(prepared.carriage))
        }
        None => (
            apply_resolution_with_spec_format_and_slot_lifecycle_traced_placements(
                &workspace,
                &resolution,
                slot_integrity,
                spec_format,
                Some(&slot_verifier),
                lifecycle,
                trace,
                &placements,
            )
            .map_err(deferred::workspace_error)?,
            None,
        ),
    };
    apply_task.set_progress(
        resolution.len() as u64,
        Some(resolution.len() as u64),
        "packages",
    );
    apply_task.finish();

    for warning in &outcome.integrity_warnings {
        tracing::warn!(target: "vibe_install::apply", "{warning}");
    }

    let record_task = progress.task("Recording installation state");

    // 9-10. Rebuild and write the lockfile. Until it and its provenance are
    //       written the install is not durable: a failure undoes this apply's
    //       slot changes, and the guards put the three files back.
    let lockfile_path = workspace.lockfile_path();
    let recorded = progress
        .checkpoint()
        .map_err(Error::from)
        .and_then(|()| {
            lock::record_lock(
                &mut lockfile,
                lock::LockInputs {
                    fetched: &fetched,
                    resolution: &resolution,
                    language_chain: &language_chain,
                    roots: &roots,
                    lockfile_path: &lockfile_path,
                },
                files,
            )
        })
        .and_then(|()| progress.checkpoint().map_err(Error::from));
    if let Err(error) = recorded {
        let error = files.undo_records(error);
        let observer =
            observer.map(|lifecycle| lifecycle as &dyn vibe_workspace::install::SlotLifecycle);
        return Err(match outcome.roll_back(observer) {
            Ok(()) => error,
            Err(reason) => Error::RollbackFailed {
                primary: Box::new(error),
                reason,
            },
        });
    }
    outcome.commit();
    files.disarm();
    progress.cancellation().committed();
    record_task.finish();

    // 11. PROP-020 §2.1 — post-install hooks run once the package is durable
    //     (lockfile written, boot regenerated). A non-zero exit is surfaced
    //     as a flagged report, not fatal; a missing interpreter is a hard
    //     error. Only slots whose materialised payload changed run their hook.
    // The apply is complete: whatever a later post-install row does, the
    // record of what this install changed is final and is handed to the
    // observer BEFORE the callbacks that could stop the run.
    if let Some(observer) = observer {
        observer.record_complete(InstallProgress::complete(&outcome));
    }
    progress.checkpoint()?;
    let post_install_reports = match outcome.take_post_install_plan() {
        Some(plan) => run_post_install_slot_lifecycle(plan, lifecycle)?,
        None => Vec::new(),
    };

    Ok(PreparedApplyReport {
        workspace,
        native,
        report: ApplyReport {
            progress: InstallProgress::complete(&outcome),
            outcome,
            post_install_reports,
            lifecycle_reports: Vec::new(),
            lifecycle_run: None,
        },
    })
}
