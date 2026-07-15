//! The fold-aware DAG → visible-rows flatten (PROP-036 §2.12): a declared-root
//! DFS with `│├└` connectors, `(*)` re-occurrence dedup, and an orphan pass so a
//! package that no declared root reaches is still shown. Extracted from `state`
//! as its own cell (the tree-shaping algorithm, distinct from the app state).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use std::collections::{BTreeMap, BTreeSet};

use super::super::model::{Package, PackageTree};
use super::modes;
use super::state::{Ordering, RowNode, VisibleRow, load_label};

/// Flatten the DAG into visible rows given a fold set (PROP-036 §2.12). Walks
/// each declared root, then an orphan pass for anything not reached — so the
/// view never hides a package.
pub(super) fn flatten(
    tree: &PackageTree,
    folded: &BTreeSet<String>,
    ordering: Ordering,
) -> Vec<VisibleRow> {
    let by_id: BTreeMap<&str, usize> = tree
        .packages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();

    let mut rows: Vec<VisibleRow> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    // Alphabetical ordering sorts the siblings (here, the roots) by group/name;
    // structure is otherwise preserved (PROP-036 §2.11).
    let mut roots: Vec<&str> = tree.roots.iter().map(|s| s.as_str()).collect();
    if ordering == Ordering::Alphabetical {
        roots.sort_unstable();
    }
    let root_count = roots.len();
    for (i, &root) in roots.iter().enumerate() {
        let is_last = i + 1 == root_count;
        walk(
            root, "", is_last, true, tree, &by_id, folded, ordering, &mut seen, &mut rows,
        );
    }

    // Orphan pass: any package genuinely not reachable from a declared root
    // (e.g. a drifted lock root) is still shown (PROP-036 §2.12). Reachability
    // is judged over the FULL graph, ignoring folds — a folded-away subtree is
    // hidden, not resurrected here.
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
                &pkg.id, "", is_last, true, tree, &by_id, folded, ordering, &mut seen, &mut rows,
            );
        }
    }
    rows
}

/// Depth-first walk producing rows. Marks a re-occurrence `(*)` and does not
/// re-expand it (DAG dedup), and a folded node `+` (does not recurse). Mirrors
/// [`super::super::plain`]'s connector/prefix construction.
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
    let has_children = !pkg.dependencies.is_empty();
    let is_folded = folded.contains(id);
    // The expand indicator is shown only on a first-seen node that has
    // children; a re-occurrence is a display leaf (its subtree lives elsewhere).
    let indicator = if has_children && !repeated {
        if is_folded { "+ " } else { "- " }
    } else {
        ""
    };
    let marker = if repeated { " (*)" } else { "" };
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
    // Alphabetical ordering sorts a node's children by group/name.
    let mut deps: Vec<&str> = pkg.dependencies.iter().map(|s| s.as_str()).collect();
    if ordering == Ordering::Alphabetical {
        deps.sort_unstable();
    }
    let n = deps.len();
    for (i, &dep) in deps.iter().enumerate() {
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
            seen,
            rows,
        );
    }
}
