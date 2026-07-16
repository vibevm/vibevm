//! The `vibe prefs` Search Everywhere window (PROP-041 §7 `#settings-search`,
//! §8 `#commands-are-actions`) — opened by the `prefs.search` action (`/` or
//! `F1`).
//!
//! The window is a captive modal over the surface. [`SearchState`] owns a
//! [`vibe_actions::search::SearchEngine`] fed by two providers
//! ([`providers`]): every setting key (the [`SettingsProvider`], keyed by path,
//! display name, description, and synonyms, with deprecated keys surfacing
//! `replaced_by`), and the `vibe.prefs` action catalogue (the
//! [`PrefsActionProvider`], run-in-place by address). Typing re-runs the
//! engine; `Tab` cycles the hybrid "All" / per-category tabs; `Enter` confirms
//! the selection, and the App applies the open-page-focus-field / run-action
//! effect by `(provider, item)` — the provider layer is read-only, so only the
//! App mutates the model.
//!
//! Mirrors `tree::tui::search::mod` (the engine usage, the tab/research/step
//! machinery, and the `confirm` shape) over [`super::state::PrefsApp`].

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#settings-search");

mod providers;
pub mod render;

use std::collections::HashSet;

use rat_salsa::Control;
use vibe_actions::ActionAddr;
use vibe_actions::search::{
    ItemRef, Modifiers, ProviderId, Query, SearchEngine, SearchProvider, SearchRow, Selected, Tab,
};

use super::AppEvent;
use super::catalogue::build_registry;
use super::dispatch;
use super::state::PrefsApp;
use providers::{ACTIONS, PrefsActionProvider, SETTINGS, SettingsProvider};

/// The open Search Everywhere window's state.
pub(crate) struct SearchState {
    engine: SearchEngine,
    enabled_in_all: HashSet<ProviderId>,
    query: String,
    tabs: Vec<Tab>,
    tab_idx: usize,
    rows: Vec<SearchRow>,
    selected_row: usize,
    /// The `(page_id, key)` for each Settings candidate, in enumeration order —
    /// the App opens the owning page + focuses the field on confirm.
    settings_targets: Vec<(String, String)>,
    /// The action addresses in the ActionProvider's enumeration order — the App
    /// dispatches a selected action's effect by its address (PROP-041 §8).
    action_addrs: Vec<ActionAddr>,
}

