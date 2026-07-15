//! Boot-graph fingerprints — the Merkle hash that drives dirty-subgraph
//! regeneration (PROP-038 §2.7).
//!
//! A unit's fingerprint hashes: its own boot identity (path + resolved
//! version), each edge's link mode and target, the **fingerprints** of its
//! static children (so any change inside the static zone — content, version,
//! edge set, or a link-type switch — flips it up the static chain to the first
//! dynamic break), and the **identities** (not fingerprints) of its dynamic
//! edges (so a change *behind* a dynamic edge does not flip it — the dynamic
//! boundary breaks propagation). A `when`-gated target is treated as dynamic
//! for propagation, matching [`super::resolve_zone`].

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-038#change-detection");

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};
use vibe_core::manifest::LinkType;

use super::{UnitId, UnitInput};

/// Compute the fingerprint of every unit (PROP-038 §2.7), bottom-up with
/// memoisation. `versions` gives each unit's resolved version — the content
/// identity for an immutable package (a mutable in-workspace source is
/// re-materialised regardless, PROP-011 §2.6, so the version key suffices).
///
/// ```
/// use std::collections::HashMap;
/// use vibe_workspace::boot::hybrid::{UnitId, UnitInput};
/// use vibe_workspace::boot::hybrid::fingerprint::fingerprints;
/// use vibe_core::Group;
///
/// let g = Group::parse("org.vibevm").unwrap();
/// let id: UnitId = (g, "a".to_string());
/// let mut table = HashMap::new();
/// table.insert(id.clone(), UnitInput {
///     own_boot_path: Some("a.md".to_string()),
///     origin: String::new(),
///     when: None,
///     edges: vec![],
/// });
/// let versions: HashMap<UnitId, String> = [(id.clone(), "1.0.0".to_string())].into_iter().collect();
/// let fps = fingerprints(&table, &versions);
/// assert!(fps.contains_key(&id));
/// ```
pub fn fingerprints(
    table: &HashMap<UnitId, UnitInput>,
    versions: &HashMap<UnitId, String>,
) -> HashMap<UnitId, String> {
    let mut memo: HashMap<UnitId, String> = HashMap::new();
    for id in table.keys() {
        compute(id, table, versions, &mut memo, &mut HashSet::new());
    }
    memo
}

/// One unit's fingerprint, recursing into static children (Merkle). `on_stack`
/// guards a cycle: a static cycle is rejected at generate time (PROP-034 §2.3),
/// so here it degrades to a stable marker rather than looping forever.
fn compute(
    id: &UnitId,
    table: &HashMap<UnitId, UnitInput>,
    versions: &HashMap<UnitId, String>,
    memo: &mut HashMap<UnitId, String>,
    on_stack: &mut HashSet<UnitId>,
) -> String {
    if let Some(fp) = memo.get(id) {
        return fp.clone();
    }
    if !on_stack.insert(id.clone()) {
        return "cycle".to_string();
    }
    let mut hasher = Sha256::new();
    hasher.update(b"unit:");
    hasher.update(pkgref(id).as_bytes());
    if let Some(unit) = table.get(id) {
        hasher.update(b"\nown:");
        hasher.update(unit.own_boot_path.as_deref().unwrap_or("-").as_bytes());
        hasher.update(b"@");
        hasher.update(version_of(id, versions));
        hasher.update(b"\ngated:");
        hasher.update(if unit.when.is_some() { b"1" } else { b"0" });

        // Sort edges deterministically so the fingerprint is stable.
        let mut edges = unit.edges.clone();
        edges.sort_by(|a, b| pkgref(&a.target).cmp(&pkgref(&b.target)));
        for edge in &edges {
            hasher.update(b"\nedge:");
            hasher.update(link_wire(edge.link).as_bytes());
            hasher.update(b"->");
            hasher.update(pkgref(&edge.target).as_bytes());
            // A static edge to a non-gated target propagates the child's
            // fingerprint (Merkle); a dynamic edge — or a gated target —
            // contributes the target's identity only, breaking propagation.
            let gated = table.get(&edge.target).and_then(|u| u.when).is_some();
            let is_static = matches!(
                edge.link,
                LinkType::Static | LinkType::StaticTransitive | LinkType::StaticHard
            );
            if is_static && !gated {
                let child = compute(&edge.target, table, versions, memo, on_stack);
                hasher.update(b" static-fp:");
                hasher.update(child.as_bytes());
            } else {
                hasher.update(b" dyn-id:");
                hasher.update(version_of(&edge.target, versions));
            }
        }
    }
    on_stack.remove(id);
    let fp = hex(&hasher.finalize());
    memo.insert(id.clone(), fp.clone());
    fp
}

/// A unit's `<group>/<name>` pkgref — the stable ordering and identity key.
fn pkgref(id: &UnitId) -> String {
    format!("{}/{}", id.0, id.1)
}

/// The resolved version bytes for a unit, or `-` when unknown.
fn version_of<'a>(id: &UnitId, versions: &'a HashMap<UnitId, String>) -> &'a [u8] {
    versions.get(id).map(String::as_bytes).unwrap_or(b"-")
}

/// The wire spelling of a link mode — part of the fingerprint so a
/// dynamic↔static switch flips it.
fn link_wire(link: LinkType) -> &'static str {
    match link {
        LinkType::Static => "static",
        LinkType::Dynamic => "dynamic",
        LinkType::StaticTransitive => "static-transitive",
        LinkType::StaticHard => "static-hard",
    }
}

/// Lowercase-hex encode a digest.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
#[path = "fingerprint/tests.rs"]
mod tests;
