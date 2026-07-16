//! Buffer → snapshot converters for the AIUI render plane (PROP-042 §2
//! `#snapshot-contract`). `text` is lossless for layout (every glyph in grid
//! order, per-row right-trim); `cells` is lossless for style (run-length-encoded
//! runs carrying fg/bg/modifiers). Neither invents or drops content.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#snapshot-contract");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Position;
use ratatui_core::style::{Color, Modifier};
use serde_json::{Value, json};
use specmark::spec;

/// The `text` snapshot: one line per row, the row's cell symbols concatenated
/// with trailing whitespace trimmed (PROP-042 §2). Column alignment within a row
/// is preserved (trim is right-only).
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#snapshot-contract")]
pub(crate) fn to_text(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in area.y..area.y + area.height {
        let mut line = String::new();
        for x in area.x..area.x + area.width {
            line.push_str(buf[Position::new(x, y)].symbol());
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// One RLE run's style key: the glyph plus its resolved fg/bg/modifier set.
#[derive(PartialEq)]
struct RunKey {
    ch: String,
    fg: Option<String>,
    bg: Option<String>,
    mods: Vec<&'static str>,
}

/// The `cells` snapshot: run-length-encoded rows carrying per-run style
/// (PROP-042 §2). `grid[y]` is an array of `{n, ch, fg?, bg?, mods?}` runs.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#snapshot-contract")]
pub(crate) fn to_cells(buf: &Buffer) -> Value {
    let area = buf.area;
    let mut grid: Vec<Value> = Vec::with_capacity(area.height as usize);
    for y in area.y..area.y + area.height {
        let mut runs: Vec<Value> = Vec::new();
        let mut cur: Option<(RunKey, u32)> = None;
        for x in area.x..area.x + area.width {
            let cell = &buf[Position::new(x, y)];
            let style = cell.style();
            let key = RunKey {
                ch: cell.symbol().to_string(),
                fg: color_str(style.fg),
                bg: color_str(style.bg),
                mods: mod_names(style.add_modifier),
            };
            match &mut cur {
                Some((k, n)) if *k == key => *n += 1,
                _ => {
                    if let Some((k, n)) = cur.take() {
                        runs.push(run_json(&k, n));
                    }
                    cur = Some((key, 1));
                }
            }
        }
        if let Some((k, n)) = cur.take() {
            runs.push(run_json(&k, n));
        }
        grid.push(Value::Array(runs));
    }
    json!({ "cols": area.width, "rows": area.height, "grid": grid })
}

/// Render one run as `{n, ch, fg?, bg?, mods?}`, omitting absent style keys.
fn run_json(k: &RunKey, n: u32) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("n".into(), json!(n));
    obj.insert("ch".into(), json!(k.ch));
    if let Some(fg) = &k.fg {
        obj.insert("fg".into(), json!(fg));
    }
    if let Some(bg) = &k.bg {
        obj.insert("bg".into(), json!(bg));
    }
    if !k.mods.is_empty() {
        obj.insert("mods".into(), json!(k.mods));
    }
    Value::Object(obj)
}

/// A colour as `#rrggbb` (truecolor), `idxN` (256-palette), or the lowercased
/// ANSI name; `None`/`Reset` render as absent.
fn color_str(c: Option<Color>) -> Option<String> {
    match c {
        None | Some(Color::Reset) => None,
        Some(Color::Rgb(r, g, b)) => Some(format!("#{r:02x}{g:02x}{b:02x}")),
        Some(Color::Indexed(i)) => Some(format!("idx{i}")),
        Some(other) => Some(format!("{other:?}").to_lowercase()),
    }
}

/// The set of style modifiers present, as stable lowercase names.
fn mod_names(m: Modifier) -> Vec<&'static str> {
    let mut v = Vec::new();
    if m.contains(Modifier::BOLD) {
        v.push("bold");
    }
    if m.contains(Modifier::DIM) {
        v.push("dim");
    }
    if m.contains(Modifier::ITALIC) {
        v.push("italic");
    }
    if m.contains(Modifier::UNDERLINED) {
        v.push("underlined");
    }
    if m.contains(Modifier::REVERSED) {
        v.push("reversed");
    }
    v
}

#[cfg(test)]
mod tests {
    use super::super::menu::test_support::fixture_tree;
    use super::super::snapshot_headless;

    /// The base-frame render is byte-stable — diff against the committed golden
    /// (PROP-042 §1/§2). `UPDATE_GOLDENS=1` refreshes it when an intentional
    /// visual change lands (a reviewed diff).
    #[test]
    fn base_frame_matches_golden() {
        let got = snapshot_headless(fixture_tree(), 74, 22, "", false).expect("render");
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/commands/tree/tui/goldens/base.snap.txt"
        );
        if std::env::var("UPDATE_GOLDENS").is_ok() {
            std::fs::write(path, &got).expect("write golden");
        }
        let golden = std::fs::read_to_string(path).unwrap_or_default();
        assert_eq!(
            got, golden,
            "base-frame snapshot drifted; run `UPDATE_GOLDENS=1 cargo test -p vibe-cli base_frame_matches_golden` to refresh"
        );
    }

    /// The `cells` format is well-formed JSON with `{cols, rows, grid[rows]}`.
    #[test]
    fn cells_snapshot_is_structured() {
        let json = snapshot_headless(fixture_tree(), 40, 8, "", true).expect("render");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(v["cols"], 40);
        assert_eq!(v["rows"], 8);
        assert_eq!(v["grid"].as_array().expect("grid array").len(), 8);
    }
}
