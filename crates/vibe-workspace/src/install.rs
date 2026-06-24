//! Apply a resolution to the workspace — the install half of the loading
//! model (PROP-009 §2.7).
//!
//! [`apply_resolution`] takes a discovered [`Workspace`] and a resolved,
//! fetched dependency set, and:
//!
//! 1. materialises each resolved package into its `vibedeps/` slot
//!    ([`crate::vibedeps`]);
//! 2. computes every node's effective boot ([`crate::boot`]) and writes
//!    its boot artifacts ([`crate::boot_artifacts`]).
//!
//! It is decoupled from the depsolver and the registry: the caller —
//! workspace-aware `vibe install` — runs `Workspace::discover` and the
//! unified resolution, then hands the result here as [`ResolvedDep`]s.
//! This keeps the orchestration unit-testable without the registry stack,
//! the same decoupling [`crate::boot`] uses.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-009#install");

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{BootCategory, Manifest, Materialization};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageKind};

use crate::boot::{self, AuthoredBoot, DependencyBoot, NodeBootInputs};
use crate::{Workspace, WorkspaceError, boot_artifacts, vibedeps};

/// A resolved, fetched dependency ready to materialise — the minimum the
/// install orchestrator needs, decoupled from the registry's richer
/// `CachedPackage`.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    /// The package's `kind` — metadata; used only for its `vibedeps/` slot
    /// directory name, never for identity (PROP-008 §2.3).
    pub kind: PackageKind,
    /// Reverse-FQDN group — with `name`, the `(group, name)` identity.
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    /// On-disk directory holding the package's fetched content tree — the
    /// source `vibedeps` materialisation copies verbatim.
    pub content_dir: PathBuf,
    /// The package's parsed manifest (its `vibe.toml`) — read for the
    /// `[boot_snippet]` contribution.
    pub manifest: Manifest,
    /// `(group, name)` of every package this one directly requires — the
    /// edges of the dependency-boot topological order.
    pub requires: Vec<(Group, String)>,
}

/// What [`apply_resolution`] did — for the caller to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallOutcome {
    /// `vibedeps/` slot paths freshly materialised this run — a new or
    /// version-bumped dependency whose content was copied.
    pub materialised: Vec<String>,
    /// `vibedeps/` slot paths skipped — already present for the resolved
    /// version, trusted and not re-copied (PROP-011 §2.3). Empty when
    /// `slot_integrity` is `Verify`.
    pub skipped: Vec<String>,
    /// `vibedeps/` slot paths pruned — present before, absent from this
    /// resolution (a version bump, or a dropped dependency).
    pub pruned: Vec<String>,
    /// `rel_path` of every node whose boot artifacts were regenerated.
    pub nodes_regenerated: Vec<String>,
}

/// Materialise a resolution into the workspace and regenerate every node's
/// boot artifacts (PROP-009 §2.7).
///
/// Materialisation is workspace-wide — one `vibedeps/` slot per resolved
/// package at the absolute root. Boot artifacts are computed per node: the
/// root from the whole resolution, a member from its own `[requires]`
/// closure, with the absolute root's foundation boot inherited downward.
///
/// `slot_integrity` governs the PROP-011 §2.3 materialise-diff skip: with
/// [`SlotIntegrity::TrustPresence`] a slot already on disk for the
/// resolved version is trusted and not re-copied; with
/// [`SlotIntegrity::Verify`] every slot is re-materialised. `vibe install`
/// passes the user-config value; `vibe reinstall --force` passes `Verify`,
/// since its whole purpose is to overwrite slots from a fresh fetch.
pub fn apply_resolution(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
) -> Result<InstallOutcome, WorkspaceError> {
    // 0. Validate every node's `<vibevm>` instruction-file block before
    //    any mutation — a malformed block aborts here, not mid-install
    //    (PROP-012 §2.4).
    validate_redirect_blocks(workspace)?;

    // 1. Materialise the resolution into `vibedeps/`. PROP-011 §2.3 — a
    //    slot already present for the resolved (immutable) version is
    //    trusted and skipped; only a new or version-bumped dependency
    //    pays the recursive copy. `SlotIntegrity::Verify` opts out, so a
    //    hand-edited slot is overwritten.
    let mut materialised = Vec::new();
    let mut skipped = Vec::new();
    for dep in resolution {
        let slot = vibedeps::slot_rel_path(dep.kind, &dep.name, &dep.version);
        let present = vibedeps::is_materialised(&workspace.root, dep.kind, &dep.name, &dep.version);
        if present && slot_integrity == SlotIntegrity::TrustPresence {
            skipped.push(slot);
        } else {
            vibedeps::materialise_with(
                &workspace.root,
                dep.kind,
                &dep.name,
                &dep.version,
                &dep.content_dir,
                copy_mode_for(&dep.manifest),
            )?;
            materialised.push(slot);
        }
    }

    // 2. Prune any `vibedeps/` slot no longer in the resolution — a
    //    version bump or a dropped dependency must leave no orphan. Both
    //    the freshly-materialised and the skipped slots belong to the
    //    current resolution and are kept.
    let kept: Vec<String> = materialised.iter().chain(&skipped).cloned().collect();
    let pruned = prune_stale_slots(&workspace.root, &kept)?;

    // 3. Regenerate every node's boot artifacts from the resolution.
    let nodes_regenerated = regenerate_boot_from(workspace, resolution)?;

    Ok(InstallOutcome {
        materialised,
        skipped,
        pruned,
        nodes_regenerated,
    })
}

