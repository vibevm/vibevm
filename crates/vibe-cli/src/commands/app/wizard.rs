//! The attended questions of an application install or update (PROP-065
//! `#plan`, `#features`, `#parameters`, `#license`).
//!
//! Each question is asked through the shared helpers of
//! [`crate::output::prompts`], which every other command's questions use
//! too, and draws the library's structured rows as aligned columns, so the
//! wizard reads like the step panel that follows it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use dialoguer::{Confirm, Input, Password};
use vibe_app::Choice;
use vibe_core::manifest::{ParameterDecl, ParameterType};

use super::display::asking;
use crate::output;
use crate::output::prompts;

pub(super) fn chooser(
    prompt: &str,
    items: &[Choice],
) -> std::result::Result<Option<usize>, String> {
    asking(|| prompts::choose_opt(prompt, items, 0).map_err(|error| error.to_string()))
}

pub(super) fn feature_chooser(
    prompt: &str,
    items: &[Choice],
    defaults: &[bool],
) -> std::result::Result<Option<Vec<usize>>, String> {
    asking(|| prompts::choose_many(prompt, items, defaults).map_err(|error| error.to_string()))
}

/// A typed parameter question (PROP-065 `#parameters`): hidden input for
/// every secret kind, otherwise a confirmation, list or line of text.
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
            _ if parameter.secret => Password::with_theme(&theme)
                .with_prompt(label)
                .allow_empty_password(preset.is_some() || !parameter.required)
                .interact()
                .map(|text| Some(text).filter(|text| !text.is_empty()))
                .map_err(failed),
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
                let picked = prompts::choose_opt(&label, &choices, default).map_err(failed)?;
                Ok(picked
                    .and_then(|index| parameter.choices.get(index))
                    .map(|choice| choice.value.clone()))
            }
            _ => {
                let mut input = Input::<String>::with_theme(&theme)
                    .with_prompt(label)
                    .allow_empty(true);
                if let Some(preset) = preset {
                    input = input.default(preset.to_owned());
                }
                input
                    .interact_text()
                    .map(|text| {
                        Some(text).filter(|text| {
                            !text.is_empty()
                                && !(parameter.kind == ParameterType::Path
                                    && preset == Some(text.as_str()))
                        })
                    })
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
        prompts::confirm(&format!("Accept the license `{name}`?"), false)
            .map_err(|error| error.to_string())
    })
}
