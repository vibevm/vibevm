//! The `vibe tree` Search Everywhere window (PROP-037 §7.3, §13) — the F1 feature.
//!
//! The window is a captive modal over the tree. [`SearchState`] owns a
//! [`vibe_actions::search::SearchEngine`] fed by the three providers
//! ([`providers`]): packages, every card field, and the `vibe.tree` action
//! catalogue. Typing re-runs the engine; `Tab` cycles the hybrid "All" /
//! per-category tabs; `Enter` confirms the selection, and the App applies the
//! reveal / open-card / run effect by `(provider, item)` — the provider layer is
//! read-only, so only the App mutates the model.
//!
//! MVP scope (PROP-039 §13.5): the action catalogue is a static list dispatched
//! by address; backing it with a live `vibe_actions::Registry` (Ctx enablement +
//! address-routed invoke) is the next increment.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f1-search");

mod providers;
pub mod render;

use std::collections::HashSet;

use rat_salsa::Control;
use vibe_actions::search::{
    ItemRef, Modifiers, ProviderId, Query, SearchEngine, SearchProvider, SearchRow, Selected, Tab,
};

use super::AppEvent;
use super::state::{App, DisplayMode, RowNode};
use providers::{
    ACTIONS, ActionProvider, FIELDS, FieldProvider, PACKAGES, PackageProvider, TREE_ACTIONS,
    TreeCtx,
};

/// The open Search Everywhere window's state.
pub struct SearchState {
    engine: SearchEngine,
    enabled_in_all: HashSet<ProviderId>,
    pub(super) query: String,
    pub(super) tabs: Vec<Tab>,
    pub(super) tab_idx: usize,
    pub(super) rows: Vec<SearchRow>,
    pub(super) selected_row: usize,
}

impl SearchState {
    /// Build the window over the current app state: materialise the three
    /// providers, build the engine + tab strip, and run the empty query.
    pub fn open(app: &App) -> Self {
        let ctx = TreeCtx {
            mode: app.display_mode,
            has_pkg_selection: app
                .selected_row()
                .map(|r| matches!(r.node, RowNode::Package(_)))
                .unwrap_or(false),
        };
        let providers: Vec<Box<dyn SearchProvider>> = vec![
            Box::new(PackageProvider::build(&app.tree)),
            Box::new(FieldProvider::build(&app.tree)),
            Box::new(ActionProvider::build(ctx)),
        ];
        let engine = SearchEngine::new(providers);
        let tabs = engine.tabs();
        let enabled_in_all: HashSet<ProviderId> = [PACKAGES, FIELDS, ACTIONS].into_iter().collect();
        let mut state = SearchState {
            engine,
            enabled_in_all,
            query: String::new(),
            tabs,
            tab_idx: 0,
            rows: Vec::new(),
            selected_row: 0,
        };
        state.research();
        state
    }

    /// Re-run the engine for the current query + tab, and land the selection on
    /// the first hit.
    fn research(&mut self) {
        let rows = {
            let tab = &self.tabs[self.tab_idx.min(self.tabs.len().saturating_sub(1))];
            self.engine.search(
                &Query {
                    text: self.query.as_str(),
                },
                tab,
                &self.enabled_in_all,
            )
        };
        self.rows = rows;
        self.selected_row = self.first_hit().unwrap_or(0);
    }

    /// Append a typed character and re-search.
    pub fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.research();
    }

    /// Delete the last character and re-search.
    pub fn backspace(&mut self) {
        self.query.pop();
        self.research();
    }

    /// Move to the next tab (wrapping) and re-search.
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.tab_idx = (self.tab_idx + 1) % self.tabs.len();
            self.research();
        }
    }

    /// Move to the previous tab (wrapping) and re-search.
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.tab_idx = (self.tab_idx + self.tabs.len() - 1) % self.tabs.len();
            self.research();
        }
    }

    /// Move the selection down to the next hit row (skipping headers).
    pub fn select_down(&mut self) {
        self.step_selection(1);
    }

    /// Move the selection up to the previous hit row (skipping headers).
    pub fn select_up(&mut self) {
        self.step_selection(-1);
    }

    fn step_selection(&mut self, delta: isize) {
        let n = self.rows.len() as isize;
        if n == 0 {
            return;
        }
        let mut i = self.selected_row as isize;
        loop {
            i += delta;
            if i < 0 || i >= n {
                return; // at an edge — keep the current selection
            }
            if matches!(self.rows[i as usize], SearchRow::Hit(_)) {
                self.selected_row = i as usize;
                return;
            }
        }
    }

    fn first_hit(&self) -> Option<usize> {
        self.rows
            .iter()
            .position(|r| matches!(r, SearchRow::Hit(_)))
    }

    /// The `(provider, item)` under the selection, if it is a hit.
    fn selected(&self) -> Option<(ProviderId, ItemRef)> {
        match self.rows.get(self.selected_row) {
            Some(SearchRow::Hit(h)) => Some((h.provider, h.item)),
            _ => None,
        }
    }

    /// Confirm the selection: notify the engine (recency + the provider's
    /// close/stay verdict) and return what to act on.
    fn confirm(&mut self, mods: Modifiers) -> Option<(ProviderId, ItemRef, Selected)> {
        let (provider, item) = self.selected()?;
        let verdict = self.engine.on_selected(provider, item, mods);
        Some((provider, item, verdict))
    }
}

/// Confirm the selected result and apply its effect to the App. Returns the
/// control-flow verdict; closes the window unless the provider asked to stay.
pub fn confirm(app: &mut App) -> Control<AppEvent> {
    let confirmed = app
        .search
        .as_mut()
        .and_then(|s| s.confirm(Modifiers::default()));
    let Some((provider, item, verdict)) = confirmed else {
        return Control::Unchanged;
    };
    let control = apply_effect(app, provider, item);
    if matches!(verdict, Selected::Close) {
        app.search = None;
    }
    control
}

