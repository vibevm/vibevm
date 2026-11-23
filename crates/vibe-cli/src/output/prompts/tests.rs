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
    let unicode = PromptGlyphs::in_set(GlyphSet::Unicode);
    assert_eq!(unicode.active, "❯");
    let ascii = PromptGlyphs::in_set(GlyphSet::Ascii);
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
    for set in [GlyphSet::Unicode, GlyphSet::Ascii] {
        let glyphs = PromptGlyphs::in_set(set);
        let theme = themed(set);
        assert_eq!(plain(&theme.active_item_prefix), glyphs.active);
        assert_eq!(plain(&theme.success_prefix), Glyph::Done.in_set(set));
        assert_eq!(plain(&theme.error_prefix), Glyph::Failed.in_set(set));
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
    let theme = themed(GlyphSet::Unicode);
    let mut line = String::new();
    theme
        .format_select_prompt_selection(&mut line, "Platform", "Windows (windows)")
        .unwrap();
    assert_eq!(
        console::strip_ansi_codes(&line),
        "✓ Platform  Windows (windows)"
    );
}

/// One module draws (PROP-067 `#coverage`): every `dialoguer` question is
/// built with the shared theme, and no file outside the display language
/// writes a style, a theme, a gutter glyph or an escape of its own.
///
/// A constructor without `.with_theme(` as its very next call is the one
/// way `dialoguer`'s bare `SimpleTheme` comes back; a `console::style`, a
/// hand-written `✓`, a box glyph hidden behind a `\u{25…}` escape, a raw
/// escape or a `vibe: warning:` prefix is the one way a command grows a
/// look of its own again. The full-screen interfaces (`tree`, `prefs`) are their
/// own surface, outside a line-oriented terminal's blocks, and test modules
/// name the glyphs they assert.
#[test]
fn nothing_is_styled_outside_the_language_module() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let output = src.join("output");
    let allowed = [
        output.join("language"),
        output.join("language.rs"),
        output.join("prompts.rs"),
        output.join("prompts"),
        output.join("progress").join("vocabulary.rs"),
        src.join("commands").join("tree").join("tui"),
        src.join("commands").join("prefs").join("tui"),
        // The terminal-title OSC of `vibe tree` inside vibeterm: a
        // protocol message to the host, not a style.
        src.join("commands").join("tree").join("host.rs"),
    ];
    const FORBIDDEN: [&str; 18] = [
        "console::style",
        "console::Style",
        "Style::new()",
        "dialoguer::theme",
        "\x1b",
        "\u{1b}",
        "\u{001b}",
        "✓",
        "✗",
        "●",
        "○",
        "✻",
        "├",
        "└",
        "│",
        "⠋",
        // A box-drawing glyph spelled as an escape is the same glyph hidden
        // from the needles above.
        "\\u{25",
        // The old hand-made prefix of a warning, an error or a note; the
        // language writes `!`, the refusal block and the dim hint instead.
        "\"vibe: ",
    ];
    let mut prompts = Vec::new();
    let mut styled = Vec::new();
    for file in rust_files(&src) {
        let text = std::fs::read_to_string(&file).expect("reading a source file");
        let cell = output.join("prompts");
        if !(file.starts_with(&cell) || file == cell.with_extension("rs")) {
            for end in constructors(&text) {
                if !text[end..].trim_start().starts_with(".with_theme(") {
                    let line = text[..end].lines().count();
                    prompts.push(format!("{}:{line}", file.display()));
                }
            }
        }
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if allowed.iter().any(|root| file.starts_with(root))
            || name == "tests.rs"
            || file.components().any(|part| part.as_os_str() == "tests")
        {
            continue;
        }
        let code = text
            .find("#[cfg(test)]\nmod tests")
            .map_or(text.as_str(), |end| &text[..end]);
        for (number, line) in code.lines().enumerate() {
            if let Some(found) = FORBIDDEN.iter().find(|needle| line.contains(**needle)) {
                styled.push(format!("{}:{} {found}", file.display(), number + 1));
            }
        }
    }
    assert!(
        prompts.is_empty(),
        "these prompts are built without `output::prompt_theme()`: {prompts:?}"
    );
    assert!(
        styled.is_empty(),
        "these lines style text outside `output::language`: {styled:#?}"
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

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-067#question")]
fn list_key_moves_chooses_and_cancels() {
    use super::ask::{ListKey, list_key};
    use console::Key;
    // Down and up wrap around both ends.
    assert_eq!(list_key(&Key::ArrowDown, 0, 2), ListKey::Move(1));
    assert_eq!(list_key(&Key::ArrowDown, 1, 2), ListKey::Move(0));
    assert_eq!(list_key(&Key::ArrowUp, 0, 2), ListKey::Move(1));
    assert_eq!(list_key(&Key::ArrowUp, 2, 3), ListKey::Move(1));
    assert_eq!(list_key(&Key::Tab, 0, 3), ListKey::Move(1));
    assert_eq!(list_key(&Key::BackTab, 0, 3), ListKey::Move(2));
    assert_eq!(list_key(&Key::Char('j'), 1, 3), ListKey::Move(2));
    assert_eq!(list_key(&Key::Char('k'), 1, 3), ListKey::Move(0));
    // Enter and Space choose the row the marker is on.
    assert_eq!(list_key(&Key::Enter, 1, 2), ListKey::Choose(1));
    assert_eq!(list_key(&Key::Char(' '), 0, 2), ListKey::Choose(0));
    // Escape and Ctrl-C cancel rather than answer.
    assert_eq!(list_key(&Key::Escape, 0, 2), ListKey::Cancel);
    assert_eq!(list_key(&Key::CtrlC, 1, 2), ListKey::Cancel);
    // No letter answers, in any layout.
    for letter in ['y', 'n', 'Y', 'N', 'н', 'т', 'Н', 'Т'] {
        assert_eq!(
            list_key(&Key::Char(letter), 0, 2),
            ListKey::Ignore,
            "{letter}"
        );
        assert_eq!(
            list_key(&Key::Char(letter), 1, 2),
            ListKey::Ignore,
            "{letter}"
        );
    }
}

#[test]
fn the_yes_or_no_marker_starts_on_the_default() {
    use super::ask::yes_or_no_row;
    assert_eq!(yes_or_no_row(true), 0, "default yes marks the `yes` row");
    assert_eq!(yes_or_no_row(false), 1, "default no marks the `no` row");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-067#question")]
fn list_rows_mark_the_selected_row_with_the_prompt_cursor() {
    use super::ask::list_row;
    let plain = |text: String| console::strip_ansi_codes(&text).into_owned();
    assert_eq!(plain(list_row("yes", true, GlyphSet::Unicode)), "❯ yes");
    assert_eq!(plain(list_row("yes", true, GlyphSet::Ascii)), "> yes");
    assert_eq!(plain(list_row("no", false, GlyphSet::Unicode)), "  no");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-067#question")]
fn kept_line_uses_the_active_glyph_set() {
    use super::ask::kept_line;
    let plain = |text: String| console::strip_ansi_codes(&text).into_owned();
    assert_eq!(
        plain(kept_line("API kind", GlyphSet::Unicode)),
        "○ API kind   kept unchanged"
    );
    assert_eq!(
        plain(kept_line("API kind", GlyphSet::Ascii)),
        "- API kind   kept unchanged"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-067#question")]
fn an_empty_text_default_is_not_drawn() {
    use super::ask::shown_default;
    let theme = themed(GlyphSet::Unicode);
    let mut empty = String::new();
    theme
        .format_input_prompt(&mut empty, "Endpoint", shown_default(""))
        .unwrap();
    assert_eq!(console::strip_ansi_codes(&empty), "? Endpoint › ");
    let mut given = String::new();
    theme
        .format_input_prompt(&mut given, "Label", shown_default("Default"))
        .unwrap();
    assert_eq!(console::strip_ansi_codes(&given), "? Label (Default) › ");
}

/// A virtual screen applying line writes and console's move-up/erase-line
/// operations. Keys are released only after a flushed, complete choice block.
#[derive(Default)]
struct QuestionScreen {
    rows: Vec<String>,
    cursor: usize,
    flushed: bool,
    reads: usize,
    clears: Vec<usize>,
}

/// Controlled terminal for the production list loop and cancellation policies.
struct QuestionTerminal {
    screen: std::cell::RefCell<QuestionScreen>,
    choices: Vec<String>,
}

impl QuestionTerminal {
    /// Require the entire block, including its last choice, before a key.
    fn new(choices: &[&str]) -> Self {
        Self {
            screen: std::cell::RefCell::new(QuestionScreen::default()),
            choices: choices.iter().map(|row| (*row).to_owned()).collect(),
        }
    }

    /// Visible nonempty screen rows after applying every cursor/erase operation.
    fn visible(&self) -> Vec<String> {
        self.screen
            .borrow()
            .rows
            .iter()
            .filter(|row| !row.is_empty())
            .cloned()
            .collect()
    }
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
impl super::ask::ListTerminal for QuestionTerminal {
    fn is_term(&self) -> bool {
        true
    }
    fn columns(&self) -> usize {
        120
    }
    fn prepare(&self) {}
    fn write_line(&self, line: &str) -> std::io::Result<()> {
        let mut screen = self.screen.borrow_mut();
        let cursor = screen.cursor;
        screen.rows.resize_with(cursor + 1, String::new);
        screen.rows[cursor] = console::strip_ansi_codes(line).into_owned();
        screen.cursor += 1;
        screen.flushed = false;
        Ok(())
    }
    fn clear_last_lines(&self, lines: usize) -> std::io::Result<()> {
        let mut screen = self.screen.borrow_mut();
        screen.clears.push(lines);
        // Each console operation moves up, erases a row and resets its column.
        for _ in 0..lines {
            screen.cursor = screen.cursor.checked_sub(1).expect("erase above screen");
            let cursor = screen.cursor;
            screen.rows[cursor].clear();
        }
        screen.flushed = false;
        Ok(())
    }
    fn flush(&self) -> std::io::Result<()> {
        self.screen.borrow_mut().flushed = true;
        Ok(())
    }
    fn read_key_raw(&self) -> std::io::Result<console::Key> {
        let mut screen = self.screen.borrow_mut();
        assert!(screen.flushed, "key requested before drawing was flushed");
        assert_eq!(screen.cursor, self.choices.len() + 1);
        assert!(screen.rows[0].starts_with("? "));
        for (index, choice) in self.choices.iter().enumerate() {
            let selected = if screen.reads == 0 { 0 } else { 1 };
            let prefix = if index == selected {
                PromptGlyphs::detect().active
            } else {
                " "
            };
            assert_eq!(screen.rows[index + 1], format!("{prefix} {choice}"));
        }
        let key = match screen.reads {
            0 => console::Key::ArrowDown,
            1 => console::Key::Escape,
            _ => panic!("read beyond Escape"),
        };
        screen.reads += 1;
        Ok(key)
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-067#question")]
fn interruptible_select_collapses_on_escape() {
    let labels = [
        "Keep native settings unchanged",
        "Write connection",
        "Other choice",
    ];
    let term = QuestionTerminal::new(&labels);
    let choices = super::ask::named(&labels);
    let picked =
        super::ask::choose_opt_interruptible_on("Native settings", &choices, 0, &term).unwrap();
    assert_eq!(picked, None);
    assert_eq!(
        term.visible(),
        [format!(
            "{} Native settings  kept unchanged",
            Glyph::Skipped.text()
        )]
    );
    let screen = term.screen.borrow();
    assert_eq!(
        screen.reads, 2,
        "Escape followed the fully drawn last choice"
    );
    assert_eq!(
        screen.clears,
        [3, 4],
        "redraw rows, then erase the whole block"
    );
    assert_eq!(screen.cursor, 1, "one kept row, no question or options");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-067#question")]
fn yes_no_escape_interrupts_without_kept_line() {
    let term = QuestionTerminal::new(&["yes", "no"]);
    let error = super::ask::yes_or_no_on("Remove settings?", true, &term).unwrap_err();
    let dialoguer::Error::IO(error) = error;
    assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
    assert!(
        term.visible().is_empty(),
        "confirmation must not leave a kept line"
    );
    let screen = term.screen.borrow();
    assert_eq!(screen.reads, 2);
    assert_eq!(screen.clears, [2, 3]);
    assert_eq!(screen.cursor, 0);
}
