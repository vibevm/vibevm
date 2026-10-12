//! Static-transitive boot closure for one consuming node.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#root");

use std::collections::{HashMap, HashSet, VecDeque};

use vibe_core::Group;
use vibe_core::manifest::{LinkType, Manifest};

use crate::boot::hybrid::{UnitId, UnitInput, resolve_zone};

use super::super::ResolvedDep;

/// Every `(group, name)` reachable through a direct `static-transitive` edge:
/// the edge target and its complete `requires` closure (PROP-035 §12).
pub(super) fn static_transitive_closure(
    node_manifest: &Manifest,
    index: &HashMap<(&Group, &str), &ResolvedDep>,
) -> HashSet<(Group, String)> {
    let mut queue: VecDeque<(Group, String)> = node_manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(group, name)| group.map(|group| (group.clone(), name.to_string())))
        .filter(|(group, name)| {
            node_manifest.requires.declared_link(group, name) == Some(LinkType::StaticTransitive)
        })
        .collect();
    let mut forced = HashSet::new();
    while let Some((group, name)) = queue.pop_front() {
        if !forced.insert((group.clone(), name.clone())) {
            continue;
        }
        if let Some(dep) = index.get(&(&group, name.as_str())) {
            for (required_group, required_name) in &dep.requires {
                queue.push_back((required_group.clone(), required_name.clone()));
            }
        }
    }
    forced
}

/// Static consumer edges to compiled units need their authored zone members
/// as separate inputs. A unit STATIC is a generated multi-document lane, not
/// a simple source document that can be parsed as one contribution.
pub(super) fn static_unit_members(
    node_manifest: &Manifest,
    resolution: &[ResolvedDep],
    table: &HashMap<UnitId, UnitInput>,
    with_static: &HashSet<UnitId>,
) -> HashSet<UnitId> {
    let mut members = HashSet::new();
    for (group, name) in node_manifest.requires.iter_pkgrefs() {
        let Some(group) = group else { continue };
        let id = (group.clone(), name.to_string());
        let selected = resolution
            .iter()
            .find(|dep| dep.group == *group && dep.name == name);
        let link = node_manifest
            .requires
            .declared_link(group, name)
            .or_else(|| selected.and_then(|dep| dep.manifest.boot_snippet.as_ref()?.link))
            .or(node_manifest.boot.default_link)
            .unwrap_or_default();
        if !with_static.contains(&id)
            || table.get(&id).is_some_and(|unit| unit.when.is_some())
            || !matches!(
                link,
                LinkType::Static | LinkType::StaticHard | LinkType::StaticTransitive
            )
        {
            continue;
        }
        members.extend(resolve_zone(&id, table).static_members);
    }
    members
}
