//! Per-unit boot emission — the hybrid linker's install-side half (PROP-038),
//! split from `bootgen.rs` per the file-length budget. `regenerate_boot_from`
//! (in `super`) builds the unit table, emits each package's own artifacts, and
//! appends the hoisted shared packages to the global root.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#units");

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{LinkType, SpecFormat};
use vibe_core::{Group, PackageName, layout};
use vibe_extension_registry::DependencyProviderId;
use vibe_spec::TransformPlan;

use crate::boot::hybrid::{self, UnitEdge, UnitId, UnitInput, ZoneMembership};
use crate::boot::{BootBand, BootEntry, EffectiveBoot};
use crate::compile_trace::TraceRun;
use crate::extension_world::LoweredOwnerRuntimes;
use crate::{WorkspaceError, boot_artifacts, vibedeps};

use super::super::ResolvedDep;

/// One unit's trace occurrence and the fresh-output observation that decides
/// whether it is declared at all — split out so the observe-then-declare law
/// has one home and this file keeps its length budget.
#[path = "hybrid_emit/unit_trace.rs"]
mod unit_trace;
use unit_trace::UnitTrace;

/// Projecting a resolved zone into an [`EffectiveBoot`] — the pure half,
/// split out per the file-length budget.
#[path = "hybrid_emit/zone_projection.rs"]
mod zone_projection;
use zone_projection::{unit_provenance, zone_to_effective};

/// Build the per-unit table (PROP-038 §2.1) from the resolution: every
/// materialised package becomes a [`UnitInput`] whose edges carry the link
/// mode from **that package's own manifest** (§2.2) — the fix for the
/// shipped bootgen's root-only seeding. Per edge `X→Y` the mode resolves by
/// the same precedence [`crate::boot::compute_effective_boot`] uses: `X`'s
/// declared `link`, then `Y`'s `[boot_snippet]` suggestion, then `X`'s
/// `[boot].default_link`, then `dynamic`.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#units",
    r = 1
)]
pub(super) fn build_unit_table(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
) -> HashMap<UnitId, UnitInput> {
    let installed = super::installed_identities(resolution);
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
            // LOGICAL resolution against the materialised slot (PROP-045
            // ##BOOT-LANE-SCOPE): the manifest names the authored form; a
            // transforming materialisation may hold the other extension.
            let active = super::active_snippet(workspace_root, &slot, snippet, &installed);
            let (own_boot_path, when) = match active.main {
                Some(contribution) => (Some(contribution.path), contribution.when),
                None => (None, None),
            };
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
                    fragments: active.fragments,
                    origin: format!("{}/{}", dep.group, dep.name),
                    when,
                    edges,
                    // PROP-035 §3 — carry the package's format so a `normal`
                    // unit is compiled (not concatenated) when it enters a
                    // static zone (PROP-035 §8). Absent a `[package]`, `simple`.
                    format: dep
                        .manifest
                        .package
                        .as_ref()
                        .map(|p| p.format)
                        .unwrap_or_default(),
                },
            )
        })
        .collect()
}

