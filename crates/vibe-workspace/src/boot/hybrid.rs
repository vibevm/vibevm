//! Per-unit recursive boot compilation — the hybrid linker (PROP-038 §2.2).
//!
//! Where [`super::compute_effective_boot`] composes **one** node's boot from
//! the global dependency closure (the PROP-009 model), this module compiles
//! **each** compilation unit — every materialised package as well as every
//! workspace node (PROP-038 §2.1) — from its **own direct edges**, reading
//! the `link` mode off each unit's own manifest (§2.2, link is a per-edge,
//! consumer-side property).
//!
//! ## The recursion (PROP-038 §2.2)
//!
//! A unit `P`'s **static zone** is `P`'s own boot content plus, for every
//! `static` / `static-transitive` edge `P→X`, `X`'s static zone, recursively.
//! The recursion **breaks at a `dynamic` edge** — that target is not compiled
//! in; it surfaces as a `dynamic` edge of `P` instead (§5.5: a compiled
//! zone's dynamic edges aggregate into the unit's own `INDEX.md`). A
//! `static-transitive` edge **forces** its whole subtree static, overriding
//! nested `dynamic` edges (§2.2) — but never a `when` gate, which stays
//! `dynamic` for OS-correctness (matching the PROP-009 rule that a `when`
//! forces the gated INDEX form irrespective of `link`).
//!
//! ## Membership then order
//!
//! [`resolve_zone`] determines *membership* (which units are compiled into
//! the zone, which edges surface as dynamic) by the recursion above;
//! [`topo_zone`] then orders the static members dependency-before-dependent
//! with the same Kahn + pkgref-min-heap tie-break `super::topo_order` uses,
//! so a unit's `STATIC.md` is byte-stable and — for an entry-point node whose
//! zone is the root's `static-transitive` closure — byte-identical to the
//! pre-hybrid global linker (the migration-safety corollary, PROP-038 §5).

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-038#edge-recursion");

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use vibe_core::Group;
use vibe_core::manifest::{LinkType, PackageFormat, WhenCondition};

pub mod fingerprint;
pub mod hoist;

#[cfg(test)]
mod fuzz;

#[cfg(test)]
pub(crate) mod testkit;

/// A compilation unit's identity — a resolved `(group, name)` (PROP-038 §2.1).
/// One node per unified package version (the resolver has already unified
/// versions, PROP-017), so the version is not part of the key.
pub type UnitId = (Group, String);

/// One direct edge from a unit to a dependency, carrying the `link` mode the
/// unit's **own** manifest declared for it (PROP-038 §2.2). This is the fix
/// for the shipped bootgen's root-only seeding: every unit's edges carry
/// their own modes, so a `dynamic`-linked package's `static` edge to its own
/// dependency is honoured.
///
/// ```
/// use vibe_workspace::boot::hybrid::UnitEdge;
/// use vibe_core::{Group, manifest::LinkType};
/// let g = Group::parse("org.vibevm").unwrap();
/// let edge = UnitEdge { target: (g, "wal".to_string()), link: LinkType::Static };
/// assert_eq!(edge.link, LinkType::Static);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitEdge {
    /// The dependency this edge points at.
    pub target: UnitId,
    /// The link mode from the consuming unit's manifest — `None` means the
    /// edge declared none (resolved by the caller's precedence before it
    /// reaches here; a bare edge defaults to [`LinkType::Dynamic`]).
    pub link: LinkType,
}

/// A compilation unit's inputs for the recursive compiler (PROP-038 §2.1).
///
/// ```
/// use std::collections::HashMap;
/// use vibe_workspace::boot::hybrid::{resolve_zone, UnitEdge, UnitId, UnitInput};
/// use vibe_core::{Group, manifest::LinkType};
///
/// let g = Group::parse("org.vibevm").unwrap();
/// let id = |n: &str| -> UnitId { (g.clone(), n.to_string()) };
/// let unit = |boot: &str, edges: Vec<UnitEdge>| UnitInput {
///     own_boot_path: Some(boot.to_string()),
///     origin: String::new(),
///     when: None,
///     edges,
///     format: Default::default(),
/// };
/// // root →(static) a — `a` compiles into root's static zone.
/// let mut table: HashMap<UnitId, UnitInput> = HashMap::new();
/// table.insert(
///     id("root"),
///     unit("root.md", vec![UnitEdge { target: id("a"), link: LinkType::Static }]),
/// );
/// table.insert(id("a"), unit("a.md", vec![]));
/// let zone = resolve_zone(&id("root"), &table);
/// assert!(zone.static_members.contains(&id("a")));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitInput {
    /// The unit's own boot content, compiled into its `STATIC.md` when the
    /// unit is statically linked — a package's `[boot_snippet]` source path.
    /// `None` for a boot-less package (it still threads the ordering) or a
    /// workspace node (whose authored boot is separate dynamic content).
    pub own_boot_path: Option<String>,
    /// Provenance label — a `<group>/<name>` pkgref, for the `STATIC.md`
    /// provenance marker.
    pub origin: String,
    /// The package's own `[boot_snippet].when` OS gate, if any. A gated
    /// unit is always `dynamic` (never compiled into a parent's static zone),
    /// irrespective of the edge's `link` — OS-specific content must never
    /// reach a session on the wrong OS (PROP-009 §2.4).
    pub when: Option<WhenCondition>,
    /// The unit's direct edges, each carrying the link mode from this unit's
    /// manifest.
    pub edges: Vec<UnitEdge>,
    /// The unit's PROP-035 §3 package format. `Normal` tells the static
    /// renderer to compile this unit's `#use`/`#source` closure (PROP-035 §8)
    /// when it is compiled into a `STATIC.md`; `Simple` (the default) keeps
    /// the verbatim concatenation.
    pub format: PackageFormat,
}

