//! The form render pass (PROP-041 §4 `#edit-form`, `#apply-indicator`, §5
//! `#provenance-view`, §6 `#validation-feedback`). Draws the open page as a
//! vertical form of typed fields — one per preference key — into the right
//! pane's inner rect. Each field renders its label + the `applies` badge
//! (PROP-040 §10, shown per field per `#apply-indicator`) + the per-type
//! control, with the focused field marked by the theme's fold glyph. A field in
//! error renders an inline warning line under its control (§6); the focused
//! field renders its provenance block under the control when the view is open
//! (§5). The write-layer selector and the Apply/Reset/move keymap hint frame the
//! form.
//!
//! Every colour + glyph comes from [`Theme`] — no hardcoded literal, so a
//! restyle touches only the theme. The text-field cursor is composed by building
//! a fresh [`TextField`] with the focus flag set when the field is focused
//! (render is pure — no mutation of the form's state).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#edit-form");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use vibe_settings::resolver::ResolvedPrefs;

use crate::commands::tree::tui::theme::Theme;
use crate::commands::tree::tui::ui::TextField;

use super::Form;
use super::control::FieldControl;
use super::validation::DiagnosticLevel;

/// Draw the form into `area` (the page pane's inner rect). No-op if `area`
/// cannot hold a single row. Fields are laid out top-to-bottom with a y cursor;
/// the write-layer selector + the keymap hint are pinned to the bottom when
/// there is room. `prefs` is read-only — the provenance view (§5) reads
/// `inspect` through it; the surface owns no preference logic (§1
/// `#surface-not-engine`).
pub fn render_form(
    area: Rect,
    buf: &mut Buffer,
    form: &Form,
    prefs: &ResolvedPrefs,
    theme: &Theme,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    buf.set_style(area, theme.panel());

    // Reserve the bottom rows for the write-layer selector + hints.
    let footer_height: u16 = 3; // blank + layer + hints
    let body_height = area.height.saturating_sub(footer_height.min(area.height));
    let body = Rect {
        height: body_height.max(1),
        ..area
    };
    let mut y = body.y;

    // Page description (dim, one row — wrapped/truncated to the pane width).
    if !form.description.is_empty() {
        y = write_line(body, y, &form.description, theme.dim(), buf);
        y = advance(y, body);
    }

    // Each field: a label row (focus marker + short name + applies badge) then
    // the per-type control rows, then an inline warning line if the field is in
    // error (§6 #validation-feedback), then the provenance block when the view
    // is open for the focused field (§5 #provenance-view).
    let glyphs = theme.glyphs();
    let diagnostics = form.diagnostics();
    for (idx, field) in form.fields.iter().enumerate() {
        let focused = idx == form.focus;
        let label = short_label(&field.key);
        let badge = field.applies_label();

        // Label row: `▸ short_label  [applies]`.
        let marker = if focused { glyphs.fold_collapsed } else { " " };
        let marker_style = if focused { theme.accent() } else { theme.dim() };
        let label_style = if focused { theme.title() } else { theme.text() };
        let line = Line::from(vec![
            Span::styled(marker, marker_style),
            Span::styled(" ", theme.text()),
            Span::styled(label, label_style),
            Span::styled("  ", theme.text()),
            Span::styled(format!("[{badge}]"), theme.dim()),
        ]);
        y = write_line_obj(body, y, line, buf);
        y = advance(y, body);

        // Control rows.
        y = render_control(body, y, &field.control, focused, theme, buf);
        y = advance(y, body);

        // Inline validation warning (§6 #validation-feedback) — one line per
        // diagnostic attached to this field, in the warning style with the rule
        // cited. An Error-level diagnostic blocks apply (the keymap hint shows
        // "blocked" when the focused field is in error).
        for diag in diagnostics.iter().filter(|d| d.field_idx == idx) {
            y = render_warning(
                body,
                y,
                &diag.key,
                &diag.message,
                diag.rule,
                diag.level,
                theme,
                buf,
            );
            y = advance(y, body);
        }

        // Provenance block for the focused field (§5 #provenance-view). Renders
        // inline under the control when `provenance_open`; reads `inspect` so
        // the surface owns no merge logic.
        if focused
            && form.provenance_open
            && let Some(iv) = prefs.inspect(&field.key)
        {
            y = super::provenance::render_provenance(
                body,
                y,
                &field.key,
                &iv,
                form.write_layer,
                theme,
                buf,
            );
            y = advance(y, body);
        }
    }

    // Write-layer selector + keymap hint pinned to the bottom.
    let footer_y = area.y + area.height.saturating_sub(footer_height.min(area.height));
    render_footer(area, footer_y, form, theme, buf);
}

