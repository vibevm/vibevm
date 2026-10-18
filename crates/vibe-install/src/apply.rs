//! The apply phase — runs only after the caller confirmed the plan:
//! merge the requested roots into `vibe.toml`, materialise the
//! resolution into `vibedeps/`, and rebuild `vibe.lock` wholesale.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use crate::InstallSource;
use crate::error::{Error, Result};
use crate::fetched::Fetched;
use crate::lifecycle::{
    InstallProgress, InstallSlotLifecycle, NoSlotLifecycleObserver, SlotLifecycleObserver,
    SlotLifecycleReport,
};
use crate::plan::PlannedInstall;
use crate::record::{
    exact_pinned_pkgref, finalize_pkgref_for_manifest, locked_package_from_fetched,
    merge_manifest_requires, merge_root_dependencies, replace_root_dependencies,
};
use crate::slot_verify::RegistrySlotVerifier;
use std::collections::BTreeSet;
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
    InstallOutcome, ResolvedDep, SlotLifecycleMode, run_post_install_slot_lifecycle,
};
use vibe_workspace::vibedeps;

mod deferred;
mod native;
mod strict_conversion;
use deferred::materialise_deferred_in_place;
use native::NativeApplyPreparation;
use strict_conversion::prepare_strict_conversion;

mod report;
pub use report::{ApplyReport, PreparedApplyReport};

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
    // stopping channel. It is a durable handoff, not a failure — so it is
    // retyped here, WITH the rows executed up to the park, before any caller
    // can mistake the sentinel for an install error.
    let mut prepared = match applied {
        Ok(prepared) => prepared,
        Err(error) => {
            return Err(match lifecycle.parked() {
                Some(delegation) => Error::Delegated {
                    delegation: Box::new(delegation),
                    reports: lifecycle.take_reports().unwrap_or_default(),
                    // Progress was recorded AS the mutation happened, so a
                    // pre-install park reports the slots that survived its
                    // rollback and a post-install park reports the whole
                    // completed apply. Neither invents a file census.
                    progress: Box::new(lifecycle.progress()),
                },
                // A failure is an outcome too: the rows this pass executed and
                // the progress it really made travel with it, so the outermost
                // command can report them.
                None => Error::SlotFailed {
                    source: Box::new(error),
                    reports: lifecycle.take_reports().unwrap_or_default(),
                    progress: Box::new(lifecycle.progress()),
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
fn apply_with_spec_format_and_slot_lifecycle<S: InstallSource + ?Sized>(
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
) -> Result<PreparedApplyReport> {
    planned.preconditions.validate()?;
    planned.validate_selected_routes()?;
    // Rebind the current source using the SAME admitted request. This is a
    // freshness check, never a solve or authorization of replacement defaults.
    if let Some(request) = &planned.normalized_request {
        if let Some(current_request) = source.normalized_request() {
            if crate::requested_record(current_request)? != crate::requested_record(request)? {
                return Err(vibe_resolver::SolveError::UnsupportedRoute(
                    "authored request changed since planning; refusing before writes".into(),
                )
                .into());
            }
        }
        let current = source.begin_normalized_transaction(request)?;
        match current.as_deref() {
            Some(current) => validate_current_source(current, &planned)?,
            None => validate_current_source(source, &planned)?,
        }
    } else {
        validate_current_source(source, &planned)?;
    }
    let preflight_configuration = crate::record::configuration_record(
        planned.normalized_request.as_ref(),
        &planned.selected_provenance,
        &planned.fetched,
    )?;
    if !planned.authored_reconfiguration {
        crate::record::validate_replayed_result(
            &planned.lockfile,
            preflight_configuration.as_ref(),
            if planned.configuration_update {
                &planned.roots
            } else {
                &planned.request.roots
            },
        )?;
    }
    let PlannedInstall {
        preconditions: _,
        manifest_pending,
        captured_catalog: _,
        configuration_update: _,
        authored_reconfiguration: _,
        strict_conversion,
        project_root,
        request,
        mut manifest,
        mut lockfile,
        language_chain,
        roots,
        mut fetched,
        mut resolution,
        // Checked against the fetched graph before any apply effects. These
        // are also captured in the D1 record above before apply effects.
        normalized_request,
        selected_provenance,
        // The plan's pre-apply tree. Step 7 below replaces it with the
        // post-write epoch, so it is deliberately not read between here and
        // there — a value that is about to be stale must not be consulted.
        workspace: _pre_apply_workspace,
        progress,
    } = planned;

    // All applicable conversions are prepared before this function's first
    // manifest/slot/lock write, and the prepared bytes are reused below.
    let strict_prepared = prepare_strict_conversion(
        strict_conversion,
        spec_format,
        &project_root,
        &resolution,
        &fetched,
    )?;

    // 6. Update `vibe.toml` `[requires].packages` with the requested
    //    roots — caret by default, `exact` pins `=<resolved>`, an
    //    explicit constraint is preserved verbatim. De-dup by
    //    `(group, name)`; a no-op in install-from-manifest mode.
    //
    //    This MUST run before the boot regeneration below:
    //    `apply_resolution` composes each node's boot from its
    //    `[requires]`, so a package installed by pkgref has to be
    //    declared first or its boot snippet is dropped from the
    //    generated `INDEX.md`.
    //
    //    A requested root absent from the fetched set means the
    //    install source returned an incomplete resolution — through
    //    the seam that is checkable input, not a construction
    //    invariant, so it surfaces as an error.
    let finalized_roots: Vec<PackageRef> = request
        .roots
        .iter()
        .map(|cli_pkgref| {
            let resolved = fetched
                .iter()
                .find(|f| {
                    Some(&f.cached.resolved.group) == cli_pkgref.group.as_ref()
                        && f.cached.resolved.name == cli_pkgref.name
                })
                .map(|f| &f.cached.resolved.version)
                .ok_or_else(|| Error::RootNotFetched {
                    pkgref: cli_pkgref.to_string(),
                })?;
            Ok(finalize_pkgref_for_manifest(
                cli_pkgref,
                resolved,
                request.exact,
            ))
        })
        .collect::<Result<_>>()?;
    let manifest_changed = if finalized_roots.is_empty() {
        false
    } else {
        merge_manifest_requires(&mut manifest, &finalized_roots)
    };
    let workspace = Workspace::discover_with_selected_manifest(&project_root, &manifest)?;
    vibe_workspace::install::preflight_resolution_with_configuration(
        &workspace,
        &resolution,
        spec_format,
        preflight_configuration.as_ref(),
    )?;
    let slots = lockfile
        .packages
        .iter()
        .filter(|p| !p.materialization.is_in_place())
        .map(|p| std::path::PathBuf::from(vibedeps::slot_rel_path(&p.group, &p.name, &p.version)))
        .chain(
            resolution
                .iter()
                .filter(|dep| {
                    !dep.manifest
                        .package
                        .as_ref()
                        .is_some_and(|p| p.materialization.is_in_place())
                })
                .map(|dep| {
                    std::path::PathBuf::from(vibedeps::slot_rel_path(
                        &dep.group,
                        &dep.name,
                        &dep.version,
                    ))
                }),
        )
        .collect::<Vec<_>>();
    let mut recovery = vibe_workspace::configuration_transaction::ConfigurationTransaction::begin(
        &_pre_apply_workspace,
        &slots,
        "install",
    )?;
    recovery.enter("manifest", false)?;
    if manifest_changed || manifest_pending {
        manifest.write(project_root.join(Manifest::FILENAME))?;
    }
    recovery.checkpoint()?;

    // 7. Rebuild the workspace so the boot computation reads the just-updated
    //    `[requires]`. This is a legitimate NEW epoch — the manifest above was
    //    rewritten, so the pre-apply tree is genuinely stale — but not a
    //    licence to re-read the selected file: the bytes it would parse are
    //    the ones this function just wrote, and re-reading them would let a
    //    concurrent writer decide what this command installed. The preloaded
    //    seam takes the in-memory value for the selected node and discovers
    //    the rest of the tree around it.
    // The workspace above already contains the exact prepared manifest delta.

    // 7a. PROP-022 §2.4 — perform the deferred incremental `in-place` updates
    //     the plan held back (a re-resolve of an already-present in-place
    //     package, which the plan refused to re-clone). Now past the operator's
    //     confirmation, `git fetch` each live slot onto its own `.git` to the
    //     resolved ref — transferring only changed objects rather than
    //     re-downloading the giant — and fold the freshly-read manifest /
    //     commit / hash back into the fetched set (→ lockfile) and the
    //     resolution (→ boot + hooks). The slot stays the `content_dir`, so the
    //     materialise pass reads the "already placed" signal and runs the hook
    //     without moving anything. This extends the canonical incremental path
    //     (`vibe update <pkg>`) to the general install re-resolve.
    recovery.enter("slot", true)?;
    materialise_deferred_in_place(source, &workspace, &mut fetched, &mut resolution)?;

    // 8. Apply: materialise each package into vibedeps/, run each freshly
    //    populated slot's pre-install hook (PROP-020 §2.1), and regenerate
    //    every node's boot artifacts. Under `slot_integrity = verify` a
    //    present slot is spot-checked through the registry-hash seam
    //    (PROP-011 §2.3/§5.2): a hash match accepts it without the copy,
    //    a divergence re-materialises it and warns — the fetched set the
    //    verifier reads is post-deferral, so an incrementally-updated
    //    in-place slot (which never consults it) still records fresh.
    let slot_verifier =
        strict_prepared.unwrap_or_else(|| RegistrySlotVerifier::from_fetched(&fetched));
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
                Ok(vibe_lifecycle::native::ArtifactCompilerNativeProvider::new(
                    platform, policies,
                ))
            };
            let prepared =
                vibe_workspace::install::apply_resolution_native_with_configuration_recovery(
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
                    preflight_configuration.as_ref(),
                    Some(&mut recovery),
                )?;
            (prepared.outcome, Some(prepared.carriage))
        }
        None => (
            vibe_workspace::install::apply_resolution_with_configuration_recovery(
                &workspace,
                &resolution,
                slot_integrity,
                spec_format,
                Some(&slot_verifier),
                lifecycle,
                trace,
                preflight_configuration.as_ref(),
                Some(&mut recovery),
            )?,
            None,
        ),
    };
    apply_task.set_progress(
        resolution.len() as u64,
        Some(resolution.len() as u64),
        "packages",
    );
    apply_task.finish();
    recovery.checkpoint()?;

    for warning in &outcome.integrity_warnings {
        tracing::warn!(target: "vibe_install::apply", "{warning}");
    }

    let record_task = progress.task("Recording installation state");

    // 9. Rebuild the lockfile from the fresh resolution — an install
    //    re-resolves the whole graph, so the recorded package set is
    //    replaced wholesale.
    let resolved_language: Option<String> = language_chain.first().cloned().filter(|l| l != "en");
    lockfile.packages.clear();
    for f in &fetched {
        let resolved = resolution
            .iter()
            .find(|candidate| {
                candidate.group == f.cached.resolved.group
                    && candidate.name == f.cached.resolved.name
            })
            .ok_or_else(|| {
                Error::Solve(
                    DepProviderError::Other(format!(
                        "resolved package `{}/{}` has no matching visibility metadata \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-050#resolver; \
                          fix: report the mismatched Fetched/ResolvedDep sets in vibe-install)",
                        f.cached.resolved.group, f.cached.resolved.name
                    ))
                    .into(),
                )
            })?;
        lockfile.packages.push(locked_package_from_fetched(
            f,
            resolved,
            resolved_language.as_deref(),
        ));
    }
    lockfile.meta.generated_at = vibe_core::timestamp::now_utc();
    if !language_chain.is_empty() && language_chain != ["en"] {
        lockfile.meta.language_chain = language_chain.clone();
    }
    let mut active_features_global: BTreeSet<String> = BTreeSet::new();
    for f in &fetched {
        let pkg_label = format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name);
        for feat in &f.feature_expansion.active_features {
            active_features_global.insert(format!("{pkg_label}/{feat}"));
        }
    }
    lockfile.meta.active_features = active_features_global.into_iter().collect();

    // 10. Mirror the declared roots into `meta.root_dependencies` so
    //     the lockfile stays a self-contained snapshot (PROP-002 §2.7).
    //     Install-from-manifest records the workspace-derived roots;
    //     an explicit install records the finalized request roots.
    if finalized_roots.is_empty() {
        // Bare install: `roots` is the complete current workspace union, so
        // mirror it exactly and drop declarations removed since the prior
        // lock.  Merging here would write a lock that freshness rejects on the
        // very next invocation.
        replace_root_dependencies(&mut lockfile, &roots);
    } else {
        // Explicit install: only the requested refs were finalized; merge
        // those into the existing full-root snapshot.
        merge_root_dependencies(&mut lockfile, &finalized_roots);
    }

    lockfile.meta.configuration = crate::record::configuration_record(
        normalized_request.as_ref(),
        &selected_provenance,
        &fetched,
    )?;
    recovery.enter("lock", false)?;
    lockfile.write(workspace.lockfile_path())?;
    recovery.checkpoint()?;
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
    recovery.enter("post-install", true)?;
    let post_install_reports = match outcome.take_post_install_plan() {
        Some(plan) => run_post_install_slot_lifecycle(plan, lifecycle)?,
        None => Vec::new(),
    };
    recovery.checkpoint()?;
    // Callback execution records soft failures on the lifecycle owner rather
    // than in the legacy HookReport list. Inspect without draining the reports
    // that the outer apply still owes its caller.
    let callback_post_failed = match observer {
        Some(observer) => observer.has_failed_post_install()?,
        None => false,
    };
    if post_install_reports
        .iter()
        .any(|report| report.status == "post-install-failed")
        || callback_post_failed
    {
        // A flagged nonfatal hook failure preserves the normal report shape,
        // but is not proof that the operation or its external effects completed.
        recovery.enter("post-install-failed", true)?;
    } else {
        recovery.complete()?;
    }

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

fn validate_current_source<S: InstallSource + ?Sized>(
    source: &S,
    planned: &PlannedInstall,
) -> Result<()> {
    if let Some(captured) = &planned.captured_catalog {
        if source.catalog_inspection().map(|c| &c.candidates) != Some(&captured.candidates) {
            return Err(vibe_resolver::SolveError::UnsupportedRoute(
                "catalog changed since planning; refusing before configuration writes".into(),
            )
            .into());
        }
    }
    for fetched in &planned.fetched {
        let row = &fetched.cached.resolved;
        let package = PackageRef::parse(&format!("{}/{}@={}", row.group, row.name, row.version))?;
        source.validate_catalog_candidate(&package)?;
        if !fetched.cached.package_meta().materialization.is_in_place()
            && vibe_registry::compute_content_hash(&fetched.cached.cache_dir)?
                != fetched.cached.content_hash
        {
            return Err(vibe_resolver::SolveError::UnsupportedRoute(format!(
                "captured payload changed for `{package}`; refusing before writes"
            ))
            .into());
        }
    }
    Ok(())
}
