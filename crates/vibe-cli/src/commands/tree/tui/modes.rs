//! The partitioned display modes — SubTables and Tabs (PROP-037 §4.2/§4.3). Both
//! render through the one Tree widget now: SubTables stacks one full tree per
//! effective-load partition under a subheader; Tabs renders the active
//! partition's tree. Each block/tab is a [`super::flatten::flatten`] call over
//! the partition's member set — a pipeline configuration (PROP-037 §3.2), not a
//! bespoke flat-list renderer. Fold state is shared across blocks by package id
//! (D5): the one `folded` set is fed to every block's flatten, so folding a
//! package in one block folds it everywhere.
//!
//! The graph utility [`reachable_from_roots`] stays here — the tree flatten's
//! orphan pass (PROP-036 §2.12) consumes it.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#modes");

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::super::model::{LoadType, PackageTree};
use super::flatten::{TreeShape, flatten};
use super::state::{Ordering, RowNode, VisibleRow};

/// The three effective-load partitions of the partitioned display modes
/// (PROP-037 §4.2/§4.3).
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

/// The SubTables rows: **one full tree per non-empty effective-load partition**,
/// each under a subheader, in the user-chosen block order (PROP-037 §4.2). Each
/// block is a [`flatten`] call over the partition's member set — a real tree
/// (`│├└─` connectors, fold, DAG dedup), not a flat list. The same `folded` set
/// feeds every block (D5 — fold state is shared by package id across blocks).
pub fn subtables_rows(
    tree: &PackageTree,
    folded: &BTreeSet<String>,
    ordering: Ordering,
    shape: TreeShape,
    static_first: bool,
) -> Vec<VisibleRow> {
    let mut rows: Vec<VisibleRow> = Vec::new();
    for group in group_order(static_first) {
        let filter = group_filter(tree, group);
        if filter.is_empty() {
            continue;
        }
        rows.push(subheader_row(group));
        rows.extend(flatten(tree, folded, ordering, shape, &filter));
    }
    rows
}

/// The active tab's tree: the one partition's member set, flattened with the
/// shared fold state (PROP-037 §4.3). An empty partition renders no rows.
pub fn tab_group_rows(
    tree: &PackageTree,
    folded: &BTreeSet<String>,
    ordering: Ordering,
    shape: TreeShape,
    group: LoadGroup,
) -> Vec<VisibleRow> {
    let filter = group_filter(tree, group);
    if filter.is_empty() {
        return Vec::new();
    }
    flatten(tree, folded, ordering, shape, &filter)
}

/// The package ids whose effective load type falls in `group` — the partition's
/// member set, the `filter` fed into that block/tab's [`flatten`].
fn group_filter(tree: &PackageTree, group: LoadGroup) -> BTreeSet<String> {
    tree.packages
        .iter()
        .filter(|p| LoadGroup::of(p.load.load_type) == group)
        .map(|p| p.id.clone())
        .collect()
}