/// The dependency-slot path (workspace-root-relative, forward-slashed) for a
/// resolved dependency — its versioned slot, or its unversioned in-place slot
/// (PROP-022 §2.4).
fn slot_rel_path(dep: &ResolvedDep) -> String {
    let in_place = dep
        .manifest
        .package
        .as_ref()
        .is_some_and(|p| p.materialization.is_in_place());
    if in_place {
        vibedeps::in_place_slot_rel_path(&dep.group, &dep.name)
    } else {
        vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)
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
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#units",
    r = 1
)]
// The borrowed recorder is one argument past the lint's threshold; the other
// seven are the emission inputs this pass already took before R3.4.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_package_units(
    workspace_root: &Path,
    self_coord: &vibe_spec::SelfCoordinate,
    resolution: &[ResolvedDep],
    table: &HashMap<UnitId, UnitInput>,
    shared: &HashSet<UnitId>,
    fingerprints: &HashMap<UnitId, String>,
    spec_format: SpecFormat,
    trace: Option<&TraceRun>,
    runtimes: &LoweredOwnerRuntimes,
) -> Result<HashSet<UnitId>, WorkspaceError> {
    let slots: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), slot_rel_path(d)))
        .collect();
    // The resolved version of each unit — the label half of a unit's trace
    // identity, read from the resolution exactly as the boot-graph
    // fingerprint's `versions` map is. `None` when there is no recorder: off
    // mode allocates no trace-only value at all, not even an empty map it
    // would never read.
    let versions: Option<HashMap<UnitId, String>> = trace.map(|_| {
        resolution
            .iter()
            .map(|d| ((d.group.clone(), d.name.clone()), d.version.to_string()))
            .collect()
    });
    // A package needs a STATIC.md when it statically links a child that is
    // NOT hoisted away — the one computation the write path and the R4.3
    // analyzer entry share, so a node lane's substituted entries are the
    // same set whether the units are being emitted or only analyzed.
    let with_static = with_static_set(table);

    // <!-- REVIEW: DRIFT-029 asked for this write to be suppressed, so a
    // materialised slot would carry no compiled boot artifacts. PROP-038 §2.1
    // `##UNIT-PER-PACKAGE` decides the opposite — "Every package materialised
    // under the dependency root carries its **own** boot artifacts" — so the task
    // stopped for an owner ruling rather than contradict the spec. Two facts
    // for whoever rules: `with_static` is computed above independently of this
    // write, so suppressing only the write leaves `bootgen.rs:305` pointing at
    // a STATIC.md that no longer exists (a hard `io_err` in `render_static`,
    // `boot_artifacts.rs:257`); and the host-side duplication the task reports
    // enters through the hoist counter, not through this loop —
    // `hoist::soft_static_pulls` counts package→package static pulls only,
    // never an entry-point node's own, so a member pulled by both the root and
    // an aggregator scores one puller and is never hoisted (§2.4). -->

    // ONE per-unit body, walked by whichever order the mode is entitled to.
    // Semantics are identical either way; only the sequence of units differs.
    let emit_unit = |id: &UnitId| -> Result<(), WorkspaceError> {
        let Some(slot) = slots.get(id) else {
            return Ok(());
        };
        let effective = zone_to_effective(
            id,
            &hybrid::resolve_zone(id, table),
            table,
            &with_static,
            &slots,
            shared,
            spec_format,
        );
        let boot_dir = workspace_root.join(slot).join(layout::current_boot_dir());
        let fp = fingerprints.get(id).map(String::as_str).unwrap_or("");
        // A unit whose zone has no static content declares no scope at all —
        // and with no recorder, not one trace-only string is built here.
        let unit_trace = trace
            .filter(|_| effective.static_entries().next().is_some())
            .map(|run| {
                UnitTrace::new(
                    run,
                    id,
                    versions
                        .as_ref()
                        .and_then(|versions| versions.get(id))
                        .map_or("", String::as_str),
                    spec_format,
                    slot,
                )
            });
        let owner = DependencyProviderId::new(
            id.0.clone(),
            PackageName::parse(&id.1).map_err(|error| WorkspaceError::UntypedBootProvenance {
                origin: format!("{}/{}", id.0, id.1),
                component: "unit package name",
                spelling: id.1.clone(),
                reason: error.to_string(),
            })?,
        );
        emit_effective(
            &boot_dir,
            workspace_root,
            self_coord,
            &effective,
            fp,
            spec_format,
            unit_trace.as_ref(),
            // THIS package's own plan, never the node's (PROP-054
            // ##COMPILE-ACTIVATION: activation authority follows the
            // artifact being written, and the artifact here is the
            // package's unit lane). It was lowered ONCE for this run,
            // before the fingerprints its digest feeds, and is read here
            // off the key it is filed under — never re-lowered, so one
            // declaration keeps one refusal surface.
            runtimes.unit(&owner)?.transform_plan().clone(),
        )
    };

    match trace {
        // A recorder present makes ORDER observable — scope ids, event
        // sequences and snapshot names would otherwise follow hash-table
        // construction. Only then are the units collected and sorted by their
        // canonical typed `(group, name)`, before the first descriptor, scope
        // or compile, so permuting the resolution permutes nothing a reader
        // sees. The returned set is unchanged: membership, not order, is its
        // contract.
        Some(_) => {
            let mut ordered: Vec<&UnitId> = with_static.iter().collect();
            ordered.sort();
            for id in ordered {
                emit_unit(id)?;
            }
        }
        // Off mode walks the set EXACTLY as it did before the trace existed —
        // no buffer, no sort, and therefore the historical compile/write order
        // and the historical first error on a tree with several bad units.
        // Sorting here would be a trace-only allocation changing untraced
        // behaviour, which is the one thing the traced siblings may not do.
        None => {
            for id in &with_static {
                emit_unit(id)?;
            }
        }
    }
    Ok(with_static)
}

