//! Read a submitted public document once; ephemeral --set answers stay separate.
use crate::cli::AppChoiceArgs;
use anyhow::Result;
use std::io::Read;
use vibe_app::configuration::{ConfigurationError, ConfigurationSubmission, parse_submission};

pub(super) fn read(answers: &AppChoiceArgs) -> Result<Option<ConfigurationSubmission>> {
    if let Some(parsed) = &answers.parsed_configuration {
        return Ok(Some(parsed.clone()));
    }
    let Some(path) = &answers.configuration_input else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    if path == std::path::Path::new("-") {
        std::io::stdin().read_to_end(&mut bytes)?;
    } else {
        bytes = std::fs::read(path)?;
    }
    parse_submission(&bytes).map(Some).map_err(|reason| {
        vibe_app::AppError::from(ConfigurationError::Invalid {
            option: "submitted input".into(),
            reason,
        })
        .into()
    })
}
pub(super) fn preload(answers: &mut AppChoiceArgs) -> Result<()> {
    answers.parsed_configuration = read(answers)?;
    Ok(())
}

/// Resolve the actual installer selection of every recipient before any shared answer is used.
pub(super) fn select_batch(
    ctx: &crate::output::Context,
    roots: Vec<vibe_app::ResolvedPackage>,
    choice: &vibe_app::InstallChoice,
    set: &[(String, String)],
    submission: Option<&ConfigurationSubmission>,
) -> Result<Vec<(vibe_app::ResolvedPackage, vibe_app::InstallChoice)>> {
    if set.is_empty() && submission.is_none() {
        return Ok(roots.into_iter().map(|r| (r, choice.clone())).collect());
    }
    let chooser =
        super::global::attended(ctx).then_some(&super::wizard::chooser as vibe_app::Chooser<'_>);
    let mut selected = Vec::new();
    for root in roots {
        let option = vibe_app::select_option(&root.options, choice, chooser)
            .map_err(vibe_app::AppError::from)?;
        selected.push((root, option));
    }
    let keys: Vec<_> = selected.iter().map(|(r, _)| r.package.key()).collect();
    let recipients: Vec<_> = selected
        .iter()
        .zip(&keys)
        .map(|((_, o), key)| (key.as_str(), o))
        .collect();
    vibe_app::configuration::classify_shared(&recipients, set, submission)
        .map_err(vibe_app::AppError::from)?;
    Ok(selected
        .into_iter()
        .map(|(root, option)| {
            (
                root,
                vibe_app::InstallChoice {
                    platform: Some(option.platform),
                    arch: Some(option.arch),
                    installer: Some(option.id),
                },
            )
        })
        .collect())
}
