//! The [`Button`] component (PROP-037 §2.5 `#button`): a labelled, focusable
//! control. The focused button is highlighted; `Enter` activates it. Dialogs
//! compose `Button`s (OK, Save, Cancel).
//!
//! ## Strategy — why invent, not wrap (PROP-037 §2.1)
//!
//! `rat-widget` *does* ship a `Button` (`rat_widget::button::Button` +
//! `ButtonState`), and §2.1 names `Button` among the components to wrap. But
//! that widget is stateful in the rat-salsa/rat-focus sense: rendering takes a
//! `&mut ButtonState` carrying a `FocusFlag`, and activation flows through
//! `HandleEvent`/`HasFocus`/`FocusBuilder`. This TUI's controller
//! ([`crate::commands::tree::tui::input`]) does not thread that focus
//! infrastructure yet — it routes raw `ct_event!` macros to direct `App`
//! mutators, and no `FocusFlag`/`FocusBuilder` exists anywhere in the tree (the
//! table is driven by `TableState`, not the focus graph). Wrapping
//! `rat_widget::button::Button` now would either force a dummy `ButtonState`
//! through every render call or drag the whole focus subsystem in before any
//! dialog is ready to own it —both worse than a minimal primitive.
//!
//! Phase 3 therefore takes §2.1 option 3 (invent on `ratatui_core`) for the
//! render surface, styled exclusively through [`theme`] so no `Color` is
//! hard-coded and the look degrades through every rendering tier (PROP-037
//! §2.2.3). The wrap (option 1) is a deliberate later retrofit: once a dialog
//! owns a focus graph, [`Button`] gains a `ButtonState`-backed companion without
//! its call sites changing, because they already talk to `ui::Button`.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#button");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use specmark::spec;

use super::super::theme::Theme;

/// A labelled, focusable button (PROP-037 §2.5).
///
/// The focused button is painted with [`Theme::selection()`] — accent ground,
/// base text, bold — the same highlight the table and the menus use, so focus
/// reads consistently and degrades through every rendering tier with no
/// hard-coded [`Color`]. An unfocused button is [`Theme::dim()`] (the codebase's
/// established inactive style; a `subtext()` helper does not exist on the
/// theme yet). Construction is a builder; [`Button::render`] draws a single row.
///
/// `Enter`-activates is wired at the controller layer when a dialog owns the
/// button; this primitive is render-only for Phase 3.
///
/// [`Color`]: ratatui_core::style::Color
// Phase-3 component foundation; lights up when P6 (quit-confirm) / P7
// (ComingSoon) compose it. Matches the `theme` module's Phase-3 `#[allow]`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Button {
    label: String,
    focused: bool,
}

#[allow(dead_code)]
impl Button {
    /// New button with `label`. Starts unfocused; call [`Button::focused`] to
    /// mark it as the active control.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            focused: false,
        }
    }

    /// Set the focus state (builder). The focused button renders with the accent
    /// selection highlight (PROP-037 §2.5).
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The button's label text.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether this button renders as focused.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// The outer width the button occupies when rendered: the label plus one
    /// cell of padding on the left (the cell to the right of the label stays
    /// filled with the style background, so two adjacent buttons read evenly).
    /// Use this to centre a button in a row.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.label.chars().count() as u16 + 1
    }

    /// Render the button into a single-row `area` (PROP-037 §2.5).
    ///
    /// The whole area is styled — [`Theme::selection()`] when focused (accent
    /// ground, base text, bold), [`Theme::dim()`] otherwise — and the padded
    /// label is written from the left. Truncates to `area.width`.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#button")]
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let style = if self.focused {
            theme.selection()
        } else {
            theme.dim()
        };
        buf.set_style(area, style);
        // The leading space is the left padding; `set_stringn` writes from
        // (area.x, area.y) and truncates at area.width. Trailing cells keep the
        // styled background filled in by `set_style` above.
        let text = format!(" {}", self.label);
        buf.set_stringn(area.x, area.y, &text, area.width as usize, style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::tui::theme::Role;
    use ratatui_core::layout::Position;
    use ratatui_core::style::{Color, Modifier};

    /// A focused button paints the accent ground under the whole area and writes
    /// the padded label.
    #[test]
    fn focused_button_paints_the_selection_highlight() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let theme = Theme::default();
        Button::new("OK")
            .focused(true)
            .render(Rect::new(0, 0, 10, 1), &mut buf, &theme);

        // The whole row is the accent background + base foreground, bold.
        let cell = &buf[Position::new(0, 0)];
        assert_eq!(
            cell.bg,
            theme.color(Role::Accent),
            "focused bg is the accent"
        );
        assert_eq!(
            cell.fg,
            theme.color(Role::Base),
            "focused fg is the base text"
        );
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "focused button is bold"
        );
        // The label is written after the leading padding space.
        assert_eq!(buf[Position::new(1, 0)].symbol(), "O");
        assert_eq!(buf[Position::new(2, 0)].symbol(), "K");
    }

    /// An unfocused button renders dim (the muted foreground), no accent ground.
    #[test]
    fn unfocused_button_is_dim() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let theme = Theme::default();
        Button::new("Cancel")
            .focused(false)
            .render(Rect::new(0, 0, 10, 1), &mut buf, &theme);

        let cell = &buf[Position::new(0, 0)];
        assert_eq!(
            cell.fg,
            theme.color(Role::Muted),
            "unfocused fg is the muted role"
        );
        assert_eq!(
            cell.bg,
            Color::Reset,
            "unfocused button has no accent ground"
        );
        assert_eq!(buf[Position::new(1, 0)].symbol(), "C");
    }

    /// `width()` is the label plus one cell of left padding.
    #[test]
    fn width_is_label_plus_padding() {
        assert_eq!(Button::new("OK").width(), 3);
        assert_eq!(Button::new("Cancel").width(), 7);
    }

    /// `focused()` is the focus toggle; `new()` starts unfocused.
    #[test]
    fn focused_toggles_and_new_starts_unfocused() {
        assert!(!Button::new("OK").is_focused());
        assert!(Button::new("OK").focused(true).is_focused());
        assert!(!Button::new("OK").focused(true).focused(false).is_focused());
    }
}
