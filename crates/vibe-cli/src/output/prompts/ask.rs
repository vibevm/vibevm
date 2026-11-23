//! The questions every command asks, as blocks of PROP-067 `#question`.
//!
//! A command asks through these and never builds a `dialoguer` prompt of
//! its own, so the blank line above a question, the key legend over the
//! first list, the column rows and the collapsed `✓ <prompt>  <value>`
//! line are the same in `init`, `uninstall`, `self gc` and the app wizard.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#question");

use dialoguer::{Input, MultiSelect, Select};
use specmark::spec;
use vibe_app::Choice;

use crate::output::language::{Glyph, GlyphSet, PromptGlyphs, Role};

use super::{ItemStyles, answer, answers, column_items, key_legend, space_before_question};

/// A yes-or-no question, drawn as a list of two rows with the marker on
/// `default`; it collapses to `✓ <prompt>  yes` once chosen.
///
/// ```text
/// ? Remove 3 dependency slots? ›        ->   ✓ Remove 3 dependency slots?  no
/// › yes
///   no
/// ```
///
/// It takes no letter answers, so the keyboard layout never matters.
/// Escape or Ctrl-C is not an answer: it returns an error of kind
/// [`std::io::ErrorKind::Interrupted`] so the command stops rather than
/// taking `no`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(crate) fn confirm(prompt: &str, default: bool) -> dialoguer::Result<bool> {
    let yes = yes_or_no(prompt, default)?;
    answer(
        &super::prompt_theme(),
        prompt,
        if yes { "yes" } else { "no" },
    );
    Ok(yes)
}

/// A confirmation whose collapsed line the caller writes itself, because
/// what was decided reads better than `yes` (`✓ remap offer  accepted …`).
/// The question is the same two-row list as [`confirm`].
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(crate) fn confirm_then(
    prompt: &str,
    default: bool,
    name: &str,
    said: impl FnOnce(bool) -> String,
) -> dialoguer::Result<bool> {
    let yes = yes_or_no(prompt, default)?;
    answer(&super::prompt_theme(), name, &said(yes));
    Ok(yes)
}

/// The two rows of a yes-or-no question, `yes` first; the answer line is
/// left to the caller.
fn yes_or_no(prompt: &str, default: bool) -> dialoguer::Result<bool> {
    yes_or_no_on(prompt, default, &console::Term::stderr())
}

/// The confirmation policy over the same terminal boundary as optional lists.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(super) fn yes_or_no_on(
    prompt: &str,
    default: bool,
    term: &impl ListTerminal,
) -> dialoguer::Result<bool> {
    let rows = ["yes".to_owned(), "no".to_owned()];
    match list_question(prompt, &rows, yes_or_no_row(default), term)? {
        Some(index) => Ok(index == 0),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "the question was interrupted",
        )
        .into()),
    }
}

/// The row the marker of a yes-or-no question starts on: `yes` is row 0,
/// `no` row 1.
pub(super) fn yes_or_no_row(default: bool) -> usize {
    if default { 0 } else { 1 }
}

/// One row of a list question: the prompt cursor of `set` in the accent
/// role on the selected row, a blank of the same width on the others.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(super) fn list_row(row: &str, active: bool, set: GlyphSet) -> String {
    if active {
        let cursor = super::role_style(Role::Accent).apply_to(PromptGlyphs::in_set(set).active);
        format!("{cursor} {row}")
    } else {
        format!("  {row}")
    }
}

/// The line an escaped interruptible choice leaves: the skipped glyph, the
/// padded question name and `kept unchanged`, all dim.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(super) fn kept_line(prompt: &str, set: GlyphSet) -> String {
    let dim = super::role_style(Role::Dim);
    format!(
        "{} {}  {}",
        dim.apply_to(Glyph::Skipped.in_set(set)),
        dim.apply_to(super::answer_name(prompt)),
        dim.apply_to("kept unchanged")
    )
}

/// The default a text question draws: none for an empty one, so the prompt
/// reads `? Endpoint ›` rather than `? Endpoint () ›`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(super) fn shown_default(default: &str) -> Option<&str> {
    (!default.is_empty()).then_some(default)
}

/// A line of text, with the value it starts from.
pub(crate) fn text(prompt: &str, default: &str, allow_empty: bool) -> dialoguer::Result<String> {
    space_before_question();
    let theme = super::prompt_theme();
    let value = Input::<String>::with_theme(&theme)
        .with_prompt(prompt)
        .default(default.to_owned())
        .show_default(shown_default(default).is_some())
        .report(false)
        .allow_empty(allow_empty)
        .interact_text()?;
    answer(&theme, prompt, &value);
    Ok(value)
}

/// One row of `choices`, which the reader must pick: escape is not an
/// answer here.
pub(crate) fn choose(prompt: &str, choices: &[Choice], default: usize) -> dialoguer::Result<usize> {
    pick(prompt, choices, default, false).map(|picked| picked.unwrap_or(default))
}

