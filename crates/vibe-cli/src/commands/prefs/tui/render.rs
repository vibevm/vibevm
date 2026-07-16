//! The draw pass for the settings TUI (PROP-041 §3). Two panes: the left is
//! the page tree (selectable, scrollable), the right is the open page's
//! placeholder panel for S1 (S2 fills the form, §4). A status line and a
//! footer keymap hint frame the surface. All colour + glyphs come from the
//! shared [`crate::commands::tree::tui::theme::Theme`] — this module owns no
//! colour or glyph literal, so a restyle touches only the theme.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#tree-widget");

use rat_widget::table::Table;
use rat_widget::table::textdata::{Cell, Row};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::BorderType;

use super::state::PrefsApp;

/// Draw the whole settings surface for this frame.
pub fn draw(area: Rect, buf: &mut Buffer, app: &mut PrefsApp) {
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
    render_body(body, buf, app);
    render_footer(footer, buf, app);

    // The lint modal overlays the surface when open (PROP-041 §6 #lint-all).
    if let Some(lint) = &app.lint {
        super::lint::render_lint(area, buf, lint, &app.theme);
    }
    // The Search Everywhere window overlays the surface when open (PROP-041 §7
    // #settings-search) — drawn last so it sits above the lint modal too.
    if let Some(search) = &app.search {
        super::search::render::draw(area, buf, search, &app.theme);
    }
}

/// The status line: the surface title + the active session context (project /
/// user-machine only) + the leaf-page count.
fn render_status(area: Rect, buf: &mut Buffer, app: &PrefsApp) {
    if area.width == 0 {
        return;
    }
    let label = app.theme.status_bar();
    let value = app.theme.status_value();
    let ctx_label = if app.ctx.has_project {
        "project (L1+L2+L3)"
    } else {
        "user-machine (L1 only)"
    };
    let page_count = app
        .registry
        .pages()
        .iter()
        .filter(|d| d.parent_id.is_some())
        .count();
    let spans = vec![
        Span::styled(" vibe prefs ", label),
        Span::styled("  session ", label),
        Span::styled(ctx_label, value),
        Span::styled("  pages ", label),
        Span::styled(page_count.to_string(), value),
    ];
    buf.set_style(area, label);
    Widget::render(Line::from(spans), area, buf);
}

/// The two-pane body: the page tree on the left, the open page on the right.
fn render_body(area: Rect, buf: &mut Buffer, app: &mut PrefsApp) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let [tree_pane, page_pane] =
        Layout::horizontal([Constraint::Length(40), Constraint::Min(0)]).areas(area);
    render_tree(tree_pane, buf, app);
    render_open_page(page_pane, buf, app);
}

/// The flattened, scrollable, selectable page tree (PROP-041 §3 #tree-widget).
fn render_tree(area: Rect, buf: &mut Buffer, app: &mut PrefsApp) {
    // Frame the pane with a themed titled block — the same rounded-border +
    // title-in-accent recipe the tree TUI uses for its chrome.
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(app.theme.border())
        .title(Line::styled(" Pages ", app.theme.title()))
        .style(app.theme.panel());
    let inner = block.inner(area);
    Widget::render(block, area, buf);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let rows = build_rows(app);
    let table = Table::default()
        .rows(rows)
        .widths([Constraint::Fill(1)])
        .style(app.theme.text())
        .select_row_style(Some(app.theme.row_selection()));
    StatefulWidget::render(table, inner, buf, &mut app.table);
}

/// Build the coloured rows for this frame. The origin hint carries the theme's
/// accent colour so a shadowed value reads at a glance (PROP-041 §3
/// `#tree-shows-origin-hint`).
fn build_rows(app: &PrefsApp) -> Vec<Row<'static>> {
    app.rows
        .iter()
        .map(|r| {
            if r.is_group {
                Row::new([Cell::from(r.label.clone())]).style(Some(app.theme.accent()))
            } else {
                // Split the label from the origin hint so the hint gets the
                // accent colour while the name stays plain.
                match r.origin_hint {
                    Some(o) => Row::new([
                        Cell::from(r.label.clone()),
                        Cell::from(format!("[{}]", o.label())).style(Some(app.theme.accent())),
                    ]),
                    None => Row::new([Cell::from(r.label.clone()), Cell::from("")]),
                }
            }
        })
        .collect()
}

/// The open page's pane (PROP-041 §4 `#edit-form`). When a page is open, a
/// themed titled panel frames the per-type edit form (built in
/// [`PrefsApp::open_selected`]); when none, a hint nudges the user to open one.
fn render_open_page(area: Rect, buf: &mut Buffer, app: &mut PrefsApp) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(app.theme.border())
        .title(Line::styled(
            format!(" {} ", open_title(app)),
            app.theme.title(),
        ))
        .style(app.theme.panel());
    let inner = block.inner(area);
    Widget::render(block, area, buf);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    match app.form.as_mut() {
        Some(form) => super::form::render::render_form(inner, buf, form, &app.prefs, &app.theme),
        None => {
            let body = " Select a page and press Enter to open it.";
            if inner.height >= 1 {
                buf.set_stringn(
                    inner.x,
                    inner.y,
                    body,
                    inner.width as usize,
                    app.theme.dim(),
                );
            }
        }
    }
}

/// The right pane's title: the open page's display name, or "Settings".
fn open_title(app: &PrefsApp) -> String {
    app.open_page_title().unwrap_or("Settings").to_owned()
}

/// The action-aware footer (PROP-041 §8 `#commands-are-actions`, PROP-037 §5.2):
/// lists the `vibe.prefs` actions **enabled** for the current context, drawn
/// from the live catalogue via [`super::catalogue::enabled_footer_keys`]. The
/// navigation keys (`↑↓` move, `←→`/Space fold, `Esc` back) are appended as
/// direct-hint entries because they are navigation, not catalogue actions
/// (mirroring the `vibe tree` footer, which mixes keymap actions + direct nav).
fn render_footer(area: Rect, buf: &mut Buffer, app: &PrefsApp) {
    if area.width == 0 {
        return;
    }
    // The enabled vibe.prefs actions for the current context (PROP-037 §5.2).
    let mut keys: Vec<(&str, &str)> = super::catalogue::enabled_footer_keys(app.action_ctx());
    // Navigation hints (not in the action catalogue — direct handlers).
    if app.open_page.is_some() {
        keys.insert(0, ("\u{2191}\u{2193}", " move  "));
        keys.push(("Esc", " back"));
    } else {
        keys.insert(0, ("\u{2191}\u{2193}", " move  "));
        keys.insert(1, ("\u{2190}\u{2192}", " fold  "));
    }
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for (k, desc) in keys {
        spans.push(Span::styled(k.to_string(), app.theme.key()));
        spans.push(Span::styled(desc.to_string(), app.theme.key_desc()));
    }
    Widget::render(Line::from(spans), area, buf);
}
