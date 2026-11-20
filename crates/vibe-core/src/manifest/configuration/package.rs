//! Replayable package selections for real incoming workspace declarations.
//! The initial operation adapter accepts explicit, non-automatic inputs. Domain
//! effects beyond dependency activation remain explicit unsupported outcomes.
use super::{ConfigurationDecl, ConfigurationEffect, ConfigurationIntentDecl};
use crate::configuration::{ChoiceOutcome, FeatureBase, FeatureInput, ImplicationReason};
use crate::manifest::{FieldPath, Manifest, Requires};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Original declarations and their separately evaluated dependency selection.
/// This belongs to the existing project lock, not a global configuration store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE")]
pub struct PackageConfiguration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_basis: Option<AutomaticBasis>,
    pub consumers: BTreeMap<String, ConfigurationIntentDecl>,
    /// Candidate-owned incoming requests, never synthetic workspace consumers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forwarded_inputs: Vec<ForwardedInput>,
    pub definition: ConfigurationDecl,
    pub declarations: Requires,
    pub features: Vec<String>,
    pub active_dependencies: Vec<String>,
    /// Empty causes mean an ordinary mandatory declaration; selected features
    /// are recorded even when they also name that mandatory declaration.
    pub dependency_causes: BTreeMap<String, Vec<String>>,
}

/// One exact selected source-owned declaration requesting a target feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForwardedInput {
    pub source_group: crate::Group,
    pub source_name: crate::PackageName,
    pub source_version: semver::Version,
    pub source_content_hash: crate::ContentHash,
    pub source_feature: String,
    pub declaration_index: usize,
    pub target: crate::PackageRef,
    pub target_feature: String,
    pub strength: super::ForwardStrength,
}

/// Historical automatic selection, not a consent receipt or authored answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomaticBasis {
    pub definition_fingerprint: String,
    pub consumer_choices: BTreeMap<String, BTreeMap<String, String>>,
}

/// Configuration admission has not produced an executable package selection.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#EVALUATION")]
pub enum PackageConfigurationError {
    #[error(
        "package `{package}` configuration is not supported by this operation: {reason} (spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS; fix: use an operation supporting the named configuration input/effect)"
    )]
    Unsupported { package: String, reason: String },
    #[error(
        "package `{package}` configuration is invalid: {reason} (spec://org.vibevm.core/vibevm/common/PROP-069#DEFINITION; fix: correct the named declaration)"
    )]
    Invalid { package: String, reason: String },
    #[error(
        "package `{package}` configuration from {consumers:?} is unresolved: {reason} (spec://org.vibevm.core/vibevm/common/PROP-069#CONSUMER-MERGE; fix: reconcile the original requires inputs before installation)"
    )]
    Incompatible {
        package: String,
        consumers: Vec<String>,
        reason: String,
    },
    #[error(
        "package `{package}` requires configuration answers: {questions:?} (spec://org.vibevm.core/vibevm/common/PROP-069#DEFAULT-CONSENT; fix: answer the named groups on the actual requires declarations)"
    )]
    NeedsSelection {
        package: String,
        questions: Vec<crate::configuration::ChoiceQuestion>,
    },
    #[error(
        "package `{package}` retained configuration disagrees with its candidate or selected graph (spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION; fix: restore the pinned source or explicitly reconfigure before effects)"
    )]
    ReplayMismatch { package: String },
}

