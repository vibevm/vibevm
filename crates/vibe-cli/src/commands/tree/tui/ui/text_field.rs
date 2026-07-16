//! [`TextField`] — a single-line editable text input (PROP-037 §2.8
//! `#text-field`). The file-path entry of the copy-to-file modal (§10.5) composes
//! this primitive.
//!
//! ## Strategy — why invent, not wrap (PROP-037 §2.1)
//!
//! `rat-widget`'s text input (`rat_widget::edit::*` + `EditState`/`EditField`)
//! is stateful in the rat-salsa/rat-focus sense: rendering threads a
//! `&mut EditState` carrying a `FocusFlag`, and editing flows through
//! `HandleEvent`/`HasFocus`/`FocusBuilder`. This TUI's controller routes raw
//! `ct_event!` macros to direct mutators and threads no focus graph (see
//! [`super::button`] for the full reasoning). Wrapping `EditState` now would drag
//! the focus subsystem in before the file-path modal (§10.5) is ready to own it
//! — the same trap `Button`/`RadioGroup` avoid by inventing.
//!
//! Phase 7 therefore takes §2.1 option 3 (invent on `ratatui_core`) for a
//! minimal primitive: append-only `type_char`/`backspace`, render the value with
//! a `█` block cursor when focused, all colour through [`theme`]. The §2.8 "a
//! later REQ enriches it" note covers the eventual wrap (cursor movement, selection,
//! validation) once a modal owns a focus graph; the §10.5 path-entry uses this
//! as-is.
//!
//! [`super::button`]: super::button

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#text-field");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use specmark::spec;

use super::super::theme::Theme;

/// A single-line editable text input (PROP-037 §2.8).
///
/// The field shows its value left-aligned; when [`focused`](Self::focused), a
/// `█` block cursor marks the (append-only) edit position at the end of the
/// value. The whole look flows through [`theme`]: a focused field paints the
/// accent ground under the cursor, so focus reads consistently with the rest of
/// the TUI and degrades through every rendering tier.
// Phase-7 component foundation; lights up when the file-path modal (§10.5)
// composes it. Matches the `theme` module's allow.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TextField {
    value: String,
    focused: bool,
}

#[allow(dead_code)]
impl TextField {
    /// New empty, unfocused field.
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: String::new(),
            focused: false,
        }
    }

    /// Set the focus state (builder). The focused field renders the `█` cursor
    /// on the accent ground.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The current value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Whether this field renders as focused.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Append `c` to the value (a typed character).
    pub fn type_char(&mut self, c: char) {
        self.value.push(c);
    }

    /// Delete the last character of the value (Backspace). No-op when empty.
    pub fn backspace(&mut self) {
        self.value.pop();
    }

    /// Render the field into a single-row `area` (PROP-037 §2.8).
    ///
    /// Fills the row with [`Theme::panel()`], writes the value in
    /// [`Theme::text()`], and — when focused — paints a `█` block cursor in
    /// [`Theme::selection()`] (accent ground) at the cell after the last
    /// character (clamped to `area.width`). The panel fill makes the field read
    /// as a bordered input box; the containing `Window`/`Group` supplies the
    /// outer chrome.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#text-field")]
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 {
            return;
        }
        buf.set_style(area, theme.panel());
        buf.set_stringn(
            area.x,
            area.y,
            &self.value,
            area.width as usize,
            theme.text(),
        );
        if self.focused {
            // The cursor sits just past the last rendered character, clamped to
            // the last cell of the row so it always shows even on overflow.
            let last = area.x + area.width.saturating_sub(1);
            let cursor_x = area
                .x
                .saturating_add(self.value.chars().count() as u16)
                .min(last);
            buf.set_stringn(cursor_x, area.y, "\u{2588}", 1, theme.selection());
        }
    }
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::tui::theme::Role;
    use ratatui_core::layout::Position;

    /// `type_char`/`backspace` append and delete; `value()` round-trips.
    #[test]
    fn type_and_backspace_edit_the_value() {
        let mut f = TextField::new();
        assert_eq!(f.value(), "");
        for c in "abc".chars() {
            f.type_char(c);
        }
        assert_eq!(f.value(), "abc");
        f.backspace();
        assert_eq!(f.value(), "ab");
        // Backspace on empty is a no-op (no panic).
        f.backspace();
        f.backspace();
        f.backspace();
        assert_eq!(f.value(), "");
    }

    /// A focused field paints the `█` cursor cell on the accent ground.
    #[test]
    fn focused_field_renders_the_block_cursor() {
        let theme = Theme::default();
        let mut f = TextField::new().focused(true);
        f.type_char('x');
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        f.render(area, &mut buf, &theme);
        // The value 'x' is at column 0; the cursor █ at column 1.
        assert_eq!(buf[Position::new(0, 0)].symbol(), "x");
        assert_eq!(
            buf[Position::new(1, 0)].symbol(),
            "\u{2588}",
            "cursor glyph"
        );
        assert_eq!(
            buf[Position::new(1, 0)].bg,
            theme.color(Role::Accent),
            "cursor on the accent ground"
        );
    }

    /// An unfocused field renders the value with no cursor.
    #[test]
    fn unfocused_field_has_no_cursor() {
        let theme = Theme::default();
        let f = TextField::new().focused(false);
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        f.render(area, &mut buf, &theme);
        assert_eq!(buf[Position::new(0, 0)].symbol(), " ");
        for x in 0..area.width {
            assert_ne!(
                buf[Position::new(x, 0)].symbol(),
                "\u{2588}",
                "no cursor when unfocused"
            );
        }
    }

    /// `new()` starts unfocused; `focused()` toggles; `default()` matches `new()`.
    #[test]
    fn focus_state_round_trips() {
        assert!(!TextField::new().is_focused());
        assert!(TextField::new().focused(true).is_focused());
        assert!(!TextField::default().is_focused());
    }
}
