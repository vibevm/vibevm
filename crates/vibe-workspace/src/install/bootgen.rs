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

use crate::boot::hybrid::{self, UnitEdge, UnitId, UnitInput, ZoneMembership};
use crate::boot::{
    self, AuthoredBoot, BootBand, BootEntry, DependencyBoot, EffectiveBoot, NodeBootInputs,
};
use crate::{Workspace, WorkspaceError, boot_artifacts, vibedeps};

use super::{ResolvedDep, io_err};

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
    let with_static = emit_package_units(&workspace.root, resolution, &table)?;

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

/// Build the per-unit table (PROP-038 §2.1) from the resolution: every
/// materialised package becomes a [`UnitInput`] whose edges carry the link
/// mode from **that package's own manifest** (§2.2) — the fix for the
/// shipped bootgen's root-only seeding. Per edge `X→Y` the mode resolves by
/// the same precedence [`boot::compute_effective_boot`] uses: `X`'s declared
/// `link`, then `Y`'s `[boot_snippet]` suggestion, then `X`'s
/// `[boot].default_link`, then `dynamic`.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-038#units",
    r = 1
)]
fn build_unit_table(resolution: &[ResolvedDep]) -> HashMap<UnitId, UnitInput> {
    // The target-suggestion precedence tier: a package's own `[boot_snippet].link`.
    let suggested: HashMap<(&Group, &str), Option<LinkType>> = resolution
        .iter()
        .map(|d| {
            (
                (&d.group, d.name.as_str()),
                d.manifest.boot_snippet.as_ref().and_then(|bs| bs.link),
            )
        })
        .collect();

    resolution
        .iter()
        .map(|dep| {
            let slot = slot_rel_path(dep);
            let snippet = dep.manifest.boot_snippet.as_ref();
            let own_boot_path = snippet
                .map(|bs| format!("{slot}/{}", bs.source.to_string_lossy().replace('\\', "/")));
            let default_link = dep.manifest.boot.default_link;
            let edges = dep
                .requires
                .iter()
                .map(|(rg, rn)| {
                    let link = dep
                        .manifest
                        .requires
                        .declared_link(rg, rn)
                        .or_else(|| suggested.get(&(rg, rn.as_str())).copied().flatten())
                        .or(default_link)
                        .unwrap_or_default();
                    UnitEdge {
                        target: (rg.clone(), rn.clone()),
                        link,
                    }
                })
                .collect();
            (
                (dep.group.clone(), dep.name.clone()),
                UnitInput {
                    own_boot_path,
                    origin: format!("{}/{}", dep.group, dep.name),
                    when: snippet.and_then(|bs| bs.when),
                    edges,
                },
            )
        })
        .collect()
}

/// The `vibedeps/` slot path (workspace-root-relative, forward-slashed) for a
/// resolved dependency — its versioned slot, or its unversioned in-place slot
/// (PROP-022 §2.4).
fn slot_rel_path(dep: &ResolvedDep) -> String {
    let in_place = dep
        .manifest
        .package
        .as_ref()
        .is_some_and(|p| p.materialization.is_in_place());
    if in_place {
        vibedeps::in_place_slot_rel_path(dep.kind, &dep.name)
    } else {
        vibedeps::slot_rel_path(dep.kind, &dep.name, &dep.version)
    }
}

/// Emit per-unit boot artifacts (PROP-038 §2.1) for every materialised package
/// that **statically links a child** — its `STATIC.md` compiles that child's
/// zone in (recursively, PROP-038 §2.2), its `INDEX.md` lists the dynamic
/// edges the zone surfaces (§5.5). A package with no static child needs none
/// (its snippet is read directly), so a tree where static reaches the lane
/// only through the root's `static-transitive` edge (today's vibevm) emits
/// nothing new — the migration-safety corollary (PROP-038 §5).
///
/// Returns the `(group, name)` set that received a `STATIC.md`, so a
/// consumer's dynamic edge to such a package points at the compiled
/// `STATIC.md` rather than the raw snippet (the parent then loads the whole
/// zone, not just the snippet).
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-038#units",
    r = 1
)]
fn emit_package_units(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    table: &HashMap<UnitId, UnitInput>,
) -> Result<HashSet<UnitId>, WorkspaceError> {
    let slots: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), slot_rel_path(d)))
        .collect();
    let zones: HashMap<UnitId, ZoneMembership> = table
        .keys()
        .map(|id| (id.clone(), hybrid::resolve_zone(id, table)))
        .collect();
    let with_static: HashSet<UnitId> = zones
        .iter()
        .filter(|(id, zone)| has_static_children(id, zone, table))
        .map(|(id, _)| id.clone())
        .collect();

    for id in &with_static {
        let Some(slot) = slots.get(id) else { continue };
        let effective = zone_to_effective(&zones[id], table, &with_static, &slots);
        let boot_dir = workspace_root.join(slot).join("spec").join("boot");
        emit_effective(&boot_dir, workspace_root, &effective)?;
    }
    Ok(with_static)
}

