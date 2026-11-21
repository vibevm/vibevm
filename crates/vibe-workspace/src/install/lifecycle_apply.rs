//! Full-resolution apply under one dependency-slot lifecycle mode.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use super::*;
use crate::install::bootgen::regenerate_boot_from_traced;

/// Apply a full resolution under exactly one dependency-slot lifecycle mode.
pub fn apply_resolution_with_spec_format_and_slot_lifecycle(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: SlotLifecycleMode<'_>,
) -> Result<InstallOutcome, WorkspaceError> {
    apply_resolution_with_spec_format_and_slot_lifecycle_traced(
        workspace,
        resolution,
        slot_integrity,
        spec_format,
        slot_verifier,
        lifecycle,
        None,
    )
}

/// The traced sibling of [`apply_resolution_with_spec_format_and_slot_lifecycle`]:
/// one borrowed run carried through the boot regeneration this apply performs.
/// Pre-install park/failure still aborts before any boot compilation — the
/// recorder is only ever BORROWED here, never opened or finished.
pub fn apply_resolution_with_spec_format_and_slot_lifecycle_traced(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: SlotLifecycleMode<'_>,
    trace: Option<&crate::compile_trace::TraceRun>,
) -> Result<InstallOutcome, WorkspaceError> {
    apply_resolution_with_spec_format_and_slot_lifecycle_traced_placements(
        workspace,
        resolution,
        slot_integrity,
        spec_format,
        slot_verifier,
        lifecycle,
        trace,
        &[],
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "the additive compatibility seam preserves the established apply inputs and adds only borrowed checked placements"
)]
pub fn apply_resolution_with_spec_format_and_slot_lifecycle_traced_placements(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: SlotLifecycleMode<'_>,
    trace: Option<&crate::compile_trace::TraceRun>,
    placements: &[&dyn PreparedSlotPlacement],
) -> Result<InstallOutcome, WorkspaceError> {
    // A malformed instruction block aborts before any mutation.
    source_admission::validate_source_admission(workspace, resolution, spec_format)?;
    validate_redirect_blocks(workspace, resolution)?;

    let observer = lifecycle.callback();
    let mut rollback = InstallRollback::new(&workspace.root);
    let Materialised {
        materialised,
        skipped,
        integrity_warnings,
        post_install_deps,
    } = match materialise_resolution_with_prepared(
        &workspace.root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format,
            slot_verifier,
            lifecycle,
            progress: vibe_core::progress::Progress::default(),
            rollback: Some(&mut rollback),
        },
        placements,
    ) {
        Ok(result) => result,
        Err(error) if !placements.is_empty() => {
            return Err(rollback.undo(
                error,
                observer,
                &placements
                    .iter()
                    .map(|p| {
                        let (g, n, _) = p.coordinate();
                        vibedeps::in_place_slot_rel_path(g, n)
                    })
                    .collect::<Vec<_>>(),
            ));
        }
        Err(error) => return Err(error),
    };

    // From here to the caller's lock write, a failure puts every slot back.
    let kept: Vec<String> = materialised.iter().chain(&skipped).cloned().collect();
    let finished = prune_stale_slots(&workspace.root, &kept, &mut rollback).and_then(|pruned| {
        rollback
            .regenerate_boot(workspace, resolution, || {
                regenerate_boot_from_traced(workspace, resolution, spec_format, trace)
            })
            .map(|nodes| (pruned, nodes))
    });
    let (pruned, nodes_regenerated) = match finished {
        Ok(finished) => finished,
        Err(error) => return Err(rollback.undo(error, observer, &materialised)),
    };

    Ok(InstallOutcome {
        materialised,
        skipped,
        integrity_warnings,
        pruned,
        nodes_regenerated,
        post_install_plan: PostInstallPlan::new(&workspace.root, post_install_deps),
        rollback: Some(rollback),
    })
}

