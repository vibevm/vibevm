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

use crate::boot::hybrid::{UnitId, UnitInput, fingerprint, hoist, resolve_zone};
use crate::boot::{self, AuthoredBoot, BootEntry, DependencyBoot, EffectiveBoot, NodeBootInputs};
use crate::{Workspace, WorkspaceError, boot_artifacts, vibedeps};

use super::{ResolvedDep, io_err};

#[path = "bootgen/hybrid_emit.rs"]
mod hybrid_emit;
use hybrid_emit::{append_hoisted, build_unit_table, emit_package_units, verify_fingerprints};

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
        // <!-- REVIEW: DRIFT-030 §4 step 1 — this append is a SECOND write path
        // into the root's static lane. `compute_effective_boot` above has
        // already emitted a static entry for any package in the root's own
        // static closure (`node_dependency_boot` → `boot.rs:246-288`, path from
        // `bootgen.rs:305-310`), and `render_static` concatenates entry by
        // entry with no dedup on `path` (`boot_artifacts.rs:224-261`). So a
        // package that is both hoisted and in the root's closure lands twice.
        // Measured on a fixture of vibevm's shape (root --static-transitive-->
        // content-minimal aggregator --static--> member): counting the
        // entry-point node in `hoist::soft_static_pulls` does clear the
        // aggregator's copy — its zone degrades to the `#use` marker §2.5
        // designs — but the root then holds the member twice, once per path.
        // PROP-038 `##HOIST-LCA` explains why the collision is structural here:
        // the hoist target is the LCA of a *continuous static zone*, and for an
        // unbroken root→aggregator→member static chain that LCA IS the root,
        // i.e. the hoist destination and the root's own compile site coincide.
        // Which mechanism owns the dedup is a design question, so DRIFT-030
        // stopped on its §8 rather than paper over it at this call. -->
        if rel == "." {
            append_hoisted(&mut effective, &shared, &table, &pulls);
        }
        // B-006 (lane dedup): roll a substituted unit-STATIC entry back to the
        // package's own snippet — or elide a contentless umbrella — once every
        // boot-bearing member of its zone is present individually in this
        // lane. A pure pass over the composition and the unit table; coverage
        // is never lost (a member missing from the lane keeps the
        // substitution in place). Sits after `append_hoisted` so the hoisted
        // single-copies count as present, and before the artifact write so the
        // rendered lane is the once-each form.
        desubstitute_covered_units(&mut effective, &table);
        boot_artifacts::write_boot_artifacts(&node_dir, &workspace.root, &effective)?;
        nodes_regenerated.push(rel.to_string());
    }
    Ok(nodes_regenerated)
}

/// B-006 (lane dedup) — de-substitute the covered units in one node's
/// effective boot. A `static` entry whose path `node_dependency_boot`
/// rewrote to a compiled per-unit `STATIC.md` (`unit_substituted`) is rolled
/// back — to the package's own snippet when the package ships one, or elided
/// to a provenance stub when it is a contentless umbrella — IF every
/// boot-bearing member of that unit's static zone is already present in this
/// lane as an individual `static` entry. If even one is missing, the
/// substitution stays in place: a lane emits each package's text **once**,
/// but never at the cost of losing coverage (PROP-038 §2.1).
///
/// The decision is a pure function of the entry set and the unit table: a
/// snapshot of the entries is taken once, and every substituted entry is
/// decided against that snapshot — the order of entries and the count of
/// consumers (static / static-transitive / static-hard in any mix) do not
/// affect the verdict. One pass (not a fixpoint) suffices for the
/// nested-umbrella shapes the contract targets, because a contentless
/// umbrella is never a boot-bearing member of its parent's zone, so it
/// cannot gate its parent's elision.
///
/// `pub` (re-exported at [`crate::install::desubstitute_covered_units`]) so
/// the once-each topology can be exercised at the unit level without a full
/// install.
pub fn desubstitute_covered_units(
    effective: &mut EffectiveBoot,
    table: &HashMap<UnitId, UnitInput>,
) {
    // Identity by origin: a substituted entry's origin is exactly its
    // `<group>/<name>` pkgref (closure-walk entries always carry that form),
    // and so is the matching `UnitInput.origin`. Hoisted entries never carry
    // a unit-STATIC — `append_hoisted` only fires for a unit with an
    // `own_boot_path` — so a substituted origin never collides with a hoist.
    let by_origin: HashMap<&str, &UnitId> = table
        .iter()
        .map(|(id, u)| (u.origin.as_str(), id))
        .collect();
    // Decide every entry against the ORIGINAL snapshot. A rolled-back entry
    // does not retroactively count as "present" for a sibling decided later
    // in the same pass — the verdict depends only on the pre-pass set.
    let snapshot: Vec<BootEntry> = effective.entries.clone();

    for entry in &mut effective.entries {
        if !(entry.unit_substituted && entry.link == LinkType::Static) {
            continue;
        }
        let Some(id) = by_origin.get(entry.origin.as_str()).copied() else {
            continue;
        };
        let zone = resolve_zone(id, table);
        // The boot-bearing members of this unit's static zone, other than the
        // unit itself — members that actually carry boot content. A boot-less
        // umbrella threads the order but contributes no text, so it is never
        // required; that is why a single pass collapses nested umbrellas. The
        // unit itself is dropped here (its own snippet is handled by the
        // de-substitute / elide branches below), so a zone whose only
        // boot-bearing member is itself is vacuously covered. Collected by
        // value to keep the `present` lookups free of reference-level dance.
        let boot_bearing: Vec<UnitId> = zone
            .static_members
            .iter()
            .filter(|m| *m != id && table.get(*m).is_some_and(|u| u.own_boot_path.is_some()))
            .cloned()
            .collect();
        let covered = boot_bearing
            .iter()
            .all(|m| present(&snapshot, &table[m].origin));
        if !covered {
            continue;
        }
        match &table[id].own_boot_path {
            // The package ships its own snippet: point back at it (its text
            // then enters the lane once, through this entry).
            Some(snippet) => {
                entry.path = snippet.clone();
                entry.unit_substituted = false;
            }
            // A contentless umbrella: emit a stub, no second copy.
            None => entry.elided = true,
        }
    }
}

