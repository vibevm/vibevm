//! The settings TUI application state (PROP-041 §3). [`PrefsApp`] owns the
//! resolved preferences (read-only — the surface owns no preference logic, §1
//! `#surface-not-engine`), the page registry, the fold set + selection, the
//! open page, and the active [`Theme`] (built from the resolved `vibe.tree.*`
//! palette/tier so the settings UI matches the `vibe tree` TUI's look).
//!
//! The rat-salsa fn-pointer entry points live in [`super`]; this module owns
//! the model + the derived, scrollable [`PageRow`] list (built by
//! [`super::page_tree::flatten`]).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-041#tree-widget");

use std::collections::{BTreeMap, BTreeSet};

use rat_widget::table::TableState;
use vibe_settings::resolver::ResolvedPrefs;
use vibe_settings::schema::Schema;

use crate::commands::tree::tui::settings as tree_settings;
use crate::commands::tree::tui::theme::Theme;

use super::catalogue::PrefsActionCtx;
use super::page_tree::{PageRow, flatten};
use super::registry::{BodyCtx, PageRegistry};
use super::search::SearchState;
use super::settings::builtin_registry;

/// The session context (PROP-041 §3 `#tree-context`): whether there is an
/// active project (L2) on disk, or this is a no-project (L1-only) session. A
/// no-project session hides [`PageScope::Project`] pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefsCtx {
    /// Whether a project root (L2) is active.
    pub has_project: bool,
}

impl PrefsCtx {
    /// Build the context from the session's project-root presence.
    #[must_use]
    pub fn new(has_project: bool) -> Self {
        Self { has_project }
    }
}

/// The interactive settings TUI application state (PROP-041 §3).
pub struct PrefsApp {
    /// The resolved preferences snapshot (read-only — PROP-041 §1
    /// `#surface-not-engine`).
    pub prefs: ResolvedPrefs,
    /// The declared preference-key schema (introspection for the origin hint +
    /// the edit form's per-type controls, §4).
    pub schema: Schema,
    /// The session context (L2 active or not).
    pub ctx: PrefsCtx,
    /// The active theme — colours + glyphs flow through this (PROP-037 §2.2),
    /// built from the resolved `vibe.tree.palette`/`vibe.tree.tier`.
    pub theme: Theme,
    /// The page registry (the enumerable source, PROP-041 §2).
    pub registry: PageRegistry,
    /// The flattened visible rows — recomputed on every fold change.
    pub rows: Vec<PageRow>,
    /// Page ids the user has collapsed.
    pub folded: BTreeSet<String>,
    /// rat-widget row selection + vertical scroll offset.
    pub table: TableState,
    /// The open page id, if any (the page pane renders its §4 edit form).
    pub open_page: Option<String>,
    /// The per-type edit form for the open page (PROP-041 §4). Built through
    /// the page's lazy body constructor ([`PageDecl::body`]) on **first** open
    /// (§2 `#page-lazy-body`); a later open reuses the cached body. `None`
    /// exactly when `open_page` is `None`.
    pub form: Option<super::form::Form>,
    /// The built page bodies (PROP-041 §2 `#page-lazy-body`): each page's body
    /// is constructed on first open and kept for the session — close/reopen
    /// (and a search/lint jump away and back) reuses the built form with its
    /// unapplied edits; the constructor never fires twice for one page.
    bodies: BTreeMap<String, super::form::Form>,
    /// The cross-layer lint modal state (PROP-041 §6 `#lint-all`). `Some` while
    /// the "check all layers" modal is open; `None` when it is closed. Built by
    /// [`PrefsApp::open_lint`] from the loaded layer files.
    pub lint: Option<super::lint::LintState>,
    /// The Search Everywhere window (PROP-041 §7 `#settings-search`). `Some`
    /// while the window is open (it then captures input); `None` when closed.
    /// Built by [`PrefsApp::open_search`] over the registry + schema + action
    /// catalogue.
    pub search: Option<SearchState>,
    /// A fatal error captured by the error handler, re-raised after the loop
    /// restores the terminal.
    pub fatal: Option<anyhow::Error>,
}

