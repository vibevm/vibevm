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

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use vibe_core::ContentHash;
use vibe_core::manifest::{Manifest, Materialization, SpecFormat};
use vibe_core::user_config::SlotIntegrity;

use crate::hooks::{
    HookContext, HookError, HookOutput, HookPhase, HookPolicy, HookReport, HookRunner,
    InterpreterProbe, Platform, SystemHookRunner, SystemProbe, run_package_hook,
};
use crate::{Workspace, WorkspaceError, layout_paths, vibedeps};

/// Stale-slot pruning — removing every dependency slot the current
/// resolution no longer names, out of line per the file-length budget.
mod prune;
use prune::prune_stale_slots;

mod hook_output;
mod hooks_run;
pub mod model;
mod slot_lifecycle;

mod bootgen;
/// B-006 (lane dedup) — de-substitute covered unit-STATIC entries. Public so
/// the once-each topology is exercisable at the unit level (no full install).
pub use bootgen::desubstitute_covered_units;
pub(crate) use bootgen::node_own_boot;
use bootgen::validate_redirect_blocks;
/// The boot-graph integrity check (PROP-038 §3) — public API for `vibe check`.
pub use model::{InstallOutcome, PostInstallPlan, ResolvedDep, SlotCheck, SlotVerifier};

pub use hook_output::{
    apply_resolution_with_spec_format_and_hook_output,
    apply_resolution_with_spec_format_and_slot_lifecycle,
    apply_resolution_with_spec_format_and_slot_lifecycle_traced,
};
use hooks_run::SubtreeOutcome;
pub use hooks_run::{
    run_post_install_hooks, run_post_install_hooks_with_output, run_post_install_slot_lifecycle,
};
use slot_lifecycle::{MaterialiseLifecycle, PreInstallPlan};
pub use slot_lifecycle::{
    SlotLifecycle, SlotLifecycleContext, SlotLifecycleMode, SlotLifecycleTarget,
};

pub use bootgen::verify_boot_graph;
/// The R4.3 lane analyzer's write-free entry (packages-2026-09 §9): one
/// selected node's lane composed and compiled under the analyzer
/// observer — the same composition regeneration runs, minus every write.
pub use bootgen::{AnalyzedLane, analyze_node_lane};
pub use bootgen::{
    regenerate_boot, regenerate_boot_from, regenerate_boot_from_traced,
    regenerate_boot_from_with_spec_format, regenerate_boot_traced,
    regenerate_boot_with_spec_format,
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

/// Internal materialisation, integrity, and hook-scheduling bookkeeping.
/// Public outcome reporting stays separate from hook eligibility.
#[derive(Debug)]
struct Materialised {
    materialised: Vec<String>,
    skipped: Vec<String>,
    integrity_warnings: Vec<String>,
    post_install_deps: Vec<ResolvedDep>,
    hook_reports: Vec<HookReport>,
}

struct MaterialiseOptions<'a> {
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&'a dyn SlotVerifier>,
    lifecycle: MaterialiseLifecycle<'a>,
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
        },
    )
}