/// Render an inline validation warning line under a field (PROP-041 §6
/// `#validation-feedback`). A `!` marker in the warning style (an `Error`-level
/// diagnostic) or `~` (a non-blocking `Warning`), the message, and the rule
/// cited in dim — so the violation reads at a glance and the user can find the
/// governing contract clause.
#[allow(clippy::too_many_arguments)] // a fixed render signature — matches render_control.
fn render_warning(
    area: Rect,
    y: u16,
    key: &str,
    message: &str,
    rule: &str,
    level: DiagnosticLevel,
    theme: &Theme,
    buf: &mut Buffer,
) -> u16 {
    if y >= area.y + area.height {
        return y;
    }
    let (glyph, style) = match level {
        DiagnosticLevel::Error => ("!", theme.warning()),
        DiagnosticLevel::Warning => ("~", theme.dim()),
    };
    let short = message_key(key);
    let line = Line::from(vec![
        Span::styled("  ", theme.text()),
        Span::styled(glyph, style),
        Span::styled(" ", theme.text()),
        Span::styled(short, style),
        Span::styled(": ", style),
        Span::styled(message.to_owned(), style),
        Span::styled(format!("  ({rule})"), theme.dim()),
    ]);
    write_line_obj(area, y, line, buf)
}

/// Render one field's control rows (the per-type surface, §4 `#form-per-type`).
fn render_control(
    area: Rect,
    mut y: u16,
    control: &FieldControl,
    focused: bool,
    theme: &Theme,
    buf: &mut Buffer,
) -> u16 {
    if y >= area.y + area.height {
        return y;
    }
    match control {
        FieldControl::Toggle(b) => {
            // `● true` / `○ false` — the flag glyph + the bool, indented.
            let glyph = if *b {
                theme.glyphs().flag_on
            } else {
                theme.glyphs().flag_off
            };
            let style = if *b { theme.accent() } else { theme.dim() };
            let line = Line::from(vec![
                Span::styled("  ", theme.text()),
                Span::styled(glyph, style),
                Span::styled(format!(" {b}"), theme.text()),
            ]);
            write_line_obj(area, y, line, buf) + 1
        }
        FieldControl::Selection(sel) => {
            // One row per option: `  ● option` / `  ○ option` (RadioGroup vocab).
            let on = theme.glyphs().flag_on;
            let off = theme.glyphs().flag_off;
            for (i, option) in sel.options().iter().enumerate() {
                if y >= area.y + area.height {
                    break;
                }
                let is_sel = i == sel.selected_index();
                let glyph = if is_sel { on } else { off };
                let g_style = if is_sel { theme.accent() } else { theme.dim() };
                let o_style = if is_sel && focused {
                    theme.title()
                } else {
                    theme.text()
                };
                let line = Line::from(vec![
                    Span::styled("  ", theme.text()),
                    Span::styled(glyph, g_style),
                    Span::styled(" ", theme.text()),
                    Span::styled(option.as_str(), o_style),
                ]);
                y = write_line_obj(area, y, line, buf) + 1;
            }
            y
        }
        FieldControl::Text { field, .. } => {
            // A one-row TextField; build a focused copy when this field is
            // focused so the `█` cursor renders (render is pure — no form mutation).
            let row = Rect {
                x: area.x + 2,
                y,
                width: area.width.saturating_sub(2),
                height: 1,
            };
            if row.width > 0 {
                let tf = TextField::new().with_value(field.value()).focused(focused);
                tf.render(row, buf, theme);
            }
            y + 1
        }
        FieldControl::NotEditable(note) => {
            write_line(area, y, &format!("  ({note})"), theme.dim(), buf) + 1
        }
    }
}

