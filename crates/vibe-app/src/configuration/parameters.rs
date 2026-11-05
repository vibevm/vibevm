//! Host validation and source-aware public realizations; secret values stay ephemeral.
use super::vocabulary::{parameter_type, read_value, value};
use super::{ConfigurationHistory, HistoricalParameter};
use crate::{HostFacts, ParamValue, ParameterPrompt};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::manifest::{
    ConfigurationDecl, ConfigurationIntentDecl, ParameterType, ParameterValue,
};
use vibe_wire::generated::shared as w;

pub(super) struct Parameters<'a> {
    pub definition: &'a ConfigurationDecl,
    pub input: &'a ConfigurationIntentDecl,
    pub previous: Option<&'a ConfigurationHistory<'a>>,
    pub history_use: super::HistoryUse<'a>,
    pub replacement: bool,
    pub fresh: &'a BTreeSet<String>,
    pub ephemeral: &'a [(String, String)],
    pub facts: HostFacts<'a>,
    pub prompt: Option<ParameterPrompt<'a>>,
}
pub(super) fn resolve(
    args: Parameters<'_>,
    out: &mut w::ConfigurationEvaluation,
) -> Vec<ParamValue> {
    let mut executable = Vec::new();
    let profile = args
        .input
        .profile
        .as_ref()
        .and_then(|id| args.definition.profiles.iter().find(|p| &p.id == id));
    let previous: BTreeMap<&str, HistoricalParameter<'_>> = args
        .previous
        .into_iter()
        .flat_map(|r| r.parameters.iter().copied())
        .map(|p| (p.id(), p))
        .collect();
    for p in &args.definition.parameters {
        if p.secret {
            let supplied = args
                .ephemeral
                .iter()
                .rev()
                .find(|(id, _)| id == &p.id)
                .map(|(_, v)| v.clone());
            let was_provided = matches!(
                previous.get(p.id.as_str()),
                Some(HistoricalParameter::Secret { provided: true, .. })
            );
            let retained_requirement = matches!(args.history_use,super::HistoryUse::Transfer{required_secrets,..} if required_secrets.contains(&p.id));
            let needs_resupply = retained_requirement
                || was_provided
                    && (!args.replacement
                        || matches!(args.history_use, super::HistoryUse::Transfer { .. }));
            let declared_default = (!needs_resupply)
                .then(|| p.default.as_ref().map(ToString::to_string))
                .flatten();
            let supplied = match (supplied, args.prompt) {
                (Some(v), _) => Some(v),
                (None, Some(ask)) => match ask(p, declared_default.as_deref()) {
                    Ok(v) => v,
                    Err(_) => {
                        problem(out, &p.id, "the secret prompt failed");
                        None
                    }
                },
                _ => None,
            };
            let explicit = supplied.is_some();
            let supplied = supplied.or(declared_default);
            let origin = if explicit {
                w::ConfigurationParameterOrigin::Explicit(Box::new(
                    w::ConfigurationParameterOriginExplicit {},
                ))
            } else if !needs_resupply && p.default.is_some() {
                w::ConfigurationParameterOrigin::Default(Box::new(
                    w::ConfigurationParameterOriginDefault {},
                ))
            } else {
                w::ConfigurationParameterOrigin::Absent(Box::new(
                    w::ConfigurationParameterOriginAbsent {},
                ))
            };
            if !explicit && !needs_resupply {
                out.reliance.push(format!("parameter:{}", p.id));
            }
            let mut provided = false;
            if let Some(raw) = supplied {
                match canonical(p, &raw, args.facts, None) {
                    Ok(v) => {
                        provided = explicit;
                        executable.push(ParamValue {
                            id: p.id.clone(),
                            value: v,
                            secret: true,
                        });
                    }
                    Err(_) => problem(
                        out,
                        &p.id,
                        "the supplied secret does not satisfy its declared rules",
                    ),
                }
            } else if needs_resupply {
                out.questions
                    .push(w::ConfigurationQuestion::SecretResupply(Box::new(
                        w::ConfigurationQuestionSecretResupply { id: p.id.clone() },
                    )));
            } else if p.required {
                out.questions
                    .push(w::ConfigurationQuestion::Parameter(Box::new(
                        w::ConfigurationQuestionParameter {
                            id: p.id.clone(),
                            secret: true,
                        },
                    )));
            }
            out.parameters
                .push(w::ConfigurationParameterResult::Secret(Box::new(
                    w::ConfigurationParameterResultSecret {
                        id: p.id.clone(),
                        origin,
                        provided,
                        resupply: needs_resupply && !provided,
                    },
                )));
            continue;
        }
        let (origin, source) = public_source(p, args.input, profile);
        if !matches!(origin, w::ConfigurationParameterOrigin::Explicit(_)) {
            out.reliance.push(format!("parameter:{}", p.id));
        }
        let wire_source = source.as_ref().map(value);
        let old_path =
            if p.kind == ParameterType::Path && !args.replacement && !args.fresh.contains(&p.id) {
                match previous.get(p.id.as_str()) {
                    Some(HistoricalParameter::Public {
                        kind,
                        origin: old_origin,
                        source: old_source,
                        reusable_local_realization,
                        ..
                    }) if **kind == w::ConfigurationParameterType::Path
                        && **old_origin == origin
                        && *old_source == wire_source.as_ref() =>
                    {
                        reusable_local_realization
                            .and_then(|v| read_value(v).ok())
                            .and_then(|v| match v {
                                ParameterValue::Text(s) => Some(s),
                                _ => None,
                            })
                    }
                    _ => None,
                }
            } else {
                None
            };
        let binding = match args.history_use {
            super::HistoryUse::Local => None,
            super::HistoryUse::Transfer { path_bindings, .. } => path_bindings.get(&p.id),
        };
        let transferring_path = p.kind == ParameterType::Path
            && matches!(args.history_use, super::HistoryUse::Transfer { .. });
        let realized = if let Some(source) = source.as_ref() {
            if transferring_path && binding.is_none() {
                out.questions
                    .push(w::ConfigurationQuestion::Parameter(Box::new(
                        w::ConfigurationQuestionParameter {
                            id: p.id.clone(),
                            secret: false,
                        },
                    )));
                None
            } else {
                let admission = binding.map(|value| p.canonical(value)).transpose();
                match admission
                    .and_then(|bound| {
                        canonical(
                            p,
                            &source.to_string(),
                            args.facts,
                            if transferring_path {
                                bound.as_deref()
                            } else {
                                old_path.as_deref()
                            },
                        )
                    })
                    .and_then(|raw| typed(p.kind, &raw).map(|value| (raw, value)))
                {
                    Ok((raw, typed)) => {
                        executable.push(ParamValue {
                            id: p.id.clone(),
                            value: raw,
                            secret: false,
                        });
                        Some(value(&typed))
                    }
                    Err(message) => {
                        problem(out, &p.id, &message);
                        None
                    }
                }
            }
        } else {
            if binding.is_some() {
                problem(
                    out,
                    &p.id,
                    "a target path binding cannot supply an absent lexical source",
                );
            }
            if p.required {
                out.questions
                    .push(w::ConfigurationQuestion::Parameter(Box::new(
                        w::ConfigurationQuestionParameter {
                            id: p.id.clone(),
                            secret: false,
                        },
                    )));
            }
            None
        };
        out.parameters
            .push(w::ConfigurationParameterResult::Public(Box::new(
                w::ConfigurationParameterResultPublic {
                    id: p.id.clone(),
                    origin,
                    source: wire_source,
                    type_: parameter_type(&p.kind),
                    value: realized,
                },
            )));
    }
    executable
}
/// The one precedence/origin owner for public lexical parameter sources.
pub(super) fn public_source(
    p: &vibe_core::manifest::ParameterDecl,
    input: &ConfigurationIntentDecl,
    profile: Option<&vibe_core::manifest::ConfigurationProfileDecl>,
) -> (w::ConfigurationParameterOrigin, Option<ParameterValue>) {
    if let Some(v) = input.parameters.get(&p.id) {
        (
            w::ConfigurationParameterOrigin::Explicit(Box::new(
                w::ConfigurationParameterOriginExplicit {},
            )),
            Some(v.clone()),
        )
    } else if let Some((profile, v)) =
        profile.and_then(|profile| profile.parameters.get(&p.id).map(|v| (profile, v)))
    {
        (
            w::ConfigurationParameterOrigin::Profile(Box::new(
                w::ConfigurationParameterOriginProfile {
                    profile: profile.id.clone(),
                },
            )),
            Some(v.clone()),
        )
    } else if let Some(v) = &p.default {
        (
            w::ConfigurationParameterOrigin::Default(Box::new(
                w::ConfigurationParameterOriginDefault {},
            )),
            Some(v.clone()),
        )
    } else {
        (
            w::ConfigurationParameterOrigin::Absent(Box::new(
                w::ConfigurationParameterOriginAbsent {},
            )),
            None,
        )
    }
}