/// The slot placement mode for a resolved package (PROP-022 §2.1).
/// `hardlink` shares bytes with the cache by link; `snapshot` (the default)
/// is a full copy. `in-place` would clone directly into the slot, but that
/// pipeline (git + source URL in the install layer) is not yet wired — until
/// it lands, an `in-place` package falls back to a snapshot copy, which is
/// correct, just not yet optimised for giant repos.
fn copy_mode_for(manifest: &Manifest) -> vibedeps::CopyMode {
    match manifest.package.as_ref().map(|p| p.materialization) {
        Some(Materialization::Hardlink) => vibedeps::CopyMode::Hardlink,
        _ => vibedeps::CopyMode::Copy,
    }
}

/// Remove every `vibedeps/` slot whose path is not in `kept`, returning
/// the removed slot paths (sorted). A `<kind>-<name>` directory left with
/// no surviving version is removed too, so `vibedeps/` holds exactly the
/// current resolution and no empty husks.
fn prune_stale_slots(
    workspace_root: &Path,
    kept: &[String],
) -> Result<Vec<String>, WorkspaceError> {
    let vibedeps_dir = workspace_root.join(vibedeps::VIBEDEPS_DIR);
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
        let kn = kind_name.file_name().to_string_lossy().into_owned();
        let mut any_kept = false;
        for version in fs::read_dir(&kind_name_dir).map_err(|e| io_err(&kind_name_dir, e))? {
            let version = version.map_err(|e| io_err(&kind_name_dir, e))?;
            let version_dir = version.path();
            if !version_dir.is_dir() {
                continue;
            }
            let ver = version.file_name().to_string_lossy().into_owned();
            let rel = format!("{}/{kn}/{ver}", vibedeps::VIBEDEPS_DIR);
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

/// Regenerate every node's boot artifacts from a given `resolution` — the
/// boot half of [`apply_resolution`], without materialising. Returns the
/// `rel_path` of every node whose artifacts were written.
pub fn regenerate_boot_from(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
) -> Result<Vec<String>, WorkspaceError> {
    // The absolute root's foundation boot — inherited by every member
    // (PROP-009 §2.2: inherited foundation flows down).
    let root_foundation: Vec<AuthoredBoot> = node_own_boot(&workspace.root, ".")?
        .into_iter()
        .filter(|b| b.category == Some(BootCategory::Foundation))
        .collect();

    let mut nodes_regenerated = Vec::new();
    for (rel, manifest) in workspace.iter_nodes() {
        let node_dir = workspace.node_abs_path(rel);
        let own = node_own_boot(&node_dir, rel)?;
        let inherited: Vec<AuthoredBoot> = if rel == "." {
            Vec::new()
        } else {
            root_foundation.clone()
        };
        let deps = node_dependency_boot(manifest, resolution);
        let effective = boot::compute_effective_boot(NodeBootInputs {
            own_boot: &own,
            inherited_foundation: &inherited,
            dependencies: &deps,
            default_link: manifest.boot.default_link,
        })?;
        boot_artifacts::write_boot_artifacts(&node_dir, &workspace.root, &effective)?;
        nodes_regenerated.push(rel.to_string());
    }
    Ok(nodes_regenerated)
}

/// Regenerate every node's boot artifacts from the materialised `vibedeps/`
/// state already on disk — no fresh resolution, no re-materialisation.
///
/// Used by `vibe uninstall` (after a slot is removed) and, later, by
/// `vibe reinstall`. The resolution is reconstructed by reading each
/// `vibedeps/` slot's own manifest.
pub fn regenerate_boot(workspace: &Workspace) -> Result<Vec<String>, WorkspaceError> {
    // PROP-012 §2.4 — reject a malformed instruction-file block before
    // any boot-artifact write.
    validate_redirect_blocks(workspace)?;
    let resolution = read_materialised(&workspace.root)?;
    regenerate_boot_from(workspace, &resolution)
}

/// Reconstruct the resolution by reading every `vibedeps/` slot's manifest.
/// A slot whose `vibe.toml` is missing or carries no `[package]` table is
/// skipped — it is not a materialised package.
fn read_materialised(workspace_root: &Path) -> Result<Vec<ResolvedDep>, WorkspaceError> {
    let vibedeps_dir = workspace_root.join(vibedeps::VIBEDEPS_DIR);
    if !vibedeps_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for kind_name in fs::read_dir(&vibedeps_dir).map_err(|e| io_err(&vibedeps_dir, e))? {
        let kind_name = kind_name.map_err(|e| io_err(&vibedeps_dir, e))?;
        let kind_name_dir = kind_name.path();
        if !kind_name_dir.is_dir() {
            continue;
        }
        for version in fs::read_dir(&kind_name_dir).map_err(|e| io_err(&kind_name_dir, e))? {
            let version = version.map_err(|e| io_err(&kind_name_dir, e))?;
            let slot = version.path();
            let manifest_path = slot.join("vibe.toml");
            if !slot.is_dir() || !manifest_path.is_file() {
                continue;
            }
            let manifest =
                Manifest::read(&manifest_path).map_err(|e| WorkspaceError::Manifest {
                    path: manifest_path.clone(),
                    source: Box::new(e),
                })?;
            let Some(pkg) = &manifest.package else {
                continue;
            };
            // A `[requires.packages]` key is group-qualified at parse time
            // (PROP-008 §2.6), so every `iter_pkgrefs` entry carries a
            // group; a defensive `filter_map` drops any that somehow does
            // not rather than panicking.
            let requires: Vec<(Group, String)> = manifest
                .requires
                .iter_pkgrefs()
                .filter_map(|(g, n)| g.map(|g| (g.clone(), n.to_string())))
                .collect();
            out.push(ResolvedDep {
                kind: pkg.kind,
                group: pkg.group.clone(),
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                content_dir: slot.clone(),
                manifest: manifest.clone(),
                requires,
            });
        }
    }
    Ok(out)
}

/// Discover a node's own authored boot files — every `*.md` in its
/// `spec/boot/`, minus the generated `INLINE.md` / `INDEX.md`. The
/// user-owned `00-core.md` / `90-user.md` are `Foundation` / `UserOverride`
/// by name convention; any other authored file is mid-band (`None`).
///
/// `pub(crate)` so [`crate::publish`] can reuse it to regenerate a
/// staged copy's boot artifacts for the published shape (PROP-009 §2.11).
pub(crate) fn node_own_boot(
    node_dir: &Path,
    node_rel: &str,
) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
    let boot_dir = node_dir.join("spec").join("boot");
    if !boot_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&boot_dir).map_err(|e| io_err(&boot_dir, e))? {
        let entry = entry.map_err(|e| io_err(&boot_dir, e))?;
        let path = entry.path();
        if !entry.file_type().map_err(|e| io_err(&path, e))?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".md") {
            continue;
        }
        // The generated artifacts are not authored boot.
        if name == boot_artifacts::INLINE_FILE || name == boot_artifacts::INDEX_FILE {
            continue;
        }
        let category = match name.as_str() {
            "00-core.md" => Some(BootCategory::Foundation),
            "90-user.md" => Some(BootCategory::UserOverride),
            _ => None,
        };
        let rel_path = if node_rel == "." {
            format!("spec/boot/{name}")
        } else {
            format!("{node_rel}/spec/boot/{name}")
        };
        files.push(AuthoredBoot {
            path: rel_path,
            category,
            origin: node_rel.to_string(),
        });
    }
    // Deterministic order — the engine keeps a band's collection order.
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Build the dependency-boot inputs for one node: the transitive closure
/// of its `[requires]` within `resolution`, each turned into a
/// [`DependencyBoot`].
fn node_dependency_boot(
    node_manifest: &Manifest,
    resolution: &[ResolvedDep],
) -> Vec<DependencyBoot> {
    let index: HashMap<(&Group, &str), &ResolvedDep> = resolution
        .iter()
        .map(|d| ((&d.group, d.name.as_str()), d))
        .collect();

    // Breadth-first transitive closure from the node's direct requires.
    // A `[requires.packages]` key is group-qualified (PROP-008 §2.6), so
    // every `iter_pkgrefs` entry carries a group.
    let mut visited: HashSet<(Group, String)> = HashSet::new();
    let mut queue: VecDeque<(Group, String)> = node_manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(g, n)| g.map(|g| (g.clone(), n.to_string())))
        .collect();
    let mut closure: Vec<&ResolvedDep> = Vec::new();
    while let Some((group, name)) = queue.pop_front() {
        if !visited.insert((group.clone(), name.clone())) {
            continue;
        }
        if let Some(dep) = index.get(&(&group, name.as_str())) {
            closure.push(dep);
            for (rg, rn) in &dep.requires {
                queue.push_back((rg.clone(), rn.clone()));
            }
        }
    }

    closure
        .iter()
        .map(|dep| {
            let slot = vibedeps::slot_rel_path(dep.kind, &dep.name, &dep.version);
            let snippet = dep.manifest.boot_snippet.as_ref();
            let boot_path = snippet
                .map(|bs| format!("{slot}/{}", bs.source.to_string_lossy().replace('\\', "/")));
            DependencyBoot {
                kind: dep.kind,
                group: dep.group.clone(),
                name: dep.name.clone(),
                boot_path,
                category: snippet.and_then(|bs| bs.category),
                // Only a direct requirement carries a consumer-declared
                // `link`; a transitive dependency reads back as `None`.
                declared_link: node_manifest.requires.declared_link(&dep.group, &dep.name),
                suggested_link: snippet.and_then(|bs| bs.link),
                // The package's `[boot_snippet].when` OS gate, if any — it
                // forces the entry `dynamic` (PROP-009 §2.4).
                when: snippet.and_then(|bs| bs.when),
                requires: dep.requires.clone(),
            }
        })
        .collect()
}

/// Validate every node's agent instruction files before any mutation
/// (PROP-012 §2.4): a malformed `<vibevm>` block aborts the operation
/// here — ahead of materialisation or any boot-artifact write — so an
/// install never half-applies. A missing instruction file is fine; it is
/// created on write.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-012#plan-time",
    r = 1
)]
fn validate_redirect_blocks(workspace: &Workspace) -> Result<(), WorkspaceError> {
    for (rel, _) in workspace.iter_nodes() {
        let node_dir = workspace.node_abs_path(rel);
        for name in boot_artifacts::REDIRECT_FILES {
            let path = node_dir.join(name);
            let content = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(io_err(&path, e)),
            };
            if let boot_artifacts::BlockLocation::Malformed(reason) =
                boot_artifacts::locate_block(&content)
            {
                return Err(WorkspaceError::MalformedRedirectBlock { path, reason });
            }
        }
    }
    Ok(())
}

/// Build a [`WorkspaceError::Io`] from a `std::io::Error` and its path.
fn io_err(path: &Path, e: std::io::Error) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
#[path = "install/tests.rs"]
mod tests;
