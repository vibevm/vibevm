//! The detail modal: `Enter` opens a centered popup showing the selected row's
//! full detail vertically; `Esc`/`Enter` close it (PROP-036 §2.11). Rendered
//! last in the draw pass so it sits on top.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Flex, Layout, Rect};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::BorderType;
use ratatui_widgets::clear::Clear;

use super::super::model::{
    Condition, ConditionKind, DeclaredLink, LoadOrigin, LoadType, Package, Source, SourceKind,
};
use super::state::{App, RowNode};
use super::theme;

/// Draw the detail modal centered over `area`.
pub fn draw(area: Rect, buf: &mut Buffer, app: &App) {
    if area.width < 8 || area.height < 5 {
        return;
    }
    let lines = detail_lines(app);
    if lines.is_empty() {
        return;
    }

    // Fit the popup to the content, clamped to the screen.
    let want_h = (lines.len() as u16).saturating_add(2); // + top/bottom border
    let h = want_h.clamp(3, area.height);
    let w = 74u16.min(area.width.saturating_sub(2)).max(24);

    let [mid] = Layout::vertical([Constraint::Length(h)])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::horizontal([Constraint::Length(w)])
        .flex(Flex::Center)
        .areas(mid);

    // Wipe the area under the popup, then frame it, then fill the detail.
    Widget::render(Clear, popup, buf);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(Line::styled(" package ", theme::title()))
        .style(theme::panel());
    let inner = block.inner(popup);
    Widget::render(block, popup, buf);

    for (i, line) in lines.into_iter().enumerate() {
        let y = i as u16;
        if y >= inner.height {
            break;
        }
        let row = Rect::new(inner.x, inner.y + y, inner.width, 1);
        Widget::render(line, row, buf);
    }
}

/// The vertical detail for the selected row.
fn detail_lines(app: &App) -> Vec<Line<'static>> {
    let Some(row) = app.selected_row() else {
        return Vec::new();
    };
    match row.node {
        // Label rows carry no detail; `open_modal` never opens them, and an
        // empty result keeps the modal closed if one ever slips through.
        RowNode::Separator | RowNode::Subheader => Vec::new(),
        RowNode::Missing => vec![label("id", &row.id), label("status", "not in the lockfile")],
        RowNode::Package(i) => match app.tree.packages.get(i) {
            Some(pkg) => package_lines(pkg),
            None => Vec::new(),
        },
    }
}

/// The full package detail, one field per line (PROP-036 §2.11).
fn package_lines(p: &Package) -> Vec<Line<'static>> {
    let declared = p.load.declared.map(declared_label).unwrap_or("(none)");
    // The fixed leading fields; the source, hash, boot path, and dependency
    // list below are conditional / variable-length, so they push onto this.
    let mut out: Vec<Line<'static>> = vec![
        heading(&p.id),
        label("group", &p.group),
        label("name", &p.name),
        label("version", &p.version),
        label("kind", &p.kind),
        label("load type", load_type_label(p.load.load_type)),
        label("declared link", declared),
        label(
            "transitive",
            &format!(
                "{}   (origin: {})",
                p.load.transitive,
                origin_label(p.load.origin)
            ),
        ),
        condition_line(&p.condition),
        label("in STATIC.md", &p.load.in_static_md.to_string()),
        label("in INDEX.md", &p.load.in_index_md.to_string()),
    ];

    push_source(&mut out, p.source.as_ref());

    out.push(label(
        "content hash",
        p.content_hash.as_deref().unwrap_or("(none)"),
    ));
    out.push(label(
        "boot path",
        p.load.boot_path.as_deref().unwrap_or("(none)"),
    ));

    out.push(label("dependencies", &format!("{}", p.dependencies.len())));
    for dep in &p.dependencies {
        out.push(Line::from(format!("    {dep}")));
    }
    out
}

/// The `when` condition line, with the full raw text when present (§2.5).
fn condition_line(c: &Condition) -> Line<'static> {
    if !c.present {
        return label("condition", "(none)");
    }
    let raw = c.raw.as_deref().unwrap_or("");
    let kind = c.kind.map(condition_kind_label).unwrap_or("");
    let value = c.value.as_deref().unwrap_or("");
    if kind.is_empty() {
        label("condition", raw)
    } else {
        label("condition", &format!("{raw}   ({kind} = {value})"))
    }
}

/// Append the source provenance lines (§2.7 `source`).
fn push_source(out: &mut Vec<Line<'static>>, source: Option<&Source>) {
    match source {
        None => out.push(label("source", "(none)")),
        Some(s) => {
            let kind = s.kind.map(source_kind_label).unwrap_or("(unknown)");
            let url = s.url.as_deref().unwrap_or("");
            out.push(label("source", &format!("{kind}   {url}")));
            if let Some(git_ref) = &s.git_ref {
                out.push(Line::from(format!("    ref: {git_ref}")));
            }
            if let Some(commit) = &s.commit {
                out.push(Line::from(format!("    commit: {commit}")));
            }
        }
    }
}

/// A bold, accent title line.
fn heading(text: &str) -> Line<'static> {
    Line::from(text.to_string()).style(theme::title())
}

/// A `label: value` detail line — a dim label, a bright value.
fn label(name: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{name}: "), theme::dim()),
        Span::styled(value.to_string(), theme::text()),
    ])
}

fn load_type_label(t: LoadType) -> &'static str {
    match t {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
    }
}

fn origin_label(o: LoadOrigin) -> &'static str {
    match o {
        LoadOrigin::Declared => "declared",
        LoadOrigin::Suggested => "suggested",
        LoadOrigin::Default => "default",
        LoadOrigin::StaticTransitive => "static-transitive",
        LoadOrigin::WhenForced => "when-forced",
        LoadOrigin::None => "none",
    }
}

fn declared_label(d: DeclaredLink) -> &'static str {
    match d {
        DeclaredLink::Static => "static",
        DeclaredLink::Dynamic => "dynamic",
        DeclaredLink::StaticTransitive => "static-transitive",
        DeclaredLink::StaticHard => "static-hard",
    }
}

fn source_kind_label(k: SourceKind) -> &'static str {
    match k {
        SourceKind::Registry => "registry",
        SourceKind::Git => "git",
        SourceKind::Override => "override",
        SourceKind::Path => "path",
        SourceKind::Embedded => "embedded",
    }
}

fn condition_kind_label(k: ConditionKind) -> &'static str {
    match k {
        ConditionKind::Os => "os",
    }
}
