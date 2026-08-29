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
//!
//! # The owner-plan frame (R4 architecture §7.1)
//!
//! A unit's lane is compiled with its OWNER's transform plan, so that plan is
//! part of what the artifact is: a changed owner plan must leave the unit
//! stale. The Merkle body therefore appends one frame, `transforms:<hex>`,
//! carrying the owner plan's digest — **only when the digest map carries that
//! unit**. No entry, no frame: every fingerprint recorded before this frame
//! existed, and every unit whose owner activates nothing, keeps its exact
//! current bytes. That absence is the whole of the historical-identity law,
//! and framing an empty plan would break every recorded fingerprint for zero
//! information.
//!
//! **Propagation needs no code of its own.** A static parent already hashes
//! its child's FINGERPRINT, so a child's plan frame flips the static chain up
//! to the first dynamic break — which is exactly right, because a static
//! parent inlines the child's zone bytes and those bytes were produced under
//! the child's plan. A dynamic parent hashes only the child's identity, so
//! the plan stays behind that boundary, exactly as content does.
//!
//! A node lane has no fingerprint at all (it always recomputes); its plan
//! identity rides the artifact header, and its equal-bytes no-op belongs to
//! the publication transaction.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#change-detection");

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};
use vibe_core::manifest::LinkType;

use super::{UnitId, UnitInput};

/// Compute the fingerprint of every unit (PROP-038 §2.7), bottom-up with
/// memoisation. `versions` gives each unit's resolved version — the content
/// identity for an immutable package (a mutable in-workspace source is
/// re-materialised regardless, PROP-011 §2.6, so the version key suffices).
///
/// `plan_digests` gives each unit's OWNER-plan digest as lowercase sha256 hex
/// (R4 architecture §7.1), with an entry present ONLY for a nonempty plan. A
/// unit absent from that map gets no plan frame and keeps its historical
/// fingerprint bytes exactly; the caller derives the map from the same
/// lowering the emission path is handed, so a unit is never fingerprinted
/// against one plan and compiled with another.
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
///     fragments: vec![],
///     origin: String::new(),
///     when: None,
///     edges: vec![],
///     format: Default::default(),
/// });
/// let versions: HashMap<UnitId, String> = [(id.clone(), "1.0.0".to_string())].into_iter().collect();
/// // No plan digests: an owner that activates nothing frames nothing.
/// let plans: HashMap<UnitId, String> = HashMap::new();
/// let fps = fingerprints(&table, &versions, &plans);
/// assert!(fps.contains_key(&id));
///
/// // A nonempty owner plan moves that unit's fingerprint, and nothing else.
/// let with_plan: HashMap<UnitId, String> =
///     [(id.clone(), "ab".repeat(32))].into_iter().collect();
/// assert_ne!(fingerprints(&table, &versions, &with_plan)[&id], fps[&id]);
/// ```
pub fn fingerprints(
    table: &HashMap<UnitId, UnitInput>,
    versions: &HashMap<UnitId, String>,
    plan_digests: &HashMap<UnitId, String>,
) -> HashMap<UnitId, String> {
    let mut memo: HashMap<UnitId, String> = HashMap::new();
    for id in table.keys() {
        compute(
            id,
            table,
            versions,
            plan_digests,
            &mut memo,
            &mut HashSet::new(),
        );
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
    plan_digests: &HashMap<UnitId, String>,
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
        hasher.update(b"\nwhen:");
        hasher.update(
            unit.when
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "-".to_string())
                .as_bytes(),
        );
        for fragment in &unit.fragments {
            hasher.update(b"\nfragment:");
            hasher.update(fragment.path.as_bytes());
            hasher.update(b"@when:");
            hasher.update(
                fragment
                    .when
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "-".to_string())
                    .as_bytes(),
            );
        }

        // Sort edges deterministically so the fingerprint is stable.
        let mut edges = unit.edges.clone();
        edges.sort_by(|a, b| pkgref(&a.target).cmp(&pkgref(&b.target)));
        for edge in &edges {
            hasher.update(b"\nedge:");
            hasher.update(link_wire(edge.link).as_bytes());
            hasher.update(b"->");
            hasher.update(pkgref(&edge.target).as_bytes());
            // A static edge propagates the child's whole contribution graph;
            // a dynamic edge contributes identity only and breaks propagation.
            let is_static = matches!(
                edge.link,
                LinkType::Static | LinkType::StaticTransitive | LinkType::StaticHard
            );
            if is_static {
                let child = compute(&edge.target, table, versions, plan_digests, memo, on_stack);
                hasher.update(b" static-fp:");
                hasher.update(child.as_bytes());
            } else {
                hasher.update(b" dyn-id:");
                hasher.update(version_of(&edge.target, versions));
            }
        }
    }
    // The owner-plan frame (R4 architecture §7.1), appended ONLY when this
    // unit's owner activated something. An absent entry writes nothing at
    // all — not an empty frame, not a marker — which is what keeps every
    // historical fingerprint byte-exact. Propagation up the static chain is
    // the Merkle above and needs nothing here.
    if let Some(digest) = plan_digests.get(id) {
        hasher.update(b"\ntransforms:");
        hasher.update(digest.as_bytes());
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
