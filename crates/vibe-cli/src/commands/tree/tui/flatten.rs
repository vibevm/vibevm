//! The fold-aware DAG → visible-rows flatten (PROP-036 §2.12), generalised over
//! the PROP-037 §3.2/§3.3 filter/shape pipeline. A declared-root/member DFS with
//! `│├└` connectors, `(*)` re-occurrence dedup, and — for the members-as-roots
//! shape — an orphan pass so a package no declared root reaches is still shown.
//! Extracted from `state` as its own cell (the tree-shaping algorithm, distinct
//! from the app state).
//!
//! The shape stage ([`TreeShape`]) lives in [`super::shape`]; this module owns
//! the one reusable core [`walk`] (connector/prefix/fold/DAG-dedup glyphs) that
//! every shape feeds via a [`ShapeCtx`]. The default [`TreeShape::MembersAsRoots`]
//! over the declared-root filter is byte-identical to the pre-shape walk.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;

use super::super::model::{Package, PackageTree};
use super::modes;
use super::shape::{ShapeCtx, compute_reaches_member};
use super::state::{Ordering, RowNode, VisibleRow, load_label};

/// Re-exported so the call site ([`super::state`]) reaches the shape via the
/// pipeline entry point: `super::flatten::TreeShape`. Also brings the type into
/// this module's scope for the [`flatten`] signature.
pub(super) use super::shape::TreeShape;

/// Flatten the DAG into visible rows given a fold set, a shape, and a filter
/// (PROP-036 §2.12 walk; PROP-037 §3.2 pipeline).
///
/// `filter` is the member set — the ids the active mode has selected. With the
/// default [`TreeShape::MembersAsRoots`] and `filter` = the declared-root set
/// the output is byte-identical to the pre-shape walk (the tree-mode default;
/// see `state::App::rebuild`).
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#tree-filters")]
pub(super) fn flatten(
    tree: &PackageTree,
    folded: &BTreeSet<String>,
    ordering: Ordering,
    shape: TreeShape,
    filter: &BTreeSet<String>,
) -> Vec<VisibleRow> {
    let by_id: BTreeMap<&str, usize> = tree
        .packages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();

    // The nodes whose closure reaches a filter member — the (c) visit predicate
    // and root gate. Computed once, fold-independent, cycle-safe.
    let reaches_member = compute_reaches_member(tree, filter);
    let ctx = ShapeCtx::new(shape, filter, &reaches_member);

    let mut rows: Vec<VisibleRow> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    let roots = shape.roots(tree, filter, ordering, &reaches_member);
    let root_count = roots.len();
    for (i, &root) in roots.iter().enumerate() {
        let is_last = i + 1 == root_count;
        walk(
            root, "", is_last, true, tree, &by_id, folded, ordering, &ctx, &mut seen, &mut rows,
        );
    }

    // Orphan pass (PROP-036 §2.12): a package genuinely not reachable from a
    // declared root (e.g. a drifted lock root) is still shown — but only under
    // the members-as-roots shape. The narrowing shapes (b)/(c) intentionally
    // prune and must not resurrect the branches they cut, so they skip it.
    // Reachability is judged over the FULL declared-root graph, ignoring folds
    // — a folded-away subtree is hidden, not resurrected here.
    if shape.uses_orphan_pass() {
        let reachable = modes::reachable_from_roots(tree, &by_id);
        let mut orphans: Vec<&Package> = tree
            .packages
            .iter()
            .filter(|p| !reachable.contains(&p.id))
            .collect();
        orphans.sort_by(|a, b| a.id.cmp(&b.id));
        if !orphans.is_empty() {
            rows.push(VisibleRow {
                node: RowNode::Separator,
                id: String::new(),
                name: "— not reached from a declared root —".to_string(),
                load: "",
                transitive: false,
                condition: false,
                in_static: false,
            });
            let n = orphans.len();
            for (k, pkg) in orphans.into_iter().enumerate() {
                let is_last = k + 1 == n;
                walk(
                    &pkg.id, "", is_last, true, tree, &by_id, folded, ordering, &ctx, &mut seen,
                    &mut rows,
                );
            }
        }
    }
    rows
}

