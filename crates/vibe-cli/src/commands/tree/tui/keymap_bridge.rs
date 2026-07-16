//! The crossterm → `vibe_actions::Key` bridge (PROP-037 §13.3 `#as-keymap`).
//!
//! The keymap resolver in `vibe_actions::keymap` is pure and terminal-agnostic
//! — it never touches `crossterm`. This module is the thin surface adapter that
//! converts a `crossterm::event::KeyEvent` into a `vibe_actions::Key` so the
//! input handler can drive `Keymap::resolve`. The `#no-render-dep` invariant
//! (PROP-039 §1) stays intact: the bridge lives in the TUI surface, never in
//! the `vibe_actions` crate.
//!
//! Drop rules (§13.3):
//! - non-`Press` events (Release/Repeat) → `None`;
//! - `CONTROL`-modified events → `None` (the TUI has no ctrl-chord bindings);
//! - unsupported codes (media keys, etc.) → `None`.
//!
//! Crossterm sends the spacebar as `Char(' ')`, which is mapped to
//! `KeyCode::Space` so it matches the keymap's `"Space"` binding.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#action-catalogue");

use ratatui_crossterm::crossterm::event::{
    Event, KeyCode as CtKeyCode, KeyEvent, KeyEventKind, KeyModifiers as CtKeyModifiers,
};
use vibe_actions::keymap::{Key, KeyCode, KeyModifiers};

/// Convert a crossterm [`Event`] into a `vibe_actions` [`Key`], or `None` if the
/// event is not a key press the keymap should see.
///
/// `pub(crate)` so the `vibe prefs` TUI (PROP-041 §8 `#commands-are-actions`)
/// can drive the same keymap resolver over the same crossterm events without
/// duplicating the bridge.
pub(crate) fn event_to_key(event: &Event) -> Option<Key> {
    let Event::Key(k) = event else {
        return None;
    };
    if k.kind != KeyEventKind::Press {
        return None;
    }
    key_event_to_key(k)
}

/// The inner conversion — assumes the event is already a Press.
fn key_event_to_key(k: &KeyEvent) -> Option<Key> {
    // The TUI has no ctrl-chord bindings today; drop them so the resolver never
    // sees a partial chord it cannot match.
    if k.modifiers.contains(CtKeyModifiers::CONTROL) {
        return None;
    }
    let code = map_code(k.code)?;
    let mut key = Key::new(code);
    if k.modifiers.contains(CtKeyModifiers::SHIFT) {
        key = key.with_mods(KeyModifiers::SHIFT);
    }
    if k.modifiers.contains(CtKeyModifiers::ALT) {
        key = key.with_mods(KeyModifiers::ALT);
    }
    Some(key)
}

/// Map a crossterm [`KeyCode`] to the `vibe_actions` equivalent. Returns `None`
/// for codes the keymap does not model (e.g. media keys).
fn map_code(code: CtKeyCode) -> Option<KeyCode> {
    Some(match code {
        CtKeyCode::Char(' ') => KeyCode::Space,
        CtKeyCode::Char(ch) => KeyCode::Char(ch),
        CtKeyCode::Enter => KeyCode::Enter,
        CtKeyCode::Tab => KeyCode::Tab,
        CtKeyCode::BackTab => KeyCode::BackTab,
        CtKeyCode::Esc => KeyCode::Esc,
        CtKeyCode::Backspace => KeyCode::Backspace,
        CtKeyCode::Delete => KeyCode::Delete,
        CtKeyCode::Insert => KeyCode::Insert,
        CtKeyCode::Home => KeyCode::Home,
        CtKeyCode::End => KeyCode::End,
        CtKeyCode::PageUp => KeyCode::PageUp,
        CtKeyCode::PageDown => KeyCode::PageDown,
        CtKeyCode::Up => KeyCode::Up,
        CtKeyCode::Down => KeyCode::Down,
        CtKeyCode::Left => KeyCode::Left,
        CtKeyCode::Right => KeyCode::Right,
        CtKeyCode::F(n) => KeyCode::F(n),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn press(code: CtKeyCode, mods: CtKeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, mods))
    }

    #[test]
    fn converts_f1_space_enter_shift_right_esc() {
        let k = event_to_key(&press(CtKeyCode::F(1), CtKeyModifiers::NONE));
        assert_eq!(k, Some(Key::new(KeyCode::F(1))));

        // Spacebar arrives as Char(' ') from crossterm.
        let k = event_to_key(&press(CtKeyCode::Char(' '), CtKeyModifiers::NONE));
        assert_eq!(k, Some(Key::new(KeyCode::Space)));

        let k = event_to_key(&press(CtKeyCode::Enter, CtKeyModifiers::NONE));
        assert_eq!(k, Some(Key::new(KeyCode::Enter)));

        let k = event_to_key(&press(CtKeyCode::Right, CtKeyModifiers::SHIFT));
        assert_eq!(
            k,
            Some(Key::new(KeyCode::Right).with_mods(KeyModifiers::SHIFT))
        );

        let k = event_to_key(&press(CtKeyCode::Esc, CtKeyModifiers::NONE));
        assert_eq!(k, Some(Key::new(KeyCode::Esc)));
    }

    #[test]
    fn drops_release_repeat_and_ctrl_events() {
        // A Release event is dropped.
        let release = Event::Key(KeyEvent {
            code: CtKeyCode::Enter,
            modifiers: CtKeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: ratatui_crossterm::crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(event_to_key(&release), None);

        // Ctrl-modified events are dropped.
        let ctrl_c = press(CtKeyCode::Char('c'), CtKeyModifiers::CONTROL);
        assert_eq!(event_to_key(&ctrl_c), None);
    }

    #[test]
    fn drops_non_key_events() {
        assert_eq!(
            event_to_key(&Event::FocusGained),
            None,
            "FocusGained is not a key"
        );
    }
}
