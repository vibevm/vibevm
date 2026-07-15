//! Per-unit boot emission — the hybrid linker's install-side half (PROP-038),
//! split from `bootgen.rs` per the file-length budget. `regenerate_boot_from`
//! (in `super`) builds the unit table, emits each package's own artifacts, and
//! appends the hoisted shared packages to the global root.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-038#units");

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use specmark::spec;
use vibe_core::Group;
use vibe_core::manifest::LinkType;

use crate::boot::hybrid::{self, UnitEdge, UnitId, UnitInput, ZoneMembership};
use crate::boot::{BootBand, BootEntry, EffectiveBoot};
use crate::{WorkspaceError, boot_artifacts, vibedeps};

use super::super::{ResolvedDep, io_err};

/// Build the per-unit table (PROP-038 §2.1) from the resolution: every
/// materialised package becomes a [`UnitInput`] whose edges carry the link
/// mode from **that package's own manifest** (§2.2) — the fix for the
/// shipped bootgen's root-only seeding. Per edge `X→Y` the mode resolves by
/// the same precedence [`crate::boot::compute_effective_boot`] uses: `X`'s
/// declared `link`, then `Y`'s `[boot_snippet]` suggestion, then `X`'s
/// `[boot].default_link`, then `dynamic`.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-038#units",
    r = 1
)]
pub(super) fn build_unit_table(resolution: &[ResolvedDep]) -> HashMap<UnitId, UnitInput> {
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
pub(super) fn emit_package_units(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    table: &HashMap<UnitId, UnitInput>,
    shared: &HashSet<UnitId>,
    fingerprints: &HashMap<UnitId, String>,
) -> Result<HashSet<UnitId>, WorkspaceError> {
    let slots: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), slot_rel_path(d)))
        .collect();
    let zones: HashMap<UnitId, ZoneMembership> = table
        .keys()
        .map(|id| (id.clone(), hybrid::resolve_zone(id, table)))
        .collect();
    // A package needs a STATIC.md when it statically links a child that is NOT
    // hoisted away — a zone whose every non-self static member is shared
    // (hoisted) reduces to #use markers, still worth emitting for the edges.
    let with_static: HashSet<UnitId> = zones
        .iter()
        .filter(|(id, zone)| has_static_children(id, zone, table))
        .map(|(id, _)| id.clone())
        .collect();

    for id in &with_static {
        let Some(slot) = slots.get(id) else { continue };
        let effective = zone_to_effective(id, &zones[id], table, &with_static, &slots, shared);
        let boot_dir = workspace_root.join(slot).join("spec").join("boot");
        let fp = fingerprints.get(id).map(String::as_str).unwrap_or("");
        emit_effective(&boot_dir, workspace_root, &effective, fp)?;
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
/// A `shared` (hoisted) static member becomes a `#use` marker (§2.5).
fn zone_to_effective(
    root_id: &UnitId,
    zone: &ZoneMembership,
    table: &HashMap<UnitId, UnitInput>,
    with_static: &HashSet<UnitId>,
    slots: &HashMap<UnitId, String>,
    shared: &HashSet<UnitId>,
) -> EffectiveBoot {
    let mut entries: Vec<BootEntry> = Vec::new();
    for member in hybrid::topo_zone(&zone.static_members, table) {
        let Some(unit) = table.get(&member) else {
            continue;
        };
        let Some(path) = &unit.own_boot_path else {
            continue; // a boot-less member threads the order but adds no text
        };
        // A shared member is hoisted to the global root STATIC.md; leave a
        // #use marker in place of its content (PROP-038 §2.5). A unit is never
        // hoisted out of its own zone (`root_id` owns the zone).
        let hoisted = &member != root_id && shared.contains(&member);
        entries.push(BootEntry {
            path: path.clone(),
            band: BootBand::Dependency,
            link: LinkType::Static,
            when: None,
            origin: unit.origin.clone(),
            use_ref: hoisted,
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
            use_ref: false,
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
    fingerprint: &str,
) -> Result<(), WorkspaceError> {
    let index = boot_dir.join(boot_artifacts::INDEX_FILE);
    // Dirty-subgraph skip (PROP-038 §2.8): if the existing INDEX carries the
    // same fingerprint, this unit's whole static zone is unchanged — skip both
    // writes. An unchanged install thus recompiles nothing and churns no git.
    let unchanged = fs::read_to_string(&index)
        .ok()
        .and_then(|existing| boot_artifacts::read_fingerprint(&existing))
        .as_deref()
        == Some(fingerprint);
    if unchanged {
        return Ok(());
    }
    fs::create_dir_all(boot_dir).map_err(|e| io_err(boot_dir, e))?;
    fs::write(
        &index,
        boot_artifacts::render_index(effective, Some(fingerprint))?,
    )
    .map_err(|e| io_err(&index, e))?;
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

/// Append the hoisted shared packages (PROP-038 §2.4) to the global root's
/// effective boot as compiled-in `static` entries in topological order — the
/// single copy every local zone references through a #use marker. Each entry's
/// provenance names the units that share it (the shared-by hint, §2.5). A
/// no-op when nothing is shared, so the root artifacts stay byte-identical on
/// a tree with no shared package (PROP-038 §5).
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-038#hoisting",
    r = 1
)]
pub(super) fn append_hoisted(
    effective: &mut EffectiveBoot,
    shared: &HashSet<UnitId>,
    table: &HashMap<UnitId, UnitInput>,
    pulls: &HashMap<UnitId, HashSet<UnitId>>,
) {
    if shared.is_empty() {
        return;
    }
    for id in hybrid::topo_zone(shared, table) {
        let Some(unit) = table.get(&id) else { continue };
        let Some(path) = &unit.own_boot_path else {
            continue;
        };
        effective.entries.push(BootEntry {
            path: path.clone(),
            band: BootBand::Dependency,
            link: LinkType::Static,
            when: None,
            origin: format!("{} [shared by {}]", unit.origin, shared_by(&id, pulls)),
            use_ref: false,
        });
    }
}

/// The sorted `<group>/<name>` list of units that soft-statically pull a
/// hoisted package — the shared-by hint (PROP-038 §2.5).
fn shared_by(id: &UnitId, pulls: &HashMap<UnitId, HashSet<UnitId>>) -> String {
    let mut names: Vec<String> = pulls
        .get(id)
        .map(|s| s.iter().map(|(g, n)| format!("{g}/{n}")).collect())
        .unwrap_or_default();
    names.sort();
    names.join(", ")
}