/// Native-aware sibling used by the production install convergence path.
/// Materialisation and pre-install callbacks finish before the exact supplied
/// resolution is lowered and compiled; no Cargo process is reachable here.
#[allow(clippy::too_many_arguments)]
pub fn apply_resolution_with_spec_format_and_slot_lifecycle_traced_native<F, G, P>(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: SlotLifecycleMode<'_>,
    trace: Option<&crate::compile_trace::TraceRun>,
    prepare: &mut F,
    run: crate::extension_world::OwnerRuntimeRunFacts,
    make_provider: &mut G,
) -> Result<NativeInstallOutcome, WorkspaceError>
where
    F: FnMut(
        &Workspace,
        &[ResolvedDep],
    ) -> Result<
        (
            crate::extension_world::ExtensionWorldEpoch,
            crate::extension_world::OwnerRuntimeLowering,
        ),
        WorkspaceError,
    >,
    G: FnMut(
        std::collections::BTreeMap<
            crate::extension_world::OwnerRuntimeId,
            vibe_spec::CompilerNativePolicy,
        >,
    ) -> Result<P, WorkspaceError>,
    P: crate::extension_world::OwnerNativeCompileProvider,
{
    apply_resolution_with_spec_format_and_slot_lifecycle_traced_native_progress(
        workspace,
        resolution,
        slot_integrity,
        spec_format,
        slot_verifier,
        lifecycle,
        trace,
        prepare,
        run,
        make_provider,
        &vibe_core::progress::Progress::default(),
    )
}

/// Observed native-aware sibling used by the install command.
#[allow(clippy::too_many_arguments)]
pub fn apply_resolution_with_spec_format_and_slot_lifecycle_traced_native_progress<F, G, P>(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: SlotLifecycleMode<'_>,
    trace: Option<&crate::compile_trace::TraceRun>,
    prepare: &mut F,
    run: crate::extension_world::OwnerRuntimeRunFacts,
    make_provider: &mut G,
    progress: &vibe_core::progress::Progress,
) -> Result<NativeInstallOutcome, WorkspaceError>
where
    F: FnMut(
        &Workspace,
        &[ResolvedDep],
    ) -> Result<
        (
            crate::extension_world::ExtensionWorldEpoch,
            crate::extension_world::OwnerRuntimeLowering,
        ),
        WorkspaceError,
    >,
    G: FnMut(
        std::collections::BTreeMap<
            crate::extension_world::OwnerRuntimeId,
            vibe_spec::CompilerNativePolicy,
        >,
    ) -> Result<P, WorkspaceError>,
    P: crate::extension_world::OwnerNativeCompileProvider,
{
    apply_resolution_with_spec_format_and_slot_lifecycle_traced_native_progress_placements(
        workspace,
        resolution,
        slot_integrity,
        spec_format,
        slot_verifier,
        lifecycle,
        trace,
        prepare,
        run,
        make_provider,
        progress,
        &[],
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "the additive native sibling preserves the predecessor's independently borrowed lowering, provider and progress inputs and adds checked placements"
)]
pub fn apply_resolution_with_spec_format_and_slot_lifecycle_traced_native_progress_placements<
    F,
    G,
    P,