/// One row of `choices`, or `None` when the reader pressed escape.
pub(crate) fn choose_opt(
    prompt: &str,
    choices: &[Choice],
    default: usize,
) -> dialoguer::Result<Option<usize>> {
    pick(prompt, choices, default, true)
}

/// A read-only answer collector whose interruption restores the terminal.
/// Existing application/confirmation prompts retain their established policy.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(crate) fn choose_opt_interruptible(
    prompt: &str,
    choices: &[Choice],
    default: usize,
) -> dialoguer::Result<Option<usize>> {
    choose_opt_interruptible_on(prompt, choices, default, &console::Term::stderr())
}

/// The optional-selector policy, including its kept line, over a terminal.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(super) fn choose_opt_interruptible_on(
    prompt: &str,
    choices: &[Choice],
    default: usize,
    term: &impl ListTerminal,
) -> dialoguer::Result<Option<usize>> {
    if choices.is_empty() {
        return Err(std::io::Error::other("answer choices are empty").into());
    }
    let labels: Vec<String> = choices.iter().map(Choice::label).collect();
    let picked = list_question(prompt, &labels, default, term)?;
    if let Some(index) = picked {
        answer(&super::prompt_theme(), prompt, &labels[index]);
    } else {
        term.write_line(&kept_line(prompt, GlyphSet::detect()))?;
    }
    Ok(picked)
}

/// What one key does to a list question whose marker is on `selected` of
/// `len` rows. Letters other than the `j`/`k` movers answer nothing, so
/// the keyboard layout never matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListKey {
    Move(usize),
    Choose(usize),
    Cancel,
    Ignore,
}

/// The pure key handling of [`list_question`].
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(super) fn list_key(key: &console::Key, selected: usize, len: usize) -> ListKey {
    use console::Key;
    if len == 0 {
        return ListKey::Cancel;
    }
    match key {
        Key::ArrowDown | Key::Tab | Key::Char('j') => ListKey::Move((selected + 1) % len),
        Key::ArrowUp | Key::BackTab | Key::Char('k') => ListKey::Move((selected + len - 1) % len),
        Key::Enter | Key::Char(' ') => ListKey::Choose(selected.min(len - 1)),
        Key::Escape | Key::CtrlC | Key::Char('d') => ListKey::Cancel,
        _ => ListKey::Ignore,
    }
}

/// Key, draw and cursor operations at the list-question boundary.
/// The real terminal delegates directly to console; tests apply these same
/// operations to an in-memory screen before supplying a key.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(super) trait ListTerminal {
    /// Whether raw key input is available.
    fn is_term(&self) -> bool;
    /// Width used to count wrapped question lines.
    fn columns(&self) -> usize;
    /// Draw the shared spacing and legend before the erasable question.
    fn prepare(&self);
    /// Write a line and advance the cursor.
    fn write_line(&self, line: &str) -> std::io::Result<()>;
    /// Erase preceding rows and return the cursor to their start.
    fn clear_last_lines(&self, lines: usize) -> std::io::Result<()>;
    /// Make every drawn row visible before reading a key.
    fn flush(&self) -> std::io::Result<()>;
    /// Read a raw key, restoring terminal mode before returning.
    fn read_key_raw(&self) -> std::io::Result<console::Key>;
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
impl ListTerminal for console::Term {
    fn is_term(&self) -> bool {
        console::Term::is_term(self)
    }
    fn columns(&self) -> usize {
        usize::from(self.size().1).max(1)
    }
    fn prepare(&self) {
        space_before_question();
        key_legend();
    }
    fn write_line(&self, line: &str) -> std::io::Result<()> {
        console::Term::write_line(self, line)
    }
    fn clear_last_lines(&self, lines: usize) -> std::io::Result<()> {
        console::Term::clear_last_lines(self, lines)
    }
    fn flush(&self) -> std::io::Result<()> {
        console::Term::flush(self)
    }
    fn read_key_raw(&self) -> std::io::Result<console::Key> {
        console::Term::read_key_raw(self)
    }
}

