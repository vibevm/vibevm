//! B-006 (lane dedup) — the once-each pass over one node's composed lane.
//!
//! Split out of `bootgen.rs` along its own seam: a pure verdict over an
//! entry set and the unit table, with no I/O, no trace and no install state.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#units");

use std::collections::HashMap;

use vibe_core::manifest::LinkType;

use crate::boot::hybrid::{UnitId, UnitInput, resolve_zone};
use crate::boot::{BootEntry, EffectiveBoot};

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
            .filter(|m| *m != id && table.get(*m).is_some_and(|u| u.has_static_boot()))
            .cloned()
            .collect();
        let covered = boot_bearing
            .iter()
            .all(|m| present(&snapshot, &table[m].origin));
        if !covered {
            continue;
        }
        let unit = &table[id];
        if unit.static_boot_count() > 1 {
            // One BootEntry cannot represent several authored files. Keep the
            // compiled unit artifact so every fragment remains present once.
            continue;
        }
        match unit.single_static_boot_path() {
            Some(snippet) => {
                entry.path = snippet.to_string();
                entry.unit_substituted = false;
            }
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
