//! Projecting one resolved unit zone into the [`EffectiveBoot`] the existing
//! boot-artifact renderers consume — split out of the emission cell per the
//! file-length budget.
//!
//! The seam is real, not just a size cut: deciding WHAT a unit's lane
//! contains (which members compile in, which edges surface, where a hoisted
//! member leaves a marker) is a different job from writing, tracing and
//! publishing that lane's files. This half is pure.
//!
//! It is also where a per-unit entry's TYPED provenance is named, from the
//! unit table's own key — see [`unit_provenance`].

use std::collections::{HashMap, HashSet};
use std::path::Path;

use vibe_core::layout;
use vibe_core::manifest::{LinkType, SpecFormat, WhenCondition};

use crate::boot::hybrid::{self, UnitId, UnitInput, ZoneMembership};
use crate::boot::{BootBand, BootEntry, BootProvenance, EffectiveBoot};
use crate::{boot_artifacts, path_to_slash};

/// Project a resolved zone into an [`EffectiveBoot`] the existing
/// [`boot_artifacts`] renderers consume: static members in topological order
/// as `static` entries, the surfaced dynamic edges as `dynamic` entries. A
/// dynamic edge to a package that itself has a `STATIC.md` points at that
/// `STATIC.md` (so the parent loads the whole zone); otherwise at the snippet.
/// A `shared` (hoisted) static member becomes a `#use` marker (§2.5).
pub(super) fn zone_to_effective(
    root_id: &UnitId,
    zone: &ZoneMembership,
    table: &HashMap<UnitId, UnitInput>,
    with_static: &HashSet<UnitId>,
    slots: &HashMap<UnitId, String>,
    shared: &HashSet<UnitId>,
    spec_format: SpecFormat,
) -> EffectiveBoot {
    let mut entries: Vec<BootEntry> = Vec::new();
    for member in hybrid::topo_zone(&zone.static_members, table) {
        let Some(unit) = table.get(&member) else {
            continue;
        };
        // A shared member is hoisted to the global root STATIC.md; leave a
        // #use marker in place of its content (PROP-038 §2.5). A unit is never
        // hoisted out of its own zone (`root_id` owns the zone).
        let hoisted = &member != root_id && shared.contains(&member);
        let provenance = unit_provenance(&member);
        let mut push = |path: &str, when: Option<WhenCondition>| {
            let link = if when.is_some() {
                LinkType::Dynamic
            } else {
                LinkType::Static
            };
            entries.push(BootEntry {
                path: path.to_string(),
                band: BootBand::Dependency,
                link,
                when,
                origin: unit.origin.clone(),
                provenance: provenance.clone(),
                use_ref: hoisted && link == LinkType::Static,
                format: unit.format,
                unit_substituted: false,
                elided: false,
            });
        };
        if let Some(path) = &unit.own_boot_path {
            push(path, unit.when.clone());
        }
        for fragment in &unit.fragments {
            push(&fragment.path, fragment.when.clone());
        }
    }
    for (target, _) in &zone.dynamic_edges {
        let Some(unit) = table.get(target) else {
            continue;
        };
        let provenance = unit_provenance(target);
        let compiled = dynamic_target_path(target, with_static, slots, spec_format);
        if let Some(path) = compiled.as_ref() {
            entries.push(dynamic_entry(path, None, &unit.origin, &provenance));
        }
        if let Some(path) = &unit.own_boot_path
            && (compiled.is_none() || unit.when.is_some())
        {
            entries.push(dynamic_entry(
                path,
                unit.when.clone(),
                &unit.origin,
                &provenance,
            ));
        }
        for fragment in &unit.fragments {
            if compiled.is_none() || fragment.when.is_some() {
                entries.push(dynamic_entry(
                    &fragment.path,
                    fragment.when.clone(),
                    &unit.origin,
                    &provenance,
                ));
            }
        }
    }
    EffectiveBoot { entries }
}

/// The typed provenance of one unit, read off the unit TABLE KEY.
///
/// The key already IS the typed `(group, name)` pair, and `UnitInput::origin`
/// is `format!("{group}/{name}")` of those same two values, authored in one
/// expression in [`build_unit_table`]. So the typed half is carried beside
/// the display half exactly as [`BootProvenance`] requires — taken from the
/// identity, never recovered from the rendering — and a second copy stored on
/// `UnitInput` would only be one more thing that could disagree with the key
/// it is filed under.
pub(super) fn unit_provenance(id: &UnitId) -> BootProvenance {
    BootProvenance::Dependency {
        group: id.0.clone(),
        name: id.1.clone(),
    }
}

fn dynamic_entry(
    path: &str,
    when: Option<WhenCondition>,
    origin: &str,
    provenance: &BootProvenance,
) -> BootEntry {
    BootEntry {
        path: path.to_string(),
        band: BootBand::Dependency,
        link: LinkType::Dynamic,
        when,
        origin: origin.to_string(),
        provenance: provenance.clone(),
        use_ref: false,
        format: Default::default(),
        unit_substituted: false,
        elided: false,
    }
}

/// Where a dynamic edge's target is read from: its compiled `STATIC.md` when
/// the target statically links children (so reading it pulls the whole zone),
/// else its raw boot snippet.
fn dynamic_target_path(
    target: &UnitId,
    with_static: &HashSet<UnitId>,
    slots: &HashMap<UnitId, String>,
    spec_format: SpecFormat,
) -> Option<String> {
    if with_static.contains(target) {
        slots.get(target).map(|slot| {
            path_to_slash(
                &Path::new(slot)
                    .join(layout::current_boot_dir())
                    .join(boot_artifacts::static_file(spec_format)),
            )
        })
    } else {
        None
    }
}
