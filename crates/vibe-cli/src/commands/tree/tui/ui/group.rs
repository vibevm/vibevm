//! [`Group`] — a bordered frame clustering child components, with an optional
//! name at the frame's top-right corner (PROP-037 §2.6 `#group`).
//!
//! ## Strategy — wrap (PROP-037 §2.1, option 1)
//!
//! Unlike [`Button`] (which invents, because `rat_widget`'s button is stateful),
//! a group frame is a *stateless* border — exactly what
//! `ratatui_widgets::block::Block` is. There is no focus graph, no event
//! handling, no per-cell state: [`Group::render`] strokes a border in
//! [`Theme::border()`], fills with [`Theme::panel()`], writes the name
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

use super::super::theme::Theme;

/// A bordered cluster of child components with an optional name at the frame's
/// top-right corner (PROP-037 §2.6). Groups give a multi-setting dialog — the F2
/// sort/shape menu (§7.2) — its visual structure.
///
/// Composes a `Block` for the frame; every colour comes from [`theme`]. Callers
/// lay their children into the inner rect [`Group::render`] returns, and never
/// touch `Block` directly for the group pattern (PROP-037 §2.1).
///
/// A group also carries the focus marker for the F2 menu's focus groups
/// (PROP-037 §5.4 `#focus-groups`): when [`Group::focused`] is set the frame
/// strokes in [`Theme::accent`] and the name in [`Theme::title`] so the user
/// sees where `Tab` has landed; an unfocused group renders dim.
#[derive(Debug, Clone)]
pub struct Group {
    name: Option<String>,
    focused: bool,
}

impl Group {
    /// An unnamed group — a plain bordered frame.
    ///
    /// (Unnamed construction + the `name` accessor are reserved for callers that
    /// build a frame dynamically; the F2 menu uses [`Group::named`] today.)
    #[must_use]
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            name: None,
            focused: false,
        }
    }

    /// A named group — the name renders at the frame's top-right corner (§2.6).
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            focused: false,
        }
    }

    /// Mark this group as the focused focus-group (PROP-037 §5.4): the frame
    /// strokes in the accent colour and the name in the title style so the user
    /// sees where `Tab` will land. Unfocused groups render dim. Used by the F2
    /// menu's multi-group render.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
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
    /// Strokes a plain (square) border, fills the panel with [`Theme::panel()`],
    /// writes the name right-aligned in the top border when set, and returns
    /// `block.inner(area)` for the caller to lay children into. When focused
    /// (PROP-037 §5.4) the border + name use the accent/title styles; otherwise
    /// the muted `border()` / `dim()` styles — the visual signal of which
    /// focus-group `Tab` is in.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#group")]
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) -> Rect {
        let border_style = if self.focused {
            theme.accent()
        } else {
            theme.border()
        };
        let title_style = if self.focused {
            theme.title()
        } else {
            theme.dim()
        };
        let mut block = Block::bordered()
            .border_style(border_style)
            .style(theme.panel());
        if let Some(name) = &self.name {
            block = block.title_top(Line::styled(format!(" {name} "), title_style).right_aligned());
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
        let theme = Theme::default();
        let inner = Group::new().render(area, &mut buf, &theme);
        assert_eq!(inner, Rect::new(1, 1, 18, 3), "inner is inset by one cell");
        // A square top-left border corner sits at the origin (plain border, not
        // rounded — distinct from `Window`'s rounded popup frame).
        let corner = buf[Position::new(0, 0)].symbol();
        assert_eq!(corner, "\u{250c}", "plain square top-left corner \u{250c}");
    }

    /// A named group renders the name somewhere in the top border row.
    #[test]
    fn named_group_writes_the_name_in_the_top_border() {
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        let theme = Theme::default();
        Group::named("Shape").render(area, &mut buf, &theme);
        // The name 'Shape' appears in the top border row (y == 0).
        let top_has_name = (0..area.width).any(|x| buf[Position::new(x, 0)].symbol() == "S");
        assert!(top_has_name, "the group name is rendered in the top border");
    }

    /// A focused group strokes its border in the accent colour (PROP-037 §5.4 —
    /// the focused focus-group is visually marked); an unfocused group strokes
    /// in the muted border. Asserted via the style of a border-side cell.
    #[test]
    fn focused_group_strokes_the_border_in_accent() {
        let area = Rect::new(0, 0, 12, 4);
        let theme = Theme::default();
        let accent = theme.accent();
        let border = theme.border();
        // Focused: a left-border cell (x=0, inner y) carries the accent style.
        let mut buf_f = Buffer::empty(area);
        Group::named("x")
            .focused(true)
            .render(area, &mut buf_f, &theme);
        let side_f = buf_f[Position::new(0, 1)].style();
        assert_eq!(
            side_f.fg, accent.fg,
            "focused group border is accent-coloured"
        );
        // Unfocused: the same cell carries the muted border style.
        let mut buf_u = Buffer::empty(area);
        Group::named("x").render(area, &mut buf_u, &theme);
        let side_u = buf_u[Position::new(0, 1)].style();
        assert_eq!(side_u.fg, border.fg, "unfocused group border is muted");
    }
}
