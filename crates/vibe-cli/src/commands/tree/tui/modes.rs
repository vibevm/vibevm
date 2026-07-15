//! The flat display modes — SubTables and Tabs (PROP-036 §2.11). Both collapse
//! the DAG to one row per package (§2.12), ordered by the current [`Ordering`]
//! and partitioned by effective load type into `static` / `dynamic` / `no-boot`
//! groups. The `t` priority swap reorders the two boot groups; `no-boot` is
//! always last.
//!
//! The tree mode ([`DisplayMode::All`]) keeps its own fold-aware walk in
//! [`super::state`]; this module is only the flat builders.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::super::model::{LoadType, Package, PackageTree};
use super::state::{Ordering, RowNode, VisibleRow, load_label};

/// The three effective-load partitions of the flat display modes (§2.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadGroup {
    /// Effective `static`.
    Static,
    /// Effective `dynamic`.
    Dynamic,
    /// Ships no boot snippet (`none`).
    NoBoot,
}

impl LoadGroup {
    /// The SubTables subheader label.
    pub fn subheader(self) -> &'static str {
        match self {
            LoadGroup::Static => "static dependencies",
            LoadGroup::Dynamic => "dynamic dependencies",
            LoadGroup::NoBoot => "no-boot",
        }
    }

    /// The Tabs tab title.
    pub fn tab_label(self) -> &'static str {
        match self {
            LoadGroup::Static => "Static",
            LoadGroup::Dynamic => "Dynamic",
            LoadGroup::NoBoot => "No-boot",
        }
    }

    /// The group a package's effective load type falls in.
    fn of(load: LoadType) -> LoadGroup {
        match load {
            LoadType::Static => LoadGroup::Static,
            LoadType::Dynamic => LoadGroup::Dynamic,
            LoadType::None => LoadGroup::NoBoot,
        }
    }
}

/// The section / tab order given the static-vs-dynamic priority (`t`). `NoBoot`
/// is always last; `t` swaps only the two boot groups.
pub fn group_order(static_first: bool) -> [LoadGroup; 3] {
    if static_first {
        [LoadGroup::Static, LoadGroup::Dynamic, LoadGroup::NoBoot]
    } else {
        [LoadGroup::Dynamic, LoadGroup::Static, LoadGroup::NoBoot]
    }
}

/// The SubTables rows: the flat, ordered package list partitioned by effective
/// load type, each non-empty group preceded by a subheader row (PROP-036 §2.11).
pub fn subtables_rows(
    tree: &PackageTree,
    ordering: Ordering,
    static_first: bool,
) -> Vec<VisibleRow> {
    let indices = ordered_indices(tree, ordering);
    let mut rows: Vec<VisibleRow> = Vec::new();
    for group in group_order(static_first) {
        let members: Vec<(usize, &Package)> = indices
            .iter()
            .filter_map(|&i| tree.packages.get(i).map(|p| (i, p)))
            .filter(|(_, p)| LoadGroup::of(p.load.load_type) == group)
            .collect();
        if members.is_empty() {
            continue;
        }
        rows.push(subheader_row(group));
        rows.extend(members.into_iter().map(|(i, p)| flat_row(i, p)));
    }
    rows
}

/// The flat package list for one tab's group, in the current ordering.
pub fn tab_group_rows(tree: &PackageTree, ordering: Ordering, group: LoadGroup) -> Vec<VisibleRow> {
    ordered_indices(tree, ordering)
        .into_iter()
        .filter_map(|i| tree.packages.get(i).map(|p| (i, p)))
        .filter(|(_, p)| LoadGroup::of(p.load.load_type) == group)
        .map(|(i, p)| flat_row(i, p))
        .collect()
}

/// The unique package indices in the current ordering (PROP-036 §2.11).
pub fn ordered_indices(tree: &PackageTree, ordering: Ordering) -> Vec<usize> {
    match ordering {
        Ordering::Alphabetical => {
            let mut idx: Vec<usize> = (0..tree.packages.len()).collect();
            idx.sort_by(|&a, &b| tree.packages[a].id.cmp(&tree.packages[b].id));
            idx
        }
        Ordering::Topological => topological_indices(tree),
    }
}

/// First-seen order in the declared-root DFS, then any unreached package
/// (sorted by id) — the analysis order.
fn topological_indices(tree: &PackageTree) -> Vec<usize> {
    let by_id: BTreeMap<&str, usize> = tree
        .packages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();
    let mut order: Vec<usize> = Vec::new();
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    for root in &tree.roots {
        dfs(root, &by_id, tree, &mut seen, &mut order);
    }
    let mut rest: Vec<usize> = (0..tree.packages.len())
        .filter(|i| !seen.contains(i))
        .collect();
    rest.sort_by(|&a, &b| tree.packages[a].id.cmp(&tree.packages[b].id));
    order.extend(rest);
    order
}

