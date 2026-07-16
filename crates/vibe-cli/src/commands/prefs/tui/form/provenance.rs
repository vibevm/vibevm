//! The provenance view for the focused field (PROP-041 §5 `#provenance-view`,
//! `#provenance-edit`). Renders an inline block under the focused field showing
//! the resolved value, each layer's contribution (`default / L1 / L2 / L3 / CLI
//! / env`), the winning `origin`, and which layers are shadowed — the visual
//! form of `vibe prefs --show-origins`. The winning layer is marked with the
//! theme's fold glyph in the accent style; shadowed layers read dim.
//!
//! Provenance is read through [`ResolvedPrefs::inspect`] (PROP-040 §5) — the
//! surface owns no merge logic (§1 `#surface-not-engine`). The "clear this
//! layer" affordance (§5 `#provenance-edit`) is the form's
//! [`clear_focused`](super::Form::clear_focused) method, wired to a key in the
//! provenance view; this module only renders the data + the hint.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#provenance");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use vibe_settings::loader::Layer;
use vibe_settings::resolver::{InspectValue, Origin};

use crate::commands::tree::tui::theme::Theme;

// ── ProvenanceRow ───────────────────────────────────────────────────────────

/// One layer's contribution in the provenance view (PROP-041 §5
/// `#provenance-view`). Built from [`InspectValue`] in precedence order
/// (highest first) so the block reads top-down as "which layer is winning?".
#[derive(Debug, Clone)]
struct ProvenanceRow {
    /// The layer tag (`"env"` / `"cli"` / `"L3"` / `"L2"` / `"L1"` / `"default"`).
    label: &'static str,
    /// The value at this layer, rendered as a TOML-ish string, or `None` when
    /// the layer does not set the path.
    value: Option<String>,
    /// The layer's [`Origin`] (used to detect the winner + shadowed layers).
    origin: Origin,
}

/// Build the per-layer rows from an [`InspectValue`] (PROP-040 §5 `#inspect`),
/// in descending precedence (env → default) so the winner reads at the top of
/// the stack it actually wins from.
fn provenance_rows(iv: &InspectValue) -> Vec<ProvenanceRow> {
    vec![
        row("env", &iv.env, Origin::Env),
        row("cli", &iv.cli, Origin::Cli),
        row("L3", &iv.l3, Origin::L3),
        row("L2", &iv.l2, Origin::L2),
        row("L1", &iv.l1, Origin::L1),
        row("default", &iv.default, Origin::Default),
    ]
}

/// Format one layer's value into a row (None when the layer is unset).
fn row(label: &'static str, value: &Option<toml::Value>, origin: Origin) -> ProvenanceRow {
    ProvenanceRow {
        label,
        value: value.as_ref().map(format_value),
        origin,
    }
}

/// Render a TOML value as a compact inline string (the provenance view is a
/// summary, not a serialiser — strings are unquoted to read as values).
fn format_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Float(f) => f.to_string(),
        other => other.to_string(),
    }
}

// ── render ──────────────────────────────────────────────────────────────────

/// Render the provenance block for the focused field into `area` starting at
/// `y` (PROP-041 §5 `#provenance-view`). Returns the y after the block. Draws:
///
/// ```text
///   provenance — vibe.tree.palette
///     resolved   catppuccin-mocha
///   ▸ L2         catppuccin-mocha
///     default    rose-pine
///     L1         (unset)
///     L3         (unset)
///   write layer: L3 — x clears here
/// ```
///
/// The winner line carries the theme's fold glyph in the accent+bold style;
/// shadowed layers (set but not winning) read in `dim`; unset layers carry
/// `(unset)` in `dim`. Every colour + glyph comes from [`Theme`].
pub fn render_provenance(
    area: Rect,
    mut y: u16,
    key: &str,
    inspect: &InspectValue,
    write_layer: Layer,
    theme: &Theme,
    buf: &mut Buffer,
) -> u16 {
    let glyphs = theme.glyphs();
    let winner = inspect.origin;

    // Header: `  provenance — vibe.tree.palette`.
    y = write_line(
        area,
        y,
        &format!("  provenance \u{2014} {key}"),
        theme.dim(),
        buf,
    );

    // Resolved value (the effective value, regardless of which layer won).
    y = write_layer_line(
        area,
        y,
        "resolved",
        &format_value(&inspect.value),
        false,
        None,
        theme,
        buf,
    );

    // Per-layer rows: env/cli/L3/L2/L1/default (highest → lowest). The winner
    // gets the accent + fold glyph; a set-but-shadowed layer reads dim with the
    // off glyph; an unset layer carries "(unset)".
    for r in provenance_rows(inspect) {
        let is_winner = r.origin == winner && winner != Origin::Default;
        match &r.value {
            Some(value) => {
                y = write_layer_line(
                    area,
                    y,
                    r.label,
                    value,
                    is_winner,
                    Some(glyphs.fold_collapsed),
                    theme,
                    buf,
                );
            }
            None => {
                let marker = "";
                let line = layer_line_spans(r.label, "(unset)", marker, false, theme);
                y = write_line_obj(area, y, line, buf);
            }
        }
    }

    // Footer: the write-layer + the clear hint (§5 #provenance-edit).
    let hint = format!(
        "  write layer: {} \u{2014} x clears here",
        write_layer.label()
    );
    y = write_line(area, y, &hint, theme.dim(), buf);
    // A trailing blank row separates the block from the next field.
    y.saturating_add(1).min(area.y + area.height)
}