impl SearchState {
    /// Build the window over the current app state: materialise the two
    /// providers, build the engine + tab strip, and run the empty query. The
    /// action provider's enablement snapshot is read **before** the window is
    /// marked open so it reflects the surface the user is leaving.
    pub(crate) fn open(app: &PrefsApp) -> Self {
        let ctx = app.action_ctx();
        let registry = build_registry();
        let (action_provider, action_addrs) = PrefsActionProvider::build(&registry, ctx);
        let settings = SettingsProvider::build(&app.registry, &app.schema);
        // Capture the (page_id, key) targets in the same order as the settings
        // candidates before handing ownership to the engine.
        let settings_targets: Vec<(String, String)> = (0..settings.len())
            .map(|i| {
                let (page, key) = settings.target(ItemRef(i)).unwrap_or(("", ""));
                (page.to_owned(), key.to_owned())
            })
            .collect();
        let providers: Vec<Box<dyn SearchProvider>> =
            vec![Box::new(settings), Box::new(action_provider)];
        let engine = SearchEngine::new(providers);
        let tabs = engine.tabs();
        let enabled_in_all: HashSet<ProviderId> = [SETTINGS, ACTIONS].into_iter().collect();
        let mut state = SearchState {
            engine,
            enabled_in_all,
            query: String::new(),
            tabs,
            tab_idx: 0,
            rows: Vec::new(),
            selected_row: 0,
            settings_targets,
            action_addrs,
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
    pub(crate) fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.research();
    }

    /// Delete the last character and re-search.
    pub(crate) fn backspace(&mut self) {
        self.query.pop();
        self.research();
    }

    /// Move to the next tab (wrapping) and re-search.
    pub(crate) fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.tab_idx = (self.tab_idx + 1) % self.tabs.len();
            self.research();
        }
    }

    /// Move to the previous tab (wrapping) and re-search.
    pub(crate) fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.tab_idx = (self.tab_idx + self.tabs.len() - 1) % self.tabs.len();
            self.research();
        }
    }

    /// Move the selection down to the next hit row (skipping headers).
    pub(crate) fn select_down(&mut self) {
        self.step_selection(1);
    }

    /// Move the selection up to the previous hit row (skipping headers).
    pub(crate) fn select_up(&mut self) {
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

    /// The `(page_id, key)` target for a settings selection, if present.
    pub(crate) fn settings_target(&self, item: ItemRef) -> Option<(&str, &str)> {
        self.settings_targets
            .get(item.0)
            .map(|(p, k)| (p.as_str(), k.as_str()))
    }

    /// The action address for an actions selection, if present.
    pub(crate) fn action_addr(&self, item: ItemRef) -> Option<&ActionAddr> {
        self.action_addrs.get(item.0)
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
/// For a setting: open the owning page focused on the field (PROP-041 §7
/// `#settings-search`); for an action: run it through the shared address
/// dispatch (PROP-041 §8 `#commands-are-actions`).
pub(crate) fn confirm(app: &mut PrefsApp) -> Control<AppEvent> {
    let confirmed = app
        .search
        .as_mut()
        .and_then(|s| s.confirm(Modifiers::default()));
    let Some((provider, item, verdict)) = confirmed else {
        return Control::Unchanged;
    };
    let control = if provider == SETTINGS {
        let target = app
            .search
            .as_ref()
            .and_then(|s| s.settings_target(item))
            .map(|(p, k)| (p.to_owned(), k.to_owned()));
        if let Some((page_id, key)) = target {
            app.open_page_focused(&page_id, &key);
        }
        Control::Changed
    } else if provider == ACTIONS {
        let addr = app
            .search
            .as_ref()
            .and_then(|s| s.action_addr(item).cloned());
        match addr {
            Some(addr) => dispatch::dispatch_by_addr(app, &addr),
            None => Control::Changed,
        }
    } else {
        Control::Changed
    };
    if matches!(verdict, Selected::Close) {
        app.search = None;
    }
    control
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::commands::prefs::tui::registry::PageDecl;
    use crate::commands::prefs::tui::state::{PrefsApp, PrefsCtx};
    use vibe_settings::loader::LayeredRaw;
    use vibe_settings::resolver::resolve;
    use vibe_settings::schema::{Deprecation, KeyMeta, KeyType, Schema, Scope};

    fn schema() -> Schema {
        let mut s = Schema::new();
        s.register(
            KeyMeta::new(
                "vibe.tree.palette",
                KeyType::String,
                Scope::User,
                "the palette",
            )
            .unwrap()
            .with_default(toml::Value::String("rose-pine".into())),
        )
        .unwrap();
        s.register(
            KeyMeta::new(
                "vibe.tree.mode",
                KeyType::String,
                Scope::User,
                "the display mode",
            )
            .unwrap(),
        )
        .unwrap();
        s.register(
            KeyMeta::new("node.sort", KeyType::String, Scope::User, "old sort")
                .unwrap()
                .with_deprecation(Deprecation::with_replacement("use tree.sort", "tree.sort")),
        )
        .unwrap();
        s
    }

    fn app() -> PrefsApp {
        let prefs = resolve(
            LayeredRaw::default(),
            &schema(),
            toml::Table::new(),
            toml::Table::new(),
        );
        let mut a = PrefsApp::new(prefs, schema(), PrefsCtx::new(true));
        // Install a small registry so the search has known candidates.
        a.registry = crate::commands::prefs::tui::registry::PageRegistry::from(vec![
            PageDecl::new("palette", "Palette", "the colour palette")
                .with_keys(&["vibe.tree.palette", "node.sort"]),
            PageDecl::new("mode", "Display mode", "the display mode")
                .with_keys(&["vibe.tree.mode"]),
        ]);
        a.rebuild();
        a.select_first();
        a
    }

    fn hits(s: &SearchState) -> Vec<&vibe_actions::search::Hit> {
        s.rows
            .iter()
            .filter_map(|r| match r {
                SearchRow::Hit(h) => Some(h),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn opens_with_all_plus_two_category_tabs() {
        let s = SearchState::open(&app());
        assert_eq!(s.tabs.len(), 3, "All + Settings + Actions");
        assert!(s.tabs[0].is_all, "the first tab is the hybrid All tab");
    }

    #[test]
    fn typing_a_key_path_surfaces_its_page() {
        let mut s = SearchState::open(&app());
        for c in "palette".chars() {
            s.type_char(c);
        }
        assert!(
            hits(&s).iter().any(|h| h.primary.contains("Palette")),
            "the palette key is found by its description"
        );
    }

    #[test]
    fn a_deprecated_key_is_searchable_by_its_old_name() {
        let mut s = SearchState::open(&app());
        for c in "node.sort".chars() {
            s.type_char(c);
        }
        assert!(
            hits(&s)
                .iter()
                .any(|h| h.secondary.as_deref().unwrap_or("").contains("tree.sort")),
            "the deprecated node.sort surfaces its replaced_by"
        );
    }

    #[test]
    fn confirming_a_setting_hit_opens_the_owning_page_focused_on_the_field() {
        let mut app = app();
        let mut s = SearchState::open(&app);
        // Type the unique mode key to land a single settings hit.
        for c in "vibe.tree.mode".chars() {
            s.type_char(c);
        }
        app.search = Some(s);
        let _ = confirm(&mut app);
        assert!(app.search.is_none(), "the window closed on confirm");
        assert_eq!(
            app.open_page.as_deref(),
            Some("mode"),
            "the owning page opened"
        );
        assert!(app.form.is_some(), "the form built for the page");
        assert!(
            app.form
                .as_ref()
                .map(|f| f.fields[f.focus].key == "vibe.tree.mode")
                .unwrap_or(false),
            "the field is focused on the matched key"
        );
    }

    #[test]
    fn confirming_an_action_runs_it_by_address() {
        let mut app = app();
        let mut s = SearchState::open(&app);
        // "Quit" is in the action catalogue; typing it lands the action hit.
        for c in "Quit".chars() {
            s.type_char(c);
        }
        // Move to the Actions tab to be sure the hit is visible there.
        s.next_tab(); // All -> Settings
        s.next_tab(); // Settings -> Actions
        assert!(
            hits(&s).iter().any(|h| h.primary == "Quit"),
            "the Quit action is found in the Actions tab"
        );
        app.search = Some(s);
        let ctrl = confirm(&mut app);
        assert!(
            matches!(ctrl, Control::<AppEvent>::Quit),
            "the action ran by address and returned Quit"
        );
    }
}
