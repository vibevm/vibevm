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
    HookContext, HookError, HookPhase, HookPolicy, HookReport, HookRunner, InterpreterProbe,
    Platform, SystemHookRunner, SystemProbe, run_package_hook,
};
use crate::{Workspace, WorkspaceError, layout_paths, vibedeps};

mod hooks_run;
pub mod model;

mod bootgen;
/// B-006 (lane dedup) — de-substitute covered unit-STATIC entries. Public so
/// the once-each topology is exercisable at the unit level (no full install).
pub use bootgen::desubstitute_covered_units;
pub(crate) use bootgen::node_own_boot;
use bootgen::validate_redirect_blocks;
/// The boot-graph integrity check (PROP-038 §3) — public API for `vibe check`.
pub use model::{InstallOutcome, ResolvedDep, SlotCheck, SlotVerifier};

use hooks_run::SubtreeOutcome;
use hooks_run::run_dep_hook;
pub use hooks_run::run_post_install_hooks;

pub use bootgen::verify_boot_graph;
pub use bootgen::{
    regenerate_boot, regenerate_boot_from, regenerate_boot_from_with_spec_format,
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
/// consulted — for a present, immutable slot, and only under
/// [`SlotIntegrity::Verify`] (PROP-011 §2.3/§5.2) — before the fast path
/// trusts the slot. A hash that matches the resolution's recorded
/// `content_hash` accepts the slot **without** materialising (the
/// always-rematerialise behaviour `verify` shipped with was stricter and
/// costlier than the contract: the spot-check replaces that pass, it does
/// not add to it); a divergence re-materialises the slot and records a
/// warn line naming the package and both hashes. `None` degrades `Verify`
/// to the shipped always-rematerialise discipline. Mutable in-workspace
/// `file://` sources (§2.6) and `in-place` packages (PROP-022 §2.4) never
/// reach the check — they re-materialise regardless of the setting.
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
    // 0. Validate every node's `<vibevm>` instruction-file block before
    //    any mutation — a malformed block aborts here, not mid-install
    //    (PROP-012 §2.4).
    validate_redirect_blocks(workspace)?;

    // 1. Materialise the resolution into dependency slots. PROP-011 §2.3 — a
    //    slot already present for the resolved (immutable) version is
    //    trusted and skipped; only a new or version-bumped dependency
    //    pays the recursive copy. Under `SlotIntegrity::Verify` that
    //    trust is earned per-slot through the `slot_verifier` seam: a
    //    hash match accepts the slot without the copy, a divergence
    //    re-materialises it (with a warn line).
    let Materialised {
        materialised,
        skipped,
        integrity_warnings,
        hook_reports,
    } = materialise_resolution_with_spec_format(
        &workspace.root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format,
            slot_verifier,
            hooks,
            probe: &SystemProbe,
            runner: &SystemHookRunner,
        },
    )?;

    // 2. Prune any dependency slot no longer in the resolution — a
    //    version bump or a dropped dependency must leave no orphan. Both
    //    the freshly-materialised and the skipped slots belong to the
    //    current resolution and are kept.
    let kept: Vec<String> = materialised.iter().chain(&skipped).cloned().collect();
    let pruned = prune_stale_slots(&workspace.root, &kept)?;

    // 3. Regenerate every node's boot artifacts from the resolution.
    let nodes_regenerated =
        regenerate_boot_from_with_spec_format(workspace, resolution, spec_format)?;

    Ok(InstallOutcome {
        materialised,
        skipped,
        integrity_warnings,
        pruned,
        nodes_regenerated,
        hook_reports,
    })
}

/// The slot bookkeeping [`apply_resolution`] needs back from the materialise
/// pass: which slots it wrote, which it trusted-and-skipped (PROP-011 §2.3),
/// the warn lines a `verify`-mode divergence produced, and the `pre-install`
/// hook reports it gathered (PROP-020 §2.1).
#[derive(Debug)]
struct Materialised {
    materialised: Vec<String>,
    skipped: Vec<String>,
    integrity_warnings: Vec<String>,
    hook_reports: Vec<HookReport>,
}

struct MaterialiseOptions<'a> {
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
    slot_verifier: Option<&'a dyn SlotVerifier>,
    hooks: Option<&'a HookPolicy>,
    probe: &'a dyn InterpreterProbe,
    runner: &'a dyn HookRunner,
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
            hooks,
            probe,
            runner,
        },
    )
}

