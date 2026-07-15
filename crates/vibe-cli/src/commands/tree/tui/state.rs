//! The interactive TUI application state and the fold-aware flatten
//! (PROP-036 §2.11, §2.12).
//!
//! [`App`] owns the model and the derived, scrollable [`VisibleRow`] list. The
//! flatten adapts the Phase-1 [`super::super::plain`] DFS — the same `│├└`
//! glyphs, `(*)` DAG dedup, and orphan pass — into "given a fold set, compute
//! the visible rows", adding a `+`/`-` expand indicator per node.
//!
//! The [`Ordering`] and [`DisplayMode`] enums each carry only their Phase-2
//! variant today; Phase 3 (the `n` ordering toggle and the `x` display-mode
//! cycle) adds variants and key handlers, never a restructure.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rat_widget::table::TableState;

use super::super::model::{LoadType, Package, PackageTree};

/// Row ordering, shown in the status line (PROP-036 §2.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ordering {
    /// Analysis order — the declared-root DFS. The Phase-2 default and, today,
    /// the only variant.
    Topological,
    // Phase 3: `Alphabetical` — the `n` toggle adds this variant here.
}

impl Ordering {
    /// The status-line label.
    pub fn label(self) -> &'static str {
        match self {
            Ordering::Topological => "topological",
        }
    }
}

/// Display mode, shown in the status line (PROP-036 §2.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    /// One all-together tree. The Phase-2 default and, today, the only variant.
    All,
    // Phase 3: `SubTables` / `Tabs` — the `x` cycle adds these variants here.
}

impl DisplayMode {
    /// The status-line label.
    pub fn label(self) -> &'static str {
        match self {
            DisplayMode::All => "all",
        }
    }
}

/// What a visible row points at. `Copy` so it can be read out from behind a
/// shared borrow of [`App::rows`] without moving the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowNode {
    /// A resolved package — an index into [`PackageTree::packages`].
    Package(usize),
    /// A dependency edge whose target is not in the lockfile.
    Missing,
    /// The "not reached from a declared root" divider (§2.12 orphan pass).
    Separator,
}

/// One flattened, rendered tree row. Owns its drawn strings so the derived
/// list outlives any borrow of the model during a render pass.
#[derive(Debug, Clone)]
pub struct VisibleRow {
    /// What this row is.
    pub node: RowNode,
    /// The bare identity (`group/name`, or the edge target for a missing node;
    /// empty for the separator) — used by the detail modal.
    pub id: String,
    /// The drawn name cell: prefix + connector + `+`/`-` indicator + id +
    /// `(*)` re-occurrence marker.
    pub name: String,
    /// The effective-load column label (meaningful for `Package` rows).
    pub load: &'static str,
    /// `T` — transitive-static flag.
    pub transitive: bool,
    /// `C` — `when`-condition flag.
    pub condition: bool,
    /// `S` — physically in `STATIC.md`.
    pub in_static: bool,
}

/// The interactive TUI application state (PROP-036 §2.11).
pub struct App {
    /// The analysed model (owned).
    pub tree: PackageTree,
    /// The flattened visible rows — recomputed on every fold change.
    pub rows: Vec<VisibleRow>,
    /// Node keys (`group/name`) the user has collapsed.
    pub folded: BTreeSet<String>,
    /// Whole-tree fold toggle backing `F`.
    pub all_folded: bool,
    /// rat-widget row selection + vertical scroll offset.
    pub table: TableState,
    /// Horizontal pan of the name column, in characters (`←`/`→`).
    pub h_offset: usize,
    /// The widest name cell — clamps the horizontal pan.
    pub max_name_width: usize,
    /// Whether the detail modal is open.
    pub modal_open: bool,
    /// Current ordering (Phase 2: always [`Ordering::Topological`]).
    pub ordering: Ordering,
    /// Current display mode (Phase 2: always [`DisplayMode::All`]).
    pub display_mode: DisplayMode,
    /// A fatal error captured by the error handler, re-raised after the loop
    /// restores the terminal.
    pub fatal: Option<anyhow::Error>,
}

impl App {
    /// Build the app over an already-analysed model and flatten it once.
    pub fn new(tree: PackageTree) -> Self {
        let mut app = App {
            tree,
            rows: Vec::new(),
            folded: BTreeSet::new(),
            all_folded: false,
            table: TableState::default(),
            h_offset: 0,
            max_name_width: 0,
            modal_open: false,
            ordering: Ordering::Topological,
            display_mode: DisplayMode::All,
            fatal: None,
        };
        app.reflatten();
        app
    }

    /// The visible row under the selection, if any.
    pub fn selected_row(&self) -> Option<&VisibleRow> {
        let idx = self.table.selected()?;
        self.rows.get(idx)
    }

    /// Toggle the fold state of the selected node (`Space`). Only package rows
    /// that actually have children fold.
    pub fn toggle_fold_selected(&mut self) {
        let Some(idx) = self.table.selected() else {
            return;
        };
        let node = match self.rows.get(idx) {
            Some(row) => row.node,
            None => return,
        };
        let RowNode::Package(i) = node else {
            return;
        };
        let id = match self.tree.packages.get(i) {
            Some(pkg) if !pkg.dependencies.is_empty() => pkg.id.clone(),
            _ => return,
        };
        if !self.folded.remove(&id) {
            self.folded.insert(id);
        }
        self.reflatten_keep_selection();
    }

