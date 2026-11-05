//! Existing console callbacks and CLI deltas feed the same public evaluator.
use super::{
    ConfigurationError, EvaluatedConfiguration, EvaluationRequest, evaluate, input, parameters,
    vocabulary,
};
use crate::steps::PlanInput;
use crate::{AppError, Choice};
use vibe_core::manifest::{ConfigurationFeatureVisibility, ConfigurationGroupDecl};
use vibe_wire::generated::shared as w;

pub(crate) fn prepare(input: &PlanInput<'_>) -> Result<EvaluatedConfiguration, AppError> {
    let option = input.option;
    let definition = option
        .configuration
        .as_ref()
        .ok_or_else(|| invalid(&option.id, "no common definition".into()))?;
    if let Some(transfer) = input.transfer {
        if input.configuration.is_some() || !input.request.is_empty() {
            return Err(invalid(&option.id,"portable transfer and ordinary configuration/feature inputs are mutually exclusive".into()));
        }
        if transfer
            .target
            .installer
            .as_ref()
            .is_some_and(|id| id != &option.id)
        {
            return Err(invalid(
                &option.id,
                "portable target decision names another installer".into(),
            ));
        }
        let result = super::transfer::prepare_required(
            super::transfer::TransferRequest {
                definition,
                history: &transfer.history,
                target: &transfer.target,
                ephemeral: input.parameters,
                facts: input.facts,
                prompt: input.prompts.parameter.filter(|_| input.attended),
            },
            &transfer.required_secrets,
        )
        .map_err(|reason| invalid(&option.id, reason))?;
        return if result.record.evaluation.resolved.is_some() {
            Ok(result)
        } else {
            Err(ConfigurationError::Unresolved {
                option: format!("{}/{}", input.resolved.package.key(), option.id),
                evaluation: Box::new(result.record.evaluation),
            }
            .into())
        };
    }
    let previous = input.previous.and_then(|p| p.configuration);
    let mut edited = input::edit(
        definition,
        input.configuration,
        previous,
        input.request,
        input.parameters,
    )
    .map_err(|e| invalid(&option.id, e))?;
    let legacy_transfer =
        input.previous.is_some() && previous.is_none() && input.configuration.is_none();
    let mut fresh = input.configuration.is_some() || input.previous.is_none();
    let mut explicitly_new = false;
    if legacy_transfer {
        let old = input
            .previous
            .ok_or_else(|| invalid(&option.id, "missing previous effective values".into()))?;
        let suggestion = format!(
            "Previous effective features: [{}]. Public parameter suggestions: {}. A new common configuration has no recovered profile, dormant answers or default history.",
            old.features.join(", "),
            old.parameters
                .iter()
                .filter(|(id, _)| definition
                    .parameters
                    .iter()
                    .any(|p| &p.id == *id && !p.secret))
                .map(|(id, value)| format!("{id}={value}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let accepted = if input.attended {
            if let Some(ask) = input.prompts.chooser {
                let rows = [
                    Choice {
                        name: "Configure anew".into(),
                        description: Some(suggestion),
                        ..Default::default()
                    },
                    Choice {
                        name: "Cancel".into(),
                        ..Default::default()
                    },
                ];
                ask("This installation has only legacy effective values", &rows)
                    .map_err(|e| invalid(&option.id, e))?
                    == Some(0)
            } else {
                false
            }
        } else {
            false
        };
        if !accepted {
            return Err(invalid(&option.id,"the previous installation has no original common intent; provide a complete --configuration-input or explicitly choose a new configuration in the console".into()));
        }
        fresh = true;
        explicitly_new = true;
    }
    let prompted = input.attended
        && input.configuration.is_none()
        && (explicitly_new || input.previous.is_none() || input.previous.is_some_and(|p| p.ask));
    if prompted {
        if input.request.profile.is_none()
            && !definition.profiles.is_empty()
            && let Some(ask) = input.prompts.chooser
        {
            let retained = edited.input.intent.profile.as_deref();
            let mut profiles: Vec<Option<_>> = definition.profiles.iter().map(Some).collect();
            profiles
                .sort_by_key(|p| p.map(|p| (retained.is_some_and(|id| id != p.id), !p.default)));
            if previous.is_some() && retained.is_none() {
                profiles.insert(0, None);
            } else {
                profiles.push(None);
            }
            let rows: Vec<_> = profiles
                .iter()
                .map(|p| match p {
                    Some(p) => Choice {
                        name: p.name.clone(),
                        description: p.description.clone(),
                        id: Some(p.id.clone()),
                        ..Default::default()
                    },
                    None => Choice {
                        name: "Custom".into(),
                        ..Default::default()
                    },
                })
                .collect();
            let selected = ask("Configuration profile", &rows)
                .map_err(|e| invalid(&option.id, e))?
                .ok_or_else(|| invalid(&option.id, "configuration cancelled".into()))?;
            edited.input.intent.profile = profiles
                .get(selected)
                .ok_or_else(|| {
                    invalid(
                        &option.id,
                        "profile callback returned an invalid row".into(),
                    )
                })?
                .map(|p| p.id.clone());
        }
        if edited.input.intent.profile.is_none()
            && let Some(ask) = input.prompts.features
        {
            let members: Vec<&str> = definition
                .groups
                .iter()
                .filter_map(|g| match g {
                    ConfigurationGroupDecl::Choice { members, .. } => Some(members),
                    _ => None,
                })
                .flatten()
                .map(String::as_str)
                .collect();
            let ordinary: Vec<_> = definition
                .features
                .iter()
                .filter(|f| {
                    f.visibility == ConfigurationFeatureVisibility::Public
                        && !members.contains(&f.id.as_str())
                })
                .collect();
            if !ordinary.is_empty() {
                let rows: Vec<_> = ordinary
                    .iter()
                    .map(|f| Choice {
                        name: f.name.clone(),
                        description: f.description.clone(),
                        id: Some(f.id.clone()),
                        ..Default::default()
                    })
                    .collect();
                let selected: Vec<_> = ordinary
                    .iter()
                    .map(|f| {
                        edited.input.intent.features.contains(&f.id)
                            || if explicitly_new {
                                input.previous.is_some_and(|p| p.features.contains(&f.id))
                            } else {
                                edited.input.intent.default_features && f.default
                            }
                    })
                    .collect();
                let picked = ask("Select features", &rows, &selected)
                    .map_err(|e| invalid(&option.id, e))?
                    .ok_or_else(|| invalid(&option.id, "configuration cancelled".into()))?;
                let positives: Vec<String> = picked
                    .iter()
                    .map(|&i| {
                        ordinary.get(i).map(|f| f.id.clone()).ok_or_else(|| {
                            invalid(
                                &option.id,
                                "feature callback returned an invalid row".into(),
                            )
                        })
                    })
                    .collect::<Result<_, _>>()?;
                edited
                    .input
                    .intent
                    .features
                    .retain(|id| !ordinary.iter().any(|f| &f.id == id));
                edited
                    .input
                    .disabled_features
                    .retain(|id| !positives.contains(id));
                edited.input.intent.features.extend(positives);
                edited.input.intent.default_features = false;
            }
        }
        if let Some(ask) = input.prompts.parameter {
            for p in definition.parameters.iter().filter(|p| !p.secret) {
                if edited.fresh.contains(&p.id) {
                    continue;
                }
                let profile = edited
                    .input
                    .intent
                    .profile
                    .as_ref()
                    .and_then(|id| definition.profiles.iter().find(|p| &p.id == id));
                let old_suggestion = if explicitly_new {
                    input
                        .previous
                        .and_then(|old| old.parameters.get(&p.id))
                        .cloned()
                } else {
                    None
                };
                let preset = edited
                    .input
                    .intent
                    .parameters
                    .get(&p.id)
                    .map(vocabulary::read_value)
                    .transpose()
                    .map_err(|e| invalid(&option.id, e))?
                    .or_else(|| profile.and_then(|pr| pr.parameters.get(&p.id).cloned()))
                    .or_else(|| p.default.clone())
                    .map(|v| v.to_string());
                let preset = old_suggestion.clone().or(preset);
                let answer = ask(p, preset.as_deref()).map_err(|e| invalid(&option.id, e))?;
                if let Some(raw) = answer.or(old_suggestion) {
                    let canonical = p.canonical(&raw).map_err(|e| invalid(&option.id, e))?;
                    edited.input.intent.parameters.insert(
                        p.id.clone(),
                        vocabulary::value(
                            &parameters::typed(p.kind, &canonical)
                                .map_err(|e| invalid(&option.id, e))?,
                        ),
                    );
                    edited.fresh.insert(p.id.clone());
                }
            }
        }
    }
    let mut reviewed = false;
    let mut secret_prompted = false;
    let mut answered = std::collections::BTreeSet::new();
    loop {
        let result = evaluate(EvaluationRequest {
            history_use: crate::configuration::HistoryUse::Local,
            definition,
            input: &edited.input,
            previous: previous.map(super::ConfigurationHistory::from),
            replacement: fresh,
            reviewed,
            expected_definition: input
                .configuration
                .and_then(|s| s.expected_definition.as_deref()),
            fresh_parameters: &edited.fresh,
            ephemeral: &edited.ephemeral,
            facts: input.facts,
            prompt: input
                .prompts
                .parameter
                .filter(|_| input.attended && !secret_prompted),
        })
        .map_err(|e| invalid(&option.id, e))?;
        secret_prompted = true;
        if result.record.evaluation.resolved.is_some()
            && (!prompted
                || result
                    .record
                    .evaluation
                    .controls
                    .iter()
                    .all(|c| !c.active || answered.contains(&c.id)))
        {
            return Ok(result);
        }
        for parameter in result.parameters.iter().filter(|p| p.secret&&result.record.evaluation.parameters.iter().any(|r|matches!(r,w::ConfigurationParameterResult::Secret(v) if v.id==p.id&&v.provided))) {
            if !edited.ephemeral.iter().any(|(id, _)| id == &parameter.id) {
                edited
                    .ephemeral
                    .push((parameter.id.clone(), parameter.value.clone()));
            }
        }
        if input.attended
            && !reviewed
            && result
                .record
                .evaluation
                .questions
                .iter()
                .any(|q| matches!(q, w::ConfigurationQuestion::Review(_)))
            && let Some(ask) = input.prompts.chooser
        {
            let rows = [
                Choice {
                    name: "Use the current definition's automatic choices".into(),
                    description: Some(
                        super::review::summary(
                            previous.ok_or_else(|| {
                                invalid(&option.id, "review has no historical definition".into())
                            })?,
                            &result.record,
                        )
                        .map_err(|_| {
                            invalid(&option.id, "cannot render definition review".into())
                        })?,
                    ),
                    ..Default::default()
                },
                Choice {
                    name: "Cancel".into(),
                    ..Default::default()
                },
            ];
            if ask("The automatic configuration basis changed", &rows)
                .map_err(|e| invalid(&option.id, e))?
                == Some(0)
            {
                reviewed = true;
                continue;
            }
        }
        let question = result
            .record
            .evaluation
            .questions
            .iter()
            .find_map(|q| match q {
                w::ConfigurationQuestion::Choice(q) if !answered.contains(&q.group) => {
                    Some((**q).clone())
                }
                _ => None,
            })
            .or_else(|| {
                if !prompted {
                    return None;
                }
                result
                    .record
                    .evaluation
                    .controls
                    .iter()
                    .find(|c| c.active && !answered.contains(&c.id))
                    .map(|c| w::ConfigurationQuestionChoice {
                        group: c.id.clone(),
                        cardinality: c.cardinality.clone(),
                        eligible_members: c
                            .members
                            .iter()
                            .filter(|m| m.eligible)
                            .map(|m| m.id.clone())
                            .collect(),
                        consumers: vec!["installer".into()],
                    })
            });
        if input.attended
            && let (Some(q), Some(ask)) = (question, input.prompts.features)
        {
            let rows: Vec<_> = q
                .eligible_members
                .iter()
                .map(|id| {
                    let f = definition.features.iter().find(|f| &f.id == id);
                    Choice {
                        id: Some(id.clone()),
                        name: f.map_or(id.as_str(), |f| f.name.as_str()).into(),
                        description: f.and_then(|f| f.description.clone()),
                        ..Default::default()
                    }
                })
                .collect();
            let defaults = definition.groups.iter().find_map(|g| match g {
                ConfigurationGroupDecl::Choice { id, defaults, .. } if id == &q.group => {
                    defaults.as_ref()
                }
                _ => None,
            });
            let selection = result
                .record
                .evaluation
                .controls
                .iter()
                .find(|c| c.id == q.group)
                .and_then(|c| c.selection.as_ref())
                .or(defaults);
            let preselected = rows
                .iter()
                .map(|r| {
                    if explicitly_new {
                        input.previous.is_some_and(|p| {
                            r.id.as_ref().is_some_and(|id| p.features.contains(id))
                        })
                    } else {
                        selection.is_some_and(|d| r.id.as_ref().is_some_and(|id| d.contains(id)))
                    }
                })
                .collect::<Vec<_>>();
            let picked = ask(&q.group, &rows, &preselected)
                .map_err(|e| invalid(&option.id, e))?
                .ok_or_else(|| invalid(&option.id, "configuration cancelled".into()))?;
            let mut members: Vec<String> = picked
                .iter()
                .map(|&i| {
                    q.eligible_members.get(i).cloned().ok_or_else(|| {
                        invalid(&option.id, "choice callback returned an invalid row".into())
                    })
                })
                .collect::<Result<_, _>>()?;
            let prior = edited.input.intent.selections.get(&q.group);
            let intent = prior
                .map(|p| p.intent.clone())
                .or_else(|| {
                    edited
                        .input
                        .intent
                        .profile
                        .as_ref()
                        .and_then(|id| definition.profiles.iter().find(|p| &p.id == id))
                        .and_then(|p| p.selections.get(&q.group))
                        .map(|p| vocabulary::intent(&p.intent))
                })
                .unwrap_or(w::ConfigurationIntent::Exact);
            if let Some(hidden) = prior.and_then(|p| p.members.as_ref()) {
                members.extend(
                    hidden
                        .iter()
                        .filter(|id| !q.eligible_members.contains(id))
                        .cloned(),
                );
            } else if prior.is_none()
                && let Some(profile) = edited
                    .input
                    .intent
                    .profile
                    .as_ref()
                    .and_then(|id| definition.profiles.iter().find(|p| &p.id == id))
                && let Some(hidden) = profile
                    .selections
                    .get(&q.group)
                    .and_then(|p| p.members.as_ref())
            {
                members.extend(
                    hidden
                        .iter()
                        .filter(|id| !q.eligible_members.contains(id))
                        .cloned(),
                );
            }
            members.sort();
            members.dedup();
            edited.input.intent.selections.insert(
                q.group.clone(),
                w::ConfigurationSelection {
                    intent,
                    members: Some(members),
                },
            );
            answered.insert(q.group.clone());
            continue;
        }
        return Err(ConfigurationError::Unresolved {
            option: option.id.clone(),
            evaluation: Box::new(result.record.evaluation),
        }
        .into());
    }
}
fn invalid(option: &str, reason: String) -> AppError {
    ConfigurationError::Invalid {
        option: option.into(),
        reason,
    }
    .into()
}
