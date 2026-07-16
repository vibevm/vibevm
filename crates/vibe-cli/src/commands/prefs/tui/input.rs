//! Key handling for the settings TUI (PROP-041 §3 `#tree-widget`, §4
//! `#edit-form`, §5 `#provenance-view`, §6 `#validation-feedback`,
//! `#lint-all`, §8 `#commands-are-actions`). The tree-nav keys (↑/↓ move,
//! ←/→ fold, Enter open, q quit) act at the base screen. When a page is open,
//! the form captures input: ↑/↓ move field focus, Space/Enter toggle/select,
//! Tab cycles the write-layer, typing edits a focused text field, `a` applies,
//! `r` resets, `?` toggles the provenance view, `x` clears the focused field at
//! the write-layer (from the provenance view), `c` opens the check-all-layers
//! modal, Esc closes the page. The lint modal, when open, captures all input:
//! ↑/↓ navigate, Enter jumps to the field, Esc closes.
//!
//! Mirrors the `vibe tree` TUI's structure (the `Event::Key(k)` + `match
//! k.code` pattern the copy/file-dest handlers use) over [`PrefsApp`]. The resize
//! repaint is handled first so the display never garbles (the same lesson the
//! tree TUI records).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#tree-widget");

use anyhow::Result;
use rat_salsa::Control;
use rat_widget::event::ct_event;
use ratatui_crossterm::crossterm::event::{Event, KeyCode, KeyEventKind};

use super::form::control::FieldControl;
use super::state::PrefsApp;

/// Handle one terminal event, returning the rat-salsa control-flow verdict.
pub fn handle(event: &Event, app: &mut PrefsApp) -> Result<Control<super::AppEvent>> {
    // A terminal resize must repaint the whole surface (rat-salsa never
    // auto-repaints on resize — the tree TUI records this lesson).
    if let Event::Resize(..) = event {
        return Ok(Control::Changed);
    }

    // The lint modal captures all input when open (PROP-041 §6 #lint-all) —
    // checked before the form so it overlays the page pane.
    if app.lint.is_some() {
        return Ok(handle_lint(event, app));
    }

    // When a page is open, the form captures input (PROP-041 §4).
    if app.form.is_some() {
        return Ok(handle_form(event, app));
    }

    let control = match event {
        ct_event!(keycode press Up) => {
            app.move_up();
            Control::Changed
        }
        ct_event!(keycode press Down) => {
            app.move_down();
            Control::Changed
        }
        ct_event!(keycode press Left) | ct_event!(keycode press Right) => {
            app.toggle_fold_selected();
            Control::Changed
        }
        ct_event!(key press ' ') => {
            app.toggle_fold_selected();
            Control::Changed
        }
        ct_event!(keycode press Enter) => {
            app.open_selected();
            Control::Changed
        }
        // `c` opens the check-all-layers modal (PROP-041 §6 #lint-all).
        ct_event!(key press 'c') | ct_event!(key press 'C') => {
            app.open_lint();
            Control::Changed
        }
        ct_event!(keycode press Esc) => Control::Quit,
        // `q` quits the settings TUI (S1; a quit-confirm gates this in a later
        // phase, mirroring the tree TUI's PROP-037 §7.4 dialog).
        ct_event!(key press 'q') | ct_event!(key press 'Q') => Control::Quit,
        _ => Control::Continue,
    };
    Ok(control)
}

/// The lint-modal handler (PROP-041 §6 `#lint-all`). `↑`/`↓` move the
/// selection, `Enter` jumps to the owning page + focuses the offending field,
/// `Esc` closes the modal. Every other key is swallowed (`Unchanged`) so the
/// modal is modal.
fn handle_lint(event: &Event, app: &mut PrefsApp) -> Control<super::AppEvent> {
    let Event::Key(k) = event else {
        return Control::Unchanged;
    };
    if k.kind != KeyEventKind::Press {
        return Control::Unchanged;
    }
    match k.code {
        KeyCode::Esc => {
            app.close_lint();
            Control::Changed
        }
        KeyCode::Up => {
            app.lint_up();
            Control::Changed
        }
        KeyCode::Down => {
            app.lint_down();
            Control::Changed
        }
        KeyCode::Enter => {
            app.lint_jump_to_selected();
            Control::Changed
        }
        _ => Control::Unchanged,
    }
}

