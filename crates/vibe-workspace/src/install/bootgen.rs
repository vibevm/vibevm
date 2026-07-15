//! Boot-artifact (re)generation — the boot half of the loading model
//! (PROP-009 §2.7), split from `install.rs` per the file-length budget.
//! `apply_resolution` (in `super`) drives materialisation then calls
//! `regenerate_boot_from` here; `regenerate_boot` reconstructs the
//! resolution from the materialised `vibedeps/` tree for uninstall /
//! reinstall.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-009#install");

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;

use specmark::spec;
use vibe_core::Group;
use vibe_core::manifest::{BootCategory, LinkType, Manifest};

use crate::boot::hybrid::{UnitId, fingerprint, hoist};
use crate::boot::{self, AuthoredBoot, DependencyBoot, NodeBootInputs};
use crate::{Workspace, WorkspaceError, boot_artifacts, vibedeps};

use super::{ResolvedDep, io_err};

#[path = "bootgen/hybrid_emit.rs"]
mod hybrid_emit;
use hybrid_emit::{append_hoisted, build_unit_table, emit_package_units};

/// Regenerate every node's boot artifacts from a given `resolution` — the
/// boot half of [`apply_resolution`], without materialising. Returns the
/// `rel_path` of every node whose artifacts were written.
pub fn regenerate_boot_from(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
) -> Result<Vec<String>, WorkspaceError> {
    // The per-unit compiler (PROP-038 §2.1): emit each materialised package's
    // own STATIC.md / INDEX.md from its own edges, and learn which packages
    // statically link a child (`with_static`) — a node's dynamic edge to such
    // a package points at its compiled STATIC.md so the whole zone loads, not
    // just the snippet. For a tree with no intermediate static edge this is a
    // no-op, keeping the node artifacts byte-identical (PROP-038 §5).
    let table = build_unit_table(resolution);
    // Boot-graph fingerprints (PROP-038 §2.7) drive the dirty-subgraph skip in
    // per-unit emission (§2.8) — a package whose fingerprint is unchanged is
    // not recompiled. Keyed on each unit's resolved version.
    let versions: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), d.version.to_string()))
        .collect();
    let fps = fingerprint::fingerprints(&table, &versions);
    // Soft hoisting (PROP-038 §2.4): a package soft-statically linked by two or
    // more units is `shared` — hoisted to the global root STATIC.md and linked
    // once, its local zones left a #use marker. `pulls` also feeds the
    // shared-by hint. For a tree with no shared package this is all empty.
    let pulls = hoist::soft_static_pulls(&table);
    let shared: HashSet<UnitId> = pulls
        .iter()
        .filter(|(pkg, pullers)| {
            pullers.len() >= 2 && table.get(pkg).is_some_and(|u| u.own_boot_path.is_some())
        })
        .map(|(pkg, _)| pkg.clone())
        .collect();
    let with_static = emit_package_units(&workspace.root, resolution, &table, &shared, &fps)?;

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
        let deps = node_dependency_boot(manifest, resolution, &with_static);
        let mut effective = boot::compute_effective_boot(NodeBootInputs {
            own_boot: &own,
            inherited_foundation: &inherited,
            dependencies: &deps,
            default_link: manifest.boot.default_link,
        })?;
        // The absolute root is the hoist point: it carries the single copy of
        // every shared package (PROP-038 §2.4).
        if rel == "." {
            append_hoisted(&mut effective, &shared, &table, &pulls);
        }
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
                // Boot-only re-derivation from materialised slots — never
                // re-materialises, so the §2.6 mutable-source flag is moot.
                source_mutable: false,
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
        if name == boot_artifacts::STATIC_FILE || name == boot_artifacts::INDEX_FILE {
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
    with_static: &HashSet<UnitId>,
) -> Vec<DependencyBoot> {
    let index: HashMap<(&Group, &str), &ResolvedDep> = resolution
        .iter()
        .map(|d| ((&d.group, d.name.as_str()), d))
        .collect();

    // The inline-transitive closure (PROP-035 §12): every package reached
    // through a direct edge the consumer declared `inline-transitive` — the
    // edge's target and its whole `requires` closure — is forced `inline`.
    let forced_inline = static_transitive_closure(node_manifest, &index);

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
            // An in-place dependency's boot snippet lives in its unversioned
            // slot (PROP-022 §2.4); a snapshot/hardlink dep's in the versioned
            // one. Field access auto-derefs the `&&ResolvedDep`.
            let in_place = dep
                .manifest
                .package
                .as_ref()
                .is_some_and(|p| p.materialization.is_in_place());
            let slot = if in_place {
                vibedeps::in_place_slot_rel_path(dep.kind, &dep.name)
            } else {
                vibedeps::slot_rel_path(dep.kind, &dep.name, &dep.version)
            };
            let snippet = dep.manifest.boot_snippet.as_ref();
            // PROP-038 §2.1: a dependency that statically links a child is read
            // through its compiled STATIC.md (carrying the whole zone), not its
            // raw snippet. A leaf keeps pointing at the snippet (byte-compat).
            let boot_path = if with_static.contains(&(dep.group.clone(), dep.name.clone())) {
                Some(format!("{slot}/spec/boot/{}", boot_artifacts::STATIC_FILE))
            } else {
                snippet
                    .map(|bs| format!("{slot}/{}", bs.source.to_string_lossy().replace('\\', "/")))
            };
            DependencyBoot {
                kind: dep.kind,
                group: dep.group.clone(),
                name: dep.name.clone(),
                boot_path,
                category: snippet.and_then(|bs| bs.category),
                // An `inline-transitive` edge (or membership in one's closure)
                // forces `inline` (PROP-035 §12); otherwise only a direct
                // requirement carries a consumer-declared `link` and a
                // transitive dependency reads back as `None`.
                declared_link: if forced_inline.contains(&(dep.group.clone(), dep.name.clone())) {
                    Some(LinkType::Static)
                } else {
                    node_manifest.requires.declared_link(&dep.group, &dep.name)
                },
                suggested_link: snippet.and_then(|bs| bs.link),
                // The package's `[boot_snippet].when` OS gate, if any — it
                // forces the entry `dynamic` (PROP-009 §2.4).
                when: snippet.and_then(|bs| bs.when),
                requires: dep.requires.clone(),
            }
        })
        .collect()
}

/// The inline-transitive closure (PROP-035 §12): every `(group, name)`
/// reachable through a direct `[requires.packages]` edge the consumer
/// declared `inline-transitive` — the edge's target and its whole `requires`
/// closure. Membership forces the boot entry `inline`.
fn static_transitive_closure(
    node_manifest: &Manifest,
    index: &HashMap<(&Group, &str), &ResolvedDep>,
) -> HashSet<(Group, String)> {
    let mut queue: VecDeque<(Group, String)> = node_manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(g, n)| g.map(|g| (g.clone(), n.to_string())))
        .filter(|(g, n)| {
            node_manifest.requires.declared_link(g, n) == Some(LinkType::StaticTransitive)
        })
        .collect();
    let mut forced: HashSet<(Group, String)> = HashSet::new();
    while let Some((group, name)) = queue.pop_front() {
        if !forced.insert((group.clone(), name.clone())) {
            continue;
        }
        if let Some(dep) = index.get(&(&group, name.as_str())) {
            for (rg, rn) in &dep.requires {
                queue.push_back((rg.clone(), rn.clone()));
            }
        }
    }
    forced
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
pub(super) fn validate_redirect_blocks(workspace: &Workspace) -> Result<(), WorkspaceError> {
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
