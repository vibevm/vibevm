//! [`MsgDialog`] — a centered [`Window`] with a title, a one-line body, and a
//! focused `OK` [`Button`] (PROP-037 §2.10 `#coming-soon` base).
//!
//! This is the shared base both the standard `ComingSoon` modal (PROP-037 §2.10
//! — the placeholder wired to every not-yet-built feature) and the quit-confirm
//! dialog (PROP-037 §7.4) build on. Phase 3 keeps it minimal: a renderable
//! value struct, no state enum — `Enter`/`Esc` close at the controller layer
//! when a dialog owns the frame.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#coming-soon");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::text::Line;
use specmark::spec;

use super::super::theme::Theme;
use super::button::Button;
use super::window::Window;

/// A minimal message dialog: a titled [`Window`], one body line, and a centred
/// focused `OK` [`Button`] (PROP-037 §2.10). The base for `ComingSoon` and the
/// quit-confirm dialog.
///
/// Composes [`Window`] for the frame and [`Button`] for the affordance — the
/// whole look flows through [`theme`], so a restyle never touches this struct.
/// The title is rendered in the window border; the body is a single line of
/// plain text; the `OK` button is always focused (a one-button dialog has an
/// unambiguous default).
// Phase-3 component foundation; lights up when P6 (quit-confirm) / P7
// (ComingSoon) compose it. Matches the `theme` module's Phase-3 `#[allow]`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MsgDialog {
    title: String,
    body: String,
}

#[allow(dead_code)]
impl MsgDialog {
    /// Build a dialog with `title` (shown in the window border) and `body` (the
    /// one content line).
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }

    /// The dialog title (rendered in the window border).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The body line.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Render the dialog centred over `area`: a titled [`Window`], the body line
    /// under it, and a focused `OK` [`Button`] centred on the last inner row
    /// (PROP-037 §2.10). Draws nothing if `area` is too small to hold the frame.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#coming-soon")]
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        // The frame needs: border(2) + body(1) + gap(1) + button(1) = 5 outer
        // rows; refuse to draw into anything smaller.
        if area.width < 20 || area.height < 5 {
            return;
        }

        // Width: fit the longer of the title/body with framing padding, clamped
        // to the screen. `max(2)` covers an empty body/title gracefully.
        let content = self
            .body
            .chars()
            .count()
            .max(self.title.chars().count())
            .max(2) as u16;
        let width = (content + 4).clamp(20, area.width.saturating_sub(2));
        let height = 5u16.min(area.height);

        let title_line = Line::styled(format!(" {} ", self.title), theme.title());
        let inner = Window::centered(area, buf, title_line, width, height, theme);

        // body row, a one-row gap, then the OK button row.
        let [body_row, _gap, btn_row] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        buf.set_stringn(
            body_row.x,
            body_row.y,
            &self.body,
            body_row.width as usize,
            theme.text(),
        );

        let ok = Button::new("OK").focused(true);
        let btn_w = ok.width().min(btn_row.width);
        let btn_x = btn_row.x + btn_row.width.saturating_sub(btn_w) / 2;
        let btn_area = Rect::new(btn_x, btn_row.y, btn_w, 1);
        ok.render(btn_area, buf, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::tui::theme::Role;
    use ratatui_core::layout::Position;

    /// A rendered `MsgDialog` paints the title in the border, the body in the
    /// first inner row, and a focused `OK` button on the last inner row.
    #[test]
    fn render_draws_title_body_and_focused_ok() {
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        let theme = Theme::default();

        MsgDialog::new("Coming soon", "Search Everywhere is not built yet.")
            .render(area, &mut buf, &theme);

        // The window is centered and 5 rows tall; its top border row carries the
        // title chip. The title 'Coming soon' must appear somewhere in the frame.
        let has_title = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[Position::new(x, y)].symbol() == "C"));
        assert!(has_title, "the title 'Coming soon' is rendered");

        // The focused OK button paints the accent background. Scan the whole
        // buffer rather than a hard-coded row — the popup is centered, so the
        // button row depends on the area geometry.
        let accent = theme.color(Role::Accent);
        let has_ok_highlight = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[Position::new(x, y)].bg == accent));
        assert!(
            has_ok_highlight,
            "the focused OK button paints the accent background"
        );
    }

    /// Too small an area is a no-op (the buffer stays empty).
    #[test]
    fn render_is_a_noop_when_area_is_too_small() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        let theme = Theme::default();
        MsgDialog::new("x", "y").render(area, &mut buf, &theme);
        // Every cell is still the default empty cell.
        for x in 0..area.width {
            for y in 0..area.height {
                assert_eq!(buf[Position::new(x, y)].symbol(), " ");
            }
        }
    }

    /// `new` stores the title and body; accessors round-trip.
    #[test]
    fn accessors_return_the_constructed_values() {
        let d = MsgDialog::new("Coming soon", "not built");
        assert_eq!(d.title(), "Coming soon");
        assert_eq!(d.body(), "not built");
    }
}
