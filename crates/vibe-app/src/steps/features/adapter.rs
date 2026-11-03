//! Legacy app definitions and diagnostics around the shared feature kernel.
//! Group order remains lexical as in the existing app presentation contract.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-APP-COMPATIBILITY");

use std::collections::{BTreeMap, BTreeSet};
use vibe_core::configuration::{
    Cardinality, ClosureGroup, EvaluationError, FeatureModel, FeatureNode, ImplicationReason,
};
use vibe_core::manifest::{FeatureDecl, InstallerOptionDecl};

use super::{FeatureResolution, Implied, ImpliedRule};
use crate::error::StepError;

pub(super) fn model(option: &InstallerOptionDecl) -> Result<FeatureModel, StepError> {
    let features = option
        .features
        .iter()
        .map(|feature| FeatureNode {
            id: feature.id.clone(),
            default: feature.default,
            parent: feature.parent.clone(),
            required: feature.required,
            requires: {
                // The app manifest permits redundant requires entries. Keep
                // their first-occurrence order when lowering into a unique model.
                let mut seen = BTreeSet::new();
                feature
                    .requires
                    .iter()
                    .filter(|id| seen.insert(*id))
                    .cloned()
                    .collect()
            },
        })
        .collect();
    let mut groups: BTreeMap<&str, Vec<&FeatureDecl>> = BTreeMap::new();
    for feature in &option.features {
        if let Some(group) = &feature.exclusive_group {
            groups.entry(group).or_default().push(feature);
        }
    }
    let groups = groups
        .into_iter()
        .map(|(id, members)| {
            ClosureGroup {
                id: id.into(),
                members: members.iter().map(|feature| feature.id.clone()).collect(),
                cardinality: Cardinality::ExactlyOne,
                // Valid legacy declarations require exactly one default. Supplying
                // Some even when empty lets the model reject malformed direct data.
                fallback: Some(
                    members
                        .iter()
                        .filter(|feature| feature.default)
                        .map(|feature| feature.id.clone())
                        .collect(),
                ),
            }
        })
        .collect();
    FeatureModel::new(features, groups).map_err(|error| StepError::FeatureConflict {
        option: option.id.clone(),
        reason: error.to_string(),
    })
}

pub(super) fn result(resolved: vibe_core::configuration::FeatureResolution) -> FeatureResolution {
    FeatureResolution {
        selected: resolved.selected,
        implied: resolved
            .implied
            .into_iter()
            .map(|implied| {
                let (rule, by) = match implied.reason {
                    ImplicationReason::Parent { child } => (ImpliedRule::Parent, Some(child)),
                    ImplicationReason::Requires { feature } => {
                        (ImpliedRule::Requires, Some(feature))
                    }
                    ImplicationReason::Required { parent } => (ImpliedRule::Required, parent),
                    ImplicationReason::GroupDefault { group } => {
                        (ImpliedRule::ExclusiveDefault, Some(group))
                    }
                };
                Implied {
                    feature: implied.feature,
                    rule,
                    by,
                }
            })
            .collect(),
    }
}

pub(super) fn unknown(option: &InstallerOptionDecl, id: String) -> StepError {
    StepError::UnknownFeature {
        option: option.id.clone(),
        feature: id,
        declared: option
            .features
            .iter()
            .map(|f| f.id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

pub(super) fn error(option: &InstallerOptionDecl, error: EvaluationError) -> StepError {
    let reason = match error {
        EvaluationError::ExplicitReplacement {
            group,
            selected,
            member,
            descendant,
        } => format!(
            "exclusive group `{group}` cannot select `{selected}`: replacing `{member}` would discard directly selected feature `{descendant}`"
        ),
        EvaluationError::UnknownFeature { id } => return unknown(option, id),
        EvaluationError::EnabledAndDisabled { id } => {
            format!("`{id}` is both selected and deselected")
        }
        EvaluationError::DisabledAncestor {
            feature,
            descendant,
        } => format!(
            "`{feature}` cannot be deselected: the selected feature `{descendant}` is beneath it"
        ),
        EvaluationError::ExplicitCardinality { group, members, .. } => format!(
            "exclusive group `{group}` has more than one explicit selection: {}",
            members.join(", ")
        ),
        EvaluationError::Cardinality { group, members, .. } => format!(
            "exclusive group `{group}` would select {}; exactly one member may be selected",
            members.join(" and ")
        ),
        EvaluationError::DisabledImplied { feature, reason } => match reason {
            ImplicationReason::Parent { child } => format!(
                "`{feature}` cannot be deselected: the selected feature `{child}` is beneath it"
            ),
            ImplicationReason::Requires { feature: by } => {
                format!("`{feature}` cannot be deselected: the selected feature `{by}` requires it")
            }
            ImplicationReason::Required { parent: None } => {
                format!("`{feature}` is a required feature and is always selected")
            }
            ImplicationReason::Required {
                parent: Some(parent),
            } => format!(
                "`{feature}` cannot be deselected: it is required while its parent `{parent}` is selected"
            ),
            ImplicationReason::GroupDefault { group } => format!(
                "`{feature}` cannot be deselected: it is the default of exclusive group `{group}` and no other member is selected"
            ),
        },
    };
    StepError::FeatureConflict {
        option: option.id.clone(),
        reason,
    }
}
