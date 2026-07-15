//! The interactive TUI application state and the fold-aware tree flatten
//! (PROP-036 §2.11, §2.12). [`App`] owns the model and the derived, scrollable
//! [`VisibleRow`] list; the tree flatten adapts the Phase-1
//! [`super::super::plain`] DFS (`│├└` glyphs, `(*)` DAG dedup, orphan pass) and
//! the flat modes (SubTables / Tabs) build their rows in [`super::modes`].

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use std::collections::BTreeSet;

use rat_widget::table::TableState;

use super::super::model::{LoadType, PackageTree};
use super::menu::MenuState;
use super::modes;
use super::search::SearchState;

/// The number of flat-mode partitions / tabs: `static`, `dynamic`, `no-boot`.
const TAB_COUNT: usize = 3;

/// Row ordering, shown in the status line (PROP-036 §2.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ordering {
    /// Analysis order — the declared-root DFS (the default).
    Topological,
    /// Sorted by `group/name`: roots, each node's children, and the orphan list
    /// are alphabetised before the walk; tree structure is otherwise preserved.
    Alphabetical,
}

impl Ordering {
    /// The status-line label.
    pub fn label(self) -> &'static str {
        match self {
            Ordering::Topological => "topological",
            Ordering::Alphabetical => "alphabetical",
        }
    }
}

/// Display mode, shown in the status line (PROP-036 §2.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    /// One all-together tree (the default).
    All,
    /// A flat list partitioned into `static` / `dynamic` / `no-boot` sub-tables,
    /// each under a subheader row.
    SubTables,
    /// A flat list behind `Static` / `Dynamic` / `No-boot` tabs.
    Tabs,
}

