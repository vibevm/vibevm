//! Static-transitive boot closure for one consuming node.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#root");

use std::collections::{HashMap, HashSet, VecDeque};

use vibe_core::Group;
use vibe_core::manifest::{LinkType, Manifest};

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
