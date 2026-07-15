//! Draw the Search Everywhere window (PROP-037 §7.3): a large centered modal with
//! a query line, the hybrid "All" + per-category tab strip, the grouped results
//! list (headers in All, one normalized row per hit), and a key hint. Drawn last
//! in the frame so it sits on top.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f1-search");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Flex, Layout, Rect};
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use ratatui_widgets::clear::Clear;

use vibe_actions::search::SearchRow;

use super::SearchState;

/// Draw the window centered over `area`.
pub fn draw(area: Rect, buf: &mut Buffer, state: &SearchState) {
    if area.width < 30 || area.height < 8 {
        return;
    }
    // A large, centered window: ~80% each way, clamped.
    let w = (area.width * 8 / 10).clamp(30, area.width.saturating_sub(2));
    let h = (area.height * 8 / 10).clamp(8, area.height.saturating_sub(2));
    let [mid] = Layout::vertical([Constraint::Length(h)])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::horizontal([Constraint::Length(w)])
        .flex(Flex::Center)
        .areas(mid);

    Widget::render(Clear, popup, buf);
    let block = Block::bordered().title(" Search Everywhere ");
    let inner = block.inner(popup);
    Widget::render(block, popup, buf);

    let [query, tabs, results, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);

    draw_query(query, buf, state);
    draw_tabs(tabs, buf, state);
    draw_results(results, buf, state);
    draw_footer(footer, buf);
}

/// The query line: a prompt glyph, the typed text, and a block cursor.
fn draw_query(area: Rect, buf: &mut Buffer, state: &SearchState) {
    let line = Line::from(vec![
        Span::styled("\u{276f} ", Style::new().fg(Color::Cyan)),
        Span::raw(state.query.clone()),
        Span::styled("\u{2588}", Style::new().add_modifier(Modifier::SLOW_BLINK)),
    ]);
    Widget::render(line, area, buf);
}

/// The tab strip: `[All] Packages  Card fields  Actions`, the active one
/// reversed (PROP-037 §7.3 hybrid + per-category tabs).
fn draw_tabs(area: Rect, buf: &mut Buffer, state: &SearchState) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, tab) in state.tabs.iter().enumerate() {
        let active = i == state.tab_idx;
        let label = format!(" {} ", tab.title);
        let style = if active {
            Style::new().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::new().add_modifier(Modifier::DIM)
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }
    Widget::render(Line::from(spans), area, buf);
}

/// The grouped results list with a scroll window that keeps the selection in
/// view. Headers (All tab only) are underlined; the selected hit is reversed;
/// disabled hits are dim; the secondary text (a keybinding / the field owner) is
/// right-aligned.
fn draw_results(area: Rect, buf: &mut Buffer, state: &SearchState) {
    if area.height == 0 || state.rows.is_empty() {
        return;
    }
    let height = area.height as usize;
    // Scroll so the selected row is visible.
    let start = if state.selected_row >= height {
        state.selected_row - height + 1
    } else {
        0
    };

    for (offset, row) in state.rows.iter().skip(start).take(height).enumerate() {
        let y = area.y + offset as u16;
        let rect = Rect::new(area.x, y, area.width, 1);
        let idx = start + offset;
        match row {
            SearchRow::Header { title, count, .. } => {
                let line = Line::from(Span::styled(
                    format!("{title}  ({count})"),
                    Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ));
                Widget::render(line, rect, buf);
            }
            SearchRow::Hit(hit) => {
                let selected = idx == state.selected_row;
                let mut base = Style::new();
                if !hit.enabled {
                    base = base.add_modifier(Modifier::DIM);
                }
                if selected {
                    base = base.fg(Color::Black).bg(Color::Cyan);
                }
                buf.set_style(rect, base);
                // Primary on the left; secondary right-aligned in a dim style.
                let indent = if matches!(state.tabs.get(state.tab_idx), Some(t) if t.is_all) {
                    "  "
                } else {
                    ""
                };
                buf.set_stringn(
                    rect.x,
                    rect.y,
                    format!("{indent}{}", hit.primary),
                    rect.width as usize,
                    base,
                );
                if let Some(secondary) = &hit.secondary {
                    let sw = secondary.chars().count() as u16;
                    if sw + 2 < rect.width {
                        let sstyle = if selected {
                            base
                        } else {
                            base.add_modifier(Modifier::DIM)
                        };
                        buf.set_stringn(
                            rect.x + rect.width - sw - 1,
                            rect.y,
                            secondary,
                            sw as usize,
                            sstyle,
                        );
                    }
                }
            }
        }
    }
}

/// The key hint.
fn draw_footer(area: Rect, buf: &mut Buffer) {
    let hint = " \u{2191}/\u{2193} select   Tab category   Enter run   Esc close";
    let style = Style::new().add_modifier(Modifier::DIM);
    buf.set_style(area, style);
    buf.set_string(area.x, area.y, hint, style);
}