/// The captive form handler (PROP-041 §4 `#edit-form`, §5 `#provenance-view`,
/// §6 `#lint-all`). Routes the terminal event to the open form's field model.
/// `↑`/`↓` move field focus; `Space`/`Enter` toggle a bool / cycle a selection;
/// `Tab` cycles the write-layer; printable chars + `Backspace` edit a focused
/// text field; `?` toggles the provenance view; `x` clears the focused field at
/// the write-layer (from the provenance view); `c` opens the lint modal; `a`
/// applies the form; `r` resets; `Esc` closes the page (back to the tree).
fn handle_form(event: &Event, app: &mut PrefsApp) -> Control<super::AppEvent> {
    let Event::Key(k) = event else {
        return Control::Unchanged;
    };
    if k.kind != KeyEventKind::Press {
        return Control::Unchanged;
    }

    // `c` opens the lint modal — handled before borrowing the form so `app.lint`
    // can be set without a borrow conflict (PROP-041 §6 #lint-all).
    if matches!(k.code, KeyCode::Char('c') | KeyCode::Char('C')) {
        app.open_lint();
        return Control::Changed;
    }

    // Pull the form out so we can move / type / apply without nested borrows of
    // `app`. The schema (for `apply`'s diff) + prefs (for `reset`/`clear`) stay
    // on `app`.
    let Some(form) = app.form.as_mut() else {
        return Control::Unchanged;
    };
    // Whether the focused field is a text field — captured before the match so
    // apply/reset/typing can gate on it without re-borrowing the form.
    let focused_is_text = form
        .focused_field()
        .map(|f| f.control.is_text())
        .unwrap_or(false);
    match k.code {
        KeyCode::Esc => {
            app.close_page();
            Control::Changed
        }
        // Field focus (↑/↓) — always active.
        KeyCode::Up => {
            form.move_up();
            Control::Changed
        }
        KeyCode::Down => {
            form.move_down();
            Control::Changed
        }
        // Tab cycles the write-layer (never types).
        KeyCode::Tab => {
            form.cycle_write_layer();
            Control::Changed
        }
        // `?` toggles the provenance view for the focused field (PROP-041 §5
        // #provenance-view). Never types (`?` is Shift+/; a text field types it
        // only via the `Char(c) if focused_is_text` arm below, which shadows
        // this only when a text field is focused — provenance is reachable from
        // a toggle/selection field first, or by moving focus after toggling).
        KeyCode::Char('?') if !focused_is_text => {
            form.toggle_provenance();
            Control::Changed
        }
        // `x` clears the focused field's value at the write-layer (PROP-041 §5
        // #provenance-edit — "clear L3 to fall back to L2"). Only active when the
        // provenance view is open; the clear-this-layer affordance is in that view.
        KeyCode::Char('x') | KeyCode::Char('X') if form.provenance_open && !focused_is_text => {
            match form.clear_focused(&app.schema, &app.prefs) {
                Ok(()) => Control::Changed,
                Err(err) => {
                    tracing::warn!(
                        %err,
                        "vibe prefs form: clear_focused failed — the layer is not changed"
                    );
                    Control::Changed
                }
            }
        }
        // Space/Enter: toggle a bool / cycle a selection / no-op on text.
        KeyCode::Char(' ') | KeyCode::Enter => {
            if let Some(field) = form.focused_field_mut() {
                field.control.activate();
            }
            Control::Changed
        }
        // Typing into a focused text field.
        KeyCode::Backspace if focused_is_text => {
            if let Some(FieldControl::Text { field, .. }) =
                form.focused_field_mut().map(|f| &mut f.control)
            {
                field.backspace();
            }
            Control::Changed
        }
        KeyCode::Char(c) if focused_is_text => {
            if let Some(FieldControl::Text { field, .. }) =
                form.focused_field_mut().map(|f| &mut f.control)
            {
                field.type_char(c);
            }
            Control::Changed
        }
        // Apply (a) / Reset (r) — only when NOT typing (a focused text field
        // swallows alphanumerics above). Reachable by moving focus to a
        // toggle/selection field first. `form` borrows `app.form`; `app.schema`
        // + `app.prefs` are disjoint fields, so they borrow cleanly alongside.
        // Apply is gated on `has_blocking_error` (§6 #validation-feedback) — a
        // field in error reports why and does not persist.
        KeyCode::Char('a') | KeyCode::Char('A') if !focused_is_text => {
            if form.has_blocking_error() {
                tracing::warn!(
                    "vibe prefs form: apply blocked — a field has a validation error \
                     (violates spec://vibevm/modules/vibe-settings/PROP-041#validation)"
                );
                Control::Changed
            } else {
                match form.apply(&app.schema) {
                    Ok(()) => Control::Changed,
                    Err(err) => {
                        tracing::warn!(
                            %err,
                            "vibe prefs form: apply failed — the change is not persisted"
                        );
                        Control::Changed
                    }
                }
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') if !focused_is_text => {
            form.reset(&app.prefs);
            Control::Changed
        }
        _ => Control::Unchanged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::prefs::tui::state::{PrefsApp, PrefsCtx};
    use ratatui_crossterm::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: ratatui_crossterm::crossterm::event::KeyEventState::NONE,
        })
    }

    #[test]
    fn down_then_enter_opens_a_leaf_page() {
        let mut app = app();
        // Row 0 is the Appearance group; row 1 is the Palette leaf.
        app.table.select(Some(1));
        let ev = press(KeyCode::Enter);
        let ctrl = handle(&ev, &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(app.open_page.is_some(), "Enter opened the leaf page");
    }

    #[test]
    fn esc_at_the_tree_quits() {
        let mut app = app();
        let ctrl = handle(&press(KeyCode::Esc), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Quit));
    }

    #[test]
    fn q_quits() {
        let mut app = app();
        let ctrl = handle(&press(KeyCode::Char('q')), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Quit));
    }

    #[test]
    fn space_toggles_a_group_fold() {
        let mut app = app();
        app.table.select(Some(0)); // Appearance group.
        let before = app.rows.len();
        let ctrl = handle(&press(KeyCode::Char(' ')), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(app.rows.len() < before, "folding hid children");
    }

    #[test]
    fn esc_on_an_open_page_closes_it() {
        let mut app = app();
        // Open the Palette leaf via Enter (builds the form — open_page + form are
        // set together by `open_selected`).
        app.table.select(Some(1));
        let _ = handle(&press(KeyCode::Enter), &mut app).unwrap();
        assert!(app.open_page.is_some());
        assert!(app.form.is_some(), "opening a page builds the form");
        // Esc closes the page (and drops the form).
        let ctrl = handle(&press(KeyCode::Esc), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(app.open_page.is_none(), "Esc closed the open page");
        assert!(app.form.is_none(), "the form dropped with the page");
    }

    #[test]
    fn resize_repaints() {
        let mut app = app();
        let ctrl = handle(&Event::Resize(80, 24), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
    }

    #[test]
    fn space_on_an_open_form_cycles_the_focused_selection() {
        // Open the Palette page (a closed-set string → Selection); Space cycles
        // the option; the form reads modified (PROP-041 §4 #form-per-type).
        let mut app = app();
        app.table.select(Some(1)); // Palette leaf.
        let _ = handle(&press(KeyCode::Enter), &mut app).unwrap();
        let form = app.form.as_ref().unwrap();
        let before = form
            .focused_field()
            .map(|f| f.control.current_value())
            .unwrap();
        // The palette starts at its default "rose-pine".
        assert_eq!(before, toml::Value::String("rose-pine".into()));
        // Space cycles to the next option.
        let ctrl = handle(&press(KeyCode::Char(' ')), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        let form = app.form.as_ref().unwrap();
        let after = form
            .focused_field()
            .map(|f| f.control.current_value())
            .unwrap();
        assert_eq!(after, toml::Value::String("catppuccin-mocha".into()));
        assert!(form.is_modified(), "the edit marked the form modified");
    }

    #[test]
    fn tab_on_an_open_form_cycles_the_write_layer() {
        let mut app = app();
        app.table.select(Some(1));
        let _ = handle(&press(KeyCode::Enter), &mut app).unwrap();
        let before = app.form.as_ref().unwrap().write_layer;
        let ctrl = handle(&press(KeyCode::Tab), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        let after = app.form.as_ref().unwrap().write_layer;
        assert_ne!(before, after, "Tab cycled the write-layer");
    }

    // ── S3: provenance toggle + clear ───────────────────────────────────────

    #[test]
    fn question_mark_toggles_the_provenance_view() {
        let mut app = app();
        app.table.select(Some(1)); // Palette leaf.
        let _ = handle(&press(KeyCode::Enter), &mut app).unwrap();
        let form = app.form.as_ref().unwrap();
        assert!(!form.provenance_open, "provenance starts closed");
        // `?` opens it.
        let ctrl = handle(&press(KeyCode::Char('?')), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(
            app.form.as_ref().unwrap().provenance_open,
            "provenance is open after ?"
        );
        // `?` again closes it.
        let ctrl = handle(&press(KeyCode::Char('?')), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(
            !app.form.as_ref().unwrap().provenance_open,
            "provenance is closed after second ?"
        );
    }

    // ── S4: check-all-layers modal ──────────────────────────────────────────

    #[test]
    fn c_at_the_base_opens_the_lint_modal() {
        let mut app = app();
        assert!(app.lint.is_none());
        let ctrl = handle(&press(KeyCode::Char('c')), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(app.lint.is_some(), "c opened the lint modal");
    }

    #[test]
    fn c_on_an_open_form_also_opens_the_lint_modal() {
        let mut app = app();
        app.table.select(Some(1));
        let _ = handle(&press(KeyCode::Enter), &mut app).unwrap();
        let ctrl = handle(&press(KeyCode::Char('c')), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(app.lint.is_some(), "c opened the lint modal from the form");
    }

    #[test]
    fn esc_closes_the_lint_modal() {
        let mut app = app();
        app.open_lint();
        assert!(app.lint.is_some());
        let ctrl = handle(&press(KeyCode::Esc), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(app.lint.is_none(), "Esc closed the lint modal");
    }

    #[test]
    fn lint_modal_captures_arrow_keys_for_navigation() {
        let mut app = app();
        app.open_lint();
        // The lint modal has no entries (clean layers in the test env) — the
        // navigation is a clamped no-op but still returns Changed (repaint).
        let ctrl = handle(&press(KeyCode::Down), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
    }
}
