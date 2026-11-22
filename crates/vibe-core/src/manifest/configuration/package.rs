//! Replayable package selections for real incoming workspace declarations.
//! The initial operation adapter accepts explicit, non-automatic inputs. Domain
//! effects beyond dependency activation remain explicit unsupported outcomes.
use super::{ConfigurationDecl, ConfigurationIntentDecl};
use crate::configuration::{ChoiceOutcome, FeatureBase, FeatureInput};
use crate::manifest::{FieldPath, Manifest, Requires};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod activation;
mod admission;
mod parameters;
mod reliance;
mod retained;

/// Original declarations and their separately evaluated dependency selection.
/// This belongs to the existing project lock, not a global configuration store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE")]
pub struct PackageConfiguration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_basis: Option<AutomaticBasis>,
    /// Actual kernel fallback reliance belongs to the selected instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closure_fallback_basis: Option<ClosureFallbackBasis>,
    pub consumers: BTreeMap<String, ConfigurationIntentDecl>,
    /// Operation-submitted originals remain distinct from authored consumers.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub submitted_inputs: BTreeMap<String, super::SubmittedInput>,
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

/// Instance-level observed closure fallback, never a consumer answer or consent.
///
/// ```
/// use std::collections::BTreeMap;
/// use vibe_core::manifest::{ConfigurationIntentDecl, Manifest, PackageConfiguration};
///
/// let manifest = Manifest::parse_str(
///     "[package]\ngroup='org.test'\nname='owner'\nkind='flow'\nversion='1.0.0'\n\
///      [[configuration.features]]\nid='plain'\nname='Plain'\n\
///      [[configuration.features]]\nid='rich'\nname='Rich'\n\
///      [[configuration.groups]]\nmode='closure'\nid='output'\nname='Output'\n\
///      cardinality='exactly-one'\nmembers=['plain','rich']\nfallback=['plain']",
/// )?;
/// let silent: ConfigurationIntentDecl =
///     toml::from_str("policy='require-selection'\ndefault_features=false")?;
/// let record = PackageConfiguration::evaluate(
///     &manifest,
///     &BTreeMap::from([("./vibe.toml".to_string(), silent)]),
/// )?;
/// // Nobody answered `output`; the kernel fell back, and the record says so.
/// let basis = record.closure_fallback_basis.expect("observed fallback");
/// assert_eq!(basis.groups, BTreeMap::from([("output".into(), "plain".into())]));
/// assert_eq!(record.features, ["plain"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureFallbackBasis {
    pub definition_fingerprint: String,
    pub groups: BTreeMap<String, String>,
}