/// The write-layer selector + the keymap hint (PROP-041 §4 `#write-layer-choice`,
/// §8 `#commands-are-actions`'s footer convention).
fn render_footer(area: Rect, y: u16, form: &Form, theme: &Theme, buf: &mut Buffer) {
    if y >= area.y + area.height {
        return;
    }
    // Write-layer line: `Write layer: L3 (user-project)`.
    let layer_desc = layer_descriptor(form.write_layer);
    let line = Line::from(vec![
        Span::styled(" Write layer: ", theme.dim()),
        Span::styled(form.write_layer.label(), theme.accent()),
        Span::styled(format!(" ({layer_desc})"), theme.dim()),
    ]);
    let next = write_line_obj(area, y, line, buf) + 1;

    if next < area.y + area.height {
        let modified = form.is_modified();
        // Keymap hint — adapts when a text field is focused (typing then apply/reset
        // move to a non-text field first) and when the provenance view is open
        // (x clears the focused layer). The `?` provenance toggle and `c`
        // check-all-layers are always listed (§5 #provenance-view, §6 #lint-all).
        let hint = if form
            .focused_field()
            .map(|f| f.control.is_text())
            .unwrap_or(false)
        {
            " \u{2191}\u{2193} move   Backspace del   ? provenance   c check   Esc back   Tab layer"
        } else if form.provenance_open {
            " \u{2191}\u{2193} move   ? close provenance   x clear layer   c check   Esc back"
        } else if modified {
            " \u{2191}\u{2193} move   Space/Enter toggle   ? provenance   c check   Tab layer   a apply   r reset   Esc back"
        } else {
            " \u{2191}\u{2193} move   Space/Enter toggle   ? provenance   c check   Tab layer   r reset   Esc back"
        };
        write_line(area, next, hint, theme.key_desc(), buf);
    }
}

/// The one-line descriptor for a layer (mirrors the role-marker wording, short).
fn layer_descriptor(layer: vibe_settings::loader::Layer) -> &'static str {
    match layer {
        vibe_settings::loader::Layer::L1 => "user-machine",
        vibe_settings::loader::Layer::L2 => "repo-shared",
        vibe_settings::loader::Layer::L3 => "user-project",
    }
}

