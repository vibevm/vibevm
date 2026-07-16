//! Address-routed dispatch (PROP-041 §8 `#commands-are-actions`, mirroring
//! `tree::tui::dispatch`) — the single source of truth for "what does a
//! `vibe.prefs` action do to the model." Both the base-mode keymap path
//! ([`super::input`]) and the Search Everywhere ACTIONS provider
//! ([`super::search`]) call [`dispatch_by_addr`] rather than each doing its own
//! string-match. The catalogue's 8 addresses are the only routing keys;
//! matching is on the address's `name()` half (all live under the `vibe.prefs`
//! group).
//!
//! [`enabled_in_base`] mirrors the catalogue's per-context enablement so the
//! keymap resolver can gate bindings without reaching into the typed
//! [`super::catalogue::PrefsActionCtx`] (the resolver stays pure, PROP-039 §9.2
//! — the tree catalogue's recorded posture).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#commands-are-actions");

use rat_salsa::Control;
use vibe_actions::ActionAddr;

use super::AppEvent;
use super::state::PrefsApp;

/// Apply a `vibe.prefs` action to the model by its address (PROP-041 §8).
/// Returns the rat-salsa control-flow verdict — `Control::Quit` for the quit
/// action, `Control::Changed` for everything else. Form actions (apply / reset
/// / layer.next / provenance.toggle) are no-ops when no form is open; their
/// enablement gates them off at the keymap, but dispatch defends anyway (a
/// search ACTIONS-provider selection can land on a disabled action's address
/// if the provider's snapshot drifted).
pub(super) fn dispatch_by_addr(app: &mut PrefsApp, addr: &ActionAddr) -> Control<AppEvent> {
    match addr.name() {
        // Base actions.
        "page.open" => app.open_selected(),
        "search" => app.open_search(),
        "lint" => app.open_lint(),
        "quit" => return Control::Quit,
        // Form actions — no-op when no form is open (defensive; the keymap gates
        // them off and the search provider marks them disabled).
        "apply" => apply_form(app),
        "reset" => {
            if let Some(form) = app.form.as_mut() {
                form.reset(&app.prefs);
            }
        }
        "layer.next" => {
            if let Some(form) = app.form.as_mut() {
                form.cycle_write_layer();
            }
        }
        "provenance.toggle" => {
            if let Some(form) = app.form.as_mut() {
                form.toggle_provenance();
            }
        }
        _ => {}
    }
    Control::Changed
}

/// Run the form's `apply` (PROP-041 §4 `#configurable-lifecycle`), gated on a
/// blocking validation error (§6 `#validation-feedback` — a field in error
/// reports why and does not persist). A failure is logged and the change is not
/// persisted; the surface still repaints so the warning style is visible.
fn apply_form(app: &mut PrefsApp) {
    let Some(form) = app.form.as_mut() else {
        return;
    };
    if form.has_blocking_error() {
        tracing::warn!(
            "vibe prefs form: apply blocked \u{2014} a field has a validation error \
             (violates spec://vibevm/modules/vibe-settings/PROP-041#validation)"
        );
        return;
    }
    if let Err(err) = form.apply(&app.schema) {
        tracing::warn!(
            %err,
            "vibe prefs form: apply failed \u{2014} the change is not persisted"
        );
    }
}

/// Whether `addr` is live in the base screen (no overlay open) — the keymap
/// resolver's enablement gate (PROP-039 §9.2). The base actions (page.open /
/// search / lint / quit) are live when the tree is the focus; the form actions
/// (apply / reset / layer.next / provenance.toggle) are live when a page is
/// open and the focused field is non-text, so their keys (a / r / Tab / ?) do
/// not shadow other interactions at the base screen.
pub(in crate::commands::prefs::tui) fn enabled_in_base(app: &PrefsApp, addr: &ActionAddr) -> bool {
    let ctx = app.action_ctx();
    match addr.name() {
        "page.open" => ctx.at_base && ctx.leaf_selected,
        "search" | "lint" | "quit" => ctx.at_base,
        "apply" | "reset" | "layer.next" | "provenance.toggle" => {
            ctx.page_open && ctx.form_editable
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::commands::prefs::tui::state::PrefsCtx;
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
        let mut a = PrefsApp::new(prefs, schema(), PrefsCtx::new(true));
        a.select_first();
        a
    }

    #[test]
    fn dispatch_page_open_opens_the_selected_leaf() {
        let mut app = app();
        app.table.select(Some(1)); // Palette leaf.
        let addr = ActionAddr::parse("action://vibe.prefs/page.open").unwrap();
        let control = dispatch_by_addr(&mut app, &addr);
        assert!(matches!(control, Control::Changed));
        assert!(app.open_page.is_some(), "the page opened");
    }

    #[test]
    fn dispatch_search_opens_the_window() {
        let mut app = app();
        assert!(app.search.is_none());
        let addr = ActionAddr::parse("action://vibe.prefs/search").unwrap();
        let _ = dispatch_by_addr(&mut app, &addr);
        assert!(app.search.is_some(), "the search window opened");
    }

    #[test]
    fn dispatch_lint_opens_the_modal() {
        let mut app = app();
        let addr = ActionAddr::parse("action://vibe.prefs/lint").unwrap();
        let _ = dispatch_by_addr(&mut app, &addr);
        assert!(app.lint.is_some(), "the lint modal opened");
    }

    #[test]
    fn dispatch_quit_returns_control_quit() {
        let mut app = app();
        let addr = ActionAddr::parse("action://vibe.prefs/quit").unwrap();
        let control = dispatch_by_addr(&mut app, &addr);
        assert!(matches!(control, Control::Quit));
    }

    #[test]
    fn dispatch_layer_next_cycles_the_form_write_layer() {
        let mut app = app();
        app.table.select(Some(1)); // Palette leaf.
        app.open_selected();
        let before = app.form.as_ref().unwrap().write_layer;
        let addr = ActionAddr::parse("action://vibe.prefs/layer.next").unwrap();
        let _ = dispatch_by_addr(&mut app, &addr);
        let after = app.form.as_ref().unwrap().write_layer;
        assert_ne!(before, after, "the write-layer cycled");
    }

    #[test]
    fn dispatch_provenance_toggle_flips_the_flag() {
        let mut app = app();
        app.table.select(Some(1));
        app.open_selected();
        assert!(!app.form.as_ref().unwrap().provenance_open);
        let addr = ActionAddr::parse("action://vibe.prefs/provenance.toggle").unwrap();
        let _ = dispatch_by_addr(&mut app, &addr);
        assert!(
            app.form.as_ref().unwrap().provenance_open,
            "provenance opened via dispatch"
        );
    }

    #[test]
    fn enabled_in_base_gates_form_actions_off_at_base() {
        let app = app();
        let apply = ActionAddr::parse("action://vibe.prefs/apply").unwrap();
        assert!(
            !enabled_in_base(&app, &apply),
            "apply is inert at base (no form)"
        );
        let search = ActionAddr::parse("action://vibe.prefs/search").unwrap();
        assert!(enabled_in_base(&app, &search), "search is live at base");
    }

    #[test]
    fn enabled_in_base_gates_page_open_on_a_leaf_selection() {
        let mut app = app();
        // Row 0 is the Appearance group — not openable.
        app.table.select(Some(0));
        let open = ActionAddr::parse("action://vibe.prefs/page.open").unwrap();
        assert!(!enabled_in_base(&app, &open), "group row is not openable");
        // A leaf is openable.
        app.table.select(Some(1));
        assert!(enabled_in_base(&app, &open), "leaf row is openable");
    }
}
