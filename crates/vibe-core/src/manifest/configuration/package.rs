//! Replayable package selections for real incoming workspace declarations.
//! The initial operation adapter accepts explicit, non-automatic inputs. Domain
//! effects beyond dependency activation remain explicit unsupported outcomes.
use super::{ConfigurationDecl, ConfigurationEffect, ConfigurationIntentDecl};
use crate::configuration::{
    ChoiceOutcome, ChoicePolicy, FeatureBase, FeatureInput, ImplicationReason,
};
use crate::manifest::{FieldPath, Manifest, Requires};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Original declarations and their separately evaluated dependency selection.
/// This belongs to the existing project lock, not a global configuration store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE")]
pub struct PackageConfiguration {
    pub consumers: BTreeMap<String, ConfigurationIntentDecl>,
    pub definition: ConfigurationDecl,
    pub declarations: Requires,
    pub features: Vec<String>,
    pub active_dependencies: Vec<String>,
    /// Empty causes mean an ordinary mandatory declaration; selected features
    /// are recorded even when they also name that mandatory declaration.
    pub dependency_causes: BTreeMap<String, Vec<String>>,
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
        )
    }

    pub(crate) fn evaluate_parts(
        package: String,
        kind: crate::PackageKind,
        definition: &ConfigurationDecl,
        requires: &Requires,
        consumers: &BTreeMap<String, ConfigurationIntentDecl>,
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
        if consumers.is_empty() {
            return Err(unsupported("no actual incoming configuration declaration"));
        }
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
            if input.default_features
                || input.profile.is_some()
                || input.policy != ChoicePolicy::RequireSelection
            {
                return Err(unsupported(
                    "automatic/default/profile basis is not connected on this package operation; supply explicit no-default input",
                ));
            }
            if !definition.parameters.is_empty() {
                return Err(unsupported(
                    "typed package parameter consumption is not connected",
                ));
            }
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
                    ConfigurationEffect::Forward { .. } => {
                        return Err(unsupported(
                            "selected cross-package forwarding requires candidate-conditioned incoming consumers",
                        ));
                    }
                    ConfigurationEffect::Subskill { .. } => {
                        return Err(unsupported("selected subskill loading is not connected"));
                    }
                    ConfigurationEffect::ConfigValue { .. } => {
                        return Err(unsupported(
                            "selected package config-value inspection/replay consumer is not connected",
                        ));
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
        Ok(Self {
            consumers: consumers.clone(),
            definition: definition.clone(),
            declarations: requires.clone(),
            features: features.selected.clone(),
            active_dependencies: active.into_iter().collect(),
            dependency_causes: causes,
        })
    }

    /// Validate durable derived fields against the retained original request and
    /// the actual pinned manifest; never infer a request from effective features.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
    pub fn validate(&self, manifest: &Manifest) -> Result<(), PackageConfigurationError> {
        let actual = Self::evaluate(manifest, &self.consumers)?;
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