/// The last segment of a dotted path (the field's short label).
fn short_label(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

/// The last segment of a dotted path for a diagnostic (mirrors `short_label` —
/// the inline warning prefixes with the short key name so the user sees which
/// field is in error at a glance).
fn message_key(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
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

/// Advance the y cursor by one blank row (clamped to the body).
fn advance(y: u16, area: Rect) -> u16 {
    y.saturating_add(1).min(area.y + area.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::prefs::tui::form::control::{FieldControl, Selection, TextKind};
    use crate::commands::prefs::tui::form::{FormField, LayerPaths};
    use crate::commands::tree::tui::ui::TextField;
    use vibe_settings::loader::{Layer, LayeredRaw};
    use vibe_settings::resolver::resolve;
    use vibe_settings::schema::{Applies, KeyMeta, KeyType, Scope};

    /// An empty resolved prefs (no layers set) — sufficient for the render tests,
    /// which do not open the provenance view (so `inspect` is never read).
    fn prefs() -> ResolvedPrefs {
        resolve(
            LayeredRaw::default(),
            &vibe_settings::schema::Schema::new(),
            toml::Table::new(),
            toml::Table::new(),
        )
    }

    fn bool_field(name: &str, value: bool, applies: Applies) -> FormField {
        let meta = KeyMeta::new(name, KeyType::Bool, Scope::User, "a bool")
            .unwrap()
            .with_applies(applies)
            .with_default(toml::Value::Boolean(value));
        FormField {
            key: name.to_owned(),
            meta,
            control: FieldControl::Toggle(value),
            baseline: toml::Value::Boolean(value),
        }
    }

    fn selection_field(name: &str, opts: &[&str], selected: usize) -> FormField {
        let meta = KeyMeta::new(name, KeyType::String, Scope::User, "a selection")
            .unwrap()
            .with_default(toml::Value::String(opts[selected].into()));
        let sel = Selection::new(
            "x",
            opts.iter().map(|s| (*s).to_owned()).collect(),
            selected,
        );
        FormField {
            key: name.to_owned(),
            meta,
            control: FieldControl::Selection(sel),
            baseline: toml::Value::String(opts[selected].into()),
        }
    }

    fn text_field(name: &str, value: &str, kind: TextKind) -> FormField {
        let ty = match kind {
            TextKind::Int => KeyType::Int,
            TextKind::String => KeyType::String,
            TextKind::Enum => KeyType::Enum,
        };
        let meta = KeyMeta::new(name, ty, Scope::User, "a text field").unwrap();
        FormField {
            key: name.to_owned(),
            meta,
            control: FieldControl::Text {
                field: TextField::new().with_value(value),
                kind,
            },
            baseline: toml::Value::String(value.into()),
        }
    }

    fn paths() -> LayerPaths {
        LayerPaths::new(
            std::path::PathBuf::from("/tmp/l1.toml"),
            std::path::PathBuf::from("/tmp/l2.toml"),
            std::path::PathBuf::from("/tmp/l3.toml"),
        )
    }

    #[test]
    fn render_form_draws_label_badge_and_toggle_control() {
        let theme = Theme::default();
        let form = Form::for_test(
            "Block",
            "block order",
            vec![bool_field("vibe.tree.flag", true, Applies::Live)],
            Layer::L3,
            paths(),
        );
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        render_form(area, &mut buf, &form, &prefs(), &theme);
        // The label 'flag' appears somewhere, and the toggle shows '● true'.
        let rendered = buffer_string(&buf, area);
        assert!(rendered.contains("flag"), "label present: {rendered}");
        assert!(rendered.contains("[live]"), "applies badge present");
        assert!(rendered.contains("true"), "toggle value present");
        assert!(rendered.contains('\u{25CF}'), "flag_on glyph (●) present");
    }

    #[test]
    fn render_form_draws_selection_options_with_on_off_glyphs() {
        let theme = Theme::default();
        let form = Form::for_test(
            "Mode",
            "the mode",
            vec![selection_field("vibe.tree.mode", &["all", "tabs"], 0)],
            Layer::L3,
            paths(),
        );
        let area = Rect::new(0, 0, 40, 14);
        let mut buf = Buffer::empty(area);
        render_form(area, &mut buf, &form, &prefs(), &theme);
        let rendered = buffer_string(&buf, area);
        assert!(rendered.contains("all"));
        assert!(rendered.contains("tabs"));
        // Both on (●) and off (○) glyphs appear.
        assert!(rendered.contains('\u{25CF}'));
        assert!(rendered.contains('\u{25CB}'));
    }

    #[test]
    fn render_form_draws_text_field_value() {
        let theme = Theme::default();
        let form = Form::for_test(
            "Tier",
            "the tier",
            vec![text_field("vibe.tree.tier", "3", TextKind::Int)],
            Layer::L3,
            paths(),
        );
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        render_form(area, &mut buf, &form, &prefs(), &theme);
        let rendered = buffer_string(&buf, area);
        assert!(rendered.contains("tier"));
        assert!(rendered.contains("3"));
    }

    #[test]
    fn render_form_shows_write_layer_and_hints_at_the_bottom() {
        let theme = Theme::default();
        let form = Form::for_test(
            "Palette",
            "the palette",
            vec![bool_field("vibe.tree.flag", true, Applies::Live)],
            Layer::L3,
            paths(),
        );
        let area = Rect::new(0, 0, 60, 12);
        let mut buf = Buffer::empty(area);
        render_form(area, &mut buf, &form, &prefs(), &theme);
        let rendered = buffer_string(&buf, area);
        assert!(rendered.contains("Write layer:"), "selector present");
        assert!(rendered.contains("L3"), "layer label present");
        assert!(
            rendered.contains("user-project"),
            "layer descriptor present"
        );
        assert!(rendered.contains("move"), "keymap hint present");
    }

    #[test]
    fn focused_field_label_carries_the_fold_glyph_marker() {
        let theme = Theme::default();
        let form = Form::for_test(
            "Page",
            "two fields",
            vec![
                bool_field("vibe.tree.a", true, Applies::Live),
                bool_field("vibe.tree.b", false, Applies::Live),
            ],
            Layer::L3,
            paths(),
        );
        let area = Rect::new(0, 0, 40, 16);
        let mut buf = Buffer::empty(area);
        render_form(area, &mut buf, &form, &prefs(), &theme);
        // The focused field (index 0) has the fold-collapsed marker (▸ at T≥1).
        let rendered = buffer_string(&buf, area);
        assert!(
            rendered.contains('\u{25B8}'),
            "focused field carries the fold marker: {rendered}"
        );
    }

    /// Flatten a buffer to its visible string (one line per row).
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
