//! Apply a resolution to the workspace — the install half of the loading
//! model (PROP-009 §2.7).
//!
//! [`apply_resolution`] takes a discovered [`Workspace`] and a resolved,
//! fetched dependency set, and:
//!
//! 1. materialises each resolved package into its dependency slot
//!    ([`crate::vibedeps`]);
//! 2. computes every node's effective boot ([`crate::boot`]) and writes
//!    its boot artifacts ([`crate::boot_artifacts`]).
//!
//! It is decoupled from the depsolver and the registry: the caller —
//! workspace-aware `vibe install` — runs `Workspace::discover` and the
//! unified resolution, then hands the result here as [`ResolvedDep`]s.
//! This keeps the orchestration unit-testable without the registry stack,
//! the same decoupling [`crate::boot`] uses.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use std::path::Path;

// The child test cells reach this through `use super::*`; the lib itself
// stopped needing it when the prune split moved its last production users
// out of this file.
#[cfg(test)]
use std::fs;

#[cfg(test)]
use vibe_core::manifest::Manifest;
use vibe_core::manifest::SpecFormat;
use vibe_core::user_config::SlotIntegrity;

use crate::hooks::{
    HookContext, HookError, HookOutput, HookPhase, HookPolicy, HookReport, HookRunner,
    InterpreterProbe, Platform, SystemHookRunner, SystemProbe, run_package_hook,
};
use crate::{Workspace, WorkspaceError, vibedeps};

/// Stale-slot pruning — removing every dependency slot the current
/// resolution no longer names, out of line per the file-length budget.
mod prune;
use prune::prune_stale_slots;

mod materialise;
mod placement;
use materialise::materialise_resolution_with_spec_format;
use placement::{copy_mode_for, is_in_place, required_source_hash};

mod hook_output;
mod hooks_run;
pub mod model;
mod slot_lifecycle;

pub(crate) mod bootgen;
/// B-006 (lane dedup) — de-substitute covered unit-STATIC entries. Public so
/// the once-each topology is exercisable at the unit level (no full install).
pub use bootgen::desubstitute_covered_units;
pub use bootgen::effective_subskills;
pub(crate) use bootgen::node_own_boot;
use bootgen::validate_redirect_blocks;
pub use hook_output::{
    apply_resolution_native_with_configuration, apply_resolution_with_configuration,
};
/// The boot-graph integrity check (PROP-038 §3) — public API for `vibe check`.
pub use model::{
    AppliedConversionCounts, AppliedConversionFailure, InstallOutcome, PostInstallPlan,
    ResolvedDep, SlotCheck, SlotVerifier,
};
use model::{MaterialiseOptions, Materialised};

pub use hook_output::{
    apply_resolution_with_spec_format_and_hook_output,
    apply_resolution_with_spec_format_and_slot_lifecycle,
    apply_resolution_with_spec_format_and_slot_lifecycle_traced,
    apply_resolution_with_spec_format_and_slot_lifecycle_traced_native,
    apply_resolution_with_spec_format_and_slot_lifecycle_traced_native_progress,
};
use hooks_run::SubtreeOutcome;
pub use hooks_run::{
    run_post_install_hooks, run_post_install_hooks_with_output, run_post_install_slot_lifecycle,
};
use slot_lifecycle::{MaterialiseLifecycle, PreInstallPlan};
pub use slot_lifecycle::{
    SlotLifecycle, SlotLifecycleContext, SlotLifecycleMode, SlotLifecycleTarget,
};

#[cfg(test)]
pub(crate) use bootgen::analyze_effective_bound_native;
pub use bootgen::verify_boot_graph;
/// The R4.3 lane analyzer's write-free entry (packages-2026-09 §9): one
/// selected node's lane composed and compiled under the analyzer
/// observer — the same composition regeneration runs, minus every write.
pub use bootgen::{
    AnalyzedBoundLane, AnalyzedLane, analyze_node_lane, analyze_node_lane_bound_native,
    compile_node_backend,
};
pub use bootgen::{
    BootRegeneration, regenerate_boot, regenerate_boot_from, regenerate_boot_from_traced,
    regenerate_boot_from_traced_prepared, regenerate_boot_from_with_spec_format,
    regenerate_boot_traced, regenerate_boot_with_spec_format,
};
mod native;
pub use native::{
    NativeInstallCarriage, NativeInstallOutcome, NativeInstallReplayError,
    regenerate_boot_from_traced_native,
};