/// The units whose own zone statically links a boot-bearing child beyond
/// the unit itself — the set every consumer's lane substitutes up to a
/// compiled per-unit STATIC (PROP-038 §2.1). Extracted from
/// [`emit_package_units`] so the write-free analyzer entry
/// (`bootgen/analyze.rs`) composes the SAME substituted lane without
/// emitting the units first.
pub(super) fn with_static_set(table: &HashMap<UnitId, UnitInput>) -> HashSet<UnitId> {
    table
        .keys()
        .map(|id| (id, hybrid::resolve_zone(id, table)))
        .filter(|(id, zone)| has_static_children(id, zone, table))
        .map(|(id, _)| id.clone())
        .collect()
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
        .any(|m| m != id && table.get(m).is_some_and(|u| u.has_static_boot()))
}

/// Write a unit's `INDEX.md` (always) and `STATIC.md` (when the zone has
/// static content) into `boot_dir`. Unlike [`boot_artifacts::write_boot_artifacts`]
/// this writes **no** redirect blocks — a dependency package slot is not an
/// agent entry point, so it carries no `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`.
///
/// Publication is transactional (R4.1 atom B; R4 architecture §7 — bare
/// byte writes are not an R4 publication path): a dirty unit renders its
/// INDEX and compiles its static lane IN FULL before the old artifact set is
/// touched, then publishes the whole triple through
/// [`boot_artifacts::publish_unit_artifacts`] — the same crash-recoverable
/// manager the node path uses — so a compile or backend refusal leaves the
/// unit's pre-existing INDEX/STATIC bytes byte-exact, and a crash
/// mid-publication rolls forward or back to one consistent set.
///
/// The trace law (R3.4), per unit that has a static artifact:
///
/// * exact fingerprint-fresh ⇒ the existing selected STATIC file is OBSERVED
///   first (no-follow/single-link), and only a successful observation declares
///   an occurrence, immediately `skipped` with the SAME output fingerprint
///   authority the prior dirty compile completed with: zero events, no
///   rewrite, mtime untouched. A refusal declares NOTHING — the already-proved
///   boot freshness stands, one bounded warning names why, and the run carries
///   no occurrence for a compile that never happened, so it can still finalise
///   `ok`;
/// * dirty ⇒ every fallible semantic byte is produced BEFORE the old artifact
///   set is touched: the INDEX is rendered here, and the occurrence is
///   acquired at the COMPILE boundary inside [`boot_artifacts`], completing
///   with the emitted-output fingerprint or failing with the original error.
///   No pre-compiler refusal here (an INDEX that cannot be rendered) can
///   leave a pending scope; a later transaction failure may legitimately
///   leave the occurrence `compiled` (the command owner finalises the run).
///
/// Nothing here changes the dirty-subgraph selection: the freshness check is
/// computed exactly as before, and an unchanged unit never enters the
/// transaction at all.
// The owner plan is one argument past the lint's threshold; the other seven
// are the emission inputs this pass already took before T10B.
#[allow(clippy::too_many_arguments)]
fn emit_effective(
    boot_dir: &Path,
    workspace_root: &Path,
    self_coord: &vibe_spec::SelfCoordinate,
    effective: &EffectiveBoot,
    fingerprint: &str,
    spec_format: SpecFormat,
    unit_trace: Option<&UnitTrace<'_>>,
    transforms: TransformPlan,
) -> Result<(), WorkspaceError> {
    let index = boot_dir.join(boot_artifacts::INDEX_FILE);
    let static_path = boot_dir.join(boot_artifacts::static_file(spec_format));
    let stale_path = boot_dir.join(if matches!(spec_format, SpecFormat::Xml) {
        boot_artifacts::STATIC_FILE
    } else {
        boot_artifacts::STATIC_XML_FILE
    });
    // Dirty-subgraph skip (PROP-038 §2.8): if the existing INDEX carries the
    // same fingerprint, this unit's whole static zone is unchanged — skip both
    // writes. An unchanged install thus recompiles nothing and churns no git.
    let unchanged = static_path.is_file()
        && !stale_path.exists()
        && fs::read_to_string(&index)
            .ok()
            .and_then(|existing| boot_artifacts::read_fingerprint(&existing))
            .as_deref()
            == Some(fingerprint);
    if unchanged {
        if let Some(unit_trace) = unit_trace {
            unit_trace.record_fresh_skip(workspace_root);
        }
        return Ok(());
    }
    // Dirty (R4.1 atom B): render/compile every fallible semantic byte BEFORE
    // touching the old artifact set. The INDEX render and the static compile
    // both precede publication, so any refusal here leaves the unit's
    // existing INDEX/STATIC bytes byte-exact — the pre-transaction code
    // published the INDEX before compiling and could not promise that.
    let index_text =
        boot_artifacts::render_index_with_spec_format(effective, Some(fingerprint), spec_format)?;
    let static_text = boot_artifacts::render_static_observed(
        effective,
        workspace_root,
        self_coord,
        spec_format,
        unit_trace.map(UnitTrace::acquisition),
        transforms,
    )?;
    // ONE crash-recoverable publication: INDEX, the selected STATIC's
    // presence/bytes, and the stale spelling's absence land together — or
    // not at all.
    boot_artifacts::publish_unit_artifacts(
        boot_dir,
        &index_text,
        static_text.as_deref(),
        spec_format,
    )
}

