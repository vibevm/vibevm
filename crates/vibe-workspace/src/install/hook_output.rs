//! Full-resolution apply with an explicit hook-subprocess stream policy.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use super::*;
use crate::hooks::ConfiguredHookRunner;

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
    match lifecycle {
        SlotLifecycleMode::None => apply_with_materialise_lifecycle(
            workspace,
            resolution,
            slot_integrity,
            spec_format,
            slot_verifier,
            MaterialiseLifecycle::None,
        ),
        SlotLifecycleMode::Callback(callback) => apply_with_materialise_lifecycle(
            workspace,
            resolution,
            slot_integrity,
            spec_format,
            slot_verifier,
            MaterialiseLifecycle::Callback(callback),
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
    let nodes_regenerated =
        regenerate_boot_from_with_spec_format(workspace, resolution, spec_format)?;

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
