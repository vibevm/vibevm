//! [`Group`] — a bordered frame clustering child components, with an optional
//! name at the frame's top-right corner (PROP-037 §2.6 `#group`).
//!
//! ## Strategy — wrap (PROP-037 §2.1, option 1)
//!
//! Unlike [`Button`] (which invents, because `rat_widget`'s button is stateful),
//! a group frame is a *stateless* border — exactly what
//! `ratatui_widgets::block::Block` is. There is no focus graph, no event
//! handling, no per-cell state: [`Group::render`] strokes a border in
//! [`theme::border()`], fills with [`theme::panel()`], writes the name
//! right-aligned in the top border, and returns `block.inner(area)`. So this is
//! the straight wrap the §2.1 strategy names for `Group`: the whole look flows
//! through [`theme`], and a restyle never touches this struct.
//!
//! [`Button`]: super::button::Button

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#group");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use specmark::spec;

use super::super::theme;

/// A bordered cluster of child components with an optional name at the frame's
/// top-right corner (PROP-037 §2.6). Groups give a multi-setting dialog — the F2
/// sort/shape menu (§7.2) — its visual structure.
///
/// Composes a `Block` for the frame; every colour comes from [`theme`]. Callers
/// lay their children into the inner rect [`Group::render`] returns, and never
/// touch `Block` directly for the group pattern (PROP-037 §2.1).
#[derive(Debug, Clone)]
pub struct Group {
    name: Option<String>,
}

impl Group {
    /// An unnamed group — a plain bordered frame.
    ///
    /// (Unnamed construction + the `name` accessor are reserved for callers that
    /// build a frame dynamically; the F2 menu uses [`Group::named`] today.)
    #[must_use]
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { name: None }
    }

    /// A named group — the name renders at the frame's top-right corner (§2.6).
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
        }
    }

    /// The group's name, if any.
    #[must_use]
    #[allow(dead_code)]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Render the bordered frame over `area` and return the inner content rect
    /// (PROP-037 §2.6).
    ///
    /// Strokes a plain (square) border in [`theme::border()`], fills the panel
    /// with [`theme::panel()`], writes the name right-aligned in the top border
    /// in [`theme::title()`] when set, and returns `block.inner(area)` for the
    /// caller to lay children into.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#group")]
    pub fn render(&self, area: Rect, buf: &mut Buffer) -> Rect {
        let mut block = Block::bordered()
            .border_style(theme::border())
            .style(theme::panel());
        if let Some(name) = &self.name {
            block =
                block.title_top(Line::styled(format!(" {name} "), theme::title()).right_aligned());
        }
        let inner = block.inner(area);
        Widget::render(block, area, buf);
        inner
    }
}

impl Default for Group {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::layout::Position;

    /// `new()` is unnamed; `named()` carries the name; `name()` round-trips.
    #[test]
    fn name_round_trips() {
        assert!(Group::new().name().is_none());
        assert_eq!(Group::named("Sort by").name(), Some("Sort by"));
    }

    /// `default()` is the unnamed group (the `Default` impl matches `new()`).
    #[test]
    fn default_is_unnamed() {
        assert!(Group::default().name().is_none());
    }

    /// `render` strokes a border and returns an inner rect inset by one cell on
    /// every side (the `block.inner(area)` contract).
    #[test]
    fn render_returns_inset_inner_and_strokes_a_border() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        let inner = Group::new().render(area, &mut buf);
        assert_eq!(inner, Rect::new(1, 1, 18, 3), "inner is inset by one cell");
        // A square top-left border corner sits at the origin (plain border, not
        // rounded — distinct from `Window`'s rounded popup frame).
        let corner = buf[(Position::new(0, 0))].symbol();
        assert_eq!(corner, "\u{250c}", "plain square top-left corner \u{250c}");
    }

    /// A named group renders the name somewhere in the top border row.
    #[test]
    fn named_group_writes_the_name_in_the_top_border() {
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        Group::named("Shape").render(area, &mut buf);
        // The name 'Shape' appears in the top border row (y == 0).
        let top_has_name = (0..area.width).any(|x| buf[(Position::new(x, 0))].symbol() == "S");
        assert!(top_has_name, "the group name is rendered in the top border");
    }
}
