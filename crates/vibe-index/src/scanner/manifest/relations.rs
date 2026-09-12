//! The dependency relations a manifest declares, projected onto the wire
//! (PROP-005 §3.2): compatibility, provides, requires, obsoletes,
//! conflicts and the feature table.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use super::package_kind;

use vibe_core::manifest::{
    Compatibility, ConflictsList, FeaturesTable, Obsoletes, Provides, Requires, RequiresAny,
};

use crate::types::{
    CompatibilityEntry, ConflictsEntry, FeaturesEntry, ObsoletesEntry, ProvidesEntry,
    RequiresAnyEntry, RequiresEntry,
};

/// Every projection builder normalises emptiness to absence: the
/// writer never emits a present-but-empty section (`"provides": {}`),
/// so an empty projection becomes `None` at the source rather than at
/// each call site.
pub fn compatibility_from(c: &Compatibility) -> Option<CompatibilityEntry> {
    let entry = CompatibilityEntry {
        min_vibe_version: c.min_vibe_version.clone(),
        requires_kinds: c.requires_kinds.iter().copied().map(package_kind).collect(),
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn provides_from(p: &Provides) -> Option<ProvidesEntry> {
    let entry = ProvidesEntry {
        capabilities: p.capabilities.iter().map(|c| c.to_string()).collect(),
    };
    (!entry.is_empty()).then_some(entry)
}

/// Flatten `[requires]` into the index entry's string lists. Registry
/// dependencies keep their `<group>/<name>@<constraint>` form; git / path
/// / `version.var` sources — which have no single constraint string —
/// degrade to the bare `<group>/<name>`. Both lists are sorted, so the
/// index is byte-deterministic.
pub fn requires_from(r: &Requires) -> Option<RequiresEntry> {
    let mut packages: Vec<String> = r.packages.iter().map(|p| p.to_string()).collect();
    for (group, name) in r
        .git_packages
        .iter()
        .map(|g| (&g.group, g.name.as_str()))
        .chain(r.path_packages.iter().map(|p| (&p.group, p.name.as_str())))
        .chain(r.var_packages.iter().map(|v| (&v.group, v.name.as_str())))
    {
        packages.push(format!("{group}/{name}"));
    }
    packages.sort();
    let mut capabilities: Vec<String> = r.capabilities.iter().map(|c| c.to_string()).collect();
    capabilities.sort();
    let entry = RequiresEntry {
        packages,
        capabilities,
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn requires_any_from(list: &[RequiresAny]) -> Vec<RequiresAnyEntry> {
    list.iter()
        .map(|ra| RequiresAnyEntry {
            one_of: ra.one_of.iter().map(|p| p.to_string()).collect(),
        })
        .collect()
}

pub fn obsoletes_from(o: &Obsoletes) -> Option<ObsoletesEntry> {
    let entry = ObsoletesEntry {
        packages: o.packages.iter().map(|p| p.to_string()).collect(),
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn conflicts_from(c: &ConflictsList) -> Option<ConflictsEntry> {
    let entry = ConflictsEntry {
        packages: c.packages.iter().map(|p| p.to_string()).collect(),
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn features_from(f: &FeaturesTable) -> Option<FeaturesEntry> {
    let entry = FeaturesEntry {
        features: f.features.clone(),
        exclusive: f.exclusive.clone(),
    };
    (!entry.is_empty()).then_some(entry)
}
