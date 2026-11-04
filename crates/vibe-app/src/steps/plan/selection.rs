//! Legacy app input precedence and attended selection around the shared kernel.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#features");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#profiles");

use super::PlanInput;
use crate::error::{AppError, StepError};
use crate::select::Choice;
use crate::steps::features::{FeatureRequest, FeatureResolution, resolve_features, tree_order};

/// The profile and the resolved feature set: flags (with a profile) as
/// named, else the recorded set of an update, else an attended question
/// (the profiles first, "custom" opening the tree), else the defaults.
pub(super) fn choose_features(
    input: &PlanInput<'_>,
) -> Result<(Option<String>, FeatureResolution), AppError> {
    let option = input.option;
    let request = input.request;
    let find = |id: &str| {
        option
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| StepError::UnknownProfile {
                option: option.id.clone(),
                profile: id.into(),
                declared: option
                    .profiles
                    .iter()
                    .map(|profile| profile.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    };
    let mut profile = request.profile.as_deref().map(find).transpose()?;
    let reask = input.previous.is_some_and(|previous| previous.ask);
    let previous = input.previous.map(|previous| previous.features);
    let mut prompted = None;
    let ask = request.is_empty() && (previous.is_none() || reask) && input.attended;
    if let (true, Some(features)) = (ask && !option.features.is_empty(), input.prompts.features) {
        let mut custom = true;
        if let (false, Some(chooser)) = (option.profiles.is_empty(), input.prompts.chooser) {
            let mut ordered: Vec<_> = option.profiles.iter().collect();
            ordered.sort_by_key(|profile| !profile.default);
            let mut choices: Vec<Choice> = ordered
                .iter()
                .map(|profile| Choice {
                    name: profile.name.clone(),
                    description: profile.description.clone(),
                    ..Choice::default()
                })
                .collect();
            choices.push(Choice {
                name: "Custom".into(),
                description: Some("choose the features one by one".into()),
                ..Choice::default()
            });
            let picked = chooser(
                &format!("Setup type of installer `{}`", option.id),
                &choices,
            )
            .map_err(|reason| StepError::FeaturePrompt { reason })?
            .ok_or_else(|| StepError::FeaturePrompt {
                reason: "the selection was cancelled".into(),
            })?;
            if let Some(chosen) = ordered.get(picked) {
                profile = Some(*chosen);
                custom = false;
            }
        }
        if custom {
            let defaults = resolve_features(option, None, &FeatureRequest::default(), None, None)?;
            let order = tree_order(&option.features);
            let choices: Vec<Choice> = order
                .iter()
                .map(|(depth, feature)| Choice {
                    name: feature.name.clone(),
                    id: Some(feature.id.clone()),
                    description: feature.description.clone(),
                    depth: *depth,
                    ..Choice::default()
                })
                .collect();
            let chosen = match previous {
                Some(recorded) if reask => recorded.to_vec(),
                _ => defaults.selected.clone(),
            };
            let preselected: Vec<bool> = order
                .iter()
                .map(|(_, feature)| chosen.contains(&feature.id))
                .collect();
            let picked = features(
                &format!("Select features of installer `{}`", option.id),
                &choices,
                &preselected,
            )
            .map_err(|reason| StepError::FeaturePrompt { reason })?
            .ok_or_else(|| StepError::FeaturePrompt {
                reason: "the selection was cancelled".into(),
            })?;
            prompted = Some(
                picked
                    .iter()
                    .filter_map(|index| order.get(*index).map(|(_, f)| f.id.clone()))
                    .collect::<Vec<_>>(),
            );
        }
    }
    let resolution = resolve_features(option, profile, request, previous, prompted.as_deref())?;
    Ok((profile.map(|profile| profile.id.clone()), resolution))
}
