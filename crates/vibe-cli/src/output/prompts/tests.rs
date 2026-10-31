use std::path::{Path, PathBuf};

use vibe_app::Choice;

use super::*;

fn choice(name: &str, id: &str) -> Choice {
    Choice::named(name, id)
}

#[test]
fn detected_rows_align_under_the_widest_name() {
    let rows = column_items(
        &[
            Choice {
                detected: true,
                ..choice("Windows", "windows")
            },
            choice("Linux", "linux"),
            choice("macOS", "macos"),
        ],
        &ItemStyles::plain(),
    );
    assert_eq!(
        rows,
        [
            "Windows  windows  [detected]",
            "Linux    linux",
            "macOS    macos",
        ]
    );
}

#[test]
fn a_column_no_row_fills_takes_no_space() {
    let rows = column_items(
        &[
            Choice {
                name: "Typical".into(),
                description: Some("the usual set".into()),
                ..Choice::default()
            },
            Choice {
                name: "Custom".into(),
                description: Some("choose the features one by one".into()),
                ..Choice::default()
            },
        ],
        &ItemStyles::plain(),
    );
    assert_eq!(
        rows,
        [
            "Typical  — the usual set",
            "Custom   — choose the features one by one",
        ]
    );
}

#[test]
fn a_nested_row_is_indented_inside_the_name_column() {
    let rows = column_items(
        &[
            Choice {
                description: Some("the editor itself".into()),
                ..choice("Core", "core")
            },
            Choice {
                depth: 1,
                description: Some("the bundled plugins".into()),
                ..choice("Plugins", "plugins")
            },
            Choice {
                depth: 2,
                ..choice("LSP", "lsp")
            },
        ],
        &ItemStyles::plain(),
    );
    assert_eq!(
        rows,
        [
            "Core       core     — the editor itself",
            "  Plugins  plugins  — the bundled plugins",
            "    LSP    lsp",
        ]
    );
}

#[test]
fn no_row_carries_trailing_padding() {
    for row in column_items(
        &[choice("A", "a"), choice("Bbbbbbbb", "bbbbbbbb")],
        &ItemStyles::plain(),
    ) {
        assert_eq!(row, row.trim_end(), "trailing padding in {row:?}");
    }
}

/// Colour is a decoration over the same columns: the padding is measured
/// on the plain text, so stripping the escapes off the styled rows yields
/// exactly the rows a terminal without colour shows.
#[test]
fn the_columns_survive_the_no_colour_fallback() {
    let choices = [
        Choice {
            detected: true,
            ..choice("Windows", "windows")
        },
        Choice {
            description: Some("the other host".into()),
            ..choice("Linux", "linux")
        },
    ];
    let coloured = column_items(&choices, &ItemStyles::forced());
    assert!(coloured.iter().any(|row| row.contains('\u{1b}')));
    let stripped: Vec<String> = coloured
        .iter()
        .map(|row| console::strip_ansi_codes(row).into_owned())
        .collect();
    assert_eq!(stripped, column_items(&choices, &ItemStyles::plain()));
}

#[test]
fn both_vocabularies_carry_one_column_cursors() {
    let unicode = Glyphs::unicode();
    assert_eq!(unicode.active, "❯");
    assert_eq!(unicode.success, "✓");
    let ascii = Glyphs::ascii();
    assert_eq!(ascii.active, ">");
    assert_eq!(ascii.checked.len(), ascii.unchecked.len());
}

/// The glyphs the theme hands `dialoguer` are the set it was built from,
/// and an inactive row's prefix is as wide as the cursor it replaces.
#[test]
fn the_theme_takes_its_glyphs_from_the_set() {
    let plain = |styled: &console::StyledObject<String>| {
        console::strip_ansi_codes(&styled.to_string()).into_owned()
    };
    for glyphs in [Glyphs::unicode(), Glyphs::ascii()] {
        let theme = themed(&glyphs);
        assert_eq!(plain(&theme.active_item_prefix), glyphs.active);
        assert_eq!(plain(&theme.success_prefix), glyphs.success);
        assert_eq!(plain(&theme.error_prefix), glyphs.error);
        assert_eq!(plain(&theme.checked_item_prefix), glyphs.checked);
        assert_eq!(plain(&theme.unchecked_item_prefix), glyphs.unchecked);
        assert_eq!(
            console::measure_text_width(&plain(&theme.inactive_item_prefix)),
            1
        );
    }
}

/// An answered question reads as its name and then its value, with the
/// success glyph in front and nothing between the two.
#[test]
fn an_answer_line_is_the_prompt_then_the_value() {
    let theme = themed(&Glyphs::unicode());
    let mut line = String::new();
    theme
        .format_select_prompt_selection(&mut line, "Platform", "Windows (windows)")
        .unwrap();
    assert_eq!(
        console::strip_ansi_codes(&line),
        "✓ Platform  Windows (windows)"
    );
}

/// Every `dialoguer` question in the binary is built with the shared
/// theme. A constructor without `.with_theme(` as its very next call is
/// the one way `dialoguer`'s bare `SimpleTheme` comes back, so this scan
/// is a cheap guard that the convention — the theme first in the chain —
/// still holds.
#[test]
fn no_prompt_is_built_without_the_shared_theme() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let cell = src.join("output").join("prompts");
    let mut offenders = Vec::new();
    for file in rust_files(&src) {
        // This cell names the constructors in its prose and in the scan.
        if file.starts_with(&cell) || file == cell.with_extension("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("reading a source file");
        for end in constructors(&text) {
            if !text[end..].trim_start().starts_with(".with_theme(") {
                let line = text[..end].lines().count();
                offenders.push(format!("{}:{line}", file.display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these prompts are built without `output::prompt_theme()`: {offenders:?}"
    );
}

/// Every `.rs` file under `root`.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }
    found
}

/// Where each `dialoguer` prompt constructor in `text` ends. `Input` is
/// spelled with a turbofish at its call sites, so it is matched by its
/// type name and the `new()` that follows it.
fn constructors(text: &str) -> Vec<usize> {
    const WHOLE: [&str; 4] = [
        "Confirm::new()",
        "MultiSelect::new()",
        "Select::new()",
        "Password::new()",
    ];
    /// How far past `Input::` a turbofish may push the `new()`.
    const TURBOFISH: usize = 48;
    let mut ends = Vec::new();
    for (start, _) in text.char_indices() {
        if !starts_name(text, start) {
            continue;
        }
        if let Some(name) = WHOLE.iter().find(|name| text[start..].starts_with(**name)) {
            ends.push(start + name.len());
        } else if text[start..].starts_with("Input::")
            && let Some(offset) = text[start..].find("new()")
            && offset < TURBOFISH
        {
            ends.push(start + offset + "new()".len());
        }
    }
    ends
}

/// Whether `start` begins an identifier rather than continuing one, so
/// `ChatInput::new` is not read as `Input::new`.
fn starts_name(text: &str, start: usize) -> bool {
    text[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_')
}