/// One exact selected source-owned declaration requesting a target feature.
///
/// ```
/// use std::collections::BTreeMap;
/// use vibe_core::manifest::{ForwardStrength, ForwardedInput, Manifest, PackageConfiguration};
/// use vibe_core::{ContentHash, PackageRef};
///
/// let target = Manifest::parse_str(
///     "[package]\ngroup='org.test'\nname='target'\nkind='flow'\nversion='1.0.0'\n\
///      [[configuration.features]]\nid='tracing'\nname='Tracing'",
/// )?;
/// let request = ForwardedInput {
///     source_group: "org.test".parse()?,
///     source_name: "parent".parse()?,
///     source_version: semver::Version::new(2, 0, 0),
///     source_content_hash: ContentHash::parse("sha256:e3b0c44298fc1c14")?,
///     source_feature: "observability".into(),
///     declaration_index: 0,
///     target: PackageRef::parse("flow:org.test/target@=1.0.0")?,
///     target_feature: "tracing".into(),
///     strength: ForwardStrength::Strong,
/// };
/// // The parent's request selects the feature without becoming a consumer.
/// let record =
///     PackageConfiguration::evaluate_with_forwarded(&target, &BTreeMap::new(), &[request])?;
/// assert_eq!(record.features, ["tracing"]);
/// assert!(record.consumers.is_empty());
/// assert_eq!(record.forwarded_inputs[0].source_feature, "observability");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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
///
/// ```
/// use std::collections::BTreeMap;
/// use vibe_core::manifest::{ConfigurationIntentDecl, Manifest, PackageConfiguration};
///
/// let manifest = Manifest::parse_str(
///     "[package]\ngroup='org.test'\nname='owner'\nkind='flow'\nversion='1.0.0'\n\
///      [[configuration.features]]\nid='compact'\nname='Compact'\n\
///      [[configuration.groups]]\nmode='choice'\nid='layout'\nname='Layout'\n\
///      cardinality='exactly-one'\nmembers=['compact']\ndefaults=['compact']",
/// )?;
/// let omitted: ConfigurationIntentDecl =
///     toml::from_str("policy='use-declared-defaults'\ndefault_features=false")?;
/// let record = PackageConfiguration::evaluate(
///     &manifest,
///     &BTreeMap::from([("./vibe.toml".to_string(), omitted)]),
/// )?;
/// // The consumer answered nothing, so its reliance on the default is kept.
/// let basis = record.automatic_basis.expect("default reliance");
/// assert_eq!(basis.consumer_choices["./vibe.toml"]["layout"], "compact");
/// assert!(record.consumers["./vibe.toml"].selections.is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomaticBasis {
    pub definition_fingerprint: String,
    pub consumer_choices: BTreeMap<String, BTreeMap<String, String>>,
    /// Lossless non-scalar choice reliance, including a declared optional empty set.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumer_choice_sets: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// Historical omitted-policy binding for inactive guarded choices, not members.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumer_dormant_choices: BTreeMap<String, Vec<String>>,
    /// Declared feature-default seeds before closure; an empty row retains reliance.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumer_feature_defaults: BTreeMap<String, Vec<String>>,
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
    /// Pure lexical targets requiring an operation-owned host existence check.
    /// This grants no IO or readiness and is safe for history inspection.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#EVALUATION")]
    pub fn host_path_targets(&self) -> Vec<&str> {
        self.definition
            .parameters
            .iter()
            .filter(|p| p.kind == crate::manifest::ParameterType::Path && p.must_exist)
            .flat_map(|p| {
                self.consumers
                    .values()
                    .filter_map(move |input| match input.parameters.get(&p.id) {
                        Some(super::ParameterValue::Text(value)) => Some(value.as_str()),
                        _ => None,
                    })
            })
            .collect()
    }

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
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#EVALUATION")]
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

    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#EVALUATION")]
    pub(crate) fn evaluate_parts(
        package: String,
        kind: crate::PackageKind,
        definition: &ConfigurationDecl,
        requires: &Requires,
        consumers: &BTreeMap<String, ConfigurationIntentDecl>,
        forwarded: &[ForwardedInput],
    ) -> Result<Self, PackageConfigurationError> {
        let scope = Scope {
            package: &package,
            kind,
            definition,
            requires,
            consumers,
            forwarded,
        };
        definition
            .validate_scope(
                requires,
                kind,
                false,
                &FieldPath::root().table("configuration"),
            )
            .map_err(|e| scope.invalid(format!("{}: {}", e.field, e.message)))?;
        requires
            .validate_configuration_metadata()
            .map_err(|reason| scope.invalid(reason))?;
        if consumers.is_empty() && forwarded.is_empty() {
            return Err(scope.unsupported("no actual incoming configuration declaration"));
        }
        if !forwarded.is_empty()
            && (!definition.parameters.is_empty() || !definition.profiles.is_empty())
        {
            return Err(scope.unsupported("profile or parameter forwarding is not connected"));
        }
        let admission::Admitted {
            mut automatic,
            guarded_reliance,
            ordinary,
            choices,
        } = admission::admit(&scope)?;
        let model = definition
            .compile_choices()
            .map_err(|e| scope.invalid(format!("{}: {}", e.field, e.message)))?;
        // Across consumers there is no last-writer feature replacement. All
        // direct seeds are simultaneous base requirements before core closure.
        let feature_base = if automatic.feature_defaults.is_empty() {
            FeatureBase::Selected(ordinary.iter().cloned().collect())
        } else {
            // Narrow single-consumer default admission keeps kernel default
            // activation; authored additions are still explicit additions.
            FeatureBase::Defaults
        };
        let explicit_additions = if automatic.feature_defaults.is_empty() {
            vec![]
        } else {
            ordinary.into_iter().collect()
        };
        let evaluation = model.evaluate(
            &FeatureInput {
                base: feature_base,
                additions: explicit_additions,
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
        automatic.settle_guarded(&scope, &evaluation, guarded_reliance)?;
        let closure_fallback_basis = reliance::closure_fallback(&scope, features, &automatic)?;
        let activation::Activation { active, causes } = activation::activate(&scope, features)?;
        let automatic_basis = automatic.into_basis(&scope)?;
        Ok(Self {
            automatic_basis,
            closure_fallback_basis,
            consumers: consumers.clone(),
            submitted_inputs: BTreeMap::new(),
            forwarded_inputs: forwarded.to_vec(),
            definition: definition.clone(),
            declarations: requires.clone(),
            features: features.selected.clone(),
            active_dependencies: active.into_iter().collect(),
            dependency_causes: causes,
        })
    }
}

/// The borrowed facts of one evaluation; every phase reads the same instance.
struct Scope<'a> {
    package: &'a str,
    kind: crate::PackageKind,
    definition: &'a ConfigurationDecl,
    requires: &'a Requires,
    consumers: &'a BTreeMap<String, ConfigurationIntentDecl>,
    forwarded: &'a [ForwardedInput],
}

impl Scope<'_> {
    fn unsupported(&self, reason: &str) -> PackageConfigurationError {
        PackageConfigurationError::Unsupported {
            package: self.package.to_owned(),
            reason: reason.into(),
        }
    }

    fn invalid(&self, reason: String) -> PackageConfigurationError {
        PackageConfigurationError::Invalid {
            package: self.package.to_owned(),
            reason,
        }
    }
}
