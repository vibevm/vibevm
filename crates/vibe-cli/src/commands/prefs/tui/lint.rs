//! The "check all layers" action (PROP-041 §6 `#lint-all`) — a flat list of
//! every schema violation across L1/L2/L3, opened as a modal over the surface.
//! Runs [`schema::validate`] on each loaded layer file, tags each diagnostic
//! with its layer, and renders a `ui::Window` modal with a selectable list.
//! Selecting an entry and pressing `Enter` jumps to the owning page focused on
//! that field (`#lint-all`'s jump-to-field); `Esc` closes the modal.
//!
//! The modal is app-level state on [`PrefsApp`](super::state::PrefsApp): it can
//! be opened whether or not a page is open, and jump-to-field opens the owning
//! page when one exists. Unknown keys (typos no page owns) still list — their
//! jump is a no-op that closes the modal (the list itself is the diagnostic
//! surface; the fix is `vibe prefs migrate` or a manual edit).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#lint-all");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use vibe_settings::loader::{Layer, load_layer};
use vibe_settings::schema::{DiagnosticKind, Schema, validate};

use crate::commands::tree::tui::theme::Theme;
use crate::commands::tree::tui::ui::Window;

use super::form::LayerPaths;

// ── LintEntry / LintState ───────────────────────────────────────────────────

/// One schema violation found in one layer file (PROP-041 §6 `#lint-all`). The
/// layer tag + the diagnostic's path/kind/message; the render flattens these
/// into one selectable row.
#[derive(Debug, Clone)]
pub struct LintEntry {
    /// The layer the diagnostic was found in (L1/L2/L3).
    pub layer: Layer,
    /// The dotted path the diagnostic is about.
    pub path: String,
    /// The diagnostic kind (unknown key / deprecated / …).
    pub kind: DiagnosticKind,
    /// The human-readable message (already cites the rule / migration target).
    pub message: String,
}

/// The modal state — the flat entry list + the selection index (PROP-041 §6
/// `#lint-all`). Owned by [`PrefsApp`](super::state::PrefsApp) when the modal
/// is open; `None` when it is closed.
#[derive(Debug, Clone)]
pub struct LintState {
    /// Every violation across L1/L2/L3, layered-tagged.
    pub entries: Vec<LintEntry>,
    /// The selected row (clamped into range on construct + navigation).
    pub selected: usize,
}

impl LintState {
    /// Build the modal state by running [`schema::validate`] on each loaded
    /// layer file (PROP-041 §6 `#lint-all`). A missing file is an empty table
    /// (PROP-040 §3 `#missing-is-default`); an unreadable one is skipped (the
    /// boot diagnostics already surface parse errors — this pass focuses on
    /// schema violations). Entries are ordered L1 → L2 → L3 then by path.
    pub fn build(schema: &Schema, paths: &LayerPaths) -> Self {
        let mut entries = Vec::new();
        for layer in [Layer::L1, Layer::L2, Layer::L3] {
            let path = paths.path(layer);
            let table = load_layer(path).unwrap_or_default();
            for diag in validate(schema, &table) {
                entries.push(LintEntry {
                    layer,
                    path: diag.path.clone(),
                    kind: diag.kind,
                    message: diag.message,
                });
            }
        }
        Self {
            entries,
            selected: 0,
        }
    }

    /// Whether the lint list is empty (the modal shows a "clean" message then).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The selected entry, if any.
    #[must_use]
    pub fn selected(&self) -> Option<&LintEntry> {
        self.entries.get(self.selected)
    }

    /// Move the selection up one row (clamped at 0).
    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Move the selection down one row (clamped at the last entry).
    pub fn down(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1).min(self.entries.len() - 1);
        }
    }
}

// ── render ──────────────────────────────────────────────────────────────────