/// The membership of one unit's static zone (PROP-038 §2.2) — the recursion's
/// output, before ordering.
///
/// ```
/// use vibe_workspace::boot::hybrid::ZoneMembership;
/// let m = ZoneMembership::default();
/// assert!(m.static_members.is_empty());
/// assert!(m.dynamic_edges.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZoneMembership {
    /// Units compiled into this unit's `STATIC.md`, deduplicated. Unordered
    /// here; [`topo_zone`] orders them.
    pub static_members: HashSet<UnitId>,
    /// The dynamic edges surfaced from the zone — this unit's own `dynamic`
    /// edges plus the `dynamic` edges of every unit compiled into the zone
    /// (§5.5). Each carries the `when` gate of its target, if any. Collected
    /// in deterministic order (sorted by pkgref) so `INDEX.md` is stable.
    pub dynamic_edges: Vec<(UnitId, Option<WhenCondition>)>,
}

/// Resolve one unit's static-zone membership by the PROP-038 §2.2 recursion.
///
/// `units` is the full unit table; a `target` absent from it (never expected
/// in a resolved closure) contributes no membership. `forced` starts `false`
/// for a normal compile and is set on descent through a `static-transitive`
/// edge — it forces the subtree static, overriding nested `dynamic` edges but
/// not a `when` gate.
///
/// ```
/// use std::collections::HashMap;
/// use vibe_workspace::boot::hybrid::{resolve_zone, UnitEdge, UnitId, UnitInput};
/// use vibe_core::{Group, manifest::LinkType};
///
/// let g = Group::parse("org.vibevm").unwrap();
/// let id = |n: &str| -> UnitId { (g.clone(), n.to_string()) };
/// let unit = |edges: Vec<UnitEdge>| UnitInput {
///     own_boot_path: None, origin: String::new(), when: None, edges,
///     format: Default::default(),
/// };
/// // root →(dynamic) a: the dynamic edge BREAKS the zone — `a` is not compiled
/// // in, it surfaces as a dynamic edge instead (contrast the static case on
/// // `UnitInput`, where `a` joins the static zone).
/// let mut table: HashMap<UnitId, UnitInput> = HashMap::new();
/// table.insert(id("root"), unit(vec![UnitEdge { target: id("a"), link: LinkType::Dynamic }]));
/// table.insert(id("a"), unit(vec![]));
/// let zone = resolve_zone(&id("root"), &table);
/// assert!(!zone.static_members.contains(&id("a")));
/// assert_eq!(zone.dynamic_edges.len(), 1);
/// assert_eq!(zone.dynamic_edges[0].0, id("a"));
/// ```
pub fn resolve_zone(root: &UnitId, units: &HashMap<UnitId, UnitInput>) -> ZoneMembership {
    let mut membership = ZoneMembership::default();
    let mut static_visited: HashSet<UnitId> = HashSet::new();
    let mut dynamic_seen: HashSet<UnitId> = HashSet::new();
    descend(
        root,
        false,
        units,
        &mut static_visited,
        &mut dynamic_seen,
        &mut membership,
    );
    // Deterministic INDEX order — sort the surfaced dynamic edges by pkgref.
    membership
        .dynamic_edges
        .sort_by(|(a, _), (b, _)| pkgref(a).cmp(&pkgref(b)));
    membership.static_members = static_visited;
    membership
}

