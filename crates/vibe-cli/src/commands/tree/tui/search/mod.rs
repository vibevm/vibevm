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
