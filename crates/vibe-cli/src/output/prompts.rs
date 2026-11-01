//! The one prompt theme of the CLI (PROP-065 `#display`, PROP-060
//! `#DISPLAY`).
//!
//! Every `dialoguer` question in this binary is built with
//! [`prompt_theme`], so a question reads like a progress row rather than
//! like `dialoguer`'s bare `SimpleTheme`: a question is a block of the
//! display language (PROP-067 `#question`), and its glyphs and roles are
//! the language's own, not a second opinion.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#question");

use std::sync::atomic::{AtomicBool, Ordering};

use console::style as styled;
use dialoguer::theme::{ColorfulTheme, Theme};

use super::language::{self, Glyph, GlyphSet, Kind, PromptGlyphs, Role, Stream, paint};

mod columns;
pub(crate) use columns::{ItemStyles, column_items};

/// A role's style for `dialoguer`, which writes to stderr.
fn role_style(role: Role) -> console::Style {
    language::style(role).for_stderr()
}

/// The width an answered question's name is padded to, so the answers of
/// one wizard read as one list with their values in one column (PROP-067
/// `#question`, `#list`).
const ANSWER_NAME: usize = 10;

/// The theme every `dialoguer` question in this binary is built with.
pub(crate) fn prompt_theme() -> ColorfulTheme {
    themed(GlyphSet::detect())
}

/// The theme over one glyph set — the pure half, so a test builds both
/// vocabularies without a terminal. Every style is a role of the display
/// language (PROP-067 `#palette`), so a question and a finished step carry
/// the very same marks.
pub(crate) fn themed(set: GlyphSet) -> ColorfulTheme {
    let prompt = PromptGlyphs::in_set(set);
    let role = role_style;
    let mark = |text: &str, role: Role| role_style(role).apply_to(text.to_owned());
    ColorfulTheme {
        defaults_style: role(Role::Accent),
        prompt_style: role(Role::Text),
        prompt_prefix: mark(Glyph::Question.in_set(set), Role::Accent),
        prompt_suffix: mark("›", Role::Dim),
        success_prefix: mark(Glyph::Done.in_set(set), Role::Ok),
        // Empty, so an answered question reads as the question's name and
        // then its value — `✓ Platform  Windows (windows)` — exactly the
        // shape of a finished step.
        success_suffix: styled(String::new()).for_stderr(),
        error_prefix: mark(Glyph::Failed.in_set(set), Role::Alert),
        error_style: role(Role::Alert),
        hint_style: role(Role::Dim),
        // The answered value: `ok` and bold, the one other place bold is
        // used (PROP-067 `#palette`).
        values_style: role(Role::Ok).bold(),
        // A background for the selected row and nothing else; the row's
        // detail columns end any wrap at their own reset, so the accent
        // cursor marks the row as well.
        active_item_style: role(Role::Accent),
        inactive_item_style: role(Role::Text),
        active_item_prefix: mark(prompt.active, Role::Accent),
        // One column wide. `dialoguer` writes `{prefix} {row}`, so a
        // one-column prefix puts an inactive name under an active one.
        inactive_item_prefix: styled(" ".to_owned()).for_stderr(),
        checked_item_prefix: mark(prompt.checked, Role::Ok),
        unchecked_item_prefix: mark(prompt.unchecked, Role::Dim),
        picked_item_prefix: mark(prompt.active, Role::Accent),
        unpicked_item_prefix: styled(" ".to_owned()).for_stderr(),
    }
}

/// Whether this process has printed its key legend yet.
static LEGEND_SHOWN: AtomicBool = AtomicBool::new(false);

/// The blank line above a question (PROP-067 `#question`): one off the
/// block before it, none after another answer, because the answers of one
/// wizard collapse into one list.
pub(crate) fn space_before_question() {
    eprint!("{}", language::before(Stream::Err, Kind::Row("answers")));
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
    eprintln!("{}", paint(Role::Dim, PromptGlyphs::detect().keys));
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
        .format_select_prompt_selection(&mut line, &answer_name(prompt), value)
        .is_ok()
    {
        eprintln!("{line}");
    }
}

/// The `✓` line of an answered multi-choice question.
pub(crate) fn answers(theme: &ColorfulTheme, prompt: &str, values: &[&str]) {
    let mut line = String::new();
    if theme
        .format_multi_select_prompt_selection(&mut line, &answer_name(prompt), values)
        .is_ok()
    {
        eprintln!("{line}");
    }
}

/// A question's name padded to the answers' value column.
fn answer_name(prompt: &str) -> String {
    format!("{prompt:<0$}", ANSWER_NAME - 1)
}

#[cfg(test)]
#[path = "prompts/tests.rs"]
mod tests;
