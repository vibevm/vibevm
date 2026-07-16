//! The draw pass: the status line, the flattened scrollable table, and the
//! mode-aware footer keymap hint (PROP-036 §2.11, PROP-037 §5). All colour comes
//! from [`super::theme`]. The detail card, the Search Everywhere window, and the
//! F-key menus are drawn last, on top.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use rat_widget::tabbed::{TabPlacement, TabType, Tabbed, TabbedState};
use rat_widget::table::Table;
use rat_widget::table::textdata::{Cell, Row};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Flex, Layout, Rect};
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};

use super::state::{App, DisplayMode, RowNode};
use super::theme::Role;
use super::ui::{Button, Window, inner_pad};
use super::{copy, modal, modes};

/// Draw the whole surface for this frame.
pub fn draw(area: Rect, buf: &mut Buffer, app: &mut App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    // A one-line gap between the tree and the footer — visual breathing room
    // (an empty line, background only; nothing is rendered into it).
    let [status, body, _gap, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(2),
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
        super::search::render::draw(area, buf, search, &app.theme);
    }
    // The F-key menus (F2/F3) — a captive dropdown (PROP-037 §7.1/§7.2).
    if app.menu.is_some() {
        super::menu::draw(area, buf, app);
    }
    // The quit-confirm dialog (PROP-037 §7.4) — drawn last, on top of everything.
    if app.confirm_quit {
        render_confirm_quit(area, buf, app);
    }
    // The copy-settings modal (Shift+F6) and its depth-2 file-dest overlay
    // (PROP-037 §10.2/§10.5). Copy-settings is drawn over the base; file-dest
    // is drawn over copy-settings when present — the fixed depth-2 cascade
    // (see `copy`'s module doc). Input capture mirrors this order in `input`.
    if app.copy_settings.is_some() {
        copy::render_settings(area, buf, app);
    }
    if app.file_dest.is_some() {
        copy::render_file_dest(area, buf, app);
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
    let label = app.theme.status_bar();
    let value = app.theme.status_value();
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
                .fg(app.theme.color(Role::Gold))
                .bg(app.theme.color(Role::Surface0))
                .add_modifier(Modifier::BOLD),
        ));
    }
    buf.set_style(area, label);
    Widget::render(Line::from(spans), area, buf);
}

/// The mode-aware footer (PROP-037 §5.2 `#keys`): **two centered rows** — the
/// F-key command row above, the navigation + Enter/Esc row below — so the hints
/// read as a balanced footer with visual rhythm, not one left-jammed line. Only
/// the keys valid in the current context appear (the letter shortcuts the
/// F-keys superseded are gone).
fn render_footer(area: Rect, buf: &mut Buffer, app: &App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let [row1, row2] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);

    // A copy/action flash takes the footer until the next input clears it —
    // centered on the footer's first row.
    if let Some(flash) = &app.flash {
        Widget::render(
            Line::styled(flash.clone(), app.theme.title()).centered(),
            row1,
            buf,
        );
        return;
    }

    // Row 1 — the F-key commands.
    let fkeys: [(&str, &str); 5] = [
        ("F1", "search"),
        ("F2", "sort"),
        ("F3", "mode"),
        ("F4", "settings"),
        ("F6", "copy"),
    ];
    Widget::render(key_row(app, &fkeys), row1, buf);

    // Row 2 — navigation, then Enter/Esc (the quit-confirm binding, §7.4).
    let mut nav: Vec<(&str, &str)> = vec![
        ("\u{2191}\u{2193}", "move"),
        ("\u{2190}\u{2192}", "pan"),
        // Every mode renders through the one Tree widget (PROP-037 §3.1, §4), so
        // Space folds in all of them.
        ("Space", "fold"),
    ];
    if app.display_mode == DisplayMode::Tabs {
        // Shift+←/→ switches tabs; plain ←/→ stays tree-pan (PROP-037 §5.3).
        // `Shift` is written `↑` per §5.2.
        nav.push(("\u{2191}\u{2190}\u{2191}\u{2192}", "tab"));
    }
    nav.push(("Enter", "details"));
    nav.push(("Esc", "quit"));
    Widget::render(key_row(app, &nav), row2, buf);
}

