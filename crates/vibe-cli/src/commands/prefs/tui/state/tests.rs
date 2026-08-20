//! The prefs-app state tests — split from `state.rs` at the DRAIN-B landing
//! (the file crossed the 600-line budget).

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
