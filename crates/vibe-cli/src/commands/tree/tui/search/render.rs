//! Draw the Search Everywhere window (PROP-037 §7.3): a rounded, iris-titled
//! panel with a query line, the hybrid "All" + per-category tab strip (pills),
//! the grouped results list (a coloured per-provider badge heads each group in
//! the All tab, one normalized row per hit), and a key hint. Drawn last so it
//! sits on top. All colour comes from [`super::super::theme`].

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f1-search");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use vibe_actions::search::SearchRow;

use super::super::theme::{Role, Theme};
use super::super::ui::Window;
use super::SearchState;

/// Draw the window centered over `area`.
pub fn draw(area: Rect, buf: &mut Buffer, state: &SearchState, theme: &Theme) {
    if area.width < 40 || area.height < 10 {
        return;
    }
    let w = (area.width * 7 / 10).clamp(40, area.width.saturating_sub(4));
    let h = (area.height * 7 / 10).clamp(10, area.height.saturating_sub(2));

    // The centered titled frame is the shared `Window` (PROP-037 §2.3); the
    // query/tabs/results/footer fill the returned inner rect.
    let inner = Window::centered(
        area,
        buf,
        Line::styled(" Search Everywhere ", theme.title()),
        w,
        h,
        theme,
    );

    // One column of horizontal padding inside the border.
    let pad = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    );
    let [query, tabs, rule, results, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(pad);

    draw_query(query, buf, state, theme);
    draw_tabs(tabs, buf, state, theme);
    // A subtle rule between the tab strip and the results.
    buf.set_string(
        rule.x,
        rule.y,
        "\u{2500}".repeat(rule.width as usize),
        theme.border(),
    );
    draw_results(results, buf, state, theme);
    draw_footer(footer, buf, theme);
}

/// The query line: an iris prompt glyph, the typed text, and a block cursor.
fn draw_query(area: Rect, buf: &mut Buffer, state: &SearchState, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled("\u{276f} ", theme.accent()),
        Span::styled(state.query.clone(), theme.text()),
        Span::styled("\u{2588}", theme.accent()),
    ]);
    Widget::render(line, area, buf);
}

/// The tab strip: the active tab a filled iris pill, the rest dim.
fn draw_tabs(area: Rect, buf: &mut Buffer, state: &SearchState, theme: &Theme) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, tab) in state.tabs.iter().enumerate() {
        let active = i == state.tab_idx;
        let label = format!(" {} ", tab.title);
        let style = if active {
            theme.selection()
        } else {
            theme.dim()
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }
    Widget::render(Line::from(spans), area, buf);
}

/// The per-provider badge colour (the coloured group heads in the All tab).
fn provider_color(provider: &str, theme: &Theme) -> Color {
    match provider {
        "packages" => theme.color(Role::Accent),
        "fields" => theme.color(Role::Foam),
        "actions" => theme.color(Role::Gold),
        _ => theme.color(Role::Rose),
    }
}

/// The grouped results list with a scroll window that keeps the selection in
/// view.
fn draw_results(area: Rect, buf: &mut Buffer, state: &SearchState, theme: &Theme) {
    if area.height == 0 || state.rows.is_empty() {
        return;
    }
    let height = area.height as usize;
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
            SearchRow::Header {
                provider,
                title,
                count,
            } => {
                // A coloured pill badge + a dim count (screenshot-5 style).
                let badge = Style::new()
                    .fg(theme.color(Role::Base))
                    .bg(provider_color(provider, theme))
                    .add_modifier(Modifier::BOLD);
                let line = Line::from(vec![
                    Span::styled(format!(" {} ", title.to_uppercase()), badge),
                    Span::styled(format!("  {count}"), theme.dim()),
                ]);
                Widget::render(line, rect, buf);
            }
            SearchRow::Hit(hit) => {
                draw_hit(rect, buf, state, hit, idx == state.selected_row, theme)
            }
        }
    }
}

/// One result row: the primary (with matched ranges bolded in the accent) on
/// the left, the secondary (keybinding / "why disabled" reason) right-aligned.
fn draw_hit(
    rect: Rect,
    buf: &mut Buffer,
    state: &SearchState,
    hit: &vibe_actions::search::Hit,
    selected: bool,
    theme: &Theme,
) {
    let base = if selected {
        theme.selection()
    } else if !hit.enabled {
        theme.dim()
    } else {
        theme.text()
    };
    buf.set_style(rect, base);
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
    // Bold the matched byte ranges into `primary` in the accent (PROP-039 §10.3);
    // on the selection bar keep the bar's fg so it stays legible.
    let hi = if selected {
        base.add_modifier(Modifier::BOLD)
    } else {
        theme.accent().add_modifier(Modifier::BOLD)
    };
    let indent_w = indent.chars().count();
    for &(bs, be) in &hit.match_ranges {
        let (Some(slice), Some(before)) = (hit.primary.get(bs..be), hit.primary.get(..bs)) else {
            continue;
        };
        let col = indent_w + before.chars().count();
        if (col as u16) < rect.width {
            buf.set_stringn(rect.x + col as u16, rect.y, slice, slice.len(), hi);
        }
    }
    if let Some(secondary) = &hit.secondary {
        let sw = secondary.chars().count() as u16;
        if sw + 2 < rect.width {
            let sstyle = if selected { base } else { theme.dim() };
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

/// The key hint.
fn draw_footer(area: Rect, buf: &mut Buffer, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled("\u{2191}/\u{2193}", theme.key()),
        Span::styled(" select   ", theme.key_desc()),
        Span::styled("Tab", theme.key()),
        Span::styled(" category   ", theme.key_desc()),
        Span::styled("Enter", theme.key()),
        Span::styled(" run   ", theme.key_desc()),
        Span::styled("Esc", theme.key()),
        Span::styled(" close", theme.key_desc()),
    ]);
    Widget::render(line, area, buf);
}