/// Apply a confirmed selection to the model (only the App may mutate it).
fn apply_effect(app: &mut App, provider: ProviderId, item: ItemRef) -> Control<AppEvent> {
    match provider {
        PACKAGES => {
            reveal_package(app, item.0);
            Control::Changed
        }
        FIELDS => {
            reveal_package(app, item.0);
            app.modal_open = true; // open the card for the revealed package
            Control::Changed
        }
        ACTIONS => run_action(app, item.0),
        _ => Control::Changed,
    }
}

/// Reveal a package in the main tree: switch to the all-together tree, unfold so
/// nothing hides it, rebuild, and select its row.
fn reveal_package(app: &mut App, pkg_idx: usize) {
    app.display_mode = DisplayMode::All;
    app.folded.clear();
    app.all_folded = false;
    app.rebuild();
    if let Some(row) = app
        .rows
        .iter()
        .position(|r| r.node == RowNode::Package(pkg_idx))
    {
        app.table.select(Some(row));
        app.table.scroll_to_selected();
    }
}

/// Run a `vibe.tree` catalogue action by its address (PROP-037 §13.5).
fn run_action(app: &mut App, idx: usize) -> Control<AppEvent> {
    let Some(spec) = TREE_ACTIONS.get(idx) else {
        return Control::Changed;
    };
    match spec.addr {
        "action://vibe.tree/ordering.cycle" => app.cycle_ordering(),
        "action://vibe.tree/mode.cycle" => app.cycle_display_mode(),
        "action://vibe.tree/priority.swap" => app.swap_priority(),
        "action://vibe.tree/fold.toggle" => app.toggle_fold_selected(),
        "action://vibe.tree/fold.all" => app.toggle_fold_all(),
        "action://vibe.tree/card.open" => {
            if app
                .selected_row()
                .map(|r| matches!(r.node, RowNode::Package(_) | RowNode::Missing))
                .unwrap_or(false)
            {
                app.modal_open = true;
            }
        }
        "action://vibe.tree/tab.next" => app.next_tab(),
        "action://vibe.tree/tab.prev" => app.prev_tab(),
        "action://vibe.tree/quit" => return Control::Quit,
        _ => {}
    }
    Control::Changed
}

#[cfg(test)]
mod tests {
    use vibe_actions::search::{Hit, SearchRow};

    use super::*;
    use crate::commands::tree::model::{
        Boot, Condition, HOST_NAMESPACE, IndexLane, Load, LoadOrigin, LoadType, Package,
        PackageTree, Project, SCHEMA_VERSION,
    };

    fn pkg(id: &str) -> Package {
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
            dependencies: Vec::new(),
        }
    }

    fn tiny_app() -> App {
        let tree = PackageTree {
            schema_version: SCHEMA_VERSION,
            generated_at: None,
            tool_version: None,
            project: Project {
                root: "/tmp/x".to_string(),
                name: None,
                is_workspace: false,
                host_namespace: HOST_NAMESPACE.to_string(),
            },
            roots: vec!["g/alpha".to_string(), "g/beta".to_string()],
            packages: vec![pkg("g/alpha"), pkg("g/beta")],
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
        };
        App::new(tree)
    }

    fn hits(s: &SearchState) -> Vec<&Hit> {
        s.rows
            .iter()
            .filter_map(|r| match r {
                SearchRow::Hit(h) => Some(h),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn opens_with_all_plus_three_category_tabs() {
        let s = SearchState::open(&tiny_app());
        assert_eq!(s.tabs.len(), 4, "All + Packages + Card fields + Actions");
        assert!(s.tabs[0].is_all, "the first tab is the hybrid All tab");
    }

    #[test]
    fn typing_a_package_name_surfaces_it() {
        let mut s = SearchState::open(&tiny_app());
        for c in "alpha".chars() {
            s.type_char(c);
        }
        assert!(
            hits(&s).iter().any(|h| h.primary.contains("alpha")),
            "g/alpha is found by name"
        );
    }

    #[test]
    fn running_an_action_from_search_mutates_the_tree() {
        let mut app = tiny_app();
        let before = app.ordering;
        let s = SearchState::open(&app);
        app.search = Some(s);
        {
            let st = app.search.as_mut().expect("open");
            for c in "ordering".chars() {
                st.type_char(c);
            }
            assert!(
                hits(st).iter().any(|h| h.provider == ACTIONS),
                "the Cycle ordering action is found"
            );
        }
        let _ = confirm(&mut app);
        assert_ne!(
            app.ordering, before,
            "the action ran and changed the ordering"
        );
        assert!(
            app.search.is_none(),
            "the window closed on the Close verdict"
        );
    }

    #[test]
    fn revealing_a_package_selects_it_in_the_all_tree() {
        let mut app = tiny_app();
        app.cycle_display_mode(); // leave All so reveal must switch back
        assert_ne!(app.display_mode, DisplayMode::All);
        let s = SearchState::open(&app);
        app.search = Some(s);
        {
            let st = app.search.as_mut().expect("open");
            st.next_tab(); // All -> Packages
            for c in "beta".chars() {
                st.type_char(c);
            }
            assert!(
                hits(st).iter().any(|h| h.provider == PACKAGES),
                "g/beta is found in the Packages tab"
            );
        }
        let _ = confirm(&mut app);
        assert_eq!(
            app.display_mode,
            DisplayMode::All,
            "reveal switched to the all-tree"
        );
        assert_eq!(
            app.selected_row().map(|r| r.id.as_str()),
            Some("g/beta"),
            "the revealed package is selected"
        );
    }
}