/// Whether the boot-bearing member `origin` (`<group>/<name>`) is present in
/// the pre-pass snapshot as an individual `static` entry — its own text, not
/// a unit-STATIC substitution and not a hoist `#use` marker. The hoisted
/// shared-by form `"<g>/<n> [shared by …]"` counts as present: that entry IS
/// the single copy of the member's text at the hoist point. A different
/// package `"<g>/<n2>"` does not match `"<g>/<n>"` — the `[` guard makes the
/// match prefix-exact on the pkgref.
fn present(snapshot: &[BootEntry], origin: &str) -> bool {
    snapshot.iter().any(|e| {
        e.link == LinkType::Static
            && !e.unit_substituted
            && !e.use_ref
            && (e.origin == origin || e.origin.starts_with(&format!("{origin} [")))
    })
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

/// Verify the per-unit boot artifacts are current (PROP-038 §3) — the integrity
/// half of `vibe check`, reconstructing the resolution from the materialised
/// `vibedeps/` tree. Returns the stale units: a package that statically links a
/// child but whose on-disk fingerprint is missing or mismatched, i.e. one the
/// dirty-subgraph should have regenerated. An empty result means the boot graph
/// is consistent.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-038#tests",
    r = 1
)]
pub fn verify_boot_graph(workspace: &Workspace) -> Result<Vec<UnitId>, WorkspaceError> {
    let resolution = read_materialised(&workspace.root)?;
    let table = build_unit_table(&resolution);
    let versions: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), d.version.to_string()))
        .collect();
    let fps = fingerprint::fingerprints(&table, &versions);
    Ok(verify_fingerprints(
        &workspace.root,
        &resolution,
        &table,
        &fps,
    ))
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
            // B-006: remember which entries had their path substituted up to a
            // unit-STATIC — `desubstitute_covered_units` rolls the substitution
            // back (or elides it) once the zone is covered member-by-member.
            let (boot_path, unit_substituted) =
                if with_static.contains(&(dep.group.clone(), dep.name.clone())) {
                    (
                        Some(format!("{slot}/spec/boot/{}", boot_artifacts::STATIC_FILE)),
                        true,
                    )
                } else {
                    (
                        snippet.map(|bs| {
                            format!("{slot}/{}", bs.source.to_string_lossy().replace('\\', "/"))
                        }),
                        false,
                    )
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
                // PROP-035 §3 — the package's declared format. A `normal`
                // dependency pulled `static` is compiled to its closure by
                // `render_static` (PROP-035 §8); absent a `[package]` table,
                // it defaults to `simple` (verbatim, fail-safe).
                format: dep
                    .manifest
                    .package
                    .as_ref()
                    .map(|p| p.format)
                    .unwrap_or_default(),
                unit_substituted,
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