fn problem(out: &mut w::ConfigurationEvaluation, id: &str, message: &str) {
    out.problems
        .push(w::ConfigurationProblem::Parameter(Box::new(
            w::ConfigurationProblemParameter {
                id: id.into(),
                message: message.into(),
            },
        )));
}
fn canonical(
    p: &vibe_core::manifest::ParameterDecl,
    raw: &str,
    facts: HostFacts<'_>,
    previous_path: Option<&str>,
) -> Result<String, String> {
    let canonical = p.canonical(raw)?;
    if p.kind != ParameterType::Path {
        return Ok(canonical);
    }
    let path = previous_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| (facts.resolve_path)(&canonical));
    if !path.is_absolute() {
        return Err("host path resolution must return an absolute path".into());
    }
    if p.must_exist && !path.exists() {
        return Err("the resolved path does not exist".into());
    }
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "the resolved path cannot be represented losslessly as UTF-8".into())
}
pub(super) fn typed(kind: ParameterType, canonical: &str) -> Result<ParameterValue, String> {
    Ok(match kind {
        ParameterType::Bool => match canonical {
            "true" => ParameterValue::Bool(true),
            "false" => ParameterValue::Bool(false),
            _ => return Err("expected canonical boolean".into()),
        },
        ParameterType::Integer => ParameterValue::Integer(
            canonical
                .parse()
                .map_err(|_| "expected canonical signed integer".to_owned())?,
        ),
        ParameterType::Path | ParameterType::String | ParameterType::Choice => {
            ParameterValue::Text(canonical.into())
        }
    })
}
