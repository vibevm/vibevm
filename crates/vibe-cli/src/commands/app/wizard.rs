//! The attended questions of an application install or update (PROP-065
//! `#plan`, `#features`, `#parameters`, `#license`).
//!
//! Each question is built with the shared prompt theme
//! ([`crate::output::prompt_theme`]) and draws the library's structured
//! rows as aligned columns, so the wizard reads like the step panel that
//! follows it instead of like `dialoguer`'s bare default.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use dialoguer::{Confirm, Input, MultiSelect, Password, Select};
use vibe_app::Choice;
use vibe_core::manifest::{ParameterDecl, ParameterType};

use super::display::asking;
use crate::output;
use crate::output::prompts;

pub(super) fn chooser(
    prompt: &str,
    items: &[Choice],
) -> std::result::Result<Option<usize>, String> {
    asking(|| {
        let theme = output::prompt_theme();
        prompts::space_before_question();
        prompts::key_legend();
        let picked = Select::with_theme(&theme)
            .with_prompt(prompt)
            .items(prompts::column_items(items, &prompts::ItemStyles::stderr()))
            .default(0)
            // The answer line is printed from the flat label below, not
            // echoed from the aligned row the list drew.
            .report(false)
            .interact_opt()
            .map_err(|error| error.to_string())?;
        if let Some(chosen) = picked.and_then(|index| items.get(index)) {
            prompts::answer(&theme, prompt, &chosen.label());
        }
        Ok(picked)
    })
}

pub(super) fn feature_chooser(
    prompt: &str,
    items: &[Choice],
    defaults: &[bool],
) -> std::result::Result<Option<Vec<usize>>, String> {
    asking(|| {
        let theme = output::prompt_theme();
        prompts::space_before_question();
        prompts::key_legend();
        let picked = MultiSelect::with_theme(&theme)
            .with_prompt(prompt)
            .items(prompts::column_items(items, &prompts::ItemStyles::stderr()))
            .defaults(defaults)
            .report(false)
            .interact_opt()
            .map_err(|error| error.to_string())?;
        if let Some(indexes) = &picked {
            let labels: Vec<String> = indexes
                .iter()
                .filter_map(|index| items.get(*index))
                .map(Choice::label)
                .collect();
            let borrowed: Vec<&str> = labels.iter().map(String::as_str).collect();
            prompts::answers(&theme, prompt, &borrowed);
        }
        Ok(picked)
    })
}

/// A typed parameter question (PROP-065 `#parameters`): a confirmation, a
/// list, a hidden entry for a secret, otherwise a line of text.
pub(super) fn parameter_prompt(
    parameter: &ParameterDecl,
    preset: Option<&str>,
) -> std::result::Result<Option<String>, String> {
    let label = match &parameter.description {
        Some(text) => format!("{} ({}) — {text}", parameter.name, parameter.id),
        None => format!("{} ({})", parameter.name, parameter.id),
    };
    let failed = |error: dialoguer::Error| error.to_string();
    let theme = output::prompt_theme();
    asking(|| {
        prompts::space_before_question();
        match parameter.kind {
            ParameterType::Bool => Confirm::with_theme(&theme)
                .with_prompt(label)
                .default(preset == Some("true"))
                .interact_opt()
                .map(|answer| answer.map(|yes| yes.to_string()))
                .map_err(failed),
            ParameterType::Choice => {
                let choices: Vec<Choice> = parameter
                    .choices
                    .iter()
                    .map(|choice| Choice::named(&choice.name, &choice.value))
                    .collect();
                let default = parameter
                    .choices
                    .iter()
                    .position(|choice| Some(choice.value.as_str()) == preset)
                    .unwrap_or(0);
                prompts::key_legend();
                let picked = Select::with_theme(&theme)
                    .with_prompt(&label)
                    .items(prompts::column_items(
                        &choices,
                        &prompts::ItemStyles::stderr(),
                    ))
                    .default(default)
                    .report(false)
                    .interact_opt()
                    .map_err(failed)?;
                if let Some(chosen) = picked.and_then(|index| choices.get(index)) {
                    prompts::answer(&theme, &label, &chosen.label());
                }
                Ok(picked
                    .and_then(|index| parameter.choices.get(index))
                    .map(|choice| choice.value.clone()))
            }
            _ if parameter.secret => Password::with_theme(&theme)
                .with_prompt(label)
                .allow_empty_password(!parameter.required)
                .interact()
                .map(|text| Some(text).filter(|text| !text.is_empty()))
                .map_err(failed),
            _ => {
                let mut input = Input::<String>::with_theme(&theme)
                    .with_prompt(label)
                    .allow_empty(true);
                if let Some(preset) = preset {
                    input = input.default(preset.to_owned());
                }
                input
                    .interact_text()
                    .map(|text| Some(text).filter(|text| !text.is_empty()))
                    .map_err(failed)
            }
        }
    })
}

/// Show the license text, then ask (PROP-065 `#license`).
pub(super) fn license_prompt(name: &str, text: &str) -> std::result::Result<bool, String> {
    asking(|| {
        prompts::space_before_question();
        eprintln!("{text}");
        Confirm::with_theme(&output::prompt_theme())
            .with_prompt(format!("Accept the license `{name}`?"))
            .default(false)
            .interact()
            .map_err(|error| error.to_string())
    })
}
