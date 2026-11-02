//! Scoped materialisation for `vibe update <pkg>`: the placement and
//! pre-install half of an apply, without pruning or boot regeneration.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use super::*;

/// Materialise a **partial** resolution — a scoped `vibe update <pkg>` subtree
/// — into dependency slots and run `pre-install` for each payload-changing slot
/// (PROP-020 §2.1), the same placement + hook flow [`apply_resolution`]
/// performs (copy / hardlink / in-place move + rollback), but
/// **without** pruning unrelated slots or regenerating boot. A scoped update
/// touches only the named subtree, so the caller removes any superseded slots
/// itself and regenerates boot from the whole materialised tree afterwards;
/// pruning here would delete every slot outside the subtree. Runs against the
/// production seams and with no [`SlotVerifier`], so `Verify` here keeps the
/// always-rematerialise discipline — the scoped update wants the fresh fetch
/// reconciled into the slot, not a hash-checked skip.
pub fn materialise_subtree(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    hooks: Option<&HookPolicy>,
) -> Result<SubtreeOutcome, WorkspaceError> {
    materialise_subtree_with_spec_format(
        workspace_root,
        resolution,
        slot_integrity,
        SpecFormat::Mixed,
        None,
        hooks,
    )
}

/// Format-aware scoped materialisation used by `vibe update`.
pub fn materialise_subtree_with_spec_format(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    hooks: Option<&HookPolicy>,
) -> Result<SubtreeOutcome, WorkspaceError> {
    let Materialised {
        materialised,
        skipped,
        integrity_warnings,
        post_install_deps,
        hook_reports,
    } = materialise_resolution_with_spec_format(
        workspace_root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format,
            slot_verifier,
            lifecycle: match hooks {
                Some(policy) => MaterialiseLifecycle::LegacyHooks {
                    policy,
                    probe: &SystemProbe,
                    runner: &SystemHookRunner,
                },
                None => MaterialiseLifecycle::None,
            },
            progress: vibe_core::progress::Progress::default(),
            rollback: None,
        },
    )?;
    Ok(SubtreeOutcome {
        materialised,
        skipped,
        integrity_warnings,
        post_install_plan: PostInstallPlan::new(workspace_root, post_install_deps),
        hook_reports,
    })
}

/// Format-aware scoped materialisation under exactly one slot-lifecycle
/// callback. Production update paths use this instead of the legacy hook
/// runner so `[hooks]` sugar and explicit slot contributions cannot double.
pub fn materialise_subtree_with_spec_format_and_slot_lifecycle(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: &dyn SlotLifecycle,
) -> Result<SubtreeOutcome, WorkspaceError> {
    let Materialised {
        materialised,
        skipped,
        integrity_warnings,
        post_install_deps,
        hook_reports,
    } = materialise_resolution_with_spec_format(
        workspace_root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format,
            slot_verifier,
            lifecycle: MaterialiseLifecycle::Callback(lifecycle),
            progress: vibe_core::progress::Progress::default(),
            rollback: None,
        },
    )?;
    Ok(SubtreeOutcome {
        materialised,
        skipped,
        integrity_warnings,
        post_install_plan: PostInstallPlan::new(workspace_root, post_install_deps),
        hook_reports,
    })
}