/// Whether a unit's static zone contains a compiled-in child beyond itself —
/// i.e. it statically links some other package that ships boot content. A
/// unit that only "contains itself" needs no `STATIC.md`; its snippet is the
/// whole of its static contribution.
fn has_static_children(
    id: &UnitId,
    zone: &ZoneMembership,
    table: &HashMap<UnitId, UnitInput>,
) -> bool {
    zone.static_members
        .iter()
        .any(|m| m != id && table.get(m).is_some_and(|u| u.own_boot_path.is_some()))
}

/// Project a resolved zone into an [`EffectiveBoot`] the existing
/// [`boot_artifacts`] renderers consume: static members in topological order
/// as `static` entries, the surfaced dynamic edges as `dynamic` entries. A
/// dynamic edge to a package that itself has a `STATIC.md` points at that
/// `STATIC.md` (so the parent loads the whole zone); otherwise at the snippet.
fn zone_to_effective(
    zone: &ZoneMembership,
    table: &HashMap<UnitId, UnitInput>,
    with_static: &HashSet<UnitId>,
    slots: &HashMap<UnitId, String>,
) -> EffectiveBoot {
    let mut entries: Vec<BootEntry> = Vec::new();
    for member in hybrid::topo_zone(&zone.static_members, table) {
        let Some(unit) = table.get(&member) else {
            continue;
        };
        let Some(path) = &unit.own_boot_path else {
            continue; // a boot-less member threads the order but adds no text
        };
        entries.push(BootEntry {
            path: path.clone(),
            band: BootBand::Dependency,
            link: LinkType::Static,
            when: None,
            origin: unit.origin.clone(),
        });
    }
    for (target, when) in &zone.dynamic_edges {
        let Some(path) = dynamic_target_path(target, with_static, slots, table) else {
            continue;
        };
        entries.push(BootEntry {
            path,
            band: BootBand::Dependency,
            link: LinkType::Dynamic,
            when: *when,
            origin: format!("{}/{}", target.0, target.1),
        });
    }
    EffectiveBoot { entries }
}

/// Where a dynamic edge's target is read from: its compiled `STATIC.md` when
/// the target statically links children (so reading it pulls the whole zone),
/// else its raw boot snippet.
fn dynamic_target_path(
    target: &UnitId,
    with_static: &HashSet<UnitId>,
    slots: &HashMap<UnitId, String>,
    table: &HashMap<UnitId, UnitInput>,
) -> Option<String> {
    if with_static.contains(target) {
        slots
            .get(target)
            .map(|slot| format!("{slot}/spec/boot/{}", boot_artifacts::STATIC_FILE))
    } else {
        table.get(target).and_then(|u| u.own_boot_path.clone())
    }
}

/// Write a unit's `INDEX.md` (always) and `STATIC.md` (when the zone has
/// static content) into `boot_dir`. Unlike [`boot_artifacts::write_boot_artifacts`]
/// this writes **no** redirect blocks — a `vibedeps/` package slot is not an
/// agent entry point, so it carries no `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`.
fn emit_effective(
    boot_dir: &Path,
    workspace_root: &Path,
    effective: &EffectiveBoot,
) -> Result<(), WorkspaceError> {
    fs::create_dir_all(boot_dir).map_err(|e| io_err(boot_dir, e))?;
    let index = boot_dir.join(boot_artifacts::INDEX_FILE);
    fs::write(&index, boot_artifacts::render_index(effective)?).map_err(|e| io_err(&index, e))?;
    let static_path = boot_dir.join(boot_artifacts::STATIC_FILE);
    match boot_artifacts::render_static(effective, workspace_root)? {
        Some(text) => fs::write(&static_path, text).map_err(|e| io_err(&static_path, e))?,
        None => {
            if static_path.exists() {
                fs::remove_file(&static_path).map_err(|e| io_err(&static_path, e))?;
            }
        }
    }
    Ok(())
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