/// Materialise a resolution and run `pre-install` after a nonempty payload
/// diff (PROP-009 §2.7, PROP-020 §2.1). Injectable seams cover hook execution
/// and the PROP-011 integrity check. Identity-current slots may be skipped;
/// verification divergence reconciles and warns. Reconciliation reporting is
/// independent: identity-only work is materialised but runs no hook. A failed
/// `pre-install` removes the offending slot and aborts (PROP-020 §2.5).
fn materialise_resolution_with_spec_format(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    options: MaterialiseOptions<'_>,
) -> Result<Materialised, WorkspaceError> {
    let MaterialiseOptions {
        slot_integrity,
        spec_format,
        slot_verifier,
        lifecycle,
    } = options;
    let mut materialised = Vec::new();
    let mut skipped = Vec::new();
    let mut integrity_warnings = Vec::new();
    let mut post_install_deps = Vec::new();
    let mut hook_reports = Vec::new();
    let mut pre_install = PreInstallPlan::new(&lifecycle, workspace_root);
    for dep in resolution {
        // PROP-022 §2.4 — an in-place package is a project-local git working
        // tree in an unversioned slot. Move the fetched clone (with its
        // `.git`) into the slot instead of the per-file `copy`, and
        // `.gitignore` it (not vendored, §2.7).
        if is_in_place(dep) {
            if spec_format.is_transformed() {
                return Err(WorkspaceError::SpecMaterialization {
                    path: dep.content_dir.clone(),
                    reason: "transformed spec formats are unavailable for in-place packages"
                        .to_string(),
                });
            }
            let rel = vibedeps::in_place_slot_rel_path(&dep.group, &dep.name);
            let slot_abs = vibedeps::in_place_slot_abs_path(workspace_root, &dep.group, &dep.name);
            // The install layer may have already placed the slot directly — an
            // incremental in-place update (PROP-022 §2.4), `git fetch`-ed onto
            // the existing `.git` rather than re-cloned — signalled by the
            // dep's `content_dir` BEING the slot. Then there is no clone to
            // move; the slot is already current and we only run the hook.
            let already_placed = dep.content_dir == slot_abs;
            if !already_placed
                && vibedeps::is_in_place_slot(workspace_root, &dep.group, &dep.name)
                && slot_integrity == SlotIntegrity::TrustPresence
            {
                skipped.push(rel);
                continue;
            }
            if !already_placed {
                required_source_hash(dep)?;
                vibedeps::materialise_in_place(
                    workspace_root,
                    &dep.group,
                    &dep.name,
                    &dep.content_dir,
                )?;
                vibedeps::ensure_gitignored(workspace_root, &rel)?;
            }
            let changed = if already_placed {
                dep.in_place_changed.unwrap_or(true)
            } else {
                true
            };
            if !changed {
                skipped.push(rel);
                continue;
            }
            // In-place keeps its git-native reset/eligibility semantics.
            post_install_deps.push(dep.clone());
            pre_install.run_or_defer(dep, &mut hook_reports)?;
            materialised.push(rel);
            continue;
        }
        let slot = vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version);
        let present =
            vibedeps::is_materialised(workspace_root, &dep.group, &dep.name, &dep.version);
        let slot_abs = vibedeps::slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version);
        // PROP-045: representation freshness precedes every presence-based
        // trust decision. A changed project/user setting always re-materialises.
        let format_current = present && vibedeps::format_is_current(&slot_abs, spec_format);
        // Mutable local `file://` sources (PROP-011 §2.6) cannot use version
        // presence as identity: the author may edit the source in place. The
        // fresh LocalRegistry fetch already put the current tree hash on the
        // resolution, so a valid record carrying that same hash earns
        // eligibility without another tree walk here. Missing, malformed, or
        // mismatched records fall through to reconciliation; its strict record
        // reader turns malformed state into a hard error instead of wiping it.
        let source_identity_current = present
            && (!dep.source_mutable
                || (format_current
                    && vibedeps::read_slot_record(&slot_abs).is_ok_and(|record| {
                        required_source_hash(dep).is_ok_and(|hash| record.source_hash == *hash)
                    })));

        // Every identity-current slot follows the configured integrity
        // strategy (§2.3/§5.2): `trust-presence` accepts it outright; `verify`
        // still spot-checks payload through the caller's seam, so local drift
        // heals even when the mutable source itself is unchanged. No verifier
        // (or an unverifiable slot) keeps the shipped rematerialise discipline.
        let trusted = source_identity_current
            && match slot_integrity {
                SlotIntegrity::TrustPresence => format_current,
                SlotIntegrity::Verify => match slot_verifier {
                    None => false,
                    Some(verifier) => {
                        match verifier.verify_slot_for_format(dep, &slot_abs, spec_format) {
                            SlotCheck::Verified => true,
                            SlotCheck::Diverged { expected, actual } => {
                                integrity_warnings.push(format!(
                                    "{}/{}@{}: vibedeps slot hashes {actual}, locked hash is \
                                     {expected} — re-materialising",
                                    dep.group, dep.name, dep.version
                                ));
                                false
                            }
                            SlotCheck::DivergedDetail { reason } => {
                                integrity_warnings.push(format!(
                                    "{}/{}@{}: {reason} — re-materialising",
                                    dep.group, dep.name, dep.version
                                ));
                                false
                            }
                            SlotCheck::Unverifiable => false,
                        }
                    }
                },
            };
        if trusted {
            skipped.push(slot);
            continue;
        }
        let source_hash = required_source_hash(dep)?;
        let materialise_report = vibedeps::materialise_with_spec_format_report(
            workspace_root,
            &dep.group,
            &dep.name,
            &dep.version,
            &dep.content_dir,
            copy_mode_for(&dep.manifest),
            spec_format,
            source_hash,
        )?;
        if materialise_report.payload_changed() {
            post_install_deps.push(dep.clone());
            pre_install.run_or_defer(dep, &mut hook_reports)?;
        }
        materialised.push(slot);
    }
    pre_install.dispatch(&materialised, &skipped)?;
    Ok(Materialised {
        materialised,
        skipped,
        integrity_warnings,
        post_install_deps,
        hook_reports,
    })
}

fn required_source_hash(dep: &ResolvedDep) -> Result<&ContentHash, WorkspaceError> {
    dep.source_hash
        .as_ref()
        .ok_or_else(|| WorkspaceError::SpecMaterialization {
            path: dep.content_dir.clone(),
            reason: format!(
                "materialisation of `{}/{}@{}` requires the fetched source_hash",
                dep.group, dep.name, dep.version
            ),
        })
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

/// The copy placement mode for a resolved **copy / hardlink** package
/// (PROP-022 §2.1). `hardlink` shares bytes with the cache by link; `copy`
/// (the default) is a full copy. An `in-place` package never reaches here — it
/// is handled by [`materialise_resolution`]'s move-into-slot branch before any
/// copy mode is chosen (PROP-022 §2.4).
fn copy_mode_for(manifest: &Manifest) -> vibedeps::CopyMode {
    match manifest.package.as_ref().map(|p| p.materialization) {
        Some(Materialization::Hardlink) => vibedeps::CopyMode::Hardlink,
        _ => vibedeps::CopyMode::Copy,
    }
}

/// `true` iff `dep` declares `in-place` materialization (PROP-022 §2.4) — the
/// git-native, unversioned, non-vendored slot. Read off the package manifest;
/// a node with no `[package]` table (never a resolved dependency) is not
/// in-place.
fn is_in_place(dep: &ResolvedDep) -> bool {
    dep.manifest
        .package
        .as_ref()
        .is_some_and(|p| p.materialization.is_in_place())
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
