//! Automatic reliance retained beside the original inputs: consumer choice
//! defaults, guarded singleton defaults and the instance closure fallback.
use super::{AutomaticBasis, ClosureFallbackBasis, PackageConfigurationError, Scope};
use crate::configuration::{
    Cardinality, ChoiceEvaluation, ChoicePolicy, FeatureResolution, ImplicationReason,
};
use crate::manifest::configuration::{ConfigurationEffect, ConfigurationGroupDecl, basis};
use std::collections::{BTreeMap, BTreeSet};

/// Historical automatic selections gathered per consumer during admission.
#[derive(Default)]
pub(super) struct Automatic {
    pub(super) choices: BTreeMap<String, BTreeMap<String, String>>,
    pub(super) choice_sets: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub(super) dormant_choices: BTreeMap<String, Vec<String>>,
    pub(super) feature_defaults: BTreeMap<String, Vec<String>>,
}

impl Automatic {
    fn is_empty(&self) -> bool {
        self.choices.is_empty()
            && self.choice_sets.is_empty()
            && self.dormant_choices.is_empty()
            && self.feature_defaults.is_empty()
    }

    /// Bind each guarded singleton default to the kernel's own activity and
    /// default use: an active group records its member, a dormant one its id.
    pub(super) fn settle_guarded(
        &mut self,
        scope: &Scope<'_>,
        evaluation: &ChoiceEvaluation,
        guarded: Vec<(String, String)>,
    ) -> Result<(), PackageConfigurationError> {
        for (consumer, group) in guarded {
            let control = evaluation
                .controls
                .iter()
                .find(|c| c.id == group)
                .ok_or_else(|| scope.invalid("guarded choice activity missing".into()))?;
            if control.active {
                let used = evaluation
                    .defaults
                    .iter()
                    .filter(|d| d.consumer == consumer && d.group == group)
                    .collect::<Vec<_>>();
                let [used] = used.as_slice() else {
                    return Err(scope.invalid("guarded default use missing or duplicate".into()));
                };
                let [member] = used.declared.as_slice() else {
                    return Err(scope.invalid("guarded default is not singleton".into()));
                };
                if control.selection.as_ref() != Some(&vec![member.clone()]) {
                    return Err(
                        scope.invalid("guarded default selection disagrees with kernel".into())
                    );
                }
                self.choices
                    .entry(consumer)
                    .or_default()
                    .insert(group, member.clone());
            } else {
                self.dormant_choices
                    .entry(consumer)
                    .or_default()
                    .push(group);
            }
        }
        for groups in self.dormant_choices.values_mut() {
            groups.sort();
            groups.dedup();
        }
        Ok(())
    }

    /// The durable basis, or none when no consumer relied on a default.
    pub(super) fn into_basis(
        self,
        scope: &Scope<'_>,
    ) -> Result<Option<AutomaticBasis>, PackageConfigurationError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(AutomaticBasis {
            definition_fingerprint: scope
                .definition
                .semantic_fingerprint()
                .map_err(|e| scope.invalid(e.to_string()))?,
            consumer_choices: self.choices,
            consumer_choice_sets: self.choice_sets,
            consumer_dormant_choices: self.dormant_choices,
            consumer_feature_defaults: self.feature_defaults,
        }))
    }
}

/// Observed kernel closure fallback of the selected instance, admitted only
/// for one root Flow consumer with a single exactly-one singleton fallback.
pub(super) fn closure_fallback(
    scope: &Scope<'_>,
    features: &FeatureResolution,
    automatic: &Automatic,
) -> Result<Option<ClosureFallbackBasis>, PackageConfigurationError> {
    let Scope {
        kind,
        definition,
        consumers,
        forwarded,
        ..
    } = *scope;
    let instance_reliance = basis::instance_reliance(
        definition,
        &[],
        Some(features),
        &BTreeSet::new(),
        basis::Authority::Reuse,
    );
    if instance_reliance.is_empty() {
        return Ok(None);
    }
    let allowed = kind == crate::PackageKind::Flow
        && consumers.len() == 1
        && consumers.contains_key("./vibe.toml")
        && forwarded.is_empty()
        && definition.parameters.is_empty()
        && definition.profiles.is_empty()
        && definition.groups.len() == 1
        && definition.features.iter().all(|f| {
            !f.default
                && f.guards.is_empty()
                && f.effects.iter().all(|effect| {
                    matches!(
                        effect,
                        ConfigurationEffect::Dependency { .. }
                            | ConfigurationEffect::ConfigValue { .. }
                    )
                })
        })
        && consumers.values().all(|i| {
            i.policy == ChoicePolicy::RequireSelection
                && !i.default_features
                && i.profile.is_none()
                && i.selections.is_empty()
                && i.parameters.is_empty()
        })
        && automatic.is_empty();
    if !allowed {
        return Err(
            scope.unsupported("closure fallback basis is not connected on this package operation")
        );
    }
    let ConfigurationGroupDecl::Closure {
        id,
        members,
        cardinality,
        fallback: Some(fallback),
        ..
    } = &definition.groups[0]
    else {
        return Err(
            scope.unsupported("closure fallback basis requires one exactly-one singleton fallback")
        );
    };
    if *cardinality != Cardinality::ExactlyOne
        || fallback.len() != 1
        || !members.contains(&fallback[0])
        || instance_reliance != BTreeSet::from([basis::InstanceReliance::Closure(id.clone())])
    {
        return Err(
            scope.unsupported("closure fallback basis requires one exactly-one singleton fallback")
        );
    }
    let reasons = features
        .implied
        .iter()
        .filter_map(|i| match &i.reason {
            ImplicationReason::GroupDefault { group } => Some((group.clone(), i.feature.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    if reasons != vec![(id.clone(), fallback[0].clone())]
        || !features.selected.contains(&fallback[0])
    {
        return Err(scope.invalid("closure fallback reason disagrees with kernel selection".into()));
    }
    Ok(Some(ClosureFallbackBasis {
        definition_fingerprint: definition
            .semantic_fingerprint()
            .map_err(|e| scope.invalid(e.to_string()))?,
        groups: reasons.into_iter().collect(),
    }))
}