/// Collect first-seen package indices, cycle-guarded on the index.
fn dfs(
    id: &str,
    by_id: &BTreeMap<&str, usize>,
    tree: &PackageTree,
    seen: &mut BTreeSet<usize>,
    order: &mut Vec<usize>,
) {
    let Some(&idx) = by_id.get(id) else {
        return;
    };
    if !seen.insert(idx) {
        return;
    }
    order.push(idx);
    if let Some(pkg) = tree.packages.get(idx) {
        for dep in &pkg.dependencies {
            dfs(dep, by_id, tree, seen, order);
        }
    }
}

/// The set of package ids reachable from any declared root over the full
/// dependency graph (folds ignored). Used by the tree flatten's orphan test in
/// [`super::state`]; a shared graph utility. Cycle-guarded on the id key.
pub(super) fn reachable_from_roots(
    tree: &PackageTree,
    by_id: &BTreeMap<&str, usize>,
) -> BTreeSet<String> {
    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<String> = tree.roots.iter().cloned().collect();
    while let Some(id) = queue.pop_front() {
        if !reached.insert(id.clone()) {
            continue;
        }
        if let Some(&idx) = by_id.get(id.as_str())
            && let Some(pkg) = tree.packages.get(idx)
        {
            for dep in &pkg.dependencies {
                if !reached.contains(dep) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }
    reached
}

/// A flat (no tree glyphs) package row.
fn flat_row(idx: usize, p: &Package) -> VisibleRow {
    VisibleRow {
        node: RowNode::Package(idx),
        id: p.id.clone(),
        name: p.id.clone(),
        load: load_label(p.load.load_type),
        transitive: p.load.transitive,
        condition: p.condition.present,
        in_static: p.load.in_static_md,
    }
}

/// A subheader label row (its value/checkbox columns are blank).
fn subheader_row(group: LoadGroup) -> VisibleRow {
    VisibleRow {
        node: RowNode::Subheader,
        id: String::new(),
        name: group.subheader().to_string(),
        load: "",
        transitive: false,
        condition: false,
        in_static: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::model::*;
    use crate::commands::tree::tui::state::{Ordering, RowNode};

    fn pkg(id: &str, load: LoadType) -> Package {
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
                load_type: load,
                transitive: false,
                declared: None,
                origin: LoadOrigin::None,
                in_static_md: matches!(load, LoadType::Static),
                in_index_md: matches!(load, LoadType::Dynamic),
                boot_path: None,
            },
            condition: Condition::absent(),
            dependencies: Vec::new(),
        }
    }

    fn tree(packages: Vec<Package>) -> PackageTree {
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
            roots: Vec::new(),
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

    #[test]
    fn subtables_partition_by_load_type_with_subheaders() {
        let t = tree(vec![
            pkg("g/s", LoadType::Static),
            pkg("g/d", LoadType::Dynamic),
            pkg("g/n", LoadType::None),
        ]);
        let rows = subtables_rows(&t, Ordering::Alphabetical, true);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "static dependencies",
                "g/s",
                "dynamic dependencies",
                "g/d",
                "no-boot",
                "g/n",
            ]
        );
    }

    #[test]
    fn t_swap_puts_dynamic_first() {
        let t = tree(vec![
            pkg("g/s", LoadType::Static),
            pkg("g/d", LoadType::Dynamic),
        ]);
        let rows = subtables_rows(&t, Ordering::Alphabetical, false);
        let first_subheader = rows
            .iter()
            .find(|r| matches!(r.node, RowNode::Subheader))
            .map(|r| r.name.as_str());
        assert_eq!(first_subheader, Some("dynamic dependencies"));
    }

    #[test]
    fn empty_group_is_skipped() {
        // No `none` packages → no "no-boot" subheader.
        let t = tree(vec![
            pkg("g/s", LoadType::Static),
            pkg("g/d", LoadType::Dynamic),
        ]);
        let rows = subtables_rows(&t, Ordering::Alphabetical, true);
        assert!(!rows.iter().any(|r| r.name == "no-boot"));
    }

    #[test]
    fn tab_group_rows_filter_to_one_group() {
        let t = tree(vec![
            pkg("g/s", LoadType::Static),
            pkg("g/d", LoadType::Dynamic),
            pkg("g/n", LoadType::None),
        ]);
        let rows = tab_group_rows(&t, Ordering::Alphabetical, LoadGroup::Dynamic);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "g/d");
        assert!(matches!(rows[0].node, RowNode::Package(_)));
    }
}
