//! The questions every command asks, as blocks of PROP-067 `#question`.
//!
//! A command asks through these and never builds a `dialoguer` prompt of
//! its own, so the blank line above a question, the key legend over the
//! first list, the column rows and the collapsed `✓ <prompt>  <value>`
//! line are the same in `init`, `uninstall`, `self gc` and the app wizard.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#question");

use dialoguer::{Confirm, Input, MultiSelect, Select};
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