/// Materialise a resolution into the workspace and regenerate every node's
/// boot artifacts (PROP-009 §2.7).
///
/// Materialisation is workspace-wide — one dependency slot per resolved
/// package at the absolute root. Boot artifacts are computed per node: the
/// root from the whole resolution, a member from its own `[requires]`
/// closure, with the absolute root's foundation boot inherited downward.
///
/// `slot_integrity` governs the PROP-011 §2.3 materialise-diff skip: with
/// [`SlotIntegrity::TrustPresence`] a slot already on disk for the
/// resolved version is trusted without a materialisation pass; with
/// [`SlotIntegrity::Verify`] a present slot is accepted only after its
/// `content_hash` checks out — see [`apply_resolution_with`], which takes
/// the verifying seam. This entry point carries no verifier, so `Verify`
/// here is the shipped always-rematerialise discipline — exactly what
/// `vibe reinstall --force` (reconcile slots from a fresh fetch) and
/// `vibe update` ask for.
pub fn apply_resolution(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    hooks: Option<&HookPolicy>,
) -> Result<InstallOutcome, WorkspaceError> {
    apply_resolution_with_spec_format(
        workspace,
        resolution,
        slot_integrity,
        SpecFormat::Mixed,
        None,
        hooks,
    )
}

/// The seam-injectable form of [`apply_resolution`]: `slot_verifier` is
/// consulted for an identity-current present slot under
/// [`SlotIntegrity::Verify`] (PROP-011 §2.3/§5.2) before the fast path trusts
/// it. A hash that matches the resolution's recorded
/// `content_hash` accepts the slot **without** materialising (the
/// always-rematerialise behaviour `verify` shipped with was stricter and
/// costlier than the contract: the spot-check replaces that pass, it does
/// not add to it); a divergence re-materialises the slot and records a
/// warn line naming the package and both hashes. `None` degrades `Verify`
/// to the shipped always-rematerialise discipline. A mutable in-workspace
/// `file://` source (§2.6) is identity-current only when its valid slot record
/// carries the freshly-fetched source hash; an `in-place` package
/// (PROP-022 §2.4) remains on its separate update path.
pub fn apply_resolution_with(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    slot_verifier: Option<&dyn SlotVerifier>,
    hooks: Option<&HookPolicy>,
) -> Result<InstallOutcome, WorkspaceError> {
    apply_resolution_with_spec_format(
        workspace,
        resolution,
        slot_integrity,
        SpecFormat::Mixed,
        slot_verifier,
        hooks,
    )
}

/// Apply a resolution with the effective PROP-045 spec representation.
/// Legacy entry points above remain source-identical by selecting `mixed`.
pub fn apply_resolution_with_spec_format(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&dyn SlotVerifier>,
    hooks: Option<&HookPolicy>,
) -> Result<InstallOutcome, WorkspaceError> {
    apply_resolution_with_spec_format_and_hook_output(
        workspace,
        resolution,
        slot_integrity,
        spec_format,
        slot_verifier,
        hooks,
        HookOutput::Inherit,
    )
}

#[cfg(test)]
fn materialise_resolution(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    slot_verifier: Option<&dyn SlotVerifier>,
    hooks: Option<&HookPolicy>,
    probe: &dyn InterpreterProbe,
    runner: &dyn HookRunner,
) -> Result<Materialised, WorkspaceError> {
    materialise_resolution_with_spec_format(
        workspace_root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format: SpecFormat::Mixed,
            slot_verifier,
            lifecycle: match hooks {
                Some(policy) => MaterialiseLifecycle::LegacyHooks {
                    policy,
                    probe,
                    runner,
                },
                None => MaterialiseLifecycle::None,
            },
            progress: vibe_core::progress::Progress::default(),
        },
    )
}

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
        conversion_counts: _,
        conversion_failures: _,
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
        conversion_counts: _,
        conversion_failures: _,
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

/// Build a [`WorkspaceError::Io`] from a `std::io::Error` and its path.
pub(super) fn io_err(path: &Path, e: std::io::Error) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
#[path = "install/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "install/tests_installed_fragments.rs"]
mod tests_installed_fragments;

#[cfg(test)]
#[path = "install/test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
#[path = "install/tests_hooks.rs"]
mod tests_hooks;

#[cfg(test)]
#[path = "install/tests_analyze_parity.rs"]
mod tests_analyze_parity;

#[cfg(test)]
#[path = "install/tests_epoch_world.rs"]
mod tests_epoch_world;

#[cfg(test)]
#[path = "install/tests_hybrid.rs"]
mod tests_hybrid;

#[cfg(test)]
#[path = "install/tests_minify_activation.rs"]
mod tests_minify_activation;

#[cfg(test)]
#[path = "install/tests_minify_units.rs"]
mod tests_minify_units;

#[cfg(test)]
#[path = "install/tests_mutable.rs"]
mod tests_mutable;

#[cfg(test)]
#[path = "install/tests_slot_lifecycle.rs"]
mod tests_slot_lifecycle;

#[cfg(test)]
#[path = "install/tests_trace.rs"]
mod tests_trace;
