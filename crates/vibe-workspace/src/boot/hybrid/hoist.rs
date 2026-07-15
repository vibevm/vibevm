//! Soft hoisting — the compile-time dedup of shared static packages
//! (PROP-038 §2.3–§2.4).
//!
//! A package soft-statically linked by more than one unit across the tree is
//! **shared**: compiling it into each consumer's `STATIC.md` would duplicate
//! it in context whenever several consumers are read. Instead it is
//! **hoisted** to the global root `STATIC.md` and linked once (the owner's
//! design: "поднимаем на уровень глобального STATIC.md … для всех библиотек,
//! которые пытались подключить её статически"), with a `#use` marker left in
//! each local zone (§2.5) so the graph edge survives and the read-set dedups
//! the read. A package pulled by exactly one unit stays **local** — compiled
//! into that one consumer, preserving locality.
//!
//! Counting rules (PROP-038 §5.2): a direct `static` edge counts the target
//! as pulled by the declaring unit; a `static-transitive` edge counts every
//! package in its forced (non-`when`-gated) subtree; `static-hard` opts out
//! (never counted, never hoisted); `dynamic` is not static and never counts.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-038#hoisting");

use std::collections::{HashMap, HashSet};

use vibe_core::manifest::LinkType;

use super::{UnitId, UnitInput};

/// For each package, the set of units that **soft-statically** pull it — a
/// direct `static` edge, or membership in a `static-transitive` edge's forced
/// subtree (§2.4, §5.2). `static-hard` and `dynamic` edges never contribute.
pub fn soft_static_pulls(table: &HashMap<UnitId, UnitInput>) -> HashMap<UnitId, HashSet<UnitId>> {
    let mut pulls: HashMap<UnitId, HashSet<UnitId>> = HashMap::new();
    for (uid, unit) in table {
        for edge in &unit.edges {
            match edge.link {
                LinkType::Static => {
                    if !gated(&edge.target, table) {
                        pulls
                            .entry(edge.target.clone())
                            .or_default()
                            .insert(uid.clone());
                    }
                }
                LinkType::StaticTransitive => {
                    for member in forced_subtree(&edge.target, table) {
                        pulls.entry(member).or_default().insert(uid.clone());
                    }
                }
                LinkType::StaticHard | LinkType::Dynamic => {}
            }
        }
    }
    pulls
}

/// The packages hoisted to the global root `STATIC.md` — those soft-pulled by
/// **two or more** distinct units, and that ship boot content of their own
/// (a content-less package has nothing to duplicate). PROP-038 §2.4.
///
/// ```
/// use std::collections::HashMap;
/// use vibe_workspace::boot::hybrid::{UnitEdge, UnitId, UnitInput};
/// use vibe_workspace::boot::hybrid::hoist::shared_packages;
/// use vibe_core::{Group, manifest::LinkType};
///
/// let g = Group::parse("org.vibevm").unwrap();
/// let id = |n: &str| -> UnitId { (g.clone(), n.to_string()) };
/// let stat = |t: &str| UnitEdge { target: id(t), link: LinkType::Static };
/// let unit = |edges: Vec<UnitEdge>| UnitInput {
///     own_boot_path: Some("x.md".to_string()),
///     origin: String::new(),
///     when: None,
///     edges,
/// };
/// // Two units static-link `shared` — it is shared, so hoisted.
/// let mut table = HashMap::new();
/// table.insert(id("a"), unit(vec![stat("shared")]));
/// table.insert(id("b"), unit(vec![stat("shared")]));
/// table.insert(id("shared"), unit(vec![]));
/// assert!(shared_packages(&table).contains(&id("shared")));
/// ```
pub fn shared_packages(table: &HashMap<UnitId, UnitInput>) -> HashSet<UnitId> {
    soft_static_pulls(table)
        .into_iter()
        .filter(|(pkg, pullers)| {
            pullers.len() >= 2 && table.get(pkg).is_some_and(|u| u.own_boot_path.is_some())
        })
        .map(|(pkg, _)| pkg)
        .collect()
}

/// Whether a package carries a `when` gate (stays dynamic even under a forced
/// static subtree — PROP-009 §2.4).
fn gated(id: &UnitId, table: &HashMap<UnitId, UnitInput>) -> bool {
    table.get(id).and_then(|u| u.when).is_some()
}

/// The forced-static subtree of a `static-transitive` edge's target: the
/// target plus every package transitively reachable through `requires`, minus
/// any `when`-gated package (which stays dynamic and breaks the force there,
/// matching [`super::resolve_zone`]). Cycle-guarded.
fn forced_subtree(root: &UnitId, table: &HashMap<UnitId, UnitInput>) -> HashSet<UnitId> {
    let mut seen: HashSet<UnitId> = HashSet::new();
    let mut stack = vec![root.clone()];
    while let Some(id) = stack.pop() {
        if seen.contains(&id) || gated(&id, table) {
            continue;
        }
        seen.insert(id.clone());
        if let Some(unit) = table.get(&id) {
            for edge in &unit.edges {
                stack.push(edge.target.clone());
            }
        }
    }
    seen
}

#[cfg(test)]
#[path = "hoist/tests.rs"]
mod tests;