/// Render the lint modal over the whole screen (PROP-041 §6 `#lint-all`). A
/// `ui::Window` centered popup lists every violation as `[L2] path — message`,
/// with the selected row highlighted in the accent style. A clean tree shows a
/// "no warnings" line. The footer hint cites the navigation keys.
pub fn render_lint(area: Rect, buf: &mut Buffer, state: &LintState, theme: &Theme) {
    let height = modal_height(state, area);
    let width = modal_width(area);
    let title = Line::styled(
        format!(
            " Check all layers \u{2014} {} warning(s) ",
            state.entries.len()
        ),
        theme.title(),
    );
    let inner = Window::centered(area, buf, title, width, height, theme);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Frame the list inside the window with a subtle inner block (no border —
    // the window already drew one) so the selection highlight reads against the
    // panel ground.
    buf.set_style(inner, theme.panel());

    if state.is_empty() {
        let body = " No warnings \u{2014} every layer is clean. ";
        let line = Line::styled(body, theme.dim());
        Widget::render(line, inner, buf);
        return;
    }

    // Reserve the bottom row for the footer hint.
    let footer_h: u16 = 1;
    let body_h = inner.height.saturating_sub(footer_h.min(inner.height));
    let body = Rect {
        height: body_h.max(1),
        ..inner
    };
    let mut y = body.y;
    let max_rows = body_h as usize;
    // Scroll window: keep the selection visible.
    let first = visible_start(state.selected, max_rows);
    for (i, entry) in state.entries.iter().enumerate().skip(first).take(max_rows) {
        if y >= body.y + body.height {
            break;
        }
        let is_sel = i == state.selected;
        let glyph = kind_glyph(entry.kind);
        let row = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", entry.layer.label()),
                if is_sel {
                    theme.title()
                } else {
                    theme.accent()
                },
            ),
            Span::styled(glyph, theme.warning()),
            Span::styled(
                format!(" {}", entry.path),
                if is_sel { theme.text() } else { theme.dim() },
            ),
            Span::styled(
                format!("  {}", truncate_for(&entry.message, inner.width)),
                if is_sel { theme.text() } else { theme.dim() },
            ),
        ]);
        if is_sel {
            buf.set_style(row, theme.row_selection());
        }
        Widget::render(line, row, buf);
        y = y.saturating_add(1);
    }

    // Footer hint.
    let footer_y = inner.y + inner.height.saturating_sub(footer_h.min(inner.height));
    if footer_y < inner.y + inner.height {
        let hint = " \u{2191}\u{2193} move   Enter jump   Esc close ";
        buf.set_stringn(
            inner.x,
            footer_y,
            hint,
            inner.width as usize,
            theme.key_desc(),
        );
    }
}

/// The glyph for a diagnostic kind (the inline marker, in the warning style).
fn kind_glyph(kind: DiagnosticKind) -> &'static str {
    match kind {
        DiagnosticKind::UnknownKey => "!",
        DiagnosticKind::Deprecated => "~",
        // WrongScope is emitted by the resolver, not validate(); kept for
        // completeness so the lint list can carry it if a future pass adds it.
        DiagnosticKind::WrongScope => "?",
    }
}

/// Truncate a message to fit within `max_width` cells (leaving room for the
/// layer tag + path that precede it on the same line). Returns a `…`-suffixed
/// prefix when truncated.
fn truncate_for(message: &str, max_width: u16) -> String {
    // Reserve ~20 cells for the `[L2] ! path ` prefix; the message gets the rest.
    let reserve: u16 = 20;
    let avail = max_width.saturating_sub(reserve) as usize;
    if message.len() <= avail {
        return message.to_owned();
    }
    let mut end = avail;
    while end > 0 && !message.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        return String::new();
    }
    format!("{}\u{2026}", &message[..end])
}

/// The first visible row index given the selection + the viewport height.
fn visible_start(selected: usize, max_rows: usize) -> usize {
    if max_rows == 0 {
        return 0;
    }
    if selected >= max_rows {
        selected - max_rows + 1
    } else {
        0
    }
}

/// The modal's outer height: the entries + a header allowance + a footer hint,
/// clamped to a reasonable share of the screen.
fn modal_height(state: &LintState, area: Rect) -> u16 {
    let rows = state.entries.len() as u16;
    // header line is in the title; body = rows; +2 for borders; +1 footer; +1
    // blank when empty.
    let needed = rows.saturating_add(3).max(5);
    // Cap at ~70% of the screen height so the underlying surface stays partly
    // visible (the window is a modal, not a full-screen takeover).
    let cap = area.height.saturating_mul(7).saturating_div(10).max(5);
    needed.min(cap)
}