/// The one list loop behind [`choose_opt_interruptible`] and [`confirm`]:
/// the prompt line, the rows with an accent cursor on the active one, and on a choice
/// the whole block erased so the caller's `✓` line stands alone. `None`
/// is escape or Ctrl-C.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
fn list_question(
    prompt: &str,
    rows: &[String],
    default: usize,
    term: &impl ListTerminal,
) -> dialoguer::Result<Option<usize>> {
    if !term.is_term() {
        // console answers every key with `Unknown` off a terminal, which
        // would redraw forever; refuse as `dialoguer` does.
        return Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "not a terminal").into());
    }
    let theme = super::prompt_theme();
    term.prepare();
    let mut label = String::new();
    dialoguer::theme::Theme::format_prompt(&theme, &mut label, prompt)
        .map_err(std::io::Error::other)?;
    term.write_line(&label)?;
    let columns = term.columns().max(1);
    let label_lines = console::measure_text_width(&label).div_ceil(columns).max(1);
    let mut selected = default.min(rows.len() - 1);
    let mut drawn = false;
    loop {
        if drawn {
            term.clear_last_lines(rows.len())?;
        }
        for (index, row) in rows.iter().enumerate() {
            term.write_line(&list_row(row, index == selected, GlyphSet::detect()))?;
        }
        term.flush()?;
        drawn = true;
        // console restores the original termios before returning this key.
        // Raw key mode returns CtrlC rather than raising SIGINT; no process
        // signal handler is installed and this widget never hides the cursor.
        match list_key(&term.read_key_raw()?, selected, rows.len()) {
            ListKey::Move(next) => selected = next,
            ListKey::Choose(chosen) => {
                term.clear_last_lines(rows.len() + label_lines)?;
                return Ok(Some(chosen));
            }
            ListKey::Cancel => {
                term.clear_last_lines(rows.len() + label_lines)?;
                return Ok(None);
            }
            ListKey::Ignore => {}
        }
    }
}

/// Rows that are only names, as choices.
pub(crate) fn named(names: &[&str]) -> Vec<Choice> {
    names
        .iter()
        .map(|name| Choice {
            name: (*name).to_owned(),
            ..Choice::default()
        })
        .collect()
}

fn pick(
    prompt: &str,
    choices: &[Choice],
    default: usize,
    optional: bool,
) -> dialoguer::Result<Option<usize>> {
    space_before_question();
    key_legend();
    let theme = super::prompt_theme();
    let select = Select::with_theme(&theme)
        .with_prompt(prompt)
        .items(column_items(choices, &ItemStyles::stderr()))
        .default(default)
        // The answer line is printed from the flat label below, not
        // echoed from the aligned row the list drew.
        .report(false);
    let picked = if optional {
        select.interact_opt()?
    } else {
        Some(select.interact()?)
    };
    if let Some(chosen) = picked.and_then(|index| choices.get(index)) {
        answer(&theme, prompt, &chosen.label());
    }
    Ok(picked)
}

/// Several rows of `choices`, or `None` when the reader pressed escape.
pub(crate) fn choose_many(
    prompt: &str,
    choices: &[Choice],
    defaults: &[bool],
) -> dialoguer::Result<Option<Vec<usize>>> {
    space_before_question();
    key_legend();
    let theme = super::prompt_theme();
    let picked = MultiSelect::with_theme(&theme)
        .with_prompt(prompt)
        .items(column_items(choices, &ItemStyles::stderr()))
        .defaults(defaults)
        .report(false)
        .interact_opt()?;
    if let Some(indexes) = &picked {
        let labels: Vec<String> = indexes
            .iter()
            .filter_map(|index| choices.get(*index))
            .map(Choice::label)
            .collect();
        let borrowed: Vec<&str> = labels.iter().map(String::as_str).collect();
        answers(&theme, prompt, &borrowed);
    }
    Ok(picked)
}

/// Hidden explicit replacement input, with no collapsed value report.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
pub(crate) fn secret(prompt: &str) -> dialoguer::Result<String> {
    use dialoguer::theme::Theme;
    use ratatui_crossterm::crossterm::{
        event::{self, Event, KeyCode, KeyModifiers},
        terminal,
    };
    struct RawGuard;
    impl Drop for RawGuard {
        fn drop(&mut self) {
            let _ = terminal::disable_raw_mode();
        }
    }
    space_before_question();
    let mut rendered = String::new();
    super::prompt_theme()
        .format_prompt(&mut rendered, prompt)
        .map_err(|_| std::io::Error::other("protected prompt unavailable"))?;
    let term = console::Term::stderr();
    if terminal::is_raw_mode_enabled()? {
        return Err(std::io::Error::other("protected input requires an unowned terminal").into());
    }
    terminal::enable_raw_mode()?;
    let guard = RawGuard;
    term.write_str(&rendered)?;
    term.flush()?;
    let result = (|| -> std::io::Result<String> {
        let mut value = String::new();
        loop {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('c')
                        if key.code == KeyCode::Esc
                            || key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Interrupted,
                            "protected input cancelled",
                        ));
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "protected input ended",
                        ));
                    }
                    KeyCode::Enter => {
                        if value.is_empty() {
                            return Err(std::io::Error::other("protected input must not be empty"));
                        }
                        return Ok(value);
                    }
                    KeyCode::Backspace => {
                        value.pop();
                    }
                    KeyCode::Char(c)
                        if !c.is_control()
                            && !key
                                .modifiers
                                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        if value.len() + c.len_utf8() > 1024 * 1024 {
                            return Err(std::io::Error::other("protected input exceeds bound"));
                        }
                        value.push(c);
                    }
                    _ => {}
                }
            }
        }
    })();
    drop(guard);
    term.write_line("")?;
    result.map_err(Into::into)
}
