//! The aligned rows of a list question (PROP-065 `#display`).
//!
//! `dialoguer` prints a row exactly as it is given, so the alignment and
//! the colour of a wizard row are decided here — one pure function from
//! the library's structured [`Choice`] rows to the strings the prompt
//! draws, testable without a terminal.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use console::measure_text_width;
use vibe_app::Choice;

use crate::output::language::{Role, paint};

/// The marker a detected row carries.
const DETECTED: &str = "[detected]";

/// How far one column stands from the next.
const GUTTER: usize = 2;

/// How the detail columns of a row are painted. The name column has none
/// on purpose — see [`column_items`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemStyles {
    /// The live roles: attributes only when this process draws colour.
    Live,
    /// No attribute at all.
    #[cfg(test)]
    Plain,
    /// The attributes forced on, so a test sees them through a pipe.
    #[cfg(test)]
    Forced,
}

impl ItemStyles {
    /// The live styles. One set covers colour and no colour both: the
    /// padding is measured on the plain text, so the columns stand either
    /// way.
    pub(crate) fn stderr() -> Self {
        Self::Live
    }

    #[cfg(test)]
    pub(crate) fn plain() -> Self {
        Self::Plain
    }

    #[cfg(test)]
    pub(crate) fn forced() -> Self {
        Self::Forced
    }

    fn apply(self, role: Role, text: &str) -> String {
        match self {
            Self::Live => paint(role, text),
            #[cfg(test)]
            Self::Plain => text.to_owned(),
            #[cfg(test)]
            Self::Forced if role == Role::Text || text.is_empty() => text.to_owned(),
            #[cfg(test)]
            Self::Forced => crate::output::language::style(role)
                .force_styling(true)
                .apply_to(text)
                .to_string(),
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
            row(
                &[
                    (Role::Text, pad(name, name_width)),
                    (Role::Dim, pad(choice.id.as_deref().unwrap_or(""), id_width)),
                    (
                        Role::Ok,
                        pad(if choice.detected { DETECTED } else { "" }, detected_width),
                    ),
                    (
                        Role::Dim,
                        choice
                            .description
                            .as_deref()
                            .map(|text| format!("— {text}"))
                            .unwrap_or_default(),
                    ),
                ],
                *styles,
            )
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
fn row(fields: &[(Role, String)], styles: ItemStyles) -> String {
    let mut filled = fields.len();
    while filled > 0 && fields[filled - 1].1.trim().is_empty() {
        filled -= 1;
    }
    let mut line = String::new();
    for (index, (role, text)) in fields[..filled].iter().enumerate() {
        let text = if index + 1 == filled {
            text.trim_end()
        } else {
            text.as_str()
        };
        line.push_str(&styles.apply(*role, text));
    }
    line
}