/// Depth-first walk producing rows. Marks a re-occurrence `(*)` and does not
/// re-expand it (DAG dedup), and a folded node shows the collapsed indicator
/// (does not recurse). Mirrors [`super::super::plain`]'s connector/prefix
/// construction. The per-shape policy lives in `ctx`: it gates which children
/// are visited (so a pruned branch simply disappears, connectors and all) and
/// the core glyph/connector logic is identical for every shape.
#[allow(clippy::too_many_arguments)]
fn walk(
    id: &str,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    tree: &PackageTree,
    by_id: &BTreeMap<&str, usize>,
    folded: &BTreeSet<String>,
    ordering: Ordering,
    ctx: &ShapeCtx,
    seen: &mut BTreeSet<String>,
    rows: &mut Vec<VisibleRow>,
) {
    // A root carries no connector; every child gets a `├─`/`└─` connector on
    // top of the accumulated vertical-bar prefix.
    let connector = if is_root {
        String::new()
    } else if is_last {
        format!("{prefix}\u{2514}\u{2500} ")
    } else {
        format!("{prefix}\u{251c}\u{2500} ")
    };

    let Some(&idx) = by_id.get(id) else {
        rows.push(VisibleRow {
            node: RowNode::Missing,
            id: id.to_string(),
            name: format!("{connector}{id}  (not in lockfile)"),
            load: "?",
            transitive: false,
            condition: false,
            in_static: false,
        });
        return;
    };
    let Some(pkg) = tree.packages.get(idx) else {
        return;
    };

    let repeated = seen.contains(id);
    // The children this shape actually follows — full set for (a), same-set for
    // (b), member-reaching for (c). The fold indicator and the connector count
    // both key off this filtered set, so a node whose model-children are all
    // pruned shows no expand affordance and a clean gutter.
    let mut children: Vec<&str> = pkg
        .dependencies
        .iter()
        .map(|s| s.as_str())
        .filter(|dep| ctx.should_visit(dep))
        .collect();
    if ordering == Ordering::Alphabetical {
        children.sort_unstable();
    }
    let has_children = !children.is_empty();
    let is_folded = folded.contains(id);
    // The expand indicator is shown only on a first-seen node that has (visited)
    // children; a re-occurrence is a display leaf (its subtree lives elsewhere).
    // Glyphs come from the theme vocabulary (PROP-037 §2.2.2): ▾/▸ Tier ≥ 1,
    // +/- Tier 0 — never a hardcoded ASCII literal.
    let indicator = if has_children && !repeated {
        if is_folded {
            format!("{} ", super::theme::fold_collapsed())
        } else {
            format!("{} ", super::theme::fold_expanded())
        }
    } else {
        String::new()
    };
    let marker = if repeated {
        format!(" {}", super::theme::dag_dedup())
    } else {
        String::new()
    };
    rows.push(VisibleRow {
        node: RowNode::Package(idx),
        id: id.to_string(),
        name: format!("{connector}{indicator}{id}{marker}"),
        load: load_label(pkg.load.load_type),
        transitive: pkg.load.transitive,
        condition: pkg.condition.present,
        in_static: pkg.load.in_static_md,
    });

    if repeated {
        return;
    }
    seen.insert(id.to_string());
    if is_folded {
        return;
    }

    // Children of a root start at column 0; deeper levels extend the prefix
    // with a vertical bar or a blank gutter.
    let child_prefix = if is_root {
        String::new()
    } else if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}\u{2502}  ")
    };
    let n = children.len();
    for (i, &dep) in children.iter().enumerate() {
        let dep_last = i + 1 == n;
        walk(
            dep,
            &child_prefix,
            dep_last,
            false,
            tree,
            by_id,
            folded,
            ordering,
            ctx,
            seen,
            rows,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::model::*;

    /// A package `id` depending on `deps` (ids), load-type `None`, no condition.
    fn pkg(id: &str, deps: &[&str]) -> Package {
        let (group, name) = id.split_once('/').unwrap_or(("g", id));
        Package {
            id: id.to_string(),
            group: group.to_string(),
            name: name.to_string(),
            kind: "flow".to_string(),
            version: "0.1.0".to_string(),
            content_hash: None,
            source: None,
            load: Load {
                load_type: LoadType::None,
                transitive: false,
                declared: None,
                origin: LoadOrigin::None,
                in_static_md: false,
                in_index_md: false,
                boot_path: None,
            },
            condition: Condition::absent(),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Build a `PackageTree` over `packages` with the given declared `roots`.
    fn tree(packages: Vec<Package>, roots: &[&str]) -> PackageTree {
        PackageTree {
            schema_version: SCHEMA_VERSION,
            generated_at: None,
            tool_version: None,
            project: Project {
                root: "/tmp/x".to_string(),
                name: None,
                is_workspace: false,
                host_namespace: HOST_NAMESPACE.to_string(),
            },
            roots: roots.iter().map(|s| s.to_string()).collect(),
            packages,
            boot: Boot {
                static_md: None,
                index_md: IndexLane {
                    present: false,
                    path: "spec/boot/INDEX.md".to_string(),
                    static_pointer: None,
                    entries: Vec::new(),
                },
            },
            in_place_specs: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// The package ids (and `""` for the separator) a flatten produced, in order.
    fn row_ids(rows: &[VisibleRow]) -> Vec<String> {
        rows.iter().map(|r| r.id.clone()).collect()
    }

    /// The canonical shape fixture: declared root `a` with `a→b→c` and `a→d`,
    /// filter `{b, d}` (a subset, not all). The three shapes visibly diverge on
    /// it: (a) `b,c,d`; (b) `b,d`; (c) `a,b,d`.
    fn fixture() -> PackageTree {
        tree(
            vec![
                pkg("g/a", &["g/b", "g/d"]),
                pkg("g/b", &["g/c"]),
                pkg("g/c", &[]),
                pkg("g/d", &[]),
            ],
            &["g/a"],
        )
    }

    fn filter_b_d() -> BTreeSet<String> {
        ["g/b".to_string(), "g/d".to_string()].into_iter().collect()
    }

    #[test]
    fn members_as_roots_shows_each_member_with_full_subtree() {
        // (a): every filter member is a forest root with its full subtree.
        // `b` roots its full subtree (`b→c` — c is a cross-set dep, included),
        // `d` is a leaf root. The orphan pass keys off the declared root `a`,
        // which reaches the whole graph, so nothing is orphaned.
        let t = fixture();
        let rows = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            &filter_b_d(),
        );
        assert_eq!(row_ids(&rows), ["g/b", "g/c", "g/d"]);
    }

    #[test]
    fn load_type_forest_roots_are_undepended_members_with_same_set_children() {
        // (b): a member is a root only if no other member depends on it; its
        // children are same-set deps only. Neither `b` nor `d` is depended on by
        // a member (`c` is not in the set, `d` has no deps), so both root; `b`'s
        // only child `c` is out-of-set and is pruned.
        let t = fixture();
        let rows = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::LoadTypeForest,
            &filter_b_d(),
        );
        assert_eq!(row_ids(&rows), ["g/b", "g/d"]);
    }

    #[test]
    fn pruned_tree_keeps_only_branches_reaching_a_member() {
        // (c): from the declared root `a`, keep only branches reaching a member.
        // `a` reaches `b` and `d` (both members) so it roots; `a→b` and `a→d`
        // stay. `b→c` reaches no member (`c` is a non-member leaf) so it is cut.
        // The declared root `a` itself is shown because it lies on a path to a
        // member, even though `a` is not a member.
        let t = fixture();
        let rows = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::PrunedTree,
            &filter_b_d(),
        );
        assert_eq!(row_ids(&rows), ["g/a", "g/b", "g/d"]);
    }

    #[test]
    fn the_three_shapes_visibly_diverge_on_one_fixture() {
        // The acceptance that the abstraction is right: one fixture, one filter,
        // three distinct row-id lists.
        let t = fixture();
        let f = filter_b_d();
        let a = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            &f,
        );
        let b = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::LoadTypeForest,
            &f,
        );
        let c = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::PrunedTree,
            &f,
        );
        assert_eq!(row_ids(&a), ["g/b", "g/c", "g/d"]);
        assert_eq!(row_ids(&b), ["g/b", "g/d"]);
        assert_eq!(row_ids(&c), ["g/a", "g/b", "g/d"]);
        assert_ne!(row_ids(&a), row_ids(&b));
        assert_ne!(row_ids(&b), row_ids(&c));
        assert_ne!(row_ids(&a), row_ids(&c));
    }

    #[test]
    fn members_as_roots_with_declared_root_filter_keeps_the_orphan_pass() {
        // The default-path proof: shape (a) over the declared-root filter is the
        // pre-shape behaviour byte-for-byte, including the PROP-036 §2.12 orphan
        // separator. `g/b` is a drifted package no declared root reaches, so it
        // lands under the separator exactly as before.
        let t = tree(vec![pkg("g/a", &[]), pkg("g/b", &[])], &["g/a"]);
        let filter: BTreeSet<String> = ["g/a".to_string()].into_iter().collect();
        let rows = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            &filter,
        );
        assert_eq!(row_ids(&rows), ["g/a", "", "g/b"]);
        assert!(rows.iter().any(|r| r.node == RowNode::Separator));
    }

    #[test]
    fn load_type_forest_drops_a_member_depended_on_by_another_member() {
        // If a member depends on another member, the depended-on member is NOT a
        // root (it appears as a same-set child). Fixture: members `{b, c}`, with
        // `b→c`. `c` is depended on by `b`, so only `b` roots; its same-set child
        // `c` shows under it.
        let t = tree(vec![pkg("g/b", &["g/c"]), pkg("g/c", &[])], &[]);
        let filter: BTreeSet<String> = ["g/b".to_string(), "g/c".to_string()].into_iter().collect();
        let rows = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::LoadTypeForest,
            &filter,
        );
        assert_eq!(row_ids(&rows), ["g/b", "g/c"]);
    }

    #[test]
    fn pruned_tree_drops_a_declared_root_that_reaches_no_member() {
        // A declared root whose subtree hits no member is pruned entirely (it is
        // not on any path to a member). Fixture: roots `[a, x]`, `a→b`, `x→y`,
        // filter `{b}`. `a` reaches member `b` so it stays (`a→b`); `x` reaches
        // only `y` (not a member) so `x` is dropped.
        let t = tree(
            vec![
                pkg("g/a", &["g/b"]),
                pkg("g/b", &[]),
                pkg("g/x", &["g/y"]),
                pkg("g/y", &[]),
            ],
            &["g/a", "g/x"],
        );
        let filter: BTreeSet<String> = ["g/b".to_string()].into_iter().collect();
        let rows = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::PrunedTree,
            &filter,
        );
        assert_eq!(row_ids(&rows), ["g/a", "g/b"]);
    }

    #[test]
    fn dag_dedup_still_applies_within_members_as_roots() {
        // The shared `↩` dedup is preserved across the shape pipeline: a diamond
        // under (a) marks the second `d` once and does not re-expand it.
        let t = tree(
            vec![
                pkg("g/a", &["g/b", "g/c"]),
                pkg("g/b", &["g/d"]),
                pkg("g/c", &["g/d"]),
                pkg("g/d", &[]),
            ],
            &["g/a"],
        );
        let filter: BTreeSet<String> = ["g/a".to_string()].into_iter().collect();
        let rows = flatten(
            &t,
            &BTreeSet::new(),
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            &filter,
        );
        let dedup = super::super::theme::dag_dedup();
        let reoccurrences = rows.iter().filter(|r| r.name.contains(dedup)).count();
        assert_eq!(reoccurrences, 1, "the second g/d is marked once");
    }
}