/// The set of package ids reachable from any declared root over the full
/// dependency graph (folds ignored). Used by the tree flatten's orphan pass in
/// [`super::flatten`]; a shared graph utility. Cycle-guarded on the id key.
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

    /// A package `id` of effective load `load` depending on `deps` (ids), no
    /// condition. `in_static_md`/`in_index_md` mirror the load type so the `S`
    /// column is consistent, but partitioning keys only off `load_type`.
    fn pkg(id: &str, load: LoadType, deps: &[&str]) -> Package {
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
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Build a `PackageTree` over `packages` with the given declared `roots`.
    /// Tests pass roots that reach every package so the MembersAsRoots orphan
    /// pass is a no-op (the orphan pass keys off declared roots, not the
    /// partition filter).
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

    /// True if the row's name carries a tree connector (`└─`/`├─`).
    fn has_connector(row: &VisibleRow) -> bool {
        row.name.contains('\u{2514}') || row.name.contains('\u{251c}')
    }

    #[test]
    fn subtables_renders_one_stacked_tree_per_load_group() {
        // Two load groups, each a 2-level chain, every package reached from a
        // declared root (so the MembersAsRoots orphan pass adds nothing).
        let t = tree(
            vec![
                pkg("g/s1", LoadType::Static, &["g/s2"]),
                pkg("g/s2", LoadType::Static, &[]),
                pkg("g/d1", LoadType::Dynamic, &["g/d2"]),
                pkg("g/d2", LoadType::Dynamic, &[]),
            ],
            &["g/s1", "g/d1"],
        );
        let folded = BTreeSet::new();
        let rows = subtables_rows(
            &t,
            &folded,
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            true,
        );

        // Two subheaders, in the user-chosen block order.
        let subheaders: Vec<&str> = rows
            .iter()
            .filter(|r| r.node == RowNode::Subheader)
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(
            subheaders,
            ["static dependencies", "dynamic dependencies"],
            "one subheader per non-empty partition, in block order"
        );

        // Each block is a TREE, not a flat list: the child rows carry `└─`/`├─`
        // connectors. A flat list would have no connectors at all.
        let connected: Vec<&VisibleRow> = rows
            .iter()
            .filter(|r| matches!(r.node, RowNode::Package(_)) && has_connector(r))
            .collect();
        assert!(
            !connected.is_empty(),
            "SubTables renders trees with connectors, not a flat list"
        );
        assert!(
            connected.iter().any(|r| r.id == "g/s2"),
            "the static child is a tree node"
        );
        assert!(
            connected.iter().any(|r| r.id == "g/d2"),
            "the dynamic child is a tree node"
        );
    }

    #[test]
    fn t_swap_puts_dynamic_block_first() {
        let t = tree(
            vec![
                pkg("g/s", LoadType::Static, &[]),
                pkg("g/d", LoadType::Dynamic, &[]),
            ],
            &["g/s", "g/d"],
        );
        let folded = BTreeSet::new();
        let rows = subtables_rows(
            &t,
            &folded,
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            false,
        );
        let first_subheader = rows
            .iter()
            .find(|r| matches!(r.node, RowNode::Subheader))
            .map(|r| r.name.as_str());
        assert_eq!(first_subheader, Some("dynamic dependencies"));
    }

    #[test]
    fn empty_group_is_skipped() {
        // No `none` packages -> no "no-boot" subheader or block.
        let t = tree(
            vec![
                pkg("g/s", LoadType::Static, &[]),
                pkg("g/d", LoadType::Dynamic, &[]),
            ],
            &["g/s", "g/d"],
        );
        let folded = BTreeSet::new();
        let rows = subtables_rows(
            &t,
            &folded,
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            true,
        );
        assert!(!rows.iter().any(|r| r.name == "no-boot"));
    }

    #[test]
    fn tabs_active_tab_renders_that_partitions_tree() {
        // The Static tab shows the static partition's tree (g/s1 -> g/s2), with
        // connectors, and NOT the dynamic package (a different partition).
        let t = tree(
            vec![
                pkg("g/s1", LoadType::Static, &["g/s2"]),
                pkg("g/s2", LoadType::Static, &[]),
                pkg("g/d1", LoadType::Dynamic, &[]),
            ],
            &["g/s1", "g/d1"],
        );
        let folded = BTreeSet::new();
        let rows = tab_group_rows(
            &t,
            &folded,
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            LoadGroup::Static,
        );
        let ids: Vec<&str> = rows
            .iter()
            .filter(|r| matches!(r.node, RowNode::Package(_)))
            .map(|r| r.id.as_str())
            .collect();
        assert!(ids.contains(&"g/s1"));
        assert!(ids.contains(&"g/s2"));
        assert!(
            rows.iter().any(|r| r.id == "g/s2" && has_connector(r)),
            "the static child carries a tree connector (a tree, not a flat list)"
        );
        assert!(
            !ids.contains(&"g/d1"),
            "the active tab shows only its own partition"
        );
    }

    #[test]
    fn folding_in_subtables_is_shared_across_every_block() {
        // g/s(static) -> g/d(dynamic) -> g/n(none). g/d carries the child g/n
        // and appears in TWO blocks: the static block (as a transitive dep of
        // g/s) and the dynamic block (as a root). Folding g/d must collapse it
        // in EVERY block -- the one `folded` set feeds every flatten (D5).
        let t = tree(
            vec![
                pkg("g/s", LoadType::Static, &["g/d"]),
                pkg("g/d", LoadType::Dynamic, &["g/n"]),
                pkg("g/n", LoadType::None, &[]),
            ],
            &["g/s"],
        );

        // Unfolded: g/n shows under g/d in both the static and dynamic blocks
        // (plus its own no-boot root).
        let unfolded = BTreeSet::new();
        let rows = subtables_rows(
            &t,
            &unfolded,
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            true,
        );
        assert!(
            rows.iter().filter(|r| r.id == "g/n").count() >= 2,
            "g/n visible under g/d in multiple blocks before folding"
        );

        // Fold g/d: it collapses everywhere, hiding g/n under it in every block.
        let mut folded = BTreeSet::new();
        folded.insert("g/d".to_string());
        let rows = subtables_rows(
            &t,
            &folded,
            Ordering::Topological,
            TreeShape::MembersAsRoots,
            true,
        );
        let gd_rows: Vec<&VisibleRow> = rows.iter().filter(|r| r.id == "g/d").collect();
        assert!(
            gd_rows.len() >= 2,
            "g/d still anchors every block it appears in"
        );
        assert!(
            gd_rows
                .iter()
                .all(|r| r.name.contains(super::super::theme::fold_collapsed())),
            "g/d shows the collapsed glyph in every block (shared fold set)"
        );
        assert_eq!(
            rows.iter().filter(|r| r.id == "g/n").count(),
            1,
            "folding g/d hid g/n under it in every block; only g/n's own no-boot root remains"
        );
    }
}
