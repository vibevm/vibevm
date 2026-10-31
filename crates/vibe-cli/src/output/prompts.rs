//! The one prompt theme of the CLI (PROP-065 `#display`, PROP-060
//! `#DISPLAY`).
//!
//! Every `dialoguer` question in this binary is built with
//! [`prompt_theme`], so a question reads like a progress row rather than
//! like `dialoguer`'s bare `SimpleTheme`: the same `✓`, a cyan cursor, the
//! detail dim. The glyph set and the colour decision are the renderer's,
//! not a second opinion — `console` drops every escape when colour is off,
//! and `output::progress::vocabulary` prints its `✓` / `✗` / spinner with
//! no capability test at all, so a question does the same wherever a
//! terminal exists.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#DISPLAY");

use std::sync::atomic::{AtomicBool, Ordering};

use console::{Style, TermFamily, style};
use dialoguer::theme::{ColorfulTheme, Theme};

mod columns;
pub(crate) use columns::{ItemStyles, column_items};

/// The glyphs one question draws with.
pub(crate) struct Glyphs {
    /// Before an unanswered question.
    prompt: &'static str,
    /// Before the row under the cursor.
    active: &'static str,
    /// Before an answered question.
    success: &'static str,
    /// Before a refused answer.
    error: &'static str,
    /// A selected multi-choice row.
    checked: &'static str,
    /// An unselected multi-choice row.
    unchecked: &'static str,
    /// The key legend of a list question.
    keys: &'static str,
}

impl Glyphs {
    /// The vocabulary the progress renderer already draws with.
    pub(crate) const fn unicode() -> Self {
        Self {
            prompt: "?",
            active: "❯",
            success: "✓",
            error: "✗",
            checked: "◉",
            unchecked: "○",
            keys: "↑↓ move · enter choose · space toggle · esc cancel",
        }
    }

    /// The fallback for a host with no terminal vocabulary at all.
    pub(crate) const fn ascii() -> Self {
        Self {
            prompt: "?",
            active: ">",
            success: "*",
            error: "-",
            checked: "[x]",
            unchecked: "[ ]",
            keys: "up/down move, enter choose, space toggle, esc cancel",
        }
    }

    /// The set this process draws with. Redirected output is NOT a reason
    /// to degrade: a question keeps its glyphs and loses only its colour,
    /// exactly as a progress row does. `TermFamily::Dummy` is the one
    /// family `console` reports as having no terminal vocabulary.
    pub(crate) fn detect() -> Self {
        match console::Term::stderr().features().family() {
            TermFamily::Dummy => Self::ascii(),
            _ => Self::unicode(),
        }
    }
}

/// The theme every `dialoguer` question in this binary is built with.
pub(crate) fn prompt_theme() -> ColorfulTheme {
    themed(&Glyphs::detect())
}

/// The theme over one glyph set — the pure half, so a test builds both
/// vocabularies without a terminal.
pub(crate) fn themed(glyphs: &Glyphs) -> ColorfulTheme {
    ColorfulTheme {
        defaults_style: Style::new().for_stderr().cyan(),
        prompt_style: Style::new().for_stderr().bold(),
        prompt_prefix: style(glyphs.prompt.to_owned()).for_stderr().cyan(),
        prompt_suffix: style("›".to_owned()).for_stderr().dim(),
        // Green bold and red bold are `output::Context`'s own `tick` and
        // `cross`, so an answered question and a finished step row carry
        // the very same mark.
        success_prefix: style(glyphs.success.to_owned()).for_stderr().green().bold(),
        // Empty, so an answered question reads as the question's name and
        // then its value — `✓ Platform  Windows (windows)` — instead of
        // carrying a separator nothing else in the CLI uses.
        success_suffix: style(String::new()).for_stderr(),
        error_prefix: style(glyphs.error.to_owned()).for_stderr().red().bold(),
        error_style: Style::new().for_stderr().red(),
        hint_style: Style::new().for_stderr().dim(),
        values_style: Style::new().for_stderr().green().bold(),
        // Bold, not reverse: the row's detail columns carry their own dim
        // and green styles, whose resets end any wrap `dialoguer` puts
        // around the whole row, so this style reaches the name and a
        // reversed block would stop mid-row on the default Windows
        // palette. Bold plus the cyan cursor marks the row instead.
        active_item_style: Style::new().for_stderr().cyan().bold(),
        inactive_item_style: Style::new().for_stderr(),
        active_item_prefix: style(glyphs.active.to_owned()).for_stderr().cyan(),
        // One column wide. `dialoguer` writes `{prefix} {row}`, so a
        // one-column prefix puts an inactive name under an active one.
        inactive_item_prefix: style(" ".to_owned()).for_stderr(),
        checked_item_prefix: style(glyphs.checked.to_owned()).for_stderr().green(),
        unchecked_item_prefix: style(glyphs.unchecked.to_owned()).for_stderr().dim(),
        picked_item_prefix: style(glyphs.active.to_owned()).for_stderr().cyan(),
        unpicked_item_prefix: style(" ".to_owned()).for_stderr(),
    }
}

/// Whether this process has printed its key legend yet.
static LEGEND_SHOWN: AtomicBool = AtomicBool::new(false);

/// The blank line above one question: a run's questions, its `✓` answers
/// and the plan it then prints read as one block only when each question
/// is spaced off the text above it (PROP-065 `#display`).
pub(crate) fn space_before_question() {
    eprintln!();
}

/// The key legend, once per process, above the first list question.
///
/// `dialoguer` draws no footer, and a legend printed after the answer
/// would read as a remark about the answer; printed above, it survives
/// the prompt's own redraw and stands as the legend of every list below
/// it.
pub(crate) fn key_legend() {
    if LEGEND_SHOWN.swap(true, Ordering::Relaxed) {
        return;
    }
    eprintln!(
        "{}",
        Style::new()
            .for_stderr()
            .dim()
            .apply_to(Glyphs::detect().keys)
    );
}

/// The `✓` line of an answered list question.
///
/// `dialoguer` reports a selection by echoing the row it drew, which for
/// an aligned row means its column padding too. A list is therefore built
/// with `report(false)` and its answer printed here from the flat label —
/// through the theme's own formatter, so it is byte-identical to the line
/// an `Input` or a `Confirm` leaves behind.
pub(crate) fn answer(theme: &ColorfulTheme, prompt: &str, value: &str) {
    let mut line = String::new();
    if theme
        .format_select_prompt_selection(&mut line, prompt, value)
        .is_ok()
    {
        eprintln!("{line}");
    }
}

/// The `✓` line of an answered multi-choice question.
pub(crate) fn answers(theme: &ColorfulTheme, prompt: &str, values: &[&str]) {
    let mut line = String::new();
    if theme
        .format_multi_select_prompt_selection(&mut line, prompt, values)
        .is_ok()
    {
        eprintln!("{line}");
    }
}

#[cfg(test)]
#[path = "prompts/tests.rs"]
mod tests;
