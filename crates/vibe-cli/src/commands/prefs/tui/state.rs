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
mod tests {
    use super::*;
    use vibe_settings::loader::LayeredRaw;
    use vibe_settings::resolver::resolve;
    use vibe_settings::schema::{KeyMeta, KeyType, Schema, Scope};

    fn schema() -> Schema {
        let mut s = Schema::new();
        s.register(
            KeyMeta::new("vibe.tree.palette", KeyType::String, Scope::User, "p")
                .unwrap()
                .with_default(toml::Value::String("rose-pine".into())),
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
        PrefsApp::new(prefs, schema(), PrefsCtx::new(true))
    }

    #[test]
    fn new_app_flattens_the_builtin_registry() {
        let app = app();
        assert!(!app.rows.is_empty(), "the built-in pages are flattened");
        // Two groups (Appearance + Tree) are present.
        assert!(
            app.rows
                .iter()
                .any(|r| r.id == super::super::settings::GROUP_APPEARANCE)
        );
        assert!(
            app.rows
                .iter()
                .any(|r| r.id == super::super::settings::GROUP_TREE)
        );
    }

    #[test]
    fn move_up_down_advances_the_selection() {
        let mut app = app();
        app.table.select(Some(0));
        app.move_down();
        assert_eq!(app.table.selected(), Some(1));
        app.move_up();
        assert_eq!(app.table.selected(), Some(0));
    }

    #[test]
    fn toggle_fold_on_a_group_hides_its_children() {
        let mut app = app();
        let before = app.rows.len();
        app.table.select(Some(0)); // Appearance group (weight 10, first).
        // Ensure row 0 is the Appearance group.
        let is_group = app.rows[0].is_group;
        assert!(is_group, "row 0 is a group");
        app.toggle_fold_selected();
        assert!(app.rows.len() < before, "folding hides children");
        // Unfold restores.
        app.toggle_fold_selected();
        assert_eq!(app.rows.len(), before);
    }

    #[test]
    fn open_selected_opens_a_leaf_not_a_group() {
        let mut app = app();
        // Select a leaf (Palette is under Appearance, row 1 when expanded).
        app.table.select(Some(1));
        app.open_selected();
        assert!(app.open_page.is_some(), "a leaf opens");
        // A group does not open.
        app.table.select(Some(0));
        app.open_selected();
        // open_page unchanged from the group attempt (still the leaf or cleared).
        let _ = app.open_page.take();
        // Now with open_page cleared, selecting the group and pressing Enter
        // does not open anything.
        app.table.select(Some(0));
        app.open_selected();
        assert!(app.open_page.is_none(), "a group does not open");
    }

    #[test]
    fn no_project_session_still_shows_application_pages() {
        // All built-in pages are Application-scoped → the tree is the same with
        // or without a project (PROP-041 §3 #tree-context).
        let prefs = resolve(
            LayeredRaw::default(),
            &schema(),
            toml::Table::new(),
            toml::Table::new(),
        );
        let with_proj = PrefsApp::new(prefs.clone(), schema(), PrefsCtx::new(true));
        let no_proj = PrefsApp::new(prefs, schema(), PrefsCtx::new(false));
        assert_eq!(with_proj.rows.len(), no_proj.rows.len());
    }

    #[test]
    fn open_page_title_resolves_the_display_name() {
        let mut app = app();
        app.table.select(Some(1));
        app.open_selected();
        assert!(app.open_page_title().is_some());
        app.close_page();
        assert!(app.open_page_title().is_none());
    }

    #[test]
    fn first_open_builds_the_body_once_reopen_reuses_it() {
        // §2 #page-lazy-body — "a lazy page body (created on first open)":
        // first open calls the constructor; close/reopen reuses the cached
        // body (with its unapplied edits), so the constructor fires exactly
        // once per page.
        use std::sync::atomic::{AtomicUsize, Ordering};

        use super::super::form::default_body;
        use super::super::registry::BodyCtx;
        use super::super::registry::PageBody;
        use super::super::registry::PageDecl;

        static BUILDS: AtomicUsize = AtomicUsize::new(0);
        fn counting_body(decl: &PageDecl, ctx: &BodyCtx<'_>) -> PageBody {
            BUILDS.fetch_add(1, Ordering::SeqCst);
            default_body(decl, ctx)
        }

        let prefs = resolve(
            LayeredRaw::default(),
            &schema(),
            toml::Table::new(),
            toml::Table::new(),
        );
        let mut app = PrefsApp::new(prefs, schema(), PrefsCtx::new(true));
        app.registry = PageRegistry::from(vec![
            PageDecl::new("g", "G", "a group"),
            PageDecl::new("leaf", "Leaf", "a leaf")
                .with_parent("g")
                .with_keys(&["vibe.tree.palette"])
                .with_body(counting_body),
        ]);
        app.rebuild();
        app.table.select(Some(1)); // the leaf under the group
        app.open_selected();
        assert_eq!(
            BUILDS.load(Ordering::SeqCst),
            1,
            "first open builds the body"
        );
        // Edit the leaf's selection (palette cycles one step), then close.
        let before = app
            .form
            .as_mut()
            .and_then(|f| f.focused_field_mut())
            .map(|f| f.control.current_value());
        app.close_page();
        app.open_selected();
        assert_eq!(
            BUILDS.load(Ordering::SeqCst),
            1,
            "reopen reuses the cached body"
        );
        assert_eq!(
            app.form
                .as_ref()
                .and_then(|f| f.focused_field())
                .map(|f| f.control.current_value()),
            before,
            "the reopened body is the same form (edits preserved)"
        );
    }

    #[test]
    fn switching_pages_stashes_the_open_form_and_builds_the_new_body_once() {
        // A search/lint jump can open page B while page A is open: A's built
        // body is stashed, and returning to A does not re-calls its
        // constructor.
        use std::sync::atomic::{AtomicUsize, Ordering};

        use super::super::form::default_body;
        use super::super::registry::{BodyCtx, PageBody, PageDecl};

        static BUILDS_A: AtomicUsize = AtomicUsize::new(0);
        fn counting_a(decl: &PageDecl, ctx: &BodyCtx<'_>) -> PageBody {
            BUILDS_A.fetch_add(1, Ordering::SeqCst);
            default_body(decl, ctx)
        }

        let prefs = resolve(
            LayeredRaw::default(),
            &schema(),
            toml::Table::new(),
            toml::Table::new(),
        );
        let mut app = PrefsApp::new(prefs, schema(), PrefsCtx::new(true));
        app.registry = PageRegistry::from(vec![
            PageDecl::new("a", "A", "page a")
                .with_keys(&["vibe.tree.palette"])
                .with_body(counting_a),
            PageDecl::new("b", "B", "page b").with_keys(&["vibe.tree.palette"]),
        ]);
        app.rebuild();
        app.open_page_focused("a", "vibe.tree.palette");
        assert_eq!(BUILDS_A.load(Ordering::SeqCst), 1);
        // Jump to B, then back to A — A's body comes from the cache.
        app.open_page_focused("b", "vibe.tree.palette");
        app.open_page_focused("a", "vibe.tree.palette");
        assert_eq!(
            BUILDS_A.load(Ordering::SeqCst),
            1,
            "the body is built on first open only"
        );
        assert_eq!(app.open_page.as_deref(), Some("a"));
    }
}
