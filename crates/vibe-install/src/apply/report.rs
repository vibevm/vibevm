//! Public apply reports and retained workspace snapshot.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use super::*;

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
