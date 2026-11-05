//! Actual local evaluation using the common kernel and canonical parameter validator.
use super::{choices, definition, parameters, vocabulary};
use crate::{HostFacts, ParamValue, ParameterPrompt};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::{configuration as k, manifest as m};
use vibe_wire::generated::shared as w;

/// Inputs for one selected installer. Prior state is evidence, never current authority.
#[specmark::spec(
    documents = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
pub struct EvaluationRequest<'a> {
    pub definition: &'a m::ConfigurationDecl,
    pub input: &'a w::ConfigurationInput,
    pub previous: Option<super::ConfigurationHistory<'a>>,
    pub history_use: super::HistoryUse<'a>,
    /// A newly submitted complete input may bind the current definition directly.
    pub replacement: bool,
    /// Explicit review of changed automatic basis; does not re-anchor parameter sources.
    pub reviewed: bool,
    pub expected_definition: Option<&'a str>,
    /// Explicit parameter setters may deliberately re-anchor a relative path.
    pub fresh_parameters: &'a BTreeSet<String>,
    /// Only declared secret parameters belong here; never persisted or echoed.
    pub ephemeral: &'a [(String, String)],
    pub facts: HostFacts<'a>,
    pub prompt: Option<ParameterPrompt<'a>>,
}
/// Public inspectable data plus ephemeral execution values.
#[derive(Debug)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#EVALUATION")]
pub struct EvaluatedConfiguration {
    pub record: w::ConfigurationRecord,
    pub parameters: Vec<ParamValue>,
}
/// Evaluate one installer scope without running effects or granting consent.
/// An unresolved evaluation contains questions/problems and no resolved output.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
pub fn evaluate(request: EvaluationRequest<'_>) -> Result<EvaluatedConfiguration, String> {
    let d = request.definition;
    // Refuse secret input before copying any submitted value into public output.
    for id in request.input.intent.parameters.keys() {
        if !d.parameters.iter().any(|p| &p.id == id) {
            return Err(format!("unknown parameter `{id}`"));
        }
        if d.parameters.iter().any(|p| &p.id == id && p.secret) {
            return Err(format!("parameter `{id}` requires ephemeral secret input"));
        }
    }
    for (id, _) in request.ephemeral {
        if !d.parameters.iter().any(|p| &p.id == id && p.secret) {
            return Err(format!(
                "parameter `{id}` is not a declared secret parameter"
            ));
        }
    }
    if let super::HistoryUse::Transfer { path_bindings, .. } = request.history_use {
        for id in path_bindings.keys() {
            if !d
                .parameters
                .iter()
                .any(|p| &p.id == id && !p.secret && p.kind == m::ParameterType::Path)
            {
                return Err(format!(
                    "target binding `{id}` is not a declared public Path parameter"
                ));
            }
        }
    }
    let fingerprint = d
        .semantic_fingerprint()
        .map_err(|e| format!("{}: {e}", e.field))?;
    let mut out = w::ConfigurationEvaluation {
        definition: fingerprint,
        input: request.input.clone(),
        normalized_consumers: vec![],
        controls: vec![],
        defaults: vec![],
        questions: vec![],
        problems: vec![],
        parameters: vec![],
        reliance: vec![],
        resolved: None,
        feature_result: None,
    };
    if let Some(expected) = request
        .expected_definition
        .filter(|expected| *expected != out.definition)
    {
        out.problems
            .push(w::ConfigurationProblem::StaleDefinition(Box::new(
                w::ConfigurationProblemStaleDefinition {
                    actual: out.definition.clone(),
                    expected: expected.into(),
                },
            )));
    }
    let input = vocabulary::read_input(&request.input.intent)?;
    let mut normalized = match d.normalize_input(&input, "installer") {
        Ok(n) => n,
        Err(e) => {
            out.problems.push(w::ConfigurationProblem::Input(Box::new(
                w::ConfigurationProblemInput {
                    field: e
                        .field
                        .to_string()
                        .replacen("configuration", "input.intent", 1),
                    message: e.message,
                },
            )));
            return Ok(EvaluatedConfiguration {
                record: w::ConfigurationRecord {
                    definition: definition::definition(d),
                    evaluation: out,
                },
                parameters: vec![],
            });
        }
    };
    for id in &request.input.disabled_features {
        match d.features.iter().find(|f| &f.id == id) {
            Some(f) if f.visibility == m::ConfigurationFeatureVisibility::Public => {}
            _ => out.problems.push(w::ConfigurationProblem::Input(Box::new(
                w::ConfigurationProblemInput {
                    field: "input.disabledFeatures".into(),
                    message: format!(
                        "unknown or internal feature `{id}` cannot be disabled directly"
                    ),
                },
            ))),
        }
    }
    normalized.features.disabled = request.input.disabled_features.clone();
    let model = d
        .compile_choices()
        .map_err(|e| format!("{}: {e}", e.field))?;
    let result = choices::project(
        model.evaluate(
            &normalized.features,
            std::slice::from_ref(&normalized.choices),
        ),
        &mut out,
    );
    if input.profile.is_some() {
        out.reliance.push("profile".into());
    } else if input.default_features {
        out.reliance.push("feature-defaults".into());
    }
    if input.policy == k::ChoicePolicy::UseDeclaredDefaults {
        for group in &d.groups {
            if let m::ConfigurationGroupDecl::Choice { id, .. } = group
                && normalized
                    .choices
                    .answers
                    .get(id)
                    .is_none_or(|a| matches!(a.selection, k::Selection::Omitted))
            {
                out.reliance.push(format!("choice:{id}"));
            }
        }
    }
    if let Some(result) = &result {
        for implied in &result.implied {
            if let k::ImplicationReason::GroupDefault { group } = &implied.reason {
                out.reliance.push(format!("closure:{group}"));
            }
        }
    }
    if !request.replacement
        && let Some(previous) = request.previous.as_ref()
    {
        for reliance in previous.reliance {
            if let Some(id) = reliance.strip_prefix("closure:") {
                let group = d.groups.iter().find_map(|g| match g {
                    m::ConfigurationGroupDecl::Closure {
                        id: group, members, ..
                    } if group == id => Some(members),
                    _ => None,
                });
                let explicit=group.is_some_and(|members|members.iter().any(|id|input.features.contains(id)
                        || matches!(&normalized.features.base,k::FeatureBase::Selected(v) if v.contains(id))
                        || normalized.choices.answers.values().any(|a|matches!(&a.selection,k::Selection::Explicit(v) if v.contains(id)))));
                if !explicit && (group.is_some() || !request.reviewed) {
                    out.reliance.push(reliance.clone());
                }
            }
        }
    }
    let params = parameters::resolve(
        parameters::Parameters {
            definition: d,
            input: &input,
            previous: request.previous.as_ref(),
            history_use: request.history_use,
            replacement: request.replacement,
            fresh: request.fresh_parameters,
            ephemeral: request.ephemeral,
            facts: request.facts,
            prompt: request.prompt,
        },
        &mut out,
    );
    if !request.replacement
        && let Some(previous) = request.previous.as_ref()
    {
        for reason in previous.reliance {
            let retain = if let Some(id) = reason.strip_prefix("choice:") {
                let still_automatic = input.policy == k::ChoicePolicy::UseDeclaredDefaults
                    && normalized
                        .choices
                        .answers
                        .get(id)
                        .is_none_or(|a| matches!(a.selection, k::Selection::Omitted));
                let exists = d.groups.iter().any(
                    |g| matches!(g,m::ConfigurationGroupDecl::Choice{id:group,..} if group==id),
                );
                still_automatic && (exists || !request.reviewed)
            } else if let Some(id) = reason.strip_prefix("parameter:") {
                let explicit = input.parameters.contains_key(id)
                    || out.parameters.iter().any(|p| match p {
                        w::ConfigurationParameterResult::Public(p) => {
                            p.id == id
                                && matches!(
                                    p.origin,
                                    w::ConfigurationParameterOrigin::Explicit(_)
                                        | w::ConfigurationParameterOrigin::Profile(_)
                                )
                        }
                        w::ConfigurationParameterResult::Secret(p) => p.id == id && p.provided,
                    });
                let exists = d.parameters.iter().any(|p| p.id == id);
                !explicit && (exists || !request.reviewed)
            } else {
                false
            };
            if retain {
                out.reliance.push(reason.clone());
            }
        }
    }
    out.reliance.sort();
    out.reliance.dedup();
    if !request.replacement
        && !request.reviewed
        && let Some(previous) = request.previous.as_ref()
    {
        let changed_basis = previous.fingerprint != out.definition && !out.reliance.is_empty();
        let new_reliance = out.reliance.iter().any(|r| !previous.reliance.contains(r));
        if changed_basis || new_reliance {
            out.questions
                .push(w::ConfigurationQuestion::Review(Box::new(
                    w::ConfigurationQuestionReview {
                        previous_definition: previous.definition.clone(),
                        current_definition: definition::definition(d),
                        previous: previous.fingerprint.to_owned(),
                        current: out.definition.clone(),
                        reasons: out.reliance.clone(),
                    },
                )));
        }
    }
    if let Some(result) = result {
        let feature_result = choices::features(&result);
        out.feature_result = Some(feature_result.clone());
        let data = data(d, &result, &mut out.problems);
        if out.questions.is_empty() && out.problems.is_empty() {
            out.resolved = Some(w::ConfigurationResolved {
                features: feature_result,
                parameters: out.parameters.clone(),
                data,
            });
        }
    }
    Ok(EvaluatedConfiguration {
        record: w::ConfigurationRecord {
            definition: definition::definition(d),
            evaluation: out,
        },
        parameters: params,
    })
}
fn data(
    d: &m::ConfigurationDecl,
    result: &k::FeatureResolution,
    problems: &mut Vec<w::ConfigurationProblem>,
) -> Vec<w::ConfigurationData> {
    let mut contributors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in &d.features {
        for effect in &f.effects {
            match effect {
                m::ConfigurationEffect::ConfigValue { name } if result.selected.contains(&f.id) => {
                    contributors
                        .entry(name.clone())
                        .or_default()
                        .push(f.id.clone());
                }
                m::ConfigurationEffect::ConfigValue { .. } => {}
                _ => problems.push(w::ConfigurationProblem::Scope(Box::new(
                    w::ConfigurationProblemScope {
                        message: format!(
                            "installer feature `{}` has an effect unsupported in installer scope",
                            f.id
                        ),
                    },
                ))),
            }
        }
    }
    contributors
        .into_iter()
        .filter_map(|(name, mut features)| {
            features.sort();
            features.dedup();
            d.config_values.get(&name).map(|v| w::ConfigurationData {
                name,
                value: vocabulary::value(v),
                features,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