/// Write one layer-value line. `marker` is the winner glyph (the fold marker)
/// when the row is the winner, else an empty string.
#[allow(clippy::too_many_arguments)] // a fixed render signature — matches form::render.
fn write_layer_line(
    area: Rect,
    y: u16,
    label: &str,
    value: &str,
    is_winner: bool,
    marker: Option<&str>,
    theme: &Theme,
    buf: &mut Buffer,
) -> u16 {
    let line = layer_line_spans(label, value, marker.unwrap_or(""), is_winner, theme);
    write_line_obj(area, y, line, buf)
}

/// Compose the styled spans for one layer-value line. A winner line indents the
/// marker (▸) in accent+bold; the label reads in accent for the winner, dim
/// otherwise; the value reads in text for the winner, dim for shadowed. Takes
/// owned `String`s so the returned `Line<'static>` outlives the borrow.
fn layer_line_spans(
    label: &str,
    value: &str,
    marker: &str,
    is_winner: bool,
    theme: &Theme,
) -> Line<'static> {
    let marker_style = if is_winner {
        theme.title()
    } else {
        theme.dim()
    };
    let label_style = if is_winner {
        theme.accent()
    } else {
        theme.dim()
    };
    let value_style = if is_winner { theme.text() } else { theme.dim() };
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled("    ", theme.text()),
        Span::styled(marker.to_owned(), marker_style),
        Span::styled(" ", theme.text()),
        Span::styled(format!("{label:<8}"), label_style),
        Span::styled(value.to_owned(), value_style),
    ];
    if is_winner {
        spans.push(Span::styled("  \u{2190} winner".to_owned(), theme.accent()));
    }
    Line::from(spans)
}

/// Write a `&str` line at `(x=area.x, y)`, truncated to the area width.
fn write_line(
    area: Rect,
    y: u16,
    text: &str,
    style: ratatui_core::style::Style,
    buf: &mut Buffer,
) -> u16 {
    if y >= area.y + area.height || area.width == 0 {
        return y;
    }
    buf.set_stringn(area.x, y, text, area.width as usize, style);
    y + 1
}

/// Write a `Line` at `(x=area.x, y)`.
fn write_line_obj(area: Rect, y: u16, line: Line, buf: &mut Buffer) -> u16 {
    if y >= area.y + area.height {
        return y;
    }
    ratatui_core::widgets::Widget::render(
        line,
        Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        },
        buf,
    );
    y + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::tui::theme::Theme;

    fn inspect_fixture() -> InspectValue {
        // L2 wins over the default; L1/L3/cli/env unset.
        InspectValue {
            value: toml::Value::String("catppuccin-mocha".into()),
            default: Some(toml::Value::String("rose-pine".into())),
            l1: None,
            l2: Some(toml::Value::String("catppuccin-mocha".into())),
            l3: None,
            cli: None,
            env: None,
            origin: Origin::L2,
        }
    }

    #[test]
    fn provenance_rows_are_in_descending_precedence() {
        let iv = inspect_fixture();
        let rows = provenance_rows(&iv);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].label, "env"); // highest first
        assert_eq!(rows[5].label, "default");
    }

    #[test]
    fn render_provenance_names_the_key_winner_and_write_layer() {
        let theme = Theme::default();
        let iv = inspect_fixture();
        let area = Rect::new(0, 0, 50, 16);
        let mut buf = Buffer::empty(area);
        let end = render_provenance(
            area,
            0,
            "vibe.tree.palette",
            &iv,
            Layer::L3,
            &theme,
            &mut buf,
        );
        // The block advanced the cursor past its lines.
        assert!(
            end > 8,
            "the provenance block drew several lines: end={end}"
        );
        let rendered = buffer_string(&buf, area);
        assert!(
            rendered.contains("provenance"),
            "header present: {rendered}"
        );
        assert!(rendered.contains("vibe.tree.palette"), "key present");
        assert!(
            rendered.contains("catppuccin-mocha"),
            "resolved value present"
        );
        assert!(rendered.contains("rose-pine"), "default value present");
        assert!(rendered.contains("winner"), "winner marker present");
        assert!(rendered.contains("(unset)"), "unset layers marked");
        assert!(
            rendered.contains("write layer: L3"),
            "write-layer hint present"
        );
    }

    #[test]
    fn default_winner_marks_no_layer_as_winner() {
        // When the default wins, no layer carries the winner marker — the value
        // is just the built-in default.
        let theme = Theme::default();
        let iv = InspectValue {
            value: toml::Value::String("rose-pine".into()),
            default: Some(toml::Value::String("rose-pine".into())),
            l1: None,
            l2: None,
            l3: None,
            cli: None,
            env: None,
            origin: Origin::Default,
        };
        let area = Rect::new(0, 0, 50, 16);
        let mut buf = Buffer::empty(area);
        render_provenance(
            area,
            0,
            "vibe.tree.palette",
            &iv,
            Layer::L3,
            &theme,
            &mut buf,
        );
        let rendered = buffer_string(&buf, area);
        assert!(
            !rendered.contains("winner"),
            "no winner marker when the default wins: {rendered}"
        );
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
