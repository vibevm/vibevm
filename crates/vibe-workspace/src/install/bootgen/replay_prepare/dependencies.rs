//! Which already-emitted units one replay candidate depends on statically.
//!
//! Out of line along the dependency-closure seam: deciding what a lane
//! depends on is a different responsibility from sealing and ordering the
//! replay closure itself.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use std::collections::{BTreeSet, HashMap, HashSet};

use vibe_core::manifest::LinkType;

use crate::boot::hybrid::{UnitId, UnitInput};
use crate::boot::{BootProvenance, EffectiveBoot};

pub(crate) fn static_dependencies(
    id: &UnitId,
    table: &HashMap<UnitId, UnitInput>,
    emitted: &HashSet<UnitId>,
) -> Box<[UnitId]> {
    let mut dependencies = table
        .get(id)
        .into_iter()
        .flat_map(|unit| &unit.edges)
        .filter(|edge| {
            emitted.contains(&edge.target)
                && matches!(
                    edge.link,
                    LinkType::Static | LinkType::StaticTransitive | LinkType::StaticHard
                )
        })
        .map(|edge| edge.target.clone())
        .collect::<Vec<_>>();
    dependencies.sort();
    dependencies.dedup();
    dependencies.into_boxed_slice()
}

pub(crate) fn node_generated_dependencies(
    effective: &EffectiveBoot,
    emitted: &HashSet<UnitId>,
) -> Box<[UnitId]> {
    effective
        .static_entries()
        .filter(|entry| entry.unit_substituted)
        .filter_map(|entry| match &entry.provenance {
            BootProvenance::Dependency { group, name } => {
                let id = (group.clone(), name.clone());
                emitted.contains(&id).then_some(id)
            }
            BootProvenance::Node => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .into_boxed_slice()
}
