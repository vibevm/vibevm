//! The draw pass: the status line, the flattened scrollable table, and the
//! footer keymap hint (PROP-036 §2.11). The detail modal is drawn last, on top,
//! by [`super::modal`].

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use rat_widget::tabbed::{TabPlacement, TabType, Tabbed, TabbedState};
use rat_widget::table::Table;
use rat_widget::table::textdata::{Cell, Row};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::widgets::StatefulWidget;

use super::modal;
use super::modes;
use super::state::{App, DisplayMode, RowNode};

/// Draw the whole surface for this frame.
pub fn draw(area: Rect, buf: &mut Buffer, app: &mut App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let [status, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    render_status(status, buf, app);
    match app.display_mode {
        DisplayMode::Tabs => render_tabs(body, buf, app),
        _ => render_table(body, buf, app),
    }
    render_footer(footer, buf);

    // The modal sits on top of everything, so it is drawn last (§2.11).
    if app.modal_open {
        modal::draw(area, buf, app);
    }
}

/// The status line: ordering · display mode · the `STATIC.md` size indicator
/// (PROP-036 §2.6).
fn render_status(area: Rect, buf: &mut Buffer, app: &App) {
    if area.width == 0 {
        return;
    }
    let (bytes, lines) = match &app.tree.boot.static_md {
        Some(lane) => (lane.bytes, lane.lines),
        None => (0, 0),
    };
    // Phase 3: the `n` ordering toggle and the `x` display-mode cycle change
    // these two labels; the line is already rendered from the enums.
    let text = format!(
        " ordering: {}   mode: {}   STATIC.md: {} bytes / {} lines   packages: {}",
        app.ordering.label(),
        app.display_mode.label(),
        bytes,
        lines,
        app.tree.packages.len(),
    );
    let bar = Style::new().fg(Color::Black).bg(Color::Cyan);
    buf.set_style(area, bar);
    buf.set_string(area.x, area.y, &text, bar);
}

/// The footer keymap hint.
fn render_footer(area: Rect, buf: &mut Buffer) {
    if area.width == 0 {
        return;
    }
    let hint = " \u{2191}/\u{2193} move   \u{2190}/\u{2192} pan   Space fold   \
                F fold-all   n order   x mode   t swap   [ ] tabs   \
                Enter detail   q quit";
    let bar = Style::new().add_modifier(Modifier::DIM);
    buf.set_style(area, bar);
    buf.set_string(area.x, area.y, hint, bar);
}

/// The flattened, scrollable, selectable table (PROP-036 §2.2, §2.11).
fn render_table(area: Rect, buf: &mut Buffer, app: &mut App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let rows = build_rows(app);
    let header = Row::new([
        Cell::from("name"),
        Cell::from("load"),
        Cell::from("T"),
        Cell::from("C"),
        Cell::from("S"),
    ])
    .style(Some(Style::new().add_modifier(Modifier::BOLD)));

    let table = Table::default()
        .rows(rows)
        .widths([
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .header(header)
        .column_spacing(1)
        // Explicit fg+bg so the selected row highlights whether or not the
        // widget holds keyboard focus (rat-ftable's unfocused fallback keeps a
        // style that already carries a colour).
        .select_row_style(Some(Style::new().fg(Color::Black).bg(Color::Cyan)));

    StatefulWidget::render(table, area, buf, &mut app.table);
}

/// The Tabs display mode: the `Tabbed` widget chrome with the active tab's flat
/// package list rendered inside its content area (PROP-036 §2.11).
fn render_tabs(area: Rect, buf: &mut Buffer, app: &mut App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let order = modes::group_order(app.static_first);
    let labels: Vec<&'static str> = order.iter().map(|g| g.tab_label()).collect();

    // Selection is driven by `app.tab`; a per-frame state suffices since we read
    // `widget_area` back within this same pass.
    let mut tabs_state = TabbedState::default();
    tabs_state.select(Some(app.tab.min(order.len().saturating_sub(1))));

    let tabbed = Tabbed::new()
        .tab_type(TabType::Attached)
        .placement(TabPlacement::Top)
        .tabs(labels)
        .select_style(Style::new().fg(Color::Black).bg(Color::Cyan));
    StatefulWidget::render(tabbed, area, buf, &mut tabs_state);

    render_table(tabs_state.widget_area, buf, app);
}

/// Build the ratatui rows for this frame, applying the horizontal pan to the
/// name cell only (`←`/`→` — PROP-036 §2.11). The value/checkbox columns stay
/// fixed; subheader rows are styled and never panned.
fn build_rows(app: &App) -> Vec<Row<'static>> {
    app.rows
        .iter()
        .map(|r| {
            let is_label = matches!(r.node, RowNode::Separator | RowNode::Subheader);
            let name: String = if is_label {
                r.name.clone()
            } else {
                r.name.chars().skip(app.h_offset).collect()
            };
            let (load, t, c, s) = match r.node {
                RowNode::Package(_) => (
                    r.load.to_string(),
                    checkbox(r.transitive),
                    checkbox(r.condition),
                    checkbox(r.in_static),
                ),
                RowNode::Missing => (
                    r.load.to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
                RowNode::Separator | RowNode::Subheader => {
                    (String::new(), String::new(), String::new(), String::new())
                }
            };
            let mut row = Row::new([
                Cell::from(name),
                Cell::from(load),
                Cell::from(t),
                Cell::from(c),
                Cell::from(s),
            ]);
            if matches!(r.node, RowNode::Subheader) {
                row = row.style(Some(
                    Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ));
            }
            row
        })
        .collect()
}

/// A single-character checkbox cell.
fn checkbox(on: bool) -> String {
    if on { "x".to_string() } else { ".".to_string() }
}
