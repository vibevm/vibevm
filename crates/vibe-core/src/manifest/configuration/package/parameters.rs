//! Public parameter admission across consumers. Representability only: an
//! effectful owner supplies its own fresh host observation before effects.
use super::{PackageConfigurationError, Scope, reliance::Automatic};
use crate::manifest::configuration::{ConfigurationIntentDecl, NormalizedConfigurationInput};
use crate::manifest::{ParameterDecl, ParameterType, ParameterValue};
use std::collections::BTreeMap;

/// One consumer's original declaration beside its normalized form.
#[derive(Clone, Copy)]
pub(super) struct Consumer<'a> {
    pub(super) key: &'a str,
    pub(super) input: &'a ConfigurationIntentDecl,
    pub(super) normalized: &'a NormalizedConfigurationInput,
}

/// Values already agreed by earlier consumers; a later disagreement refuses.
#[derive(Default)]
pub(super) struct PublicParameters {
    booleans: BTreeMap<String, bool>,
    scalars: BTreeMap<String, ParameterValue>,
}

impl PublicParameters {
    pub(super) fn admit(
        &mut self,
        scope: &Scope<'_>,
        consumer: Consumer<'_>,
        automatic: &Automatic,
    ) -> Result<(), PackageConfigurationError> {
        for parameter in &scope.definition.parameters {
            if matches!(
                parameter.kind,
                ParameterType::Integer
                    | ParameterType::String
                    | ParameterType::Choice
                    | ParameterType::Path
            ) {
                self.scalar(scope, consumer, parameter, automatic)?;
            } else {
                self.boolean(scope, consumer, parameter)?;
            }
        }
        Ok(())
    }

    fn scalar(
        &mut self,
        scope: &Scope<'_>,
        consumer: Consumer<'_>,
        parameter: &ParameterDecl,
        automatic: &Automatic,
    ) -> Result<(), PackageConfigurationError> {
        let Scope {
            kind,
            definition,
            consumers,
            forwarded,
            ..
        } = *scope;
        // Representability only. Effectful owners must supply a
        // fresh host observation; lock/show never claim readiness.
        if parameter.kind == ParameterType::Path
            && parameter.must_exist
            && (consumers.len() != 1
                || !consumers.contains_key("./vibe.toml")
                || !forwarded.is_empty()
                || !definition.groups.is_empty()
                || !definition.features.is_empty())
        {
            return Err(scope.unsupported(
                "public path must_exist=true requires a separately specified planning-host base and check for this consumer",
            ));
        }
        let integer_profile = parameter.kind == ParameterType::Integer
            && parameter.min.is_some()
            && parameter.max.is_some()
            && definition.profiles.len() == 1
            && definition.profiles[0].parameters.len() == 1
            && definition.profiles[0]
                .parameters
                .contains_key(&parameter.id)
            && definition.profiles[0].selections.is_empty()
            && definition.profiles[0].features.is_empty()
            && forwarded.is_empty();
        if kind != crate::PackageKind::Flow
            || parameter.secret
            || !parameter.required
            || parameter.default.is_some()
            || (!definition.profiles.is_empty() && !integer_profile)
            || definition.parameters.len() != 1
            || !automatic.choices.is_empty()
            || !automatic.choice_sets.is_empty()
        {
            return Err(scope.unsupported(match parameter.kind {
                ParameterType::Integer => {
                    "only one explicit required public integer without defaults, profiles or automatic choices is connected"
                }
                ParameterType::Path => {
                    "only one explicit required public lexical path without defaults, profiles or automatic choices is connected"
                }
                _ => {
                    "only one explicit required public string or choice without defaults, profiles or automatic choices is connected"
                }
            }));
        }
        let Some(value) = consumer.normalized.parameters.get(&parameter.id) else {
            return Err(missing(scope, consumer, parameter));
        };
        if self
            .scalars
            .get(&parameter.id)
            .is_some_and(|previous| previous != value)
        {
            return Err(PackageConfigurationError::Incompatible {
                package: scope.package.to_owned(),
                consumers: consumers.keys().cloned().collect(),
                reason: format!(
                    "conflicting explicit public {} parameter `{}`",
                    match parameter.kind {
                        ParameterType::Integer => "integer",
                        ParameterType::String => "string",
                        ParameterType::Path => "path",
                        _ => "choice",
                    },
                    parameter.id
                ),
            });
        }
        self.scalars.insert(parameter.id.clone(), value.clone());
        Ok(())
    }

    fn boolean(
        &mut self,
        scope: &Scope<'_>,
        consumer: Consumer<'_>,
        parameter: &ParameterDecl,
    ) -> Result<(), PackageConfigurationError> {
        let Scope {
            kind,
            definition,
            consumers,
            forwarded,
            ..
        } = *scope;
        let Consumer {
            input, normalized, ..
        } = consumer;
        let optional_bool = !parameter.required
            && !parameter.secret
            && parameter.kind == ParameterType::Bool
            && parameter.default.is_none()
            && kind == crate::PackageKind::Flow
            && definition.parameters.len() == 1
            && definition.profiles.is_empty()
            && definition.groups.is_empty()
            && definition
                .features
                .iter()
                .all(|feature| feature.guards.is_empty())
            && forwarded.is_empty()
            && consumers.values().all(|input| input.profile.is_none());
        if optional_bool && !normalized.parameters.contains_key(&parameter.id) {
            return Ok(()); // Absence does not vote, and is never false.
        }
        if parameter.secret
            || parameter.kind != ParameterType::Bool
            || (!parameter.required && !optional_bool)
            || (parameter.default.is_some() && definition.parameters.len() != 1)
            || (input.profile.is_some()
                && (definition.parameters.len() != 1
                    || parameter.default.is_some()
                    || definition
                        .profiles
                        .iter()
                        .find(|profile| Some(&profile.id) == input.profile.as_ref())
                        .is_none_or(|profile| {
                            profile.parameters.keys().any(|id| id != &parameter.id)
                        })))
        {
            return Err(scope.unsupported(
                "only explicitly supplied required public bool parameters without defaults are connected",
            ));
        }
        let Some(ParameterValue::Bool(value)) = normalized
            .parameters
            .get(&parameter.id)
            .or(parameter.default.as_ref())
        else {
            return Err(missing(scope, consumer, parameter));
        };
        if self
            .booleans
            .get(&parameter.id)
            .is_some_and(|previous| previous != value)
        {
            return Err(PackageConfigurationError::Incompatible {
                package: scope.package.to_owned(),
                consumers: consumers.keys().cloned().collect(),
                reason: format!(
                    "conflicting {}public boolean parameter `{}`",
                    if parameter.default.is_none()
                        && consumers
                            .values()
                            .all(|input| input.parameters.contains_key(&parameter.id))
                    {
                        "explicit "
                    } else {
                        "effective "
                    },
                    parameter.id
                ),
            });
        }
        self.booleans.insert(parameter.id.clone(), *value);
        Ok(())
    }
}

fn missing(
    scope: &Scope<'_>,
    consumer: Consumer<'_>,
    parameter: &ParameterDecl,
) -> PackageConfigurationError {
    PackageConfigurationError::Incompatible {
        package: scope.package.to_owned(),
        consumers: vec![consumer.key.to_owned()],
        reason: format!(
            "consumer `{}` is missing required public parameter `{}`",
            consumer.key, parameter.id
        ),
    }
}