impl PackageConfiguration {
    /// Evaluate actual incoming declarations through the shared core once.
    /// No filesystem/source lookup, implicit profile/default approval or effects.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#OWNER-LOCAL-ACTIVATION"
    )]
    pub fn evaluate(
        manifest: &Manifest,
        consumers: &BTreeMap<String, ConfigurationIntentDecl>,
    ) -> Result<Self, PackageConfigurationError> {
        Self::evaluate_with_forwarded(manifest, consumers, &[])
    }

    /// Evaluate separately typed candidate-owned requests without inventing user input.
    pub fn evaluate_with_forwarded(
        manifest: &Manifest,
        consumers: &BTreeMap<String, ConfigurationIntentDecl>,
        forwarded: &[ForwardedInput],
    ) -> Result<Self, PackageConfigurationError> {
        let metadata =
            manifest
                .require_package()
                .map_err(|e| PackageConfigurationError::Invalid {
                    package: "<missing package>".into(),
                    reason: e.to_string(),
                })?;
        let package = format!("{}/{}", metadata.group, metadata.name);
        let definition = manifest.configuration.as_ref().ok_or_else(|| {
            PackageConfigurationError::Unsupported {
                package: package.clone(),
                reason: "the selected package has no common definition".into(),
            }
        })?;
        if !manifest.requires_any.is_empty()
            || !manifest.requires.capabilities.is_empty()
            || !manifest.conditional_deps.is_empty()
            || !crate::manifest::application_requirements(&manifest.installer_options).is_empty()
        {
            return Err(PackageConfigurationError::Unsupported { package, reason: "configured owner OR/capability/conditional/installer requirements need their selected-edge projection".into() });
        }
        Self::evaluate_parts(
            package,
            metadata.kind,
            definition,
            &manifest.requires,
            consumers,
            forwarded,
        )
    }

    pub(crate) fn evaluate_parts(
        package: String,
        kind: crate::PackageKind,
        definition: &ConfigurationDecl,
        requires: &Requires,
        consumers: &BTreeMap<String, ConfigurationIntentDecl>,
        forwarded: &[ForwardedInput],
    ) -> Result<Self, PackageConfigurationError> {
        let unsupported = |reason: &str| PackageConfigurationError::Unsupported {
            package: package.clone(),
            reason: reason.into(),
        };
        let invalid = |reason: String| PackageConfigurationError::Invalid {
            package: package.clone(),
            reason,
        };
        definition
            .validate_scope(
                requires,
                kind,
                false,
                &FieldPath::root().table("configuration"),
            )
            .map_err(|e| invalid(format!("{}: {}", e.field, e.message)))?;
        requires
            .validate_configuration_metadata()
            .map_err(invalid)?;
        if consumers.is_empty() && forwarded.is_empty() {
            return Err(unsupported("no actual incoming configuration declaration"));
        }
        if !forwarded.is_empty()
            && (!definition.parameters.is_empty() || !definition.profiles.is_empty())
        {
            return Err(unsupported(
                "profile or parameter forwarding is not connected",
            ));
        }
        let mut public_parameters: BTreeMap<String, bool> = BTreeMap::new();
        let mut automatic_choices = BTreeMap::new();
        let mut ordinary = BTreeSet::new();
        let mut choices = Vec::new();
        for (consumer, input) in consumers {
            input
                .validate_shape_at(&FieldPath::root().table("configuration"))
                .map_err(|e| invalid(format!("{consumer}: {}: {}", e.field, e.message)))?;
            let normalized = definition.normalize_input(input, consumer).map_err(|e| {
                PackageConfigurationError::Incompatible {
                    package: package.clone(),
                    consumers: vec![consumer.clone()],
                    reason: format!("{}: {}", e.field, e.message),
                }
            })?;
            if input.default_features && input.profile.is_none() {
                return Err(unsupported(
                    "automatic/default/profile basis is not connected on this package operation; supply explicit no-default input",
                ));
            }
            let reasons = super::basis::consumer_reliance(
                definition,
                super::basis::ConsumerInput {
                    original: input,
                    normalized: &normalized,
                },
                &BTreeSet::new(),
                super::basis::Authority::Reuse,
            );
            let mut selected_defaults = BTreeMap::new();
            for reason in reasons {
                if reason == super::basis::ConsumerReliance::Profile {
                    // Profile reliance is retained by original profile input and
                    // historical definition, never a fabricated default choice.
                    continue;
                }
                let super::basis::ConsumerReliance::Choice(group_id) = reason else {
                    return Err(unsupported(
                        "only omitted exactly-one choice defaults are connected",
                    ));
                };
                let Some(super::ConfigurationGroupDecl::Choice { cardinality, defaults: Some(defaults), members, guards, .. }) = definition.groups.iter().find(|group| matches!(group, super::ConfigurationGroupDecl::Choice { id, .. } if id == &group_id)) else {
                    return Err(unsupported("automatic choice needs a declared exactly-one default"));
                };
                if *cardinality != crate::configuration::Cardinality::ExactlyOne
                    || defaults.len() != 1
                    || !guards.is_empty()
                    || !members.contains(&defaults[0])
                {
                    return Err(unsupported(
                        "only unguarded exactly-one single-member defaults are connected",
                    ));
                }
                selected_defaults.insert(group_id, defaults[0].clone());
            }
            if !selected_defaults.is_empty() {
                automatic_choices.insert(consumer.clone(), selected_defaults);
            }
            for parameter in &definition.parameters {
                if parameter.secret
                    || parameter.kind != crate::manifest::ParameterType::Bool
                    || !parameter.required
                    || parameter.default.is_some()
                    || input.profile.is_some()
                {
                    return Err(unsupported(
                        "only explicitly supplied required public bool parameters without defaults are connected",
                    ));
                }
                let Some(crate::manifest::ParameterValue::Bool(value)) =
                    input.parameters.get(&parameter.id)
                else {
                    return Err(PackageConfigurationError::Incompatible {
                        package: package.clone(),
                        consumers: vec![consumer.clone()],
                        reason: format!(
                            "consumer `{consumer}` is missing required public parameter `{}`",
                            parameter.id
                        ),
                    });
                };
                if public_parameters
                    .get(&parameter.id)
                    .is_some_and(|previous| previous != value)
                {
                    return Err(PackageConfigurationError::Incompatible {
                        package: package.clone(),
                        consumers: consumers.keys().cloned().collect(),
                        reason: format!(
                            "conflicting explicit public boolean parameter `{}`",
                            parameter.id
                        ),
                    });
                }
                public_parameters.insert(parameter.id.clone(), *value);
            }
            if let FeatureBase::Selected(base) = normalized.features.base {
                ordinary.extend(base);
            }
            ordinary.extend(normalized.features.additions);
            choices.push(normalized.choices);
        }
        let mut forwarded_features = BTreeSet::new();
        for request in forwarded {
            if request.strength != super::ForwardStrength::Strong
                || !request.target.version.is_exact_pin()
                || request.target.qualified_name() != package
                || !forwarded_features.insert(request.target_feature.clone())
            {
                return Err(invalid(
                    "invalid or duplicate candidate-owned forwarded request".into(),
                ));
            }
            let original = ConfigurationIntentDecl {
                policy: crate::configuration::ChoicePolicy::RequireSelection,
                profile: None,
                features: vec![request.target_feature.clone()],
                default_features: false,
                selections: BTreeMap::new(),
                parameters: BTreeMap::new(),
            };
            // This is a typed candidate-owned input, never inserted in consumers.
            let normalized = definition
                .normalize_input(&original, "candidate-owned forwarding")
                .map_err(|error| invalid(error.to_string()))?;
            if let FeatureBase::Selected(base) = normalized.features.base {
                ordinary.extend(base);
            }
            ordinary.extend(normalized.features.additions);
            choices.push(normalized.choices);
        }
        let model = definition
            .compile_choices()
            .map_err(|e| invalid(format!("{}: {}", e.field, e.message)))?;
        // Across consumers there is no last-writer feature replacement. All
        // direct seeds are simultaneous base requirements before core closure.
        let evaluation = model.evaluate(
            &FeatureInput {
                base: FeatureBase::Selected(ordinary.into_iter().collect()),
                additions: vec![],
                disabled: vec![],
            },
            &choices,
        );
        let ChoiceOutcome::Resolved { features } = &evaluation.outcome else {
            if evaluation.conflicts.is_empty()
                && evaluation.invalid_answers.is_empty()
                && !evaluation.questions.is_empty()
            {
                return Err(PackageConfigurationError::NeedsSelection {
                    package,
                    questions: evaluation.questions,
                });
            }
            return Err(PackageConfigurationError::Incompatible {
                package,
                consumers: consumers.keys().cloned().collect(),
                reason: format!(
                    "questions={:?}; conflicts={:?}; invalid answers={:?}",
                    evaluation.questions, evaluation.conflicts, evaluation.invalid_answers
                ),
            });
        };
        if features
            .implied
            .iter()
            .any(|i| matches!(i.reason, ImplicationReason::GroupDefault { .. }))
        {
            return Err(unsupported(
                "closure fallback basis is not connected on this package operation",
            ));
        }
        let mut active = requires
            .iter_pkgrefs()
            .filter_map(|(group, name)| group.map(|g| format!("{g}/{name}")))
            .filter(|key| !requires.optional.get(key).copied().unwrap_or(false))
            .collect::<BTreeSet<_>>();
        let mut causes = active
            .iter()
            .map(|key| (key.clone(), Vec::new()))
            .collect::<BTreeMap<_, _>>();
        for feature in &definition.features {
            if !features.selected.contains(&feature.id) {
                continue;
            }
            for effect in &feature.effects {
                match effect {
                    ConfigurationEffect::Dependency { target } => {
                        let (_, group, name) = crate::manifest::package::parse_pkgref_key(target)
                            .map_err(|e| invalid(e.to_string()))?;
                        let key = format!("{group}/{name}");
                        active.insert(key.clone());
                        let entries = causes.entry(key).or_default();
                        if !entries.contains(&feature.id) {
                            entries.push(feature.id.clone());
                        }
                    }
                    ConfigurationEffect::Forward {
                        target, strength, ..
                    } => {
                        let declaration = requires
                            .packages
                            .iter()
                            .find(|edge| edge.qualified_name() == *target);
                        if *strength != super::ForwardStrength::Strong
                            || !forwarded.is_empty()
                            || declaration.is_none_or(|edge| !edge.version.is_exact_pin())
                        {
                            return Err(unsupported(
                                "selected cross-package forwarding requires candidate-conditioned incoming consumers; only exact compulsory strong edges are connected",
                            ));
                        }
                        let key = declaration
                            .ok_or_else(|| {
                                invalid("forwarded dependency declaration is missing".into())
                            })?
                            .qualified_name();
                        active.insert(key.clone());
                        let entries = causes.entry(key).or_default();
                        if !entries.contains(&feature.id) {
                            entries.push(feature.id.clone());
                        }
                    }
                    ConfigurationEffect::Subskill { path } => {
                        if !path.starts_with("subskills/") || path.rsplit_once('/').is_none() {
                            return Err(unsupported(
                                "selected subskill loading is not connected for bare files; declaration-backed subskills/ member required",
                            ));
                        }
                        // This is selection only. Source admission supplies the
                        // independent declaration/member witness before apply.
                    }
                    ConfigurationEffect::ConfigValue { name } => {
                        if !definition.config_values.contains_key(name) {
                            return Err(invalid("selected config value is not declared".into()));
                        }
                    }
                }
            }
        }
        let registry: BTreeSet<_> = requires
            .packages
            .iter()
            .map(|p| p.qualified_name())
            .collect();
        if active.iter().any(|key| !registry.contains(key)) {
            return Err(unsupported(
                "an active declaration uses a non-registry source",
            ));
        }
        if requires
            .configurations
            .keys()
            .any(|key| active.contains(key))
        {
            return Err(unsupported(
                "active transitive configuration requires candidate-conditioned incoming consumers",
            ));
        }
        let automatic_basis = if automatic_choices.is_empty() {
            None
        } else {
            Some(AutomaticBasis {
                definition_fingerprint: definition
                    .semantic_fingerprint()
                    .map_err(|e| invalid(e.to_string()))?,
                consumer_choices: automatic_choices,
            })
        };
        Ok(Self {
            automatic_basis,
            consumers: consumers.clone(),
            forwarded_inputs: forwarded.to_vec(),
            definition: definition.clone(),
            declarations: requires.clone(),
            features: features.selected.clone(),
            active_dependencies: active.into_iter().collect(),
            dependency_causes: causes,
        })
    }

    /// Derive selected public authored data from an admitted historical record.
    /// No parameter, path, environment or secret input is expanded.
    pub fn selected_data(
        &self,
        kind: crate::PackageKind,
    ) -> Result<BTreeMap<String, crate::manifest::ParameterValue>, PackageConfigurationError> {
        let evaluated = Self::evaluate_parts(
            self.forwarded_inputs
                .first()
                .map(|input| input.target.qualified_name())
                .unwrap_or_else(|| "retained package".into()),
            kind,
            &self.definition,
            &self.declarations,
            &self.consumers,
            &self.forwarded_inputs,
        )?;
        if &evaluated != self {
            return Err(PackageConfigurationError::ReplayMismatch {
                package: "retained package".into(),
            });
        }
        let mut values = BTreeMap::new();
        for feature in &self.definition.features {
            if !self.features.contains(&feature.id) {
                continue;
            }
            for effect in &feature.effects {
                if let ConfigurationEffect::ConfigValue { name } = effect {
                    let value = self.definition.config_values.get(name).ok_or_else(|| {
                        PackageConfigurationError::Invalid {
                            package: "retained package".into(),
                            reason: "unknown config data key".into(),
                        }
                    })?;
                    values.insert(name.clone(), value.clone());
                }
            }
        }
        Ok(values)
    }

    /// Selected source members, not an admission of delivery or cache bytes.
    pub fn selected_subskill_paths(&self) -> BTreeSet<String> {
        self.definition
            .features
            .iter()
            .filter(|feature| self.features.contains(&feature.id))
            .flat_map(|feature| &feature.effects)
            .filter_map(|effect| match effect {
                ConfigurationEffect::Subskill { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// Validated public bool projection, separate from selected effect data.
    pub fn selected_parameters(
        &self,
        kind: crate::PackageKind,
    ) -> Result<BTreeMap<String, crate::manifest::ParameterValue>, PackageConfigurationError> {
        self.selected_data(kind)?;
        let mut values = BTreeMap::new();
        for parameter in &self.definition.parameters {
            let value = self
                .consumers
                .values()
                .next()
                .and_then(|input| input.parameters.get(&parameter.id))
                .ok_or_else(|| PackageConfigurationError::ReplayMismatch {
                    package: "retained package".into(),
                })?;
            values.insert(parameter.id.clone(), value.clone());
        }
        Ok(values)
    }

    /// Validate durable derived fields against the retained original request and
    /// the actual pinned manifest; never infer a request from effective features.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
    pub fn validate(&self, manifest: &Manifest) -> Result<(), PackageConfigurationError> {
        let actual =
            Self::evaluate_with_forwarded(manifest, &self.consumers, &self.forwarded_inputs)?;
        if &actual != self {
            let package = manifest
                .package
                .as_ref()
                .map(|p| format!("{}/{}", p.group, p.name))
                .unwrap_or_default();
            return Err(PackageConfigurationError::ReplayMismatch { package });
        }
        Ok(())
    }

    /// Internal resolver/visibility dependency view; source bytes and the record's
    /// full declarations remain intact. Only active declaration metadata survives.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#DORMANT-SOURCE-BOUNDARY"
    )]
    pub fn project(&self, manifest: &Manifest) -> Result<Manifest, PackageConfigurationError> {
        self.validate(manifest)?;
        let active: BTreeSet<_> = self.active_dependencies.iter().cloned().collect();
        let mut view = manifest.clone();
        view.requires
            .packages
            .retain(|p| active.contains(&p.qualified_name()));
        view.requires
            .git_packages
            .retain(|p| active.contains(&format!("{}/{}", p.group, p.name)));
        view.requires
            .path_packages
            .retain(|p| active.contains(&format!("{}/{}", p.group, p.name)));
        view.requires
            .var_packages
            .retain(|p| active.contains(&format!("{}/{}", p.group, p.name)));
        view.requires.links.retain(|key, _| active.contains(key));
        view.requires.accesses.retain(|key, _| active.contains(key));
        view.requires
            .friend_flags
            .retain(|key, _| active.contains(key));
        view.requires.excludes.retain(|key, _| active.contains(key));
        view.requires.optional.clear();
        view.requires.configurations.clear();
        view.configuration = None;
        Ok(view)
    }
}
