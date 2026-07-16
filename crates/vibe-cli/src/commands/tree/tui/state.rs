//! The interactive TUI application state and the fold-aware tree flatten
//! (PROP-036 §2.11, §2.12). [`App`] owns the model and the derived, scrollable
//! [`VisibleRow`] list; the tree flatten adapts the Phase-1
//! [`super::super::plain`] DFS (`│├└` glyphs, `(*)` DAG dedup, orphan pass); the
//! partitioned modes (SubTables / Tabs) build their stacked / per-tab trees in
//! [`super::modes`] over the one [`super::flatten`] walk — every mode renders a
//! tree (PROP-037 §3.1, §4).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use std::collections::BTreeSet;

use rat_widget::table::TableState;

use super::super::model::PackageTree;
// `TreeShape` is the shape stage of the PROP-037 §3.2/§3.3 pipeline, selected
// per context by the F2 sort menu (§7.2) and carried into the flatten walk.
use super::copy::{CopySettings, FileDest};
use super::flatten::TreeShape;
use super::menu::MenuState;
use super::modes;
use super::search::SearchState;
use super::settings::TreeSettings;
use super::theme::Theme;

pub(super) use super::row::load_label;
pub use super::row::{RowNode, VisibleRow};

/// The number of partition tabs: `static`, `dynamic`, `no-boot`.
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

/// Display mode, shown in the status line (PROP-036 §2.11, PROP-037 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    /// One all-together tree (the default).
    All,
    /// Several trees stacked vertically — one per effective-load partition, each
    /// under a subheader (PROP-037 §4.2).
    SubTables,
    /// One tree per tab — the active tab shows that partition's tree
    /// (PROP-037 §4.3).
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

/// The interactive TUI application state (PROP-036 §2.11).
pub struct App {
    /// The analysed model (owned).
    pub tree: PackageTree,
    /// The active theme — the single source of colour, glyphs, and rendering
    /// tier (PROP-037 §2.2, §9). Built from the resolved `vibe.tree.palette` /
    /// `vibe.tree.tier` settings on launch; every renderer reads it by
    /// reference. Defaults to Rosé Pine / Tier 3 when no settings are loaded.
    pub theme: Theme,
    /// The settings cell, present on the live launch path and `None` in unit
    /// tests (so a model mutator that persists — e.g. the F2 menu — is a no-op
    /// against the disk in tests). Carries the `vibe.tree.*` schema + paths.
    pub settings: Option<TreeSettings>,
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
    /// The open copy-settings modal (Shift+F6) — a depth-2 captive copy field (§10.2).
    pub copy_settings: Option<CopySettings>,
    /// The open file-dest modal (depth-2 over copy-settings); Esc returns to it (§10.5).
    pub file_dest: Option<FileDest>,
    /// Whether the quit-confirm dialog is open (PROP-037 §7.4 `#quit-confirm`).
    /// A bare `Esc` at the base screen opens it; `Enter` activates the focused
    /// button (OK quits, Cancel cancels), `Esc` cancels, Tab/←/→ move focus.
    pub confirm_quit: bool,
    /// The focused button in the quit-confirm dialog (PROP-037 §7.4): `false` =
    /// OK (the default), `true` = Cancel. Tab/←/→ toggle it.
    pub confirm_cancel_focused: bool,
    /// A transient footer flash (e.g. an F6-copy confirmation); cleared on the
    /// next input event (PROP-037 §10).
    pub flash: Option<String>,
    /// Current row ordering (`n`).
    pub ordering: Ordering,
    /// The active tree shape (PROP-037 §3.3 `#tree-shapes`) — the forest policy
    /// carried into every `flatten`, in every mode. Selected per context by the
    /// F2 sort menu (§7.2); defaults to [`TreeShape::MembersAsRoots`] (the
    /// byte-identical continuation of the pre-shape walk).
    pub shape: TreeShape,
    /// Current display mode (`x`).
    pub display_mode: DisplayMode,
    /// Whether `static` sorts before `dynamic` in the partitioned modes (`t`).
    pub static_first: bool,
    /// The active tab index in [`DisplayMode::Tabs`] (`Shift+←`/`Shift+→`).
    pub tab: usize,
    /// A fatal error captured by the error handler, re-raised after the loop
    /// restores the terminal.
    pub fatal: Option<anyhow::Error>,
}

