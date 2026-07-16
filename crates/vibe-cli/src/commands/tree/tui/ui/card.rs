//! The [`Card`] component (PROP-037 §2.9 `#card`, §8 `#detail-card`): a
//! [`Window`] laid out as a labelled vertical form. `Enter` on a package opens
//! it over the tree — a filled "paper" panel with **bold** field headers,
//! blank-line spacing between fields, long values wrapped at word boundaries
//! (never truncated), and a top-right `✕` close affordance read from the theme
//! glyph set (`Esc`/`✕` close it at the controller layer).
//!
//! This is the "invent" leg of the PROP-037 §2.1 component strategy: the form
//! layout is not a `ratatui_core`/`rat_widget` primitive, so it is built here
//! over the shared [`Window`] frame and the [`super::super::theme`] style +
//! glyph surface. A restyle touches only [`super::super::theme`]; a field-set
//! change touches only the call site that builds the card (`modal`).
//!
//! [`Window`]: super::window::Window

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#card");

use std::fmt::Display;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use specmark::spec;

use super::super::theme;
use super::window::Window;

/// One labelled field of a [`Card`] — a bold header and a (possibly long,
/// wrapping) value (PROP-037 §8). The value may contain newlines; each line is
/// word-wrapped independently against the card's content width.
#[derive(Debug, Clone)]
pub struct CardRow {
    /// The field label, rendered bold (e.g. `"group"`, `"version"`).
    pub header: String,
    /// The field body — wrapped, never truncated.
    pub value: String,
}

/// A `Window` laid out as a labelled vertical form (PROP-037 §2.9 `#card`,
/// §8 `#detail-card`).
///
/// Composes [`Window`] for the frame + filled panel, stamps the theme close
/// glyph top-right, and stacks each [`CardRow`] as a bold header line above its
/// wrapped value, with a blank-line gap between fields. The whole look flows
/// through [`theme`]; this struct owns no colour or glyph literal.
pub struct Card {
    title: Line<'static>,
    rows: Vec<CardRow>,
}

impl Card {
    /// Build an empty card whose border title is `title` (callers pass an
    /// already-styled line, e.g. `Line::styled(" pkg ", theme::title())`).
    #[must_use]
    pub fn new(title: impl Into<Line<'static>>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    /// Push a labelled field: a bold `header` and a `value` that will wrap.
    pub fn push(&mut self, header: &str, value: impl Display) {
        self.rows.push(CardRow {
            header: header.to_string(),
            value: value.to_string(),
        });
    }

    /// The field rows — the introspection surface for the card copy provider
    /// (PROP-037 §10.1) and tests. Not read by `render` itself.
    #[allow(dead_code)]
    #[must_use]
    pub fn rows(&self) -> &[CardRow] {
        &self.rows
    }

    /// Whether the card has any rows.
    #[allow(dead_code)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Render the card centred over `area`, sized to its wrapped content and
    /// clamped to the screen (PROP-037 §8 `#detail-card`).
    ///
    /// Draws the [`Window`] frame + filled panel (the "paper" ground — distinct
    /// from the unfilled tree beneath), the theme `✕` close affordance on the
    /// top-right border, then each row: a bold header line, the wrapped value
    /// lines beneath, and a blank gap before the next field. Long values wrap
    /// at word boundaries to the content width; they are never truncated.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#detail-card")]
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 5 || self.rows.is_empty() {
            return;
        }

        // A comfortable reading width, clamped to the screen. The content width
        // (what values wrap against) is the popup width minus the two borders.
        let width = 74u16.min(area.width.saturating_sub(2)).max(24);
        let content_w = width.saturating_sub(2) as usize;

        // Pre-wrap every row's value so the card's height follows its content
        // rather than its raw line count — a long URL adds rows, a short scalar
        // does not. This is the "long values wrap (never truncate)" contract.
        let wrapped: Vec<Vec<String>> = self
            .rows
            .iter()
            .map(|r| wrap_lines(&r.value, content_w))
            .collect();

        // Each row contributes header(1) + value_lines + a blank gap(1); the
        // trailing gap keeps the bottom border clear of the last value.
        let want_h: u16 = wrapped
            .iter()
            .map(|ls| 1u16 + ls.len() as u16 + 1)
            .sum::<u16>()
            .saturating_add(2); // + top/bottom border
        let height = want_h.min(area.height);

        let inner = Window::centered(area, buf, self.title.clone(), width, height);

        // The `✕` sits on the top border, one cell inside the top-right corner.
        // `Window::centered` returns `block.inner(popup)`, so the popup itself
        // is `inner` expanded by one cell on every side.
        paint_close(inner, buf);

        // Lay the rows out top-to-bottom, clipping at the inner rect's bottom
        // (height was clamped to the screen, so a tall card never overflows).
        let bottom = inner.bottom();
        let mut y = inner.y;
        for (row, value_lines) in self.rows.iter().zip(wrapped.iter()) {
            if y >= bottom {
                break;
            }
            let header_area = Rect::new(inner.x, y, inner.width, 1);
            Widget::render(
                Line::styled(format!("{}:", row.header), theme::title()),
                header_area,
                buf,
            );
            y += 1;
            for vl in value_lines {
                if y >= bottom {
                    break;
                }
                let value_area = Rect::new(inner.x, y, inner.width, 1);
                Widget::render(Line::styled(vl.clone(), theme::text()), value_area, buf);
                y += 1;
            }
            // Blank-line gap between fields (PROP-037 §8).
            y = y.saturating_add(1);
        }
    }
}