/// Materialise a resolution into dependency slots and run each freshly-populated
/// slot's `pre-install` hook (PROP-009 §2.7, PROP-020 §2.1). The interpreter
/// `probe` and process `runner` are seams so the hook paths — run, skip, and
/// the pre-install-failure rollback — are unit-tested without spawning
/// processes.
///
/// PROP-011 §2.3: a slot already present for the resolved (immutable) version
/// is trusted and skipped under [`SlotIntegrity::TrustPresence`]; under
/// [`SlotIntegrity::Verify`] it is trusted only when the `slot_verifier`
/// seam confirms its `content_hash` (a divergence re-materialises it and
/// warns; no verifier keeps the always-rematerialise discipline). Only a new,
/// version-bumped, or untrusted dependency enters materialisation and
/// re-runs hooks (a skipped slot was never reset, so re-running its hook
/// would compound an earlier run, PROP-020 §2.1). A `pre-install` failure
/// removes the offending slot and aborts (PROP-020 §2.5).
fn materialise_resolution_with_spec_format(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    options: MaterialiseOptions<'_>,
) -> Result<Materialised, WorkspaceError> {
    let MaterialiseOptions {
        slot_integrity,
        spec_format,
        slot_verifier,
        hooks,
        probe,
        runner,
    } = options;
    let mut materialised = Vec::new();
    let mut skipped = Vec::new();
    let mut integrity_warnings = Vec::new();
    let mut hook_reports = Vec::new();
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
            // PROP-020 §2.1 — run the pre-install hook against the fresh
            // in-place working tree. The re-clone / incremental update IS the
            // §2.4 reset, so the hook stays a pure function of the upstream
            // content; a failure rolls the slot back (PROP-020 §2.5).
            if let Some(policy) = hooks {
                match run_dep_hook(
                    HookPhase::PreInstall,
                    dep,
                    workspace_root,
                    policy,
                    probe,
                    runner,
                ) {
                    Ok(Some(report)) => hook_reports.push(report),
                    Ok(None) => {}
                    Err(err) => {
                        let _ =
                            vibedeps::remove_in_place_slot(workspace_root, &dep.group, &dep.name);
                        return Err(WorkspaceError::from(err));
                    }
                }
            }
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
        // A mutable local `file://` source (PROP-011 §2.6) is never
        // presence-trusted: slot-present-for-a-version is not a proxy for
        // correctness when the source is a working tree edited in place, so it
        // falls through to re-materialise regardless of `slot_integrity`.
        //
        // An immutable present slot is trusted per the `slot_integrity`
        // strategy (§2.3/§5.2): `trust-presence` accepts it outright;
        // `verify` first spot-checks its `content_hash` through the
        // caller's `slot_verifier` seam — a matching hash accepts the
        // slot WITHOUT materialising (the always-rematerialise behaviour `verify`
        // shipped with was stricter and costlier than the contract), a
        // divergence re-materialises the slot and records a warn line,
        // and no verifier at all (or an unverifiable slot) keeps the
        // shipped always-rematerialise discipline.
        let trusted = present
            && !dep.source_mutable
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
        vibedeps::materialise_with_spec_format(
            workspace_root,
            &dep.group,
            &dep.name,
            &dep.version,
            &dep.content_dir,
            copy_mode_for(&dep.manifest),
            spec_format,
            source_hash,
        )?;
        if let Some(policy) = hooks {
            match run_dep_hook(
                HookPhase::PreInstall,
                dep,
                workspace_root,
                policy,
                probe,
                runner,
            ) {
                Ok(Some(report)) => hook_reports.push(report),
                Ok(None) => {}
                Err(err) => {
                    // PROP-020 §2.5 — preparation failed; vibevm never uses a
                    // half-prepared slot, so roll it back before surfacing.
                    let _ =
                        vibedeps::remove_slot(workspace_root, &dep.group, &dep.name, &dep.version);
                    return Err(WorkspaceError::from(err));
                }
            }
        }
        materialised.push(slot);
    }
    Ok(Materialised {
        materialised,
        skipped,
        integrity_warnings,
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
/// — into dependency slots and run each freshly-materialised slot's `pre-install`
/// hook (PROP-020 §2.1), the same placement + hook flow [`apply_resolution`]
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
        hook_reports,
    } = materialise_resolution_with_spec_format(
        workspace_root,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format,
            slot_verifier,
            hooks,
            probe: &SystemProbe,
            runner: &SystemHookRunner,
        },
    )?;
    Ok(SubtreeOutcome {
        materialised,
        skipped,
        integrity_warnings,
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

/// Remove every dependency slot whose path is not in `kept`, returning
/// the removed slot paths (sorted). A `<kind>-<name>` directory left with
/// no surviving version is removed too, so the dependency tree holds exactly the
/// current resolution and no empty husks.
fn prune_stale_slots(
    workspace_root: &Path,
    kept: &[String],
) -> Result<Vec<String>, WorkspaceError> {
    let vibedeps_dir = workspace_root.join(vibe_core::layout::current_vibedeps_root());
    if !vibedeps_dir.is_dir() {
        return Ok(Vec::new());
    }
    let keep: HashSet<&str> = kept.iter().map(String::as_str).collect();
    let mut pruned = Vec::new();
    for kind_name in fs::read_dir(&vibedeps_dir).map_err(|e| io_err(&vibedeps_dir, e))? {
        let kind_name = kind_name.map_err(|e| io_err(&vibedeps_dir, e))?;
        let kind_name_dir = kind_name.path();
        if !kind_name_dir.is_dir() {
            continue;
        }
        // An in-place slot is the `<kind>-<name>` dir itself — a git working
        // tree (PROP-022 §2.4), not a container of versioned slots. Skip it:
        // its lifecycle is the move-into-slot / destructive-guard path, never
        // version pruning.
        if kind_name_dir.join(".git").exists() {
            continue;
        }
        let kn = kind_name.file_name().to_string_lossy().into_owned();
        let mut any_kept = false;
        for version in fs::read_dir(&kind_name_dir).map_err(|e| io_err(&kind_name_dir, e))? {
            let version = version.map_err(|e| io_err(&kind_name_dir, e))?;
            let version_dir = version.path();
            if !version_dir.is_dir() {
                continue;
            }
            let ver = version.file_name().to_string_lossy().into_owned();
            let rel = layout_paths::vibedeps(format!("{kn}/{ver}"));
            if keep.contains(rel.as_str()) {
                any_kept = true;
            } else {
                fs::remove_dir_all(&version_dir).map_err(|e| io_err(&version_dir, e))?;
                pruned.push(rel);
            }
        }
        if !any_kept {
            let _ = fs::remove_dir(&kind_name_dir);
        }
    }
    pruned.sort();
    Ok(pruned)
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
#[path = "install/tests_hybrid.rs"]
mod tests_hybrid;