/// Build one centered footer row: `key desc` pairs separated by a dim `•`, the
/// key in the accent key style and the description muted (PROP-037 §5.2). The
/// row is centre-aligned so it sits under the middle of the screen, not jammed
/// to the left edge.
fn key_row(app: &App, entries: &[(&str, &str)]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(entries.len() * 4);
    for (i, (k, d)) in entries.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  \u{2022}  ", app.theme.dim()));
        }
        spans.push(Span::styled((*k).to_string(), app.theme.key()));
        spans.push(Span::styled(format!(" {d}"), app.theme.key_desc()));
    }
    Line::from(spans).centered()
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
        Cell::from("S"),
        Cell::from("T"),
        Cell::from("C"),
    ])
    .style(Some(app.theme.header()));

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
        .style(app.theme.text())
        // Explicit fg+bg so the selected row highlights whether or not the widget
        // holds keyboard focus.
        .select_row_style(Some(app.theme.row_selection()));

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
        .style(app.theme.dim())
        .select_style(app.theme.selection());
    StatefulWidget::render(tabbed, area, buf, &mut tabs_state);

    render_table(tabs_state.widget_area, buf, app);
}

/// Build the coloured table rows for this frame, applying the horizontal pan to
/// the name cell only (`←`/`→`). The load cell and the `T`/`C`/`S` flags carry
/// their semantic colour (PROP-037 §2.2); subheader / separator rows are styled.
fn build_rows(app: &App) -> Vec<Row<'static>> {
    let theme = &app.theme;
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
                    Cell::from(r.load.to_string()).style(Some(theme.load(r.load))),
                    // Column order S, T, C; each flag's ON colour is its column's
                    // own (S=Gold, T=Muted, C=Accent) so the three read at a
                    // glance, while OFF stays the shared dim style.
                    flag_cell(r.in_static, theme, Role::Gold),
                    flag_cell(r.transitive, theme, Role::Muted),
                    flag_cell(r.condition, theme, Role::Accent),
                ]),
                RowNode::Missing => Row::new([
                    Cell::from(name).style(Some(Style::new().fg(theme.color(Role::Love)))),
                    Cell::from("?").style(Some(theme.dim())),
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
                .style(Some(theme.dim())),
                RowNode::Subheader => Row::new([
                    Cell::from(name),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
                .style(Some(
                    theme
                        .accent()
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )),
            }
        })
        .collect()
}

/// A single-character flag cell: `●` in the column's own colour when on, `○`
/// in the shared dim flag-off style when off. The glyph comes from the theme
/// vocabulary (PROP-037 §2.2.2): ●/○ Tier ≥ 1, x/. Tier 0 — never a hardcoded
/// ASCII literal.
fn flag_cell(on: bool, theme: &super::theme::Theme, on_role: Role) -> Cell<'static> {
    if on {
        Cell::from(theme.glyphs().flag_on).style(Some(Style::new().fg(theme.color(on_role))))
    } else {
        Cell::from(theme.glyphs().flag_off).style(Some(theme.flag_off()))
    }
}

/// The quit-confirm dialog (PROP-037 §7.4 `#quit-confirm`): a centered titled
/// window with a body line and two buttons — **OK** (default-focused) and
/// **Cancel**. `Enter` activates the focused button (OK quits, Cancel cancels);
/// `Esc` cancels; Tab/←/→ move the focus between the buttons (handled in
/// `input::handle_confirm_quit`). Drawn last, over everything.
fn render_confirm_quit(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = &app.theme;
    let title = Line::styled(" Really quit? ", theme.title());
    // Seven rows tall — 2 border + interior padding above/below the three
    // content rows (body · gap · buttons) — and wide enough for air around the
    // widest element, so the dialog reads as a window, not a jammed box
    // (PROP-037 §2.2.5 `#spacing`).
    let width = 36u16.min(area.width.saturating_sub(4));
    let height = 7u16.min(area.height.saturating_sub(2));
    let inner = Window::centered(area, buf, title, width, height, theme);
    let content = inner_pad(inner);
    if content.width < 14 || content.height < 3 {
        return;
    }
    // The three content rows, centred vertically inside the padded content so
    // the body and buttons sit off both the top and the bottom border.
    let [body_row, _gap, btn_row] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .flex(Flex::Center)
    .areas(content);

    Widget::render(
        Line::styled("Quit vibe tree?", theme.text()).centered(),
        body_row,
        buf,
    );

    // OK is focused unless the user moved focus to Cancel; the pair is centred.
    let ok = Button::new("OK").focused(!app.confirm_cancel_focused);
    let cancel = Button::new("Cancel").focused(app.confirm_cancel_focused);
    let ok_w = ok.width();
    let cancel_w = cancel.width();
    let gap = 2u16;
    let total = ok_w.saturating_add(gap).saturating_add(cancel_w);
    let start = btn_row.x + btn_row.width.saturating_sub(total) / 2;
    ok.render(Rect::new(start, btn_row.y, ok_w, 1), buf, theme);
    cancel.render(
        Rect::new(start + ok_w + gap, btn_row.y, cancel_w, 1),
        buf,
        theme,
    );
}
