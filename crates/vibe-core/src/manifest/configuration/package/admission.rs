//! Admission of actual consumer declarations and candidate-owned forwarded
//! requests into one simultaneous kernel input.
use super::parameters::{Consumer, PublicParameters};
use super::{PackageConfigurationError, Scope, reliance::Automatic};
use crate::configuration::{
    Cardinality, ChoicePolicy, ConsumerChoices, FeatureBase, MergeIntent, Selection,
};
use crate::manifest::FieldPath;
use crate::manifest::configuration::{
    ConfigurationFeatureVisibility, ConfigurationGroupDecl, ConfigurationGuard,
    ConfigurationIntentDecl, NormalizedConfigurationInput, basis,
};
use std::collections::{BTreeMap, BTreeSet};

/// Everything admission hands to the kernel and to the later reliance checks.
#[derive(Default)]
pub(super) struct Admitted {
    pub(super) automatic: Automatic,
    /// Guarded singleton defaults whose activity only the kernel can decide.
    pub(super) guarded_reliance: Vec<(String, String)>,
    pub(super) ordinary: BTreeSet<String>,
    pub(super) choices: Vec<ConsumerChoices>,
}

impl Admitted {
    fn absorb(&mut self, normalized: NormalizedConfigurationInput) {
        if let FeatureBase::Selected(base) = normalized.features.base {
            self.ordinary.extend(base);
        }
        self.ordinary.extend(normalized.features.additions);
        self.choices.push(normalized.choices);
    }
}

pub(super) fn admit(scope: &Scope<'_>) -> Result<Admitted, PackageConfigurationError> {
    let mut admitted = Admitted::default();
    let mut public = PublicParameters::default();
    for (key, input) in scope.consumers {
        input
            .validate_shape_at(&FieldPath::root().table("configuration"))
            .map_err(|e| scope.invalid(format!("{key}: {}: {}", e.field, e.message)))?;
        let normalized = scope.definition.normalize_input(input, key).map_err(|e| {
            PackageConfigurationError::Incompatible {
                package: scope.package.to_owned(),
                consumers: vec![key.clone()],
                reason: format!("{}: {}", e.field, e.message),
            }
        })?;
        let consumer = Consumer {
            key,
            input,
            normalized: &normalized,
        };
        if input.default_features && input.profile.is_none() {
            let declared = declared_feature_defaults(scope)?;
            admitted
                .automatic
                .feature_defaults
                .insert(key.clone(), declared);
        }
        choice_defaults(scope, consumer, &mut admitted)?;
        public.admit(scope, consumer, &admitted.automatic)?;
        admitted.absorb(normalized);
    }
    forwarded(scope, &mut admitted)?;
    Ok(admitted)
}