>(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: SlotLifecycleMode<'_>,
    trace: Option<&crate::compile_trace::TraceRun>,
    prepare: &mut F,
    run: crate::extension_world::OwnerRuntimeRunFacts,
    make_provider: &mut G,
    progress: &vibe_core::progress::Progress,
    placements: &[&dyn PreparedSlotPlacement],
) -> Result<NativeInstallOutcome, WorkspaceError>
where
    F: FnMut(
        &Workspace,
        &[ResolvedDep],
    ) -> Result<
        (
            crate::extension_world::ExtensionWorldEpoch,
            crate::extension_world::OwnerRuntimeLowering,
        ),
        WorkspaceError,
    >,
    G: FnMut(
        std::collections::BTreeMap<
            crate::extension_world::OwnerRuntimeId,
            vibe_spec::CompilerNativePolicy,
        >,
    ) -> Result<P, WorkspaceError>,
    P: crate::extension_world::OwnerNativeCompileProvider,
{
    source_admission::validate_source_admission(workspace, resolution, spec_format)?;
    validate_redirect_blocks(workspace, resolution)?;
    let mut rollback = InstallRollback::new(&workspace.root);
    let observer = lifecycle.callback();
    let Materialised {
        materialised,
        skipped,
        integrity_warnings,
        post_install_deps,
    } = match materialise_resolution_with_prepared(
        &workspace.root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format,
            slot_verifier,
            lifecycle,
            progress: progress.clone(),
            rollback: Some(&mut rollback),
        },
        placements,
    ) {
        Ok(result) => result,
        Err(error) if !placements.is_empty() => {
            return Err(rollback.undo(
                error,
                observer,
                &placements
                    .iter()
                    .map(|p| {
                        let (g, n, _) = p.coordinate();
                        vibedeps::in_place_slot_rel_path(g, n)
                    })
                    .collect::<Vec<_>>(),
            ));
        }
        Err(error) => return Err(error),
    };
    let kept = materialised
        .iter()
        .chain(&skipped)
        .cloned()
        .collect::<Vec<_>>();
    // From here to the caller's lock write, a failure puts every slot back.
    let finished = native_boot(
        workspace,
        resolution,
        &kept,
        &mut rollback,
        NativeBoot {
            spec_format,
            trace,
            prepare,
            run,
            make_provider,
            progress,
        },
    );
    let (pruned, nodes_regenerated, carriage) = match finished {
        Ok(finished) => finished,
        Err(error) => return Err(rollback.undo(error, observer, &materialised)),
    };
    Ok(NativeInstallOutcome {
        outcome: InstallOutcome {
            materialised,
            skipped,
            integrity_warnings,
            pruned,
            nodes_regenerated,
            post_install_plan: PostInstallPlan::new(&workspace.root, post_install_deps),
            rollback: Some(rollback),
        },
        carriage,
    })
}

/// The native apply's steps after materialisation, each one able to fail
/// while every slot change is still undoable.
struct NativeBoot<'a, F, G> {
    spec_format: SpecFormat,
    trace: Option<&'a crate::compile_trace::TraceRun>,
    prepare: &'a mut F,
    run: crate::extension_world::OwnerRuntimeRunFacts,
    make_provider: &'a mut G,
    progress: &'a vibe_core::progress::Progress,
}

type NativeBootResult = Result<(Vec<String>, Vec<String>, NativeInstallCarriage), WorkspaceError>;

fn native_boot<F, G, P>(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    kept: &[String],
    rollback: &mut InstallRollback,
    steps: NativeBoot<'_, F, G>,
) -> NativeBootResult
where
    F: FnMut(
        &Workspace,
        &[ResolvedDep],
    ) -> Result<
        (
            crate::extension_world::ExtensionWorldEpoch,
            crate::extension_world::OwnerRuntimeLowering,
        ),
        WorkspaceError,
    >,
    G: FnMut(
        std::collections::BTreeMap<
            crate::extension_world::OwnerRuntimeId,
            vibe_spec::CompilerNativePolicy,
        >,
    ) -> Result<P, WorkspaceError>,
    P: crate::extension_world::OwnerNativeCompileProvider,
{
    let NativeBoot {
        spec_format,
        trace,
        prepare,
        run,
        make_provider,
        progress,
    } = steps;
    let pruned = prune_stale_slots(&workspace.root, kept, rollback)?;
    // This callback is deliberately HERE: every slot and pre-install effect
    // is now visible, while boot publication and the lock write have not run.
    let (world, lowering) = prepare(workspace, resolution)?;
    let boot_task = progress.task("Generating boot artifacts");
    boot_task.set_progress(0, Some(workspace.iter_nodes().count() as u64), "nodes");
    let (nodes_regenerated, carriage) = rollback.regenerate_boot(workspace, resolution, || {
        super::bootgen::regenerate_boot_from_traced_native_prepared(
            workspace,
            resolution,
            world,
            spec_format,
            trace,
            lowering,
            run,
            make_provider,
        )
    })?;
    boot_task.set_progress(
        nodes_regenerated.len() as u64,
        Some(nodes_regenerated.len() as u64),
        "nodes",
    );
    boot_task.finish();
    Ok((pruned, nodes_regenerated, carriage))
}
