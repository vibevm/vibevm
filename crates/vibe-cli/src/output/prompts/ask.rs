//! The questions every command asks, as blocks of PROP-067 `#question`.
//!
//! A command asks through these and never builds a `dialoguer` prompt of
//! its own, so the blank line above a question, the key legend over the
//! first list, the column rows and the collapsed `✓ <prompt>  <value>`
//! line are the same in `init`, `uninstall`, `self gc` and the app wizard.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#question");

use dialoguer::{Confirm, Input, MultiSelect, Select};
use specmark::spec;
use vibe_app::Choice;

use super::{ItemStyles, answer, answers, column_items, key_legend, space_before_question};

/// A yes-or-no question; it collapses to `✓ <prompt>  yes` by itself,
/// because `dialoguer` reports a confirmation through the theme.
///
/// ```text
/// ? Remove 3 dependency slots? › (y/n)   ->   ✓ Remove 3 dependency slots?  no
/// ```
pub(crate) fn confirm(prompt: &str, default: bool) -> dialoguer::Result<bool> {
    space_before_question();
    Confirm::with_theme(&super::prompt_theme())
        .with_prompt(prompt)
        .default(default)
        .interact()
}

/// A confirmation whose collapsed line the caller writes itself, because
/// what was decided reads better than `yes` (`✓ remap offer  accepted …`).
pub(crate) fn confirm_then(
    prompt: &str,
    default: bool,
    name: &str,
    said: impl FnOnce(bool) -> String,
) -> dialoguer::Result<bool> {
    space_before_question();
    let theme = super::prompt_theme();
    let yes = Confirm::with_theme(&theme)
        .with_prompt(prompt)
        .default(default)
        .report(false)
        .interact()?;
    answer(&theme, name, &said(yes));
    Ok(yes)
}

/// A line of text, with the value it starts from.
pub(crate) fn text(prompt: &str, default: &str, allow_empty: bool) -> dialoguer::Result<String> {
    space_before_question();
    Input::<String>::with_theme(&super::prompt_theme())
        .with_prompt(prompt)
        .default(default.to_owned())
        .show_default(true)
        .allow_empty(allow_empty)
        .interact_text()
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
    use console::{Key, Term};
    if choices.is_empty() {
        return Err(std::io::Error::other("answer choices are empty").into());
    }
    let term = Term::stderr();
    let theme = super::prompt_theme();
    space_before_question();
    key_legend();
    let mut label = String::new();
    dialoguer::theme::Theme::format_prompt(&theme, &mut label, prompt)
        .map_err(std::io::Error::other)?;
    term.write_line(&label)?;
    let mut selected = default.min(choices.len() - 1);
    let mut drawn = false;
    loop {
        if drawn {
            term.clear_last_lines(choices.len())?;
        }
        for (index, choice) in choices.iter().enumerate() {
            term.write_line(&format!(
                "{} {}",
                if index == selected { "›" } else { " " },
                choice.label()
            ))?;
        }
        term.flush()?;
        drawn = true;
        // console restores the original termios before returning this key.
        // Raw key mode returns CtrlC rather than raising SIGINT; no process
        // signal handler is installed and this widget never hides the cursor.
        match term.read_key_raw()? {
            Key::ArrowDown | Key::Tab | Key::Char('j') => selected = (selected + 1) % choices.len(),
            Key::ArrowUp | Key::BackTab | Key::Char('k') => {
                selected = (selected + choices.len() - 1) % choices.len()
            }
            Key::Enter | Key::Char(' ') => {
                term.clear_last_lines(choices.len())?;
                answer(&theme, prompt, &choices[selected].label());
                return Ok(Some(selected));
            }
            Key::Escape | Key::CtrlC | Key::Char('d') => return Ok(None),
            _ => {}
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