/// Append the hoisted shared packages (PROP-038 §2.4) to the global root's
/// effective boot as compiled-in `static` entries in topological order — the
/// single copy every local zone references through a #use marker. Each entry's
/// provenance names the units that share it (the shared-by hint, §2.5). A
/// no-op when nothing is shared, so the root artifacts stay byte-identical on
/// a tree with no shared package (PROP-038 §5).
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#hoisting",
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
        // The `[shared by …]` suffix is DISPLAY: it names who pulls the
        // single copy, not who declared it. The typed provenance is the
        // hoisted unit's own identity, unchanged by hoisting.
        let origin = format!("{} [shared by {}]", unit.origin, shared_by(&id, pulls));
        let provenance = unit_provenance(&id);
        let mut push = |path: &str| {
            effective.entries.push(BootEntry {
                path: path.to_string(),
                band: BootBand::Dependency,
                link: LinkType::Static,
                when: None,
                origin: origin.clone(),
                provenance: provenance.clone(),
                use_ref: false,
                format: unit.format,
                unit_substituted: false,
                elided: false,
            });
        };
        if unit.when.is_none()
            && let Some(path) = &unit.own_boot_path
        {
            push(path);
        }
        for fragment in &unit.fragments {
            if fragment.when.is_none() {
                push(&fragment.path);
            }
        }
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

/// Verify per-unit boot-graph integrity (PROP-038 §3) — the check half of the
/// dirty-subgraph. Recompute each unit that *should* carry a `STATIC.md` (it
/// statically links a child) and confirm the fingerprint recorded in its
/// on-disk `INDEX.md` matches a fresh recomputation. Returns the stale units
/// (missing artifact, or a mismatched fingerprint the regeneration should have
/// refreshed), sorted for a deterministic report.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#tests",
    r = 1
)]
pub(super) fn verify_fingerprints(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    table: &HashMap<UnitId, UnitInput>,
    fingerprints: &HashMap<UnitId, String>,
) -> Vec<UnitId> {
    let slots: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), slot_rel_path(d)))
        .collect();
    let mut stale: Vec<UnitId> = Vec::new();
    for id in table.keys() {
        let zone = hybrid::resolve_zone(id, table);
        if !has_static_children(id, &zone, table) {
            continue; // no per-unit STATIC.md is expected for this unit
        }
        let Some(slot) = slots.get(id) else { continue };
        let index = workspace_root.join(slot).join(layout::current_boot_index());
        let stored = fs::read_to_string(&index)
            .ok()
            .and_then(|t| boot_artifacts::read_fingerprint(&t));
        if stored.as_deref() != fingerprints.get(id).map(String::as_str) {
            stale.push(id.clone());
        }
    }
    stale.sort();
    stale
}