/// The modal's outer width: ~70% of the screen, capped to a readable max.
fn modal_width(area: Rect) -> u16 {
    let cap = area.width.saturating_mul(7).saturating_div(10).max(40);
    cap.min(80)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use vibe_settings::schema::{Deprecation, KeyMeta, KeyType, Scope};

    fn paths(dir: &tempfile::TempDir) -> LayerPaths {
        LayerPaths::new(
            dir.path().join("l1.toml"),
            dir.path().join("l2.toml"),
            dir.path().join("settings.local.toml"),
        )
    }

    fn schema() -> Schema {
        let mut s = Schema::new();
        s.register(
            KeyMeta::new("vibe.tree.palette", KeyType::String, Scope::User, "palette").unwrap(),
        )
        .unwrap();
        s.register(
            KeyMeta::new("node.sort", KeyType::String, Scope::User, "sort")
                .unwrap()
                .with_deprecation(Deprecation::with_replacement("use tree.sort", "tree.sort")),
        )
        .unwrap();
        s
    }

    #[test]
    fn build_collects_unknown_and_deprecated_across_layers() {
        let dir = tempdir().unwrap();
        // L2: a typo + a deprecated key.
        fs::write(
            dir.path().join("l2.toml"),
            "vibe.tree.palate = \"oops\"\nnode.sort = \"name\"\n",
        )
        .unwrap();
        // L3: another typo.
        fs::write(dir.path().join("settings.local.toml"), "ghost.key = 1\n").unwrap();
        let state = LintState::build(&schema(), &paths(&dir));
        assert_eq!(state.entries.len(), 3, "three violations across L2+L3");
        // Layer-tagged: L2 carries the typo + deprecated; L3 carries the ghost.
        let l2: Vec<&LintEntry> = state
            .entries
            .iter()
            .filter(|e| e.layer == Layer::L2)
            .collect();
        assert_eq!(l2.len(), 2);
        let l3: Vec<&LintEntry> = state
            .entries
            .iter()
            .filter(|e| e.layer == Layer::L3)
            .collect();
        assert_eq!(l3.len(), 1);
        assert!(l3[0].path == "ghost.key");
    }

    #[test]
    fn build_on_clean_layers_yields_no_entries() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("l2.toml"),
            "vibe.tree.palette = \"rose-pine\"\n",
        )
        .unwrap();
        let state = LintState::build(&schema(), &paths(&dir));
        assert!(state.is_empty());
    }

    #[test]
    fn build_skips_missing_layers() {
        // All three absent → empty (missing-is-default).
        let dir = tempdir().unwrap();
        let state = LintState::build(&schema(), &paths(&dir));
        assert!(state.is_empty());
    }

    #[test]
    fn navigation_clamps_at_the_ends() {
        let mut state = LintState {
            entries: vec![
                LintEntry {
                    layer: Layer::L2,
                    path: "a".into(),
                    kind: DiagnosticKind::UnknownKey,
                    message: "m".into(),
                },
                LintEntry {
                    layer: Layer::L2,
                    path: "b".into(),
                    kind: DiagnosticKind::UnknownKey,
                    message: "m".into(),
                },
            ],
            selected: 0,
        };
        state.up();
        assert_eq!(state.selected, 0, "clamped at top");
        state.down();
        assert_eq!(state.selected, 1);
        state.down();
        assert_eq!(state.selected, 1, "clamped at bottom");
    }

    #[test]
    fn render_lint_draws_entries_and_footer() {
        let theme = Theme::default();
        let state = LintState {
            entries: vec![
                LintEntry {
                    layer: Layer::L2,
                    path: "tree.palate".into(),
                    kind: DiagnosticKind::UnknownKey,
                    message: "unknown setting".into(),
                },
                LintEntry {
                    layer: Layer::L3,
                    path: "node.sort".into(),
                    kind: DiagnosticKind::Deprecated,
                    message: "deprecated".into(),
                },
            ],
            selected: 0,
        };
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        render_lint(area, &mut buf, &state, &theme);
        let rendered = buffer_string(&buf, area);
        assert!(rendered.contains("Check all layers"), "title present");
        assert!(rendered.contains("tree.palate"), "entry path present");
        assert!(rendered.contains("node.sort"), "second entry present");
        assert!(rendered.contains("move"), "footer hint present");
    }

    #[test]
    fn render_lint_clean_shows_no_warnings_line() {
        let theme = Theme::default();
        let state = LintState {
            entries: Vec::new(),
            selected: 0,
        };
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        render_lint(area, &mut buf, &state, &theme);
        let rendered = buffer_string(&buf, area);
        assert!(rendered.contains("No warnings"), "clean message present");
    }

    fn buffer_string(buf: &Buffer, area: Rect) -> String {
        let mut out = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                out.push_str(buf[ratatui_core::layout::Position::new(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }
}
