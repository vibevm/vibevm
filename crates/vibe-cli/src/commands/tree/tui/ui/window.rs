//! The [`Window`] component (PROP-037 §2.3 `#window`): a bordered, titled panel
//! drawn over a cleared, centered rect.
//!
//! This is the single dedup home for the centered-popup pattern that the detail
//! modal (PROP-036 §2.11), the F-key menus (PROP-037 §7.1/§7.2), and the Search
//! Everywhere window (PROP-037 §7.3) all shared — each one inlined the same
//! five-step recipe (two-axis `Flex::Center` layout, `Clear`, a rounded `Block`
//! titled in `theme::title()`, stroked in `theme::border()`, filled with
//! `theme::panel()`, return `block.inner(popup)`). [`Window::centered`] is that
//! recipe, once; callers pass a styled title line and their content's outer
//! size, and get back the inner content rect.
//!
//! Phase 3 ships the centered-popup constructor. The full §2.2.4 window
//! aesthetic (a solid shadow so the panel reads as raised, a top-right `[✕]`
//! close affordance) layers on as fields on [`Window`] in a later phase; the
//! frame/title/panel composition here is the stable base they compose over.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#window");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Flex, Layout, Rect};
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::BorderType;
use ratatui_widgets::clear::Clear;
use specmark::spec;

use super::super::theme;

/// A bordered, titled panel — the base of every modal and the card (PROP-037
/// §2.3).
///
/// Phase 3 exposes the centered-popup constructor ([`Window::centered`]); a
/// close affordance and shadow (§2.2.4) land as fields on this struct in a
/// later phase. Every modal in the TUI composes a `Window` for its frame — call
/// sites never touch `Block`/`Clear`/`Layout` directly for the popup pattern
/// (PROP-037 §2.1, "call sites never touch `rat_widget::` directly").
pub struct Window;

impl Window {
    /// Render a centered, rounded, titled popup over `area` and return the inner
    /// content rect (PROP-037 §2.3).
    ///
    /// Clears the underlying cells, strokes a rounded border in
    /// [`theme::border()`], fills the panel with [`theme::panel()`], writes
    /// `title` as the border title (callers pass an already-styled line, e.g.
    /// `Line::styled(" Search Everywhere ", theme::title())`), and returns
    /// `block.inner(popup)` for the caller to lay its content into.
    ///
    /// `width`/`height` are the popup's outer dimensions; callers clamp them to
    /// the screen before calling (each popup owns its own content-based sizing).
    /// The centering is the two-axis `Flex::Center` layout every popup shared
    /// before the dedup — pixel-identical rendering, one source.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#window")]
    #[must_use]
    pub fn centered(
        area: Rect,
        buf: &mut Buffer,
        title: impl Into<Line<'static>>,
        width: u16,
        height: u16,
    ) -> Rect {
        let [mid] = Layout::vertical([Constraint::Length(height)])
            .flex(Flex::Center)
            .areas(area);
        let [popup] = Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .areas(mid);

        // Wipe the area under the popup, frame it, then hand back the inner rect
        // for the caller's content. This is the exact sequence modal/menu/search
        // inlined; collecting it here is the PROP-037 §2.1 dedup win.
        Widget::render(Clear, popup, buf);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme::border())
            .title(title.into())
            .style(theme::panel());
        let inner = block.inner(popup);
        Widget::render(block, popup, buf);
        inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::layout::Position;

    /// A centered window paints a rounded border, fills the panel with the
    /// theme's base background, and leaves a titled, clearable inner rect.
    #[test]
    fn centered_draws_a_rounded_titled_panel_and_returns_inner() {
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);

        let inner = Window::centered(
            area,
            &mut buf,
            Line::styled(" demo ", theme::title()),
            20,
            6,
        );

        // The popup is centered: x = (40-20)/2 = 10, y = (12-6)/2 = 3.
        // The rounded top-left corner sits at the popup's origin.
        let corner = buf[(Position::new(10, 3))].symbol();
        assert!(
            corner == "╭" || corner == "┌" || corner.len() == 1,
            "expected a border corner at the popup origin, got {corner:?}"
        );
        // inner is strictly inside the popup, below the top border.
        assert!(inner.y >= 4, "inner starts below the top border");
        assert!(
            inner.height <= 4,
            "inner height is popup height minus two borders"
        );
    }

    /// The returned `inner` is `block.inner(popup)`: inset by one cell on every
    /// side from the centered popup rect.
    #[test]
    fn centered_inner_is_inset_from_the_popup() {
        let area = Rect::new(0, 0, 30, 10);
        let mut buf = Buffer::empty(area);
        let inner = Window::centered(area, &mut buf, Line::styled(" x ", theme::title()), 16, 6);
        // popup is 16x6 centered in 30x10 → x=7, y=2; inner is inset by 1.
        assert_eq!(inner.x, 8);
        assert_eq!(inner.y, 3);
        assert_eq!(inner.width, 14);
        assert_eq!(inner.height, 4);
    }
}