impl App {
    /// Build the app over an already-analysed model and flatten it once. The
    /// theme defaults to Rosé Pine / Tier 3 and `settings` is `None` — the
    /// launch path ([`super::run`]) follows this with [`App::apply_prefs`] to
    /// load the resolved `vibe.tree.*` settings and re-skin/re-mode the app.
    pub fn new(tree: PackageTree) -> Self {
        let mut app = App {
            tree,
            theme: Theme::default(),
            settings: None,
            rows: Vec::new(),
            folded: BTreeSet::new(),
            all_folded: false,
            table: TableState::default(),
            h_offset: 0,
            max_name_width: 0,
            modal_open: false,
            search: None,
            menu: None,
            copy_settings: None,
            file_dest: None,
            confirm_quit: false,
            confirm_cancel_focused: false,
            flash: None,
            ordering: Ordering::Topological,
            shape: TreeShape::default(),
            display_mode: DisplayMode::All,
            static_first: true,
            tab: 0,
            fatal: None,
        };
        app.rebuild();
        app
    }

    /// Load the resolved `vibe.tree.*` settings and apply them: rebuild the
    /// theme (palette + tier) and set the model's mode / sort / shape /
    /// static-first from the snapshot, then reflatten (PROP-037 §9). Stores
    /// the [`TreeSettings`] on the app so later menu changes can persist back.
    /// Called by the launch path after [`App::new`]; a no-op for the theme when
    /// settings are absent/corrupt (the defaults already in place win).
    pub fn apply_prefs(&mut self, settings: TreeSettings) {
        let prefs = settings.load();
        self.theme = settings.theme(&prefs);
        let snap = settings.snapshot(&prefs);
        self.display_mode = snap.mode;
        self.ordering = snap.sort;
        self.shape = snap.shape;
        self.static_first = snap.static_first;
        self.settings = Some(settings);
        self.rebuild();
        self.reset_selection_top();
    }

    /// The visible row under the selection, if any.
    pub fn selected_row(&self) -> Option<&VisibleRow> {
        let idx = self.table.selected()?;
        self.rows.get(idx)
    }