impl DisplayMode {
    /// The status-line label.
    pub fn label(self) -> &'static str {
        match self {
            DisplayMode::All => "all",
            DisplayMode::SubTables => "sub-tables",
            DisplayMode::Tabs => "tabs",
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
    /// A flat-mode section subheader (`static dependencies`, …).
    Subheader,
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
    /// The open Search Everywhere window (F1), if any (PROP-037 §7.3).
    pub search: Option<SearchState>,
    /// The open F-key selection menu (F2/F3), if any (PROP-037 §7.1/§7.2).
    pub menu: Option<MenuState>,
    /// Current row ordering (`n`).
    pub ordering: Ordering,
    /// Current display mode (`x`).
    pub display_mode: DisplayMode,
    /// Whether `static` sorts before `dynamic` in the flat modes (`t`).
    pub static_first: bool,
    /// The active tab index in [`DisplayMode::Tabs`] (`[`/`]`/`Tab`).
    pub tab: usize,
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
            search: None,
            menu: None,
            ordering: Ordering::Topological,
            display_mode: DisplayMode::All,
            static_first: true,
            tab: 0,
            fatal: None,
        };
        app.rebuild();
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
        // Folding is a tree-mode affordance; inert in the flat modes.
        if self.display_mode != DisplayMode::All {
            return;
        }
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
        self.rebuild_keep_selection();
    }

    /// Fold or unfold the whole tree (`F`). Folds every node that has children,
    /// or clears the fold set.
    pub fn toggle_fold_all(&mut self) {
        if self.display_mode != DisplayMode::All {
            return;
        }
        self.all_folded = !self.all_folded;
        self.folded.clear();
        if self.all_folded {
            for pkg in &self.tree.packages {
                if !pkg.dependencies.is_empty() {
                    self.folded.insert(pkg.id.clone());
                }
            }
        }
        self.rebuild_keep_selection();
    }

    /// Recompute the rows and clamp the selection to the new row count.
    fn rebuild_keep_selection(&mut self) {
        let prev = self.table.selected().unwrap_or(0);
        self.rebuild();
        let next = if self.rows.is_empty() {
            None
        } else {
            Some(prev.min(self.rows.len() - 1))
        };
        self.table.select(next);
    }

    /// Rebuild [`App::rows`] for the current display mode + ordering, keeping
    /// `table.rows` and the pan clamp in sync so key handling stays correct
    /// between renders (PROP-036 §2.11–§2.12).
    pub fn rebuild(&mut self) {
        self.rows = match self.display_mode {
            DisplayMode::All => super::flatten::flatten(&self.tree, &self.folded, self.ordering),
            DisplayMode::SubTables => {
                modes::subtables_rows(&self.tree, self.ordering, self.static_first)
            }
            DisplayMode::Tabs => {
                let group = modes::group_order(self.static_first)[self.tab.min(TAB_COUNT - 1)];
                modes::tab_group_rows(&self.tree, self.ordering, group)
            }
        };
        self.max_name_width = self
            .rows
            .iter()
            .map(|r| r.name.chars().count())
            .max()
            .unwrap_or(0);
        self.h_offset = self.h_offset.min(self.max_name_width);
        self.table.rows = self.rows.len();
    }

    /// Cycle the row ordering: Topological ↔ Alphabetical (`n`). Applies to
    /// every display mode.
    pub fn cycle_ordering(&mut self) {
        self.ordering = match self.ordering {
            Ordering::Topological => Ordering::Alphabetical,
            Ordering::Alphabetical => Ordering::Topological,
        };
        self.rebuild();
        self.reset_selection_top();
    }

    /// Cycle the display mode: All → SubTables → Tabs → All (`x`).
    pub fn cycle_display_mode(&mut self) {
        self.display_mode = match self.display_mode {
            DisplayMode::All => DisplayMode::SubTables,
            DisplayMode::SubTables => DisplayMode::Tabs,
            DisplayMode::Tabs => DisplayMode::All,
        };
        self.rebuild();
        self.reset_selection_top();
    }

    /// Set the display mode to a specific value (the F3 menu, PROP-037 §7.1).
    pub fn set_display_mode(&mut self, mode: DisplayMode) {
        self.display_mode = mode;
        self.rebuild();
        self.reset_selection_top();
    }

    /// Set the row ordering to a specific value (the F2 menu, PROP-037 §7.2).
    pub fn set_ordering(&mut self, ordering: Ordering) {
        self.ordering = ordering;
        self.rebuild();
        self.reset_selection_top();
    }

    /// Swap whether `static` or `dynamic` comes first in the flat modes (`t`).
    pub fn swap_priority(&mut self) {
        self.static_first = !self.static_first;
        self.rebuild();
        self.reset_selection_top();
    }

    /// Select the next tab, wrapping — [`DisplayMode::Tabs`] only (`]` / `Tab`).
    pub fn next_tab(&mut self) {
        self.step_tab(1);
    }

    /// Select the previous tab, wrapping — [`DisplayMode::Tabs`] only (`[`).
    pub fn prev_tab(&mut self) {
        self.step_tab(-1);
    }

    fn step_tab(&mut self, delta: isize) {
        if self.display_mode != DisplayMode::Tabs {
            return;
        }
        let n = TAB_COUNT as isize;
        self.tab = (((self.tab as isize + delta) % n + n) % n) as usize;
        self.rebuild();
        self.reset_selection_top();
    }

    /// Move the selection to the first selectable row and scroll to the top —
    /// after a mode / ordering / tab change.
    fn reset_selection_top(&mut self) {
        let first = self
            .rows
            .iter()
            .position(|r| matches!(r.node, RowNode::Package(_) | RowNode::Missing));
        let sel = first.or(if self.rows.is_empty() { None } else { Some(0) });
        self.table.select(sel);
        self.table.set_row_offset(0);
    }
}

/// The effective-load column label (PROP-036 §2.3).
pub(super) fn load_label(load: LoadType) -> &'static str {
    match load {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
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

    fn package_ids(app: &App) -> Vec<String> {
        app.rows
            .iter()
            .filter(|r| matches!(r.node, RowNode::Package(_)))
            .map(|r| r.id.clone())
            .collect()
    }

    #[test]
    fn alphabetical_ordering_sorts_siblings_preserving_structure() {
        // A root whose children are declared c, a, b.
        let mut app = App::new(tree(
            vec![
                pkg("g/root", &["g/c", "g/a", "g/b"]),
                pkg("g/a", &[]),
                pkg("g/b", &[]),
                pkg("g/c", &[]),
            ],
            &["g/root"],
        ));
        // Topological keeps the declared sibling order.
        assert_eq!(package_ids(&app), ["g/root", "g/c", "g/a", "g/b"]);
        // `n` → Alphabetical sorts siblings; the root stays first (structure kept).
        app.cycle_ordering();
        assert_eq!(app.ordering, Ordering::Alphabetical);
        assert_eq!(package_ids(&app), ["g/root", "g/a", "g/b", "g/c"]);
    }
}
