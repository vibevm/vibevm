//! The draw pass: the status line, the flattened scrollable table, and the
//! mode-aware footer keymap hint (PROP-036 §2.11, PROP-037 §5). All colour comes
//! from [`super::theme`]. The detail card, the Search Everywhere window, and the
//! F-key menus are drawn last, on top.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use rat_widget::tabbed::{TabPlacement, TabType, Tabbed, TabbedState};
use rat_widget::table::Table;
use rat_widget::table::textdata::{Cell, Row};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};

use super::state::{App, DisplayMode, RowNode};
use super::{modal, modes, theme};

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
    render_footer(footer, buf, app);

    // The card sits on top of everything, drawn last (§2.11).
    if app.modal_open {
        modal::draw(area, buf, app);
    }
    // Search Everywhere (F1) — a captive window over everything (PROP-037 §7.3).
    if let Some(search) = &app.search {
        super::search::render::draw(area, buf, search);
    }
    // The F-key menus (F2/F3) — a captive dropdown (PROP-037 §7.1/§7.2).
    if let Some(menu) = &app.menu {
        super::menu::draw(area, buf, menu);
    }
}

/// The status line: ordering · display mode · the `STATIC.md` size · the package
/// count · the in-place `@spec` count · a non-fatal diagnostics indicator
/// (shown only when something drifted).
fn render_status(area: Rect, buf: &mut Buffer, app: &App) {
    if area.width == 0 {
        return;
    }
    let (bytes, lines) = match &app.tree.boot.static_md {
        Some(lane) => (lane.bytes, lane.lines),
        None => (0, 0),
    };
    let label = theme::status_bar();
    let value = theme::status_value();
    let mut spans = vec![
        Span::styled(" ordering ", label),
        Span::styled(app.ordering.label(), value),
        Span::styled("   mode ", label),
        Span::styled(app.display_mode.label(), value),
        Span::styled("   STATIC.md ", label),
        Span::styled(format!("{bytes}b / {lines}L"), value),
        Span::styled("   packages ", label),
        Span::styled(app.tree.packages.len().to_string(), value),
        Span::styled("   @spec ", label),
        Span::styled(app.tree.in_place_specs.len().to_string(), value),
    ];
    // A drifted lockfile / other non-fatal findings surface here; hidden when
    // clean so a healthy tree carries no warning noise.
    if !app.tree.diagnostics.is_empty() {
        spans.push(Span::styled(
            format!("   \u{26a0} {} diag", app.tree.diagnostics.len()),
            Style::new()
                .fg(theme::GOLD)
                .bg(theme::SURFACE0)
                .add_modifier(Modifier::BOLD),
        ));
    }
    buf.set_style(area, label);
    Widget::render(Line::from(spans), area, buf);
}

/// The mode-aware footer (PROP-037 §5): only the keys valid in the current
/// context — the letter shortcuts the F-keys superseded are gone.
fn render_footer(area: Rect, buf: &mut Buffer, app: &App) {
    if area.width == 0 {
        return;
    }
    // A copy/action flash takes the footer until the next input clears it.
    if let Some(flash) = &app.flash {
        Widget::render(Line::styled(format!(" {flash}"), theme::title()), area, buf);
        return;
    }
    // Shared commands, then the mode-specific navigation.
    let mut keys: Vec<(&str, &str)> = vec![
        ("F1", " search  "),
        ("F2", " sort  "),
        ("F3", " mode  "),
        ("F6", " copy  "),
        ("\u{2191}\u{2193}", " move  "),
        ("\u{2190}\u{2192}", " pan  "),
        // Every mode renders through the one Tree widget (PROP-037 §3.1, §4), so
        // Space folds in all of them.
        ("Space", " fold  "),
    ];
    if app.display_mode == DisplayMode::Tabs {
        // Shift+←/→ switches tabs; plain ←/→ stays tree-pan (PROP-037 §5.3).
        // `Shift` is written `↑` per §5.2.
        keys.push(("\u{2191}\u{2190}\u{2191}\u{2192}", " tab  "));
    }
    keys.push(("Enter", " details  "));
    keys.push(("q", " quit"));

    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for (k, desc) in keys {
        spans.push(Span::styled(k.to_string(), theme::key()));
        spans.push(Span::styled(desc.to_string(), theme::key_desc()));
    }
    Widget::render(Line::from(spans), area, buf);
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
    .style(Some(theme::header()));

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
        .style(theme::text())
        // Explicit fg+bg so the selected row highlights whether or not the widget
        // holds keyboard focus.
        .select_row_style(Some(theme::row_selection()));

    StatefulWidget::render(table, area, buf, &mut app.table);
}

/// The Tabs display mode: the `Tabbed` chrome with the active tab's flat package
/// list inside its content area (PROP-036 §2.11).
fn render_tabs(area: Rect, buf: &mut Buffer, app: &mut App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let order = modes::group_order(app.static_first);
    let labels: Vec<&'static str> = order.iter().map(|g| g.tab_label()).collect();

    let mut tabs_state = TabbedState::default();
    tabs_state.select(Some(app.tab.min(order.len().saturating_sub(1))));

    let tabbed = Tabbed::new()
        .tab_type(TabType::Attached)
        .placement(TabPlacement::Top)
        .tabs(labels)
        .style(theme::dim())
        .select_style(theme::selection());
    StatefulWidget::render(tabbed, area, buf, &mut tabs_state);

    render_table(tabs_state.widget_area, buf, app);
}

/// Build the coloured table rows for this frame, applying the horizontal pan to
/// the name cell only (`←`/`→`). The load cell and the `T`/`C`/`S` flags carry
/// their semantic colour (PROP-037 §2.2); subheader / separator rows are styled.
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
            match r.node {
                RowNode::Package(_) => Row::new([
                    Cell::from(name),
                    Cell::from(r.load.to_string()).style(Some(theme::load(r.load))),
                    flag_cell(r.transitive),
                    flag_cell(r.condition),
                    flag_cell(r.in_static),
                ]),
                RowNode::Missing => Row::new([
                    Cell::from(name).style(Some(Style::new().fg(theme::LOVE))),
                    Cell::from("?").style(Some(theme::dim())),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ]),
                RowNode::Separator => Row::new([
                    Cell::from(name),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
                .style(Some(theme::dim())),
                RowNode::Subheader => Row::new([
                    Cell::from(name),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
                .style(Some(
                    theme::accent().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )),
            }
        })
        .collect()
}

/// A single-character flag cell in its on/off colour.
fn flag_cell(on: bool) -> Cell<'static> {
    // The glyph comes from the theme vocabulary (PROP-037 §2.2.2): ●/○
    // Tier ≥ 1, x/. Tier 0 — never a hardcoded ASCII literal.
    if on {
        Cell::from(theme::flag_on_glyph()).style(Some(theme::flag_on()))
    } else {
        Cell::from(theme::flag_off_glyph()).style(Some(theme::flag_off()))
    }
}