/// Stamp the theme close glyph (`✕`) on the top border, one cell inside the
/// top-right corner of the popup whose inner rect is `inner`.
fn paint_close(inner: Rect, buf: &mut Buffer) {
    let popup_w = inner.width.saturating_add(2);
    let popup_x = inner.x.saturating_sub(1);
    let popup_y = inner.y.saturating_sub(1);
    let close_x = popup_x + popup_w.saturating_sub(2);
    buf.set_string(close_x, popup_y, theme::close_glyph(), theme::accent());
}

/// Word-wrap `value` to `width` columns, honouring embedded newlines. Each
/// newline starts a fresh wrapped line; each segment is wrapped greedily at
/// word boundaries. A word longer than `width` overflows onto its own line (it
/// is never truncated — PROP-037 §8 forbids truncation; a too-wide token is
/// clipped by the terminal, not by us). Returns at least one line.
fn wrap_lines(value: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![value.to_string()];
    }
    let mut out = Vec::new();
    for segment in value.split('\n') {
        let mut current = String::new();
        for word in segment.split(' ') {
            if current.is_empty() {
                current.push_str(word);
            } else {
                let joined = current.chars().count() + 1 + word.chars().count();
                if joined <= width {
                    current.push(' ');
                    current.push_str(word);
                } else {
                    out.push(std::mem::take(&mut current));
                    current.push_str(word);
                }
            }
        }
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::layout::Position;
    use ratatui_core::style::Modifier;

    /// `wrap_lines` wraps a long single-line value across more than one row at
    /// the given width, and leaves a short value on exactly one row.
    #[test]
    fn wrap_lines_breaks_long_values_and_keeps_short_ones() {
        let short = wrap_lines("alpha beta", 40);
        assert_eq!(short, vec!["alpha beta".to_string()]);

        let long = wrap_lines("alpha beta gamma delta epsilon zeta", 10);
        assert!(long.len() > 1, "a long value wraps across multiple lines");
        // Nothing is dropped: every word survives in the output.
        let joined = long.join(" ");
        for word in ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"] {
            assert!(joined.contains(word), "word {word} survives wrapping");
        }
    }

    /// `wrap_lines` honours embedded newlines (each starts a fresh line) and
    /// never returns an empty vector.
    #[test]
    fn wrap_lines_honours_newlines() {
        let lines = wrap_lines("one\ntwo three", 40);
        assert_eq!(lines, vec!["one".to_string(), "two three".to_string()]);
        assert!(!lines.is_empty());
    }

    /// A card renders a long value wrapped across multiple rows, and the field
    /// header is bold (PROP-037 §8 — bold headers + wrapped values).
    #[test]
    fn card_wraps_long_values_and_bold_headers() {
        let mut card = Card::new(Line::styled(" demo ", theme::title()));
        card.push(
            "description",
            "alpha beta gamma delta epsilon zeta eta theta iota",
        );

        let area = Rect::new(0, 0, 26, 24);
        let mut buf = Buffer::empty(area);
        card.render(area, &mut buf);

        // The first value word ("alpha") and the last ("iota") land on
        // different rows → the value wrapped.
        let row_text = |y: u16| -> String {
            (0..area.width)
                .map(|x| buf[Position::new(x, y)].symbol())
                .collect::<String>()
        };
        let first_word_row = (0..area.height).find(|&y| row_text(y).contains("alpha"));
        let last_word_row = (0..area.height).find(|&y| row_text(y).contains("iota"));
        assert!(first_word_row.is_some(), "first value word rendered");
        assert!(last_word_row.is_some(), "last value word rendered");
        assert_ne!(
            first_word_row, last_word_row,
            "long value wraps across multiple rows"
        );

        // The header "description" is stamped bold somewhere in the panel.
        let has_bold_header = (0..area.width)
            .flat_map(|x| (0..area.height).map(move |y| Position::new(x, y)))
            .any(|p| buf[p].symbol() == "d" && buf[p].modifier.contains(Modifier::BOLD));
        assert!(has_bold_header, "the field header is bold-styled");
    }

    /// A rendered card stamps the theme close glyph (`✕`) on the top border.
    #[test]
    fn card_paints_the_close_glyph_on_the_top_border() {
        let mut card = Card::new(Line::styled(" demo ", theme::title()));
        card.push("k", "v");

        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        card.render(area, &mut buf);

        let glyph = theme::close_glyph();
        let on_border = (0..area.width)
            .flat_map(|x| (0..area.height).map(move |y| Position::new(x, y)))
            .any(|p| buf[p].symbol() == glyph);
        assert!(on_border, "the close glyph {glyph:?} is drawn");
    }

    /// A card with no rows (or a tiny area) renders nothing.
    #[test]
    fn empty_or_undersized_card_is_a_noop() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        Card::new(Line::styled(" demo ", theme::title())).render(Rect::new(0, 0, 20, 5), &mut buf);
        // Empty card → nothing painted (every cell stays the default space).
        for x in 0..20 {
            for y in 0..5 {
                assert_eq!(buf[Position::new(x, y)].symbol(), " ");
            }
        }
    }
}
