//! The shape stage of the PROP-037 §3.2 filter/shape pipeline — the
//! [`TreeShape`] enum (§3.3 `#tree-shapes`) and the strategy it contributes to
//! the one reusable core walk in [`super::flatten`]: the root-set selection, the
//! per-child visit predicate, and whether the orphan pass runs (D4). A shape is
//! a policy triple carried into the walk, never a bespoke walker.
//!
//! [`super::flatten`]: super::flatten

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#tree-shapes");

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use specmark::spec;

use super::super::model::PackageTree;
use super::state::Ordering;

/// The three forest shapes the pipeline offers (PROP-037 §3.3 `#tree-shapes`),
/// selectable per context, **default = (a)**. A shape is a triple of
/// root-set / visit-predicate / orphan-pass policy carried into the one reusable
/// [`super::flatten`] walk — not a bespoke walker.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum TreeShape {
    /// **(a) members-as-roots + full subtrees** — each filter member is a forest
    /// root shown with its entire dependency subtree (cross-type deps included);
    /// DAG dedup via `↩`. The orphan pass still runs, keyed off the *declared*
    /// roots, so a drifted package no declared root reaches is never hidden.
    /// This is the default and the byte-identical continuation of the pre-shape
    /// `flatten` when the filter is the declared-root set.
    #[default]
    MembersAsRoots,
    /// **(b) load-type forest** — a package is a root only if no other filter
    /// member depends on it; children are its same-set dependencies (cross-set
    /// deps omitted). A shape that intentionally narrows never resurrects the
    /// pruned edges as orphans, so the orphan pass is suppressed.
    #[allow(dead_code)] // selected by the F2 sort menu (Phase 5+); exercised in tests today.
    LoadTypeForest,
    /// **(c) pruned tree** — the tree from the declared roots, keeping only
    /// branches that reach a filter member. Orphan pass suppressed (a pruned
    /// shape must not resurrect the branches it just cut).
    #[allow(dead_code)] // selected by the F2 sort menu (Phase 5+); exercised in tests today.
    PrunedTree,
}

#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#tree-shapes")]
impl TreeShape {
    /// Whether this shape runs the orphan pass. Only [`TreeShape::MembersAsRoots`]
    /// does — it is the "show everything" default and keeps the PROP-036 §2.12
    /// guarantee that a drifted lock root is never hidden. The two narrowing
    /// shapes suppress it by design.
    pub(super) fn uses_orphan_pass(self) -> bool {
        match self {
            TreeShape::MembersAsRoots => true,
            TreeShape::LoadTypeForest | TreeShape::PrunedTree => false,
        }
    }

    /// Whether the core walk should recurse into `child`. The full-subtree shape
    /// follows every edge; the load-type forest follows only same-set edges; the
    /// pruned tree follows only edges on a branch that still reaches a member.
    fn should_visit(
        self,
        child: &str,
        filter: &BTreeSet<String>,
        reaches_member: &BTreeSet<String>,
    ) -> bool {
        match self {
            TreeShape::MembersAsRoots => true,
            TreeShape::LoadTypeForest => filter.contains(child),
            TreeShape::PrunedTree => reaches_member.contains(child),
        }
    }

    /// The ordered root-id list for this shape over the given tree + filter.
    ///
    /// Ordering follows the declared-root order first (members that are declared
    /// roots, in that order), then any remaining members sorted by id, so the
    /// default path preserves the PROP-036 topological order byte-for-byte;
    /// [`Ordering::Alphabetical`] then sorts the whole root list.
    pub(super) fn roots<'a>(
        self,
        tree: &'a PackageTree,
        filter: &'a BTreeSet<String>,
        ordering: Ordering,
        reaches_member: &BTreeSet<String>,
    ) -> Vec<&'a str> {
        let is_member = |id: &str| filter.contains(id);
        match self {
            // (a) every filter member is a forest root.
            TreeShape::MembersAsRoots => ordered_members(tree, filter, ordering, &is_member),
            // (b) only filter members no other filter member depends on.
            TreeShape::LoadTypeForest => {
                let depended_on = members_depended_on_by_members(filter, tree);
                let is_root = |id: &str| is_member(id) && !depended_on.contains(id);
                ordered_members(tree, filter, ordering, &is_root)
            }
            // (c) the declared roots, dropping any that reach no member.
            TreeShape::PrunedTree => {
                let mut roots: Vec<&str> = tree
                    .roots
                    .iter()
                    .map(|s| s.as_str())
                    .filter(|r| reaches_member.contains(*r))
                    .collect();
                if ordering == Ordering::Alphabetical {
                    roots.sort_unstable();
                }
                roots
            }
        }
    }
}