impl PrefsApp {
    /// Build the app over a resolved snapshot + schema + context. The theme is
    /// derived from the resolved `vibe.tree.*` palette/tier so the settings UI
    /// and the `vibe tree` TUI read as one surface (PROP-041 §1
    /// `#built-on-tree-tui`). The built-in page registry
    /// ([`super::settings`]) is installed; more pages register declarations
    /// the same way.
    pub fn new(prefs: ResolvedPrefs, schema: Schema, ctx: PrefsCtx) -> Self {
        let theme = build_theme(&prefs);
        let registry = builtin_registry();
        let mut app = PrefsApp {
            prefs,
            schema,
            ctx,
            theme,
            registry,
            rows: Vec::new(),
            folded: BTreeSet::new(),
            table: TableState::default(),
            open_page: None,
            form: None,
            bodies: BTreeMap::new(),
            lint: None,
            search: None,
            fatal: None,
        };
        app.rebuild();
        app
    }

    /// Recompute [`PrefsApp::rows`] for the current registry + fold set + scope
    /// context, keeping `table.rows` in sync so key handling stays correct
    /// between renders.
    pub fn rebuild(&mut self) {
        let glyphs = self.theme.glyphs();
        let tree = self.registry.tree(self.ctx.has_project);
        self.rows = flatten(&tree, &self.folded, &self.prefs, glyphs);
        self.table.rows = self.rows.len();
    }

    /// Select the first row (called by `init` on launch).
    pub fn select_first(&mut self) {
        if !self.rows.is_empty() {
            self.table.select(Some(0));
            self.table.set_row_offset(0);
        }
    }

    /// The visible row under the selection, if any.
    pub fn selected_row(&self) -> Option<&PageRow> {
        let idx = self.table.selected()?;
        self.rows.get(idx)
    }

    /// Move the selection up one row, keeping it visible.
    pub fn move_up(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        if self.table.selected().is_none() {
            self.table.select(Some(0));
            return;
        }
        self.table.move_up(1);
        self.table.scroll_to_selected();
    }

    /// Move the selection down one row, keeping it visible.
    pub fn move_down(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        if self.table.selected().is_none() {
            self.table.select(Some(0));
            return;
        }
        self.table.move_down(1);
        self.table.scroll_to_selected();
    }

    /// Toggle the fold state of the selected group row (`←`/`→` or `Space`).
    /// A leaf row is a no-op.
    pub fn toggle_fold_selected(&mut self) {
        let Some(idx) = self.table.selected() else {
            return;
        };
        let Some(row) = self.rows.get(idx) else {
            return;
        };
        if !row.is_group {
            return;
        }
        let id = row.id.clone();
        if !self.folded.remove(&id) {
            self.folded.insert(id);
        }
        self.rebuild_keep_selection();
    }

    /// Open the focused leaf page (`Enter`). A group row is a no-op (it folds,
    /// not opens). Sets [`PrefsApp::open_page`] and takes the page's body —
    /// built through its lazy constructor on **first** open (§2
    /// `#page-lazy-body`), reused from the cache afterwards (§4
    /// `#form-per-type`).
    pub fn open_selected(&mut self) {
        let Some(row) = self.selected_row() else {
            return;
        };
        if row.is_openable() {
            let id = row.id.clone();
            self.open_page_by_id(&id);
        }
    }

    /// Close the open page (`Esc` from the page pane). The built body returns
    /// to the cache — re-opening reuses it instead of re-calling the lazy
    /// constructor (§2 `#page-lazy-body`: created on first open).
    pub fn close_page(&mut self) {
        self.stash_open_form();
        self.open_page = None;
    }

    /// Open page `id` through the one open path: stash the currently open form
    /// back into the body cache (a search/lint jump can switch pages
    /// directly), then take the cached body or build it through the page's
    /// lazy constructor.
    fn open_page_by_id(&mut self, id: &str) {
        self.stash_open_form();
        self.open_page = Some(id.to_owned());
        self.form = self.take_or_build_body(id);
    }

    /// The page's body: the cached one when it was built before, else the
    /// lazy constructor's product (first open — §2 `#page-lazy-body`).
    fn take_or_build_body(&mut self, id: &str) -> Option<super::form::Form> {
        if let Some(form) = self.bodies.remove(id) {
            return Some(form);
        }
        let decl = self.registry.pages().iter().find(|d| d.id == id)?;
        let ctx = BodyCtx {
            prefs: &self.prefs,
            schema: &self.schema,
            has_project: self.ctx.has_project,
        };
        Some((decl.body)(decl, &ctx).form)
    }

    /// Return the open form to the body cache (close or page switch).
    fn stash_open_form(&mut self) {
        if let (Some(form), Some(id)) = (self.form.take(), self.open_page.clone()) {
            self.bodies.insert(id, form);
        }
    }