    /// Fold or unfold the whole tree (`F`). Folds every node that has children,
    /// or clears the fold set.
    pub fn toggle_fold_all(&mut self) {
        self.all_folded = !self.all_folded;
        self.folded.clear();
        if self.all_folded {
            for pkg in &self.tree.packages {
                if !pkg.dependencies.is_empty() {
                    self.folded.insert(pkg.id.clone());
                }
            }
        }
        self.reflatten_keep_selection();
    }

    /// Recompute the rows and clamp the selection to the new row count.
    fn reflatten_keep_selection(&mut self) {
        let prev = self.table.selected().unwrap_or(0);
        self.reflatten();
        let next = if self.rows.is_empty() {
            None
        } else {
            Some(prev.min(self.rows.len() - 1))
        };
        self.table.select(next);
    }

    /// Rebuild [`App::rows`] from the model, honouring the fold set
    /// (PROP-036 §2.12). Keeps `table.rows` and the pan clamp in sync so key
    /// handling stays correct between renders.
    pub fn reflatten(&mut self) {
        self.rows = flatten(&self.tree, &self.folded);
        self.max_name_width = self
            .rows
            .iter()
            .map(|r| r.name.chars().count())
            .max()
            .unwrap_or(0);
        self.h_offset = self.h_offset.min(self.max_name_width);
        self.table.rows = self.rows.len();
    }
}

/// The effective-load column label (PROP-036 §2.3).
fn load_label(load: LoadType) -> &'static str {
    match load {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
    }
}

/// Flatten the DAG into visible rows given a fold set (PROP-036 §2.12). Walks
/// each declared root, then an orphan pass for anything not reached — so the
/// view never hides a package.
fn flatten(tree: &PackageTree, folded: &BTreeSet<String>) -> Vec<VisibleRow> {
    let by_id: BTreeMap<&str, usize> = tree
        .packages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();

    let mut rows: Vec<VisibleRow> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    let root_count = tree.roots.len();
    for (i, root) in tree.roots.iter().enumerate() {
        let is_last = i + 1 == root_count;
        walk(
            root, "", is_last, true, tree, &by_id, folded, &mut seen, &mut rows,
        );
    }

    // Orphan pass: any package genuinely not reachable from a declared root
    // (e.g. a drifted lock root) is still shown (PROP-036 §2.12). Reachability
    // is judged over the FULL graph, ignoring folds — a folded-away subtree is
    // hidden, not resurrected here.
    let reachable = reachable_from_roots(tree, &by_id);
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
                &pkg.id, "", is_last, true, tree, &by_id, folded, &mut seen, &mut rows,
            );
        }
    }
    rows
}

/// The set of package ids reachable from any declared root over the full
/// dependency graph (folds ignored). Cycle-guarded on the `group/name` key.
fn reachable_from_roots(tree: &PackageTree, by_id: &BTreeMap<&str, usize>) -> BTreeSet<String> {
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
    let n = pkg.dependencies.len();
    for (i, dep) in pkg.dependencies.iter().enumerate() {
        let dep_last = i + 1 == n;
        walk(
            dep,
            &child_prefix,
            dep_last,
            false,
            tree,
            by_id,
            folded,
            seen,
            rows,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::model::*;

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

    #[test]
    fn folding_a_parent_hides_its_subtree() {
        let app_tree = tree(vec![pkg("g/a", &["g/b"]), pkg("g/b", &[])], &["g/a"]);
        let mut app = App::new(app_tree);
        assert_eq!(app.rows.len(), 2, "root + child visible");
        // Select and fold the root.
        app.table.select(Some(0));
        app.toggle_fold_selected();
        assert_eq!(app.rows.len(), 1, "child hidden under a folded root");
        assert!(app.rows[0].name.contains("+ "), "folded node shows `+`");
        // Unfold restores.
        app.toggle_fold_selected();
        assert_eq!(app.rows.len(), 2);
        assert!(app.rows[0].name.contains("- "), "unfolded node shows `-`");
    }

    #[test]
    fn a_diamond_marks_the_reoccurrence_and_does_not_reexpand() {
        // a -> b, a -> c, b -> d, c -> d.
        let app = App::new(tree(
            vec![
                pkg("g/a", &["g/b", "g/c"]),
                pkg("g/b", &["g/d"]),
                pkg("g/c", &["g/d"]),
                pkg("g/d", &[]),
            ],
            &["g/a"],
        ));
        let reoccurrences = app.rows.iter().filter(|r| r.name.contains("(*)")).count();
        assert_eq!(reoccurrences, 1, "the second `g/d` is marked once");
    }

    #[test]
    fn an_orphan_is_shown_under_a_separator() {
        // `g/b` is a package no root reaches.
        let app = App::new(tree(vec![pkg("g/a", &[]), pkg("g/b", &[])], &["g/a"]));
        assert!(app.rows.iter().any(|r| r.node == RowNode::Separator));
        assert!(app.rows.iter().any(|r| r.id == "g/b"));
    }
}