/// One step of the §2.2 recursion. `static_visited` dedups the static zone
/// (a package reached by several static paths is compiled once); `dynamic_seen`
/// dedups the surfaced dynamic edges.
fn descend(
    id: &UnitId,
    forced: bool,
    units: &HashMap<UnitId, UnitInput>,
    static_visited: &mut HashSet<UnitId>,
    dynamic_seen: &mut HashSet<UnitId>,
    membership: &mut ZoneMembership,
) {
    if !static_visited.insert(id.clone()) {
        return;
    }
    let Some(unit) = units.get(id) else {
        return;
    };
    // Deterministic descent — sort edges by pkgref so the walk (and thus the
    // dedup outcome for diamonds) is stable across runs.
    let mut edges = unit.edges.clone();
    edges.sort_by(|a, b| pkgref(&a.target).cmp(&pkgref(&b.target)));
    for edge in &edges {
        // A `when`-gated target is dynamic irrespective of the edge or the
        // forced flag (PROP-009 §2.4 — OS-correctness). Otherwise a
        // `static-transitive` ancestor (`forced`) or a `static` /
        // `static-transitive` edge keeps it in the static zone.
        let gated = units.get(&edge.target).and_then(|u| u.when).is_some();
        let stays_static = !gated
            && (forced
                || matches!(
                    edge.link,
                    LinkType::Static | LinkType::StaticTransitive | LinkType::StaticHard
                ));
        if stays_static {
            // Only `static-transitive` forces the subtree; `static` and
            // `static-hard` compile the target but honour its own edges.
            let child_forced = forced || edge.link == LinkType::StaticTransitive;
            descend(
                &edge.target,
                child_forced,
                units,
                static_visited,
                dynamic_seen,
                membership,
            );
        } else if dynamic_seen.insert(edge.target.clone()) {
            let when = units.get(&edge.target).and_then(|u| u.when);
            membership.dynamic_edges.push((edge.target.clone(), when));
        }
    }
}

/// Order a zone's static members dependency-before-dependent — the same Kahn
/// algorithm with a `<group>/<name>` min-heap tie-break as
/// `super::topo_order`, so the emitted `STATIC.md` is byte-stable and
/// migration-safe (PROP-038 §5). Returns the members in compiled order; the
/// root unit (the zone's owner) sorts last, after everything it builds on.
///
/// ```
/// use std::collections::HashMap;
/// use vibe_workspace::boot::hybrid::{resolve_zone, topo_zone, UnitEdge, UnitId, UnitInput};
/// use vibe_core::{Group, manifest::LinkType};
///
/// let g = Group::parse("org.vibevm").unwrap();
/// let id = |n: &str| -> UnitId { (g.clone(), n.to_string()) };
/// let unit = |edges: Vec<UnitEdge>| UnitInput {
///     own_boot_path: None, origin: String::new(), when: None, edges,
///     format: Default::default(),
/// };
/// // root →(static) a: both compile in; the dependency `a` orders before the
/// // dependent `root`.
/// let mut table: HashMap<UnitId, UnitInput> = HashMap::new();
/// table.insert(id("root"), unit(vec![UnitEdge { target: id("a"), link: LinkType::Static }]));
/// table.insert(id("a"), unit(vec![]));
/// let zone = resolve_zone(&id("root"), &table);
/// assert_eq!(topo_zone(&zone.static_members, &table), vec![id("a"), id("root")]);
/// ```
pub fn topo_zone(members: &HashSet<UnitId>, units: &HashMap<UnitId, UnitInput>) -> Vec<UnitId> {
    let ids: Vec<UnitId> = members.iter().cloned().collect();
    let index: HashMap<&UnitId, usize> = ids.iter().enumerate().map(|(i, id)| (id, i)).collect();
    let n = ids.len();

    // `in_degree[i]` counts the in-zone packages `i` requires; only edges to
    // members that are themselves in the static zone impose ordering (a
    // dynamic edge left the zone).
    let mut in_degree = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, id) in ids.iter().enumerate() {
        let Some(unit) = units.get(id) else { continue };
        for edge in &unit.edges {
            if let Some(&j) = index.get(&edge.target) {
                // `i` requires `j` (in-zone) → `j` precedes `i`.
                in_degree[i] += 1;
                dependents[j].push(i);
            }
        }
    }

    let mut ready: BinaryHeap<Reverse<(String, usize)>> = (0..n)
        .filter(|&i| in_degree[i] == 0)
        .map(|i| Reverse((pkgref(&ids[i]), i)))
        .collect();
    let mut order: Vec<UnitId> = Vec::with_capacity(n);
    while let Some(Reverse((_, i))) = ready.pop() {
        order.push(ids[i].clone());
        for &d in &dependents[i] {
            in_degree[d] -= 1;
            if in_degree[d] == 0 {
                ready.push(Reverse((pkgref(&ids[d]), d)));
            }
        }
    }
    order
}

/// The `<group>/<name>` pkgref of a unit — the deterministic ordering key,
/// matching `super::topo_order`'s tie-break.
fn pkgref(id: &UnitId) -> String {
    format!("{}/{}", id.0, id.1)
}

#[cfg(test)]
#[path = "hybrid/tests.rs"]
mod tests;
