//! The aligned rows of a list question (PROP-065 `#display`).
//!
//! `dialoguer` prints a row exactly as it is given, so the alignment and
//! the colour of a wizard row are decided here — one pure function from
//! the library's structured [`Choice`] rows to the strings the prompt
//! draws, testable without a terminal.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use console::{Style, measure_text_width};
use vibe_app::Choice;

/// The marker a detected row carries.
const DETECTED: &str = "[detected]";

/// How far one column stands from the next.
const GUTTER: usize = 2;

/// The styles the detail columns of a row carry. The name column has
/// none on purpose — see [`column_items`].
pub(crate) struct ItemStyles {
    /// The stable id, second column.
    id: Style,
    /// The `[detected]` marker.
    detected: Style,
    /// The trailing sentence.
    description: Style,
}

impl ItemStyles {
    /// The live styles. One set covers colour and no colour both:
    /// `console` emits no escape when colour is off, and the padding is
    /// measured on the plain text, so the columns stand either way.
    pub(crate) fn stderr() -> Self {
        Self {
            id: Style::new().for_stderr().dim(),
            detected: Style::new().for_stderr().green(),
            description: Style::new().for_stderr().dim(),
        }
    }

    /// The no-colour set a test asserts the plain rows against.
    #[cfg(test)]
    pub(crate) fn plain() -> Self {
        Self {
            id: Style::new(),
            detected: Style::new(),
            description: Style::new(),
        }
    }

    /// The same styles with the escapes forced on, so a test sees them
    /// even though its own stderr is a pipe.
    #[cfg(test)]
    pub(crate) fn forced() -> Self {
        let forced = |style: Style| style.force_styling(true);
        Self {
            id: forced(Style::new().dim()),
            detected: forced(Style::new().green()),
            description: forced(Style::new().dim()),
        }
    }
}

/// Render `choices` as aligned rows: the name (indented by its depth)
/// padded to the widest name, then the id, then `[detected]`, then the
/// description, each column a gutter from the one before. A column no row
/// fills is dropped entirely, and the last filled column of a row keeps no
/// trailing padding.
///
/// The name column is deliberately left unstyled. `dialoguer` wraps a
/// whole row in its active-item style, and any style nested inside the row
/// ends that wrap at its own reset — so leaving the name bare is what lets
/// the cursor's bold reach it while the dim id and green marker keep their
/// own colour.
pub(crate) fn column_items(choices: &[Choice], styles: &ItemStyles) -> Vec<String> {
    let names: Vec<String> = choices
        .iter()
        .map(|choice| format!("{}{}", "  ".repeat(choice.depth), choice.name))
        .collect();
    let name_width = widest(names.iter().map(String::as_str));
    let id_width = widest(choices.iter().filter_map(|choice| choice.id.as_deref()));
    let detected_width = if choices.iter().any(|choice| choice.detected) {
        measure_text_width(DETECTED)
    } else {
        0
    };
    choices
        .iter()
        .zip(&names)
        .map(|(choice, name)| {
            row(&[
                (&Style::new(), pad(name, name_width)),
                (
                    &styles.id,
                    pad(choice.id.as_deref().unwrap_or(""), id_width),
                ),
                (
                    &styles.detected,
                    pad(if choice.detected { DETECTED } else { "" }, detected_width),
                ),
                (
                    &styles.description,
                    choice
                        .description
                        .as_deref()
                        .map(|text| format!("— {text}"))
                        .unwrap_or_default(),
                ),
            ])
        })
        .collect()
}

/// The widest display width in `items`, or zero when there are none.
fn widest<'a>(items: impl Iterator<Item = &'a str>) -> usize {
    items.map(measure_text_width).max().unwrap_or(0)
}

/// `text` filled to `width` plus the gutter; a column no row fills takes
/// no space at all.
fn pad(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let filled = measure_text_width(text);
    format!(
        "{text}{}",
        " ".repeat(width.saturating_sub(filled) + GUTTER)
    )
}

/// One row: the filled columns styled and joined, with the padding of the
/// last filled one removed so a bold cursor row drags no tail of blanks.
fn row(fields: &[(&Style, String)]) -> String {
    let mut filled = fields.len();
    while filled > 0 && fields[filled - 1].1.trim().is_empty() {
        filled -= 1;
    }
    let mut line = String::new();
    for (index, (style, text)) in fields[..filled].iter().enumerate() {
        let text = if index + 1 == filled {
            text.trim_end()
        } else {
            text.as_str()
        };
        line.push_str(&style.apply_to(text).to_string());
    }
    line
}
