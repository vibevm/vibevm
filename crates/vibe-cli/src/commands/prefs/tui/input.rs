//! Key handling for the settings TUI (PROP-041 §3 `#tree-widget`, §4
//! `#edit-form`, §8 `#commands-are-actions`). The tree-nav keys (↑/↓ move,
//! ←/→ fold, Enter open, q quit) act at the base screen. When a page is open,
//! the form captures input: ↑/↓ move field focus, Space/Enter toggle/select,
//! Tab cycles the write-layer, typing edits a focused text field, `a` applies,
//! `r` resets, Esc closes the page.
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
        ct_event!(keycode press Esc) => Control::Quit,
        // `q` quits the settings TUI (S1; a quit-confirm gates this in a later
        // phase, mirroring the tree TUI's PROP-037 §7.4 dialog).
        ct_event!(key press 'q') | ct_event!(key press 'Q') => Control::Quit,
        _ => Control::Continue,
    };
    Ok(control)
}

/// The captive form handler (PROP-041 §4 `#edit-form`). Routes the terminal
/// event to the open form's field model. `↑`/`↓` move field focus; `Space`/`Enter`
/// toggle a bool / cycle a selection; `Tab` cycles the write-layer; printable
/// chars + `Backspace` edit a focused text field; `a` applies the form; `r`
/// resets; `Esc` closes the page (back to the tree).
fn handle_form(event: &Event, app: &mut PrefsApp) -> Control<super::AppEvent> {
    let Event::Key(k) = event else {
        return Control::Unchanged;
    };
    if k.kind != KeyEventKind::Press {
        return Control::Unchanged;
    }
    // Pull the form out so we can move / type / apply without nested borrows of
    // `app`. The schema (for `apply`'s diff) + prefs (for `reset`) stay on `app`.
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
        KeyCode::Char('a') | KeyCode::Char('A') if !focused_is_text => {
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
}