    /// Open the "check all layers" modal (PROP-041 §6 `#lint-all`, wired to `c`).
    /// Builds the modal state by running `schema::validate` over each loaded
    /// layer file (L1/L2/L3). The modal captures input while open.
    pub fn open_lint(&mut self) {
        let paths = super::form::LayerPaths::from_env();
        self.lint = Some(super::lint::LintState::build(&self.schema, &paths));
    }

    /// Close the lint modal (`Esc` from the modal).
    pub fn close_lint(&mut self) {
        self.lint = None;
    }

    /// Open the Search Everywhere window (PROP-041 §7 `#settings-search`, wired
    /// to `/` / `F1`). The window captures input while open.
    pub fn open_search(&mut self) {
        let state = SearchState::open(self);
        self.search = Some(state);
    }

    /// Close the Search Everywhere window (`Esc`).
    pub fn close_search(&mut self) {
        self.search = None;
    }

    /// Open a specific page by id and focus the field for `key` (PROP-041 §7
    /// `#settings-search`'s "selecting a result opens the owning page focused on
    /// that field", and §6 `#lint-all`'s jump-to-field). A no-op on the form
    /// side when the page id is unknown (the registry find returns `None`).
    pub fn open_page_focused(&mut self, page_id: &str, key: &str) {
        // Only open when the page is actually declared (a search hit always
        // carries a real page id, but a lint jump may name a key no page owns).
        if !self.registry.pages().iter().any(|d| d.id == page_id) {
            return;
        }
        self.open_page_by_id(page_id);
        if let Some(form) = &mut self.form {
            form.focus_key(key);
        }
    }

    /// The action-enablement snapshot for the `vibe.prefs` catalogue (PROP-039
    /// §6.2, PROP-041 §8 `#commands-are-actions`). Read by the keymap resolver
    /// gate + the footer + the search ActionProvider.
    #[must_use]
    pub fn action_ctx(&self) -> PrefsActionCtx {
        PrefsActionCtx {
            at_base: self.open_page.is_none() && self.lint.is_none() && self.search.is_none(),
            page_open: self.open_page.is_some(),
            leaf_selected: self
                .selected_row()
                .map(|r| r.is_openable())
                .unwrap_or(false),
            form_editable: self
                .form
                .as_ref()
                .and_then(|f| f.focused_field())
                .map(|f| !f.control.is_text())
                .unwrap_or(false),
            has_blocking_error: self
                .form
                .as_ref()
                .map(|f| f.has_blocking_error())
                .unwrap_or(false),
        }
    }

    /// Move the lint selection up one row.
    pub fn lint_up(&mut self) {
        if let Some(lint) = &mut self.lint {
            lint.up();
        }
    }

    /// Move the lint selection down one row.
    pub fn lint_down(&mut self) {
        if let Some(lint) = &mut self.lint {
            lint.down();
        }
    }

    /// Jump to the field owning the selected lint entry (PROP-041 §6 `#lint-all`'s
    /// jump-to-field, wired to `Enter` in the modal). Opens the owning page and
    /// focuses the offending field; closes the modal. A diagnostic whose key no
    /// page owns (a typo / retired name) closes the modal without jumping — the
    /// list itself is the diagnostic surface for those.
    pub fn lint_jump_to_selected(&mut self) {
        let Some(entry) = self.lint.as_ref().and_then(|l| l.selected()).cloned() else {
            self.close_lint();
            return;
        };
        self.close_lint();
        // Find the page that owns this key, then open + focus via the shared
        // path the search result selection also uses.
        let page_id = self
            .registry
            .pages()
            .iter()
            .find(|d| d.keys.iter().any(|k| k == &entry.path))
            .map(|d| d.id.clone());
        if let Some(id) = page_id {
            self.open_page_focused(&id, &entry.path);
        }
    }

    /// The display name for the open page, if any (titles the right pane).
    pub fn open_page_title(&self) -> Option<&str> {
        let id = self.open_page.as_deref()?;
        self.registry
            .pages()
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.display_name.as_str())
    }

    /// Recompute rows, clamping the selection to the new row count.
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
}

/// Build the active [`Theme`] from the resolved `vibe.tree.palette`/`vibe.tree.
/// tier` keys (PROP-037 §9, PROP-041 §1 `#built-on-tree-tui`). Reuses the tree
/// TUI's settings bridge so the two surfaces share one look — a restyle of the
/// palette key re-skins both.
fn build_theme(prefs: &ResolvedPrefs) -> Theme {
    tree_settings::TreeSettings::new().theme(prefs)
}

#[cfg(test)]
mod tests;
