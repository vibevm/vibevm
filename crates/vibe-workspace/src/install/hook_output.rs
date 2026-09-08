//! Full-resolution apply with an explicit hook-subprocess stream policy.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use super::*;
use crate::hooks::ConfiguredHookRunner;
use crate::install::bootgen::regenerate_boot_from_traced;

/// Additive execution-policy seam for callers that must contain hook streams.
/// Existing install-family APIs preserve inherited subprocess I/O by routing
/// through [`HookOutput::Inherit`].
pub fn apply_resolution_with_spec_format_and_hook_output(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    hooks: Option<&HookPolicy>,
    hook_output: HookOutput,
) -> Result<InstallOutcome, WorkspaceError> {
    let lifecycle = match hooks {
        Some(policy) => SlotLifecycleMode::LegacyHooks {
            policy,
            output: hook_output,
        },
        None => SlotLifecycleMode::None,
    };
    apply_resolution_with_spec_format_and_slot_lifecycle(
        workspace,
        resolution,
        slot_integrity,
        spec_format,
        slot_verifier,
        lifecycle,
    )
}

/// Apply a full resolution under exactly one dependency-slot lifecycle mode.
///
/// The mode is neutral to lifecycle implementation and structurally prevents
/// legacy `[hooks]` execution from being combined with a lifecycle callback.
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
    match lifecycle {
        SlotLifecycleMode::None => apply_with_materialise_lifecycle(
            workspace,
            resolution,
            slot_integrity,
            spec_format,
            slot_verifier,
            MaterialiseLifecycle::None,
            trace,
        ),
        SlotLifecycleMode::Callback(callback) => apply_with_materialise_lifecycle(
            workspace,
            resolution,
            slot_integrity,
            spec_format,
            slot_verifier,
            MaterialiseLifecycle::Callback(callback),
            trace,
        ),
        SlotLifecycleMode::LegacyHooks { policy, output } => {
            let runner = ConfiguredHookRunner::new(output);
            apply_with_materialise_lifecycle(
                workspace,
                resolution,
                slot_integrity,
                spec_format,
                slot_verifier,
                MaterialiseLifecycle::LegacyHooks {
                    policy,
                    probe: &SystemProbe,
                    runner: &runner,
                },
                trace,
            )
        }
    }
}

fn apply_with_materialise_lifecycle(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: MaterialiseLifecycle<'_>,
    trace: Option<&crate::compile_trace::TraceRun>,
) -> Result<InstallOutcome, WorkspaceError> {
    // A malformed instruction block aborts before any mutation.
    validate_redirect_blocks(workspace)?;

    let Materialised {
        materialised,
        skipped,
        integrity_warnings,
        post_install_deps,
        hook_reports,
    } = materialise_resolution_with_spec_format(
        &workspace.root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format,
            slot_verifier,
            lifecycle,
        },
    )?;

    let kept: Vec<String> = materialised.iter().chain(&skipped).cloned().collect();
    let pruned = prune_stale_slots(&workspace.root, &kept)?;
    let nodes_regenerated = regenerate_boot_from_traced(workspace, resolution, spec_format, trace)?;

    Ok(InstallOutcome {
        materialised,
        skipped,
        integrity_warnings,
        pruned,
        nodes_regenerated,
        post_install_plan: PostInstallPlan::new(&workspace.root, post_install_deps),
        hook_reports,
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
    validate_redirect_blocks(workspace)?;
    let materialise = |lifecycle| {
        materialise_resolution_with_spec_format(
            &workspace.root,
            resolution,
            MaterialiseOptions {
                slot_integrity,
                spec_format,
                slot_verifier,
                lifecycle,
            },
        )
    };
    let Materialised {
        materialised,
        skipped,
        integrity_warnings,
        post_install_deps,
        hook_reports,
    } = match lifecycle {
        SlotLifecycleMode::None => materialise(MaterialiseLifecycle::None)?,
        SlotLifecycleMode::Callback(callback) => {
            materialise(MaterialiseLifecycle::Callback(callback))?
        }
        SlotLifecycleMode::LegacyHooks { policy, output } => {
            let runner = ConfiguredHookRunner::new(output);
            materialise(MaterialiseLifecycle::LegacyHooks {
                policy,
                probe: &SystemProbe,
                runner: &runner,
            })?
        }
    };
    let kept = materialised
        .iter()
        .chain(&skipped)
        .cloned()
        .collect::<Vec<_>>();
    let pruned = prune_stale_slots(&workspace.root, &kept)?;
    // This callback is deliberately HERE: every slot and pre-install effect
    // is now visible, while boot publication and the lock write have not run.
    let (world, lowering) = prepare(workspace, resolution)?;
    let (nodes_regenerated, carriage) =
        super::bootgen::regenerate_boot_from_traced_native_prepared(
            workspace,
            resolution,
            world,
            spec_format,
            trace,
            lowering,
            run,
            make_provider,
        )?;
    Ok(NativeInstallOutcome {
        outcome: InstallOutcome {
            materialised,
            skipped,
            integrity_warnings,
            pruned,
            nodes_regenerated,
            post_install_plan: PostInstallPlan::new(&workspace.root, post_install_deps),
            hook_reports,
        },
        carriage,
    })
}
