//! Capture metadata before effects; only actual writes claim mutation ownership.
use super::{
    Error, MetadataFiles, PackageRef, Result, finalize_pkgref_for_manifest, merge_manifest_requires,
};

use super::{
    InstallSlotLifecycle, InstallSource, NativeApplyPreparation, PlannedInstall,
    PreparedApplyReport, SlotIntegrity, SlotLifecycleMode, SpecFormat,
};

#[allow(clippy::too_many_arguments)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub(super) fn apply_with_spec_format_and_slot_lifecycle<S: InstallSource + ?Sized>(
    source: &S,
    mut planned: PlannedInstall,
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
    crate::configuration_admission::planned(&planned)?;
    crate::source_admission::planned(&planned, spec_format)?;
    let before_manifest = planned.manifest.clone();
    let manifest_changed = prepare_manifest(&mut planned)?;
    let workspace = vibe_workspace::Workspace::discover_with_selected_manifest(
        &planned.project_root,
        &planned.manifest,
    )?;
    planned.workspace.validate_authored_transition(
        &workspace,
        &planned.project_root,
        &planned.manifest,
    )?;
    crate::configuration_admission::workspace_inputs_for_install(
        &workspace,
        &planned.project_root,
        &planned.manifest,
    )?;

    crate::configuration_admission::unchanged_inputs(
        &planned.workspace,
        &workspace,
        &planned.project_root,
        &planned.manifest,
    )?;

    let mut files = MetadataFiles::capture(
        &planned.project_root,
        &planned.workspace.lockfile_path(),
        &before_manifest,
        manifest_changed,
    )?;
    let cancellation = planned.progress.cancellation().clone();
    super::apply_metadata_checked(
        source,
        planned,
        slot_integrity,
        spec_format,
        lifecycle,
        observer,
        trace,
        native,
        workspace,
        &mut files,
    )
    .map_err(|error| {
        let error = files.undo_all(error);
        if !cancellation.acknowledged() {
            return error;
        }
        // A stop rolls back the outputs of agent steps that completed earlier
        // in this install too: they are the operation's pre-commit effects,
        // undone through their receipt-checked inverse (PROP-073
        // `#OPERATION-EFFECTS-LAW`); a foreign replacement is preserved and
        // named, and the install is then never reported `restored`.
        let error = match cancellation.undo_completed_effects() {
            Ok(()) => error,
            Err(reason) => Error::RollbackFailed {
                primary: Box::new(error),
                reason,
            },
        };
        // An acknowledged stop is `restored` exactly when every owned undo
        // succeeded, whichever owner observed the stop (a hook, a checkpoint,
        // or the cancellable boot publication).
        if !restoration_failed(&error) {
            cancellation.restored();
        }
        error
    })
}

/// Whether undoing owned publication itself failed: guarded metadata files
/// that could not be put back, or the slot/boot rollback log.
fn restoration_failed(error: &Error) -> bool {
    matches!(
        error,
        Error::RollbackFailed { .. }
            | Error::Workspace(vibe_workspace::WorkspaceError::InstallRollback { .. })
    )
}

fn prepare_manifest(planned: &mut PlannedInstall) -> Result<bool> {
    let PlannedInstall {
        request,
        manifest,
        fetched,
        ..
    } = planned;
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
        merge_manifest_requires(manifest, &finalized_roots)
    };
    Ok(manifest_changed)
}