/// The per-walk shape context: the active [`TreeShape`] plus the two precomputed
/// sets its visit predicate consults. Bundled so the core walk in
/// [`super::flatten`] takes one extra parameter instead of three.
///
/// [`super::flatten`]: super::flatten
pub(super) struct ShapeCtx<'a> {
    shape: TreeShape,
    filter: &'a BTreeSet<String>,
    /// The set of ids whose dependency closure contains a filter member (incl.
    /// a member itself). Only consulted by [`TreeShape::PrunedTree`].
    reaches_member: &'a BTreeSet<String>,
}

impl<'a> ShapeCtx<'a> {
    /// Build a walk context over the active shape and its two precomputed sets.
    pub(super) fn new(
        shape: TreeShape,
        filter: &'a BTreeSet<String>,
        reaches_member: &'a BTreeSet<String>,
    ) -> Self {
        Self {
            shape,
            filter,
            reaches_member,
        }
    }

    /// Whether the walk should recurse into `child` under this shape.
    pub(super) fn should_visit(&self, child: &str) -> bool {
        self.shape
            .should_visit(child, self.filter, self.reaches_member)
    }
}

/// The nodes whose dependency closure contains a filter member (including a
/// member itself) — the (c) visit predicate and root gate. Computed once,
/// fold-independent, cycle-safe (BFS up the reversed edge set, seeded from the
/// members). Exposed so the walk precomputes it once for the whole flatten.
pub(super) fn compute_reaches_member(
    tree: &PackageTree,
    filter: &BTreeSet<String>,
) -> BTreeSet<String> {
    // Reverse adjacency: for each edge parent → child, record child → parents.
    let mut parents_of: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for p in &tree.packages {
        for dep in &p.dependencies {
            parents_of
                .entry(dep.as_str())
                .or_default()
                .push(p.id.as_str());
        }
    }
    let mut reached: BTreeSet<&str> = BTreeSet::new();
    let mut queue: VecDeque<&str> = filter.iter().map(|s| s.as_str()).collect();
    while let Some(id) = queue.pop_front() {
        if !reached.insert(id) {
            continue;
        }
        if let Some(parents) = parents_of.get(id) {
            for &parent in parents {
                if !reached.contains(parent) {
                    queue.push_back(parent);
                }
            }
        }
    }
    reached.into_iter().map(|s| s.to_string()).collect()
}

/// The filter members satisfying `keep`, ordered declared-roots-first then
/// sorted-by-id, with the alphabetical override sorting the whole list. This is
/// the shared root-ordering rule for the member-rooted shapes (a)/(b).
fn ordered_members<'a>(
    tree: &'a PackageTree,
    filter: &'a BTreeSet<String>,
    ordering: Ordering,
    keep: &dyn Fn(&str) -> bool,
) -> Vec<&'a str> {
    let declared: Vec<&str> = tree
        .roots
        .iter()
        .map(|s| s.as_str())
        .filter(|r| keep(r))
        .collect();
    let declared_set: BTreeSet<&str> = declared.iter().copied().collect();
    let mut extras: Vec<&str> = filter
        .iter()
        .map(|s| s.as_str())
        .filter(|r| keep(r) && !declared_set.contains(*r))
        .collect();
    extras.sort_unstable();
    let mut roots = declared;
    roots.extend(extras);
    if ordering == Ordering::Alphabetical {
        roots.sort_unstable();
    }
    roots
}

/// The set of filter members that some filter member lists as a dependency —
/// i.e. the non-roots of the load-type forest (b).
fn members_depended_on_by_members(
    filter: &BTreeSet<String>,
    tree: &PackageTree,
) -> BTreeSet<String> {
    let mut depended: BTreeSet<String> = BTreeSet::new();
    for p in &tree.packages {
        if filter.contains(&p.id) {
            for dep in &p.dependencies {
                if filter.contains(dep) {
                    depended.insert(dep.clone());
                }
            }
        }
    }
    depended
}