    /// Toggle the fold state of the selected node (`Space`). Every display mode
    /// renders through the one Tree widget (PROP-037 §3.1, §4), so Space folds in
    /// all of them; only package rows that actually have children fold, and a
    /// subheader / separator selection is a no-op.
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
        self.rebuild_keep_selection();
    }

    /// Fold or unfold the whole tree (`F`). Folds every node that has children,
    /// or clears the fold set. Applies in every mode (each is a tree).
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
    /// between renders (PROP-036 §2.11–§2.12, PROP-037 §4). Every mode runs the
    /// one [`super::flatten`] walk over a partition-specific filter; the shared
    /// `folded` set feeds every block so fold state is global by package id (D5).
    pub fn rebuild(&mut self) {
        let glyphs = self.theme.glyphs();
        self.rows = match self.display_mode {
            // Tree mode = the active shape over the declared-root filter, which
            // reproduces the pre-shape walk byte-for-byte under the default
            // members-as-roots shape (the member set equals the declared roots,
            // so (a)'s root set + declared-root orphan pass is exactly the
            // PROP-036 §2.12 flatten). Phase 5+ will swap this filter for the
            // search/selection set.
            DisplayMode::All => {
                let filter: BTreeSet<String> = self.tree.roots.iter().cloned().collect();
                super::flatten::flatten(
                    &self.tree,
                    &self.folded,
                    self.ordering,
                    self.shape,
                    &filter,
                    glyphs,
                )
            }
            // SubTables = several trees stacked vertically, one per effective-
            // load partition under a subheader (PROP-037 §4.2). Each block is a
            // full `flatten` over that partition's member set.
            DisplayMode::SubTables => modes::subtables_rows(
                &self.tree,
                &self.folded,
                self.ordering,
                self.shape,
                self.static_first,
                glyphs,
            ),
            // Tabs = one tree per tab; the active tab shows that partition's
            // tree (PROP-037 §4.3).
            DisplayMode::Tabs => {
                let group = modes::group_order(self.static_first)[self.tab.min(TAB_COUNT - 1)];
                modes::tab_group_rows(
                    &self.tree,
                    &self.folded,
                    self.ordering,
                    self.shape,
                    group,
                    glyphs,
                )
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

    /// Set the tree shape to a specific value (the F2 sort menu, PROP-037 §3.3
    /// `#tree-shapes`). Mirrors [`App::set_ordering`]: rebuild + reset selection.
    #[allow(dead_code)] // selected by the F2 sort menu (§7.2, Phase 5+); exercised in tests today.
    pub fn set_shape(&mut self, shape: TreeShape) {
        self.shape = shape;
        self.rebuild();
        self.reset_selection_top();
    }

    /// Swap whether `static` or `dynamic` comes first in the partitioned modes
    /// (`t`).
    pub fn swap_priority(&mut self) {
        self.static_first = !self.static_first;
        self.rebuild();
        self.reset_selection_top();
    }

    /// Set whether `static` sorts before `dynamic` in the partitioned modes to a
    /// specific value (the F2 sort menu "Block order" group, PROP-037 §7.2).
    /// Mirrors [`App::set_ordering`] / [`App::set_shape`]: rebuild + reset the
    /// selection to the top.
    #[allow(dead_code)] // selected by the F2 sort menu "Block order" group (§7.2, Phase 7); exercised in tests.
    pub fn set_static_first(&mut self, static_first: bool) {
        self.static_first = static_first;
        self.rebuild();
        self.reset_selection_top();
    }

    /// Select the next tab, wrapping — [`DisplayMode::Tabs`] only (`Shift+→`).
    pub fn next_tab(&mut self) {
        self.step_tab(1);
    }

    /// Select the previous tab, wrapping — [`DisplayMode::Tabs`] only (`Shift+←`).
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
        assert!(
            app.rows[0].name.contains(app.theme.glyphs().fold_collapsed),
            "folded node shows the collapsed glyph"
        );
        // Unfold restores.
        app.toggle_fold_selected();
        assert_eq!(app.rows.len(), 2);
        assert!(
            app.rows[0].name.contains(app.theme.glyphs().fold_expanded),
            "unfolded node shows the expanded glyph"
        );
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
        let dedup = app.theme.glyphs().dag_dedup;
        let reoccurrences = app.rows.iter().filter(|r| r.name.contains(dedup)).count();
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

    #[test]
    fn set_shape_rebuilds_the_tree_mode_walk() {
        // `set_shape` is the F2-sort-menu mutator (PROP-037 §3.3 `#tree-shapes`),
        // and the shape field drives the tree-mode (All) walk too (§4.1). Under
        // the default members-as-roots shape the declared-root walk shows the full
        // subtree; switching to PrunedTree keeps only branches reaching a filter
        // member, so over the declared-root filter only `g/root` itself remains.
        let mut app = App::new(tree(
            vec![
                pkg("g/root", &["g/a", "g/b"]),
                pkg("g/a", &[]),
                pkg("g/b", &[]),
            ],
            &["g/root"],
        ));
        assert_eq!(app.shape, TreeShape::MembersAsRoots);
        assert_eq!(package_ids(&app), ["g/root", "g/a", "g/b"]);
        app.set_shape(TreeShape::PrunedTree);
        assert_eq!(app.shape, TreeShape::PrunedTree);
        assert_eq!(package_ids(&app), ["g/root"]);
        // Back to the default restores the full subtree.
        app.set_shape(TreeShape::MembersAsRoots);
        assert_eq!(package_ids(&app), ["g/root", "g/a", "g/b"]);
    }
}
