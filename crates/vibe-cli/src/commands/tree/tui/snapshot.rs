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
    use std::path::PathBuf;

    use super::super::menu::test_support::fixture_tree;
    use super::super::snapshot_headless;

    /// The committed-golden directory (manifest-dir-relative, absolute at compile
    /// time so the test is CWD-independent).
    const GOLDENS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands/tree/tui/goldens");

    /// The path of a render-plane text golden by name (`<name>.snap.txt`).
    fn golden_path(name: &str) -> PathBuf {
        PathBuf::from(GOLDENS_DIR).join(format!("{name}.snap.txt"))
    }

    /// Diff a rendered `text` snapshot against its committed golden
    /// (PROP-042 §1/§2). `UPDATE_GOLDENS=1` refreshes the file in place — run it
    /// once to seed a new scenario or when an intentional visual change lands,
    /// then review the diff before committing. A drift fails with the refresh
    /// recipe so the fix is mechanical.
    fn assert_text_golden(name: &str, got: &str) {
        let path = golden_path(name);
        if std::env::var("UPDATE_GOLDENS").is_ok() {
            std::fs::write(&path, got).expect("write golden");
        }
        let golden = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            got, golden,
            "render golden `{name}` drifted; run \
             `UPDATE_GOLDENS=1 cargo test -p vibe-cli matches_golden` to refresh"
        );
    }

    // --- Seed scenarios (PROP-042 §8 / TERMINAL-AIUI-PLAN §8): each is a
    // `(fixture, size, key-script) → committed .snap.txt`. Adding a scenario =
    // one test fn + `UPDATE_GOLDENS=1` to seed it. They catch spacing / centring
    // / truncation / footer-clip regressions with no terminal.

    /// Scenario 1 — the base frame: the two-row centred footer, the status line,
    /// the table over the one-package fixture.
    #[test]
    fn base_frame_matches_golden() {
        let got = snapshot_headless(fixture_tree(), 74, 22, "", false).expect("render");
        assert_text_golden("base", &got);
    }

    /// Scenario 2 — the F2 sort menu over the base frame: group frames inset from
    /// the window, options inset from the group frames, the hint row intact.
    #[test]
    fn f2_sort_menu_matches_golden() {
        let got = snapshot_headless(fixture_tree(), 74, 22, "F2", false).expect("render");
        assert_text_golden("f2-sort-menu", &got);
    }

    /// Scenario 3 — the F3 display-mode menu: the single-group flat list, the
    /// inert `Tab` (one partition on the one-package fixture).
    #[test]
    fn f3_mode_menu_matches_golden() {
        let got = snapshot_headless(fixture_tree(), 74, 22, "F3", false).expect("render");
        assert_text_golden("f3-mode-menu", &got);
    }

    /// Scenario 4 — the quit-confirm dialog: a bare `Esc` at the base screen
    /// opens it; `OK`/`Cancel` render centred with air.
    #[test]
    fn quit_dialog_matches_golden() {
        let got = snapshot_headless(fixture_tree(), 74, 22, "Esc", false).expect("render");
        assert_text_golden("quit-dialog", &got);
    }

    /// Scenario 5 — narrow width (`56×20`): graceful degradation. This documents
    /// the footer-clip edge the render plane must keep visible (not hide): a
    /// regression here is a layout change a human must review.
    #[test]
    fn narrow_width_matches_golden() {
        let got = snapshot_headless(fixture_tree(), 56, 20, "", false).expect("render");
        assert_text_golden("narrow-width", &got);
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
