//! Key handling for the settings TUI (PROP-041 §3 `#tree-widget`, §8
//! `#commands-are-actions`). S1 carries the direct tree-nav keys: `↑`/`↓` move,
//! `←`/`→` (and `Space`) fold/expand a group, `Enter` opens the focused leaf
//! page, `Esc` closes the open page (or, at the tree, is a no-op pending a
//! quit-confirm — S1 quits on `q`). When a page is open, the page pane captures
//! input: only `Esc` (back) acts; everything else is swallowed. S2 will route
//! through the `vibe-actions` keymap + the form's field model (§4, §8).
//!
//! Mirrors the `vibe tree` TUI's structure but over [`PrefsApp`]. The resize
//! repaint is handled first so the display never garbles (the same lesson the
//! tree TUI records).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#tree-widget");

use anyhow::Result;
use rat_salsa::Control;
use rat_widget::event::ct_event;
use ratatui_crossterm::crossterm::event::Event;

use super::state::PrefsApp;

/// Handle one terminal event, returning the rat-salsa control-flow verdict.
pub fn handle(event: &Event, app: &mut PrefsApp) -> Result<Control<super::AppEvent>> {
    // A terminal resize must repaint the whole surface (rat-salsa never
    // auto-repaints on resize — the tree TUI records this lesson).
    if let Event::Resize(..) = event {
        return Ok(Control::Changed);
    }

    // When a page is open, the page pane captures input: Esc returns to the
    // tree; everything else is swallowed (S1 placeholder; S2 routes to the
    // form's field model, §4).
    if app.open_page.is_some() {
        return Ok(handle_open_page(event, app));
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
        // `q` quits the settings TUI (S1; a quit-confirm gates this in S2,
        // mirroring the tree TUI's PROP-037 §7.4 dialog).
        ct_event!(key press 'q') | ct_event!(key press 'Q') => Control::Quit,
        _ => Control::Continue,
    };
    Ok(control)
}

/// The captive open-page handler (S1): `Esc` returns to the tree; everything
/// else is swallowed. S2 fills the form-field routing here (§4).
fn handle_open_page(event: &Event, app: &mut PrefsApp) -> Control<super::AppEvent> {
    match event {
        ct_event!(keycode press Esc) | ct_event!(keycode press Enter) => {
            app.close_page();
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
        app.open_page = Some("x".into());
        let ctrl = handle(&press(KeyCode::Esc), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
        assert!(app.open_page.is_none(), "Esc closed the open page");
    }

    #[test]
    fn resize_repaints() {
        let mut app = app();
        let ctrl = handle(&Event::Resize(80, 24), &mut app).unwrap();
        assert!(matches!(ctrl, Control::Changed));
    }
}
