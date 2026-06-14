//! Capability resolution for the resolvo engine (PROP-017 §3).
//!
//! vibevm capabilities are virtual: a package `[provides]` a capability,
//! another `[requires.capabilities]` it. resolvo enumerates package
//! *versions* lazily, but the git-backed registry has no "who provides
//! capability X" reverse index, so resolvo cannot enumerate capability
//! *providers* the same way (PROP-017 §8 records the fuller reverse-index
//! evolution). The near-term bridge is a **pre-scan**: before the solve,
//! walk the transitive package closure from the roots, read every
//! reachable version's `[provides]`, and build a capability → providers
//! index. A `[requires.capabilities]` entry then becomes a resolvo `Union`
//! over the matching providers — pulling one into the graph, which the
//! naive cell (matching only against already-processed packages) cannot.
//! A post-solve check yields the clean `CapabilityUnmet` verdict for any
//! selected package whose requirement is genuinely unprovided.

use std::collections::{HashMap, HashSet, VecDeque};

use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};

use crate::naive::capability_version_for_provider;
use crate::{SolveError, VersionEnumerator};

/// One `(package, version)` that provides a capability, tagged with the
/// capability version it advertises.
#[derive(Clone)]
pub(crate) struct CapProvider {
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    pub provided_version: semver::Version,
}

/// `capability (qualified) → the package versions that provide it`.
pub(crate) type CapIndex = HashMap<String, Vec<CapProvider>>;

/// Walk the package closure reachable from `roots` (over `[requires.packages]`
/// and `[[requires_any]]` edges, across every available version) and index
/// every `[provides]` capability. Per-package errors are skipped — the
/// main solve surfaces anything genuinely required. This is the eager
/// step the lazy package path otherwise avoids (PROP-017 §8).
pub(crate) fn prescan<P: VersionEnumerator>(provider: &P, roots: &[PackageRef]) -> CapIndex {
    let mut index: CapIndex = HashMap::new();
    let mut seen: HashSet<(Group, String)> = HashSet::new();
    let mut queue: VecDeque<(Group, String)> = VecDeque::new();
    for r in roots {
        if let Some(g) = r.group.clone() {
            queue.push_back((g, r.name.to_string()));
        }
    }
    while let Some((group, name)) = queue.pop_front() {
        if !seen.insert((group.clone(), name.clone())) {
            continue;
        }
        let Ok(versions) = provider.list_versions(&group, &name) else {
            continue;
        };
        for v in versions {
            let Ok(manifest) = provider.fetch_manifest(&group, &name, &v) else {
                continue;
            };
            for cap in &manifest.provides.capabilities {
                index.entry(cap.qualified()).or_default().push(CapProvider {
                    group: group.clone(),
                    name: name.clone(),
                    version: v.clone(),
                    provided_version: capability_version_for_provider(cap, &v),
                });
            }
            for d in &manifest.requires.packages {
                if let Some(g) = d.group.clone() {
                    queue.push_back((g, d.name.to_string()));
                }
            }
            for disj in &manifest.requires_any {
                for alt in &disj.one_of {
                    if let Some(g) = alt.group.clone() {
                        queue.push_back((g, alt.name.to_string()));
                    }
                }
            }
        }
    }
    index
}

/// After the solve, verify every selected package's `[requires.capabilities]`
/// is met by some package in the selected set — the clean `CapabilityUnmet`
/// verdict (the naive cell's check, applied to the resolvo result). Each
/// entry is `(qualified-name, chosen-version, manifest)`.
pub(crate) fn verify(selected: &[(String, semver::Version, Manifest)]) -> Result<(), SolveError> {
    let mut providers: HashMap<String, Vec<semver::Version>> = HashMap::new();
    for (_, version, manifest) in selected {
        for cap in &manifest.provides.capabilities {
            providers
                .entry(cap.qualified())
                .or_default()
                .push(capability_version_for_provider(cap, version));
        }
    }
    for (qualified_name, _, manifest) in selected {
        for cap_req in &manifest.requires.capabilities {
            let met = providers
                .get(&cap_req.qualified())
                .map(|pvs| pvs.iter().any(|pv| cap_req.version.matches(pv)))
                .unwrap_or(false);
            if !met {
                return Err(SolveError::CapabilityUnmet {
                    capability: cap_req.to_string(),
                    requirer: qualified_name.clone(),
                });
            }
        }
    }
    Ok(())
}