/// Declared feature-default seeds of the one consumer allowed to rely on them.
fn declared_feature_defaults(scope: &Scope<'_>) -> Result<Vec<String>, PackageConfigurationError> {
    let Scope {
        kind,
        definition,
        consumers,
        forwarded,
        ..
    } = *scope;
    if kind != crate::PackageKind::Flow
        || consumers.len() != 1
        || !forwarded.is_empty()
        || !definition.groups.is_empty()
        || !definition.profiles.is_empty()
        || !definition.parameters.is_empty()
        || definition.features.iter().any(|feature| {
            !feature.guards.is_empty()
                || feature.visibility != ConfigurationFeatureVisibility::Public
        })
    {
        return Err(scope.unsupported(
            "feature defaults require one public ordinary Flow without groups, profiles, parameters or forwarding",
        ));
    }
    Ok(definition
        .features
        .iter()
        .filter(|feature| feature.default)
        .map(|feature| feature.id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

/// Record which omitted choice defaults this consumer relies on.
fn choice_defaults(
    scope: &Scope<'_>,
    consumer: Consumer<'_>,
    admitted: &mut Admitted,
) -> Result<(), PackageConfigurationError> {
    let definition = scope.definition;
    let reasons = basis::consumer_reliance(
        definition,
        basis::ConsumerInput {
            original: consumer.input,
            normalized: consumer.normalized,
        },
        &BTreeSet::new(),
        basis::Authority::Reuse,
    );
    let mut selected_defaults = BTreeMap::new();
    let mut selected_default_sets = BTreeMap::new();
    for reason in reasons {
        if reason == basis::ConsumerReliance::FeatureDefaults {
            continue; // Already bound to declared seeds, never effective closure.
        }
        if reason == basis::ConsumerReliance::Profile {
            // Profile reliance is retained by original profile input and
            // historical definition, never a fabricated default choice.
            continue;
        }
        let basis::ConsumerReliance::Choice(group_id) = reason else {
            return Err(scope.unsupported("only omitted exactly-one choice defaults are connected"));
        };
        let Some(ConfigurationGroupDecl::Choice {
            cardinality,
            defaults: Some(defaults),
            members,
            guards,
            ..
        }) = definition.groups.iter().find(
            |group| matches!(group, ConfigurationGroupDecl::Choice { id, .. } if id == &group_id),
        )
        else {
            return Err(scope.unsupported("automatic choice needs a declared exactly-one default"));
        };
        if *cardinality != Cardinality::ExactlyOne {
            let multi_groups = definition
                .groups
                .iter()
                .filter(|group| {
                    matches!(group, ConfigurationGroupDecl::Choice { cardinality, .. }
                        if *cardinality != Cardinality::ExactlyOne)
                })
                .count();
            if !guards.is_empty()
                || !definition.parameters.is_empty()
                || !definition.profiles.is_empty()
                || multi_groups != 1
            {
                return Err(scope.unsupported(
                    "only one unguarded non-scalar choice default group without parameters or profiles is connected",
                ));
            }
            let canonical = defaults
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            selected_default_sets.insert(group_id, canonical);
            continue;
        }
        if !guards.is_empty() {
            if !independent_explicit_gate(scope, consumer, &group_id, guards)
                || defaults.len() != 1
                || !members.contains(&defaults[0])
            {
                return Err(scope.unsupported(
                    "guarded singleton defaults require one independent explicit gate",
                ));
            }
            admitted
                .guarded_reliance
                .push((consumer.key.to_owned(), group_id));
            continue;
        }
        if defaults.len() != 1 || !members.contains(&defaults[0]) {
            return Err(scope
                .unsupported("only unguarded exactly-one single-member defaults are connected"));
        }
        selected_defaults.insert(group_id, defaults[0].clone());
    }
    if !selected_defaults.is_empty() {
        admitted
            .automatic
            .choices
            .insert(consumer.key.to_owned(), selected_defaults);
    }
    if !selected_default_sets.is_empty() {
        admitted
            .automatic
            .choice_sets
            .insert(consumer.key.to_owned(), selected_default_sets);
    }
    Ok(())
}

/// Only the qualified one independent explicit gate / one guarded singleton
/// group. Kernel decides activity/default use.
fn independent_explicit_gate(
    scope: &Scope<'_>,
    consumer: Consumer<'_>,
    group_id: &str,
    guards: &[ConfigurationGuard],
) -> bool {
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
    kind == crate::PackageKind::Flow
        && consumers.len() == 1
        && forwarded.is_empty()
        && definition.parameters.is_empty()
        && definition.profiles.is_empty()
        && definition.groups.len() == 2
        && guards.len() == 1
        && matches!(&guards[0], ConfigurationGuard::Member(gate_member)
            if definition.groups.iter().any(|group| matches!(group,
                ConfigurationGroupDecl::Choice{id,members,guards,cardinality,..}
                if id != group_id && guards.is_empty()
                    && *cardinality==Cardinality::ExactlyOne
                    && members.contains(gate_member)
                    && input.selections.get(id).is_some_and(|answer| answer.intent == MergeIntent::Exact)
                    && normalized.choices.answers.get(id).is_some_and(|answer|
                        matches!(&answer.selection, Selection::Explicit(members) if members.len()==1)))))
        && definition.features.iter().all(|f| f.guards.is_empty())
}

/// Typed candidate-owned inputs, never inserted among the consumers.
fn forwarded(scope: &Scope<'_>, admitted: &mut Admitted) -> Result<(), PackageConfigurationError> {
    let mut forwarded_features = BTreeSet::new();
    for request in scope.forwarded {
        if !request.target.version.is_exact_pin()
            || request.target.qualified_name() != scope.package
            || !forwarded_features.insert(request.target_feature.clone())
        {
            return Err(
                scope.invalid("invalid or duplicate candidate-owned forwarded request".into())
            );
        }
        let original = ConfigurationIntentDecl {
            policy: ChoicePolicy::RequireSelection,
            profile: None,
            features: vec![request.target_feature.clone()],
            default_features: false,
            selections: BTreeMap::new(),
            parameters: BTreeMap::new(),
        };
        let normalized = scope
            .definition
            .normalize_input(&original, "candidate-owned forwarding")
            .map_err(|error| scope.invalid(error.to_string()))?;
        admitted.absorb(normalized);
    }
    Ok(())
}
