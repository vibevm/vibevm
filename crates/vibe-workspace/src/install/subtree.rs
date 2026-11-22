//! Scoped materialisation for `vibe update <pkg>`: the placement and
//! pre-install half of an apply, without pruning or boot regeneration.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use super::*;
use specmark::spec;

/// What [`materialise_subtree_with_spec_format_and_slot_lifecycle`] placed —
/// reporting plus one-shot post-install authority.
#[derive(Debug)]
pub struct SubtreeOutcome {
    pub materialised: Vec<String>,
    pub skipped: Vec<String>,
    pub integrity_warnings: Vec<String>,
    pub(super) post_install_plan: Option<PostInstallPlan>,
}

impl SubtreeOutcome {
    /// Take the post-install plan produced by this subtree update, at most once.
    pub fn take_post_install_plan(&mut self) -> Option<PostInstallPlan> {
        self.post_install_plan.take()
    }
}

/// Materialise a **partial** resolution — a scoped `vibe update <pkg>` subtree
/// — into dependency slots and raise the pre-install event of each
/// payload-changing slot, the same placement flow
/// [`apply_resolution_with_spec_format_and_slot_lifecycle_traced`] performs
/// (copy / hardlink / in-place move + rollback), but **without**
/// pruning unrelated slots or regenerating boot. A scoped update touches only
/// the named subtree, so the caller removes any superseded slots itself and
/// regenerates boot from the whole materialised tree afterwards; pruning here
/// would delete every slot outside the subtree.
///
/// Every slot change is recorded in `rollback`, the caller's undo log that
/// already holds its prune, so the caller can restore the previous tree when
/// a later step fails or a stop is acknowledged before its lock is published.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install")]
pub fn materialise_subtree_with_spec_format_and_slot_lifecycle(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: &dyn SlotLifecycle,
    rollback: &mut InstallRollback,
) -> Result<SubtreeOutcome, WorkspaceError> {
    let Materialised {
        materialised,
        skipped,
        integrity_warnings,
        post_install_deps,
    } = materialise_resolution_with_spec_format(
        workspace_root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format,
            slot_verifier,
            lifecycle: SlotLifecycleMode::Callback(lifecycle),
            progress: vibe_core::progress::Progress::default(),
            rollback: Some(rollback),
        },
    )?;
    Ok(SubtreeOutcome {
        materialised,
        skipped,
        integrity_warnings,
        post_install_plan: PostInstallPlan::new(workspace_root, post_install_deps),
    })
}
