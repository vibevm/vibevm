//! Package syntax and effect projection around the single local closure kernel.
//! This table cannot bind foreign dependency declarations or execute effects.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-PACKAGE-COMPATIBILITY");

use super::{FeatureError, FeatureExpansion, FeatureValue};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::configuration::{
    Cardinality, ClosureGroup, DefinitionError, EvaluationError, FeatureBase, FeatureInput,
    FeatureModel, FeatureNode,
};
use vibe_core::manifest::FeaturesTable;

/// A nonempty collection; expansion keeps the first diagnostic, validation lists all.
pub(super) struct DefinitionFailures {
    pub first: FeatureError,
    further: Vec<FeatureError>,
}

impl DefinitionFailures {
    fn one(first: FeatureError) -> Self {
        Self {
            first,
            further: vec![],
        }
    }
    pub fn into_errors(self) -> impl Iterator<Item = FeatureError> {
        std::iter::once(self.first).chain(self.further)
    }
}

pub(super) struct CompiledFeatures {
    model: FeatureModel,
    effects: BTreeMap<String, Vec<FeatureValue>>,
}

impl CompiledFeatures {
    pub fn new(table: &FeaturesTable) -> Result<Self, DefinitionFailures> {
        let mut nodes = Vec::new();
        let mut effects = BTreeMap::new();
        let mut failures = Vec::new();
        for (name, activations) in &table.features {
            let mut requires = Vec::new();
            let mut seen = BTreeSet::new();
            let mut projected = Vec::new();
            for raw in activations {
                match FeatureValue::parse(raw) {
                    Ok(FeatureValue::Feature(target)) => {
                        if seen.insert(target.clone()) {
                            if !table.features.contains_key(&target) {
                                failures.push(FeatureError::UnknownFeature(format!(
                                    "{target} (referenced by feature `{name}`)"
                                )));
                            }
                            requires.push(target);
                        }
                    }
                    Ok(effect) => projected.push(effect),
                    Err(FeatureError::Malformed(_)) => failures.push(FeatureError::Malformed(
                        format!("feature `{name}` activation `{raw}`"),
                    )),
                    Err(error) => failures.push(error),
                }
            }
            nodes.push(FeatureNode {
                id: name.clone(),
                default: false,
                parent: None,
                required: false,
                requires,
            });
            effects.insert(name.clone(), projected);
        }
        let mut groups = Vec::new();
        for (id, members) in &table.exclusive {
            let mut seen = BTreeSet::new();
            let members: Vec<_> = members
                .iter()
                .filter(|m| seen.insert(*m))
                .cloned()
                .collect();
            for member in &members {
                if !table.features.contains_key(member) {
                    failures.push(FeatureError::UnknownFeature(format!(
                        "{member} (referenced by exclusive group `{id}`)"
                    )));
                }
            }
            // A legacy empty set constrains nothing; the kernel has no empty group.
            if !members.is_empty() {
                groups.push(ClosureGroup {
                    id: id.clone(),
                    members,
                    cardinality: Cardinality::ZeroOrOne,
                    fallback: None,
                });
            }
        }
        let mut failures = failures.into_iter();
        if let Some(first) = failures.next() {
            return Err(DefinitionFailures {
                first,
                further: failures.collect(),
            });
        }
        let model = FeatureModel::new(nodes, groups)
            .map_err(|error| DefinitionFailures::one(definition_error(error)))?;
        Ok(Self { model, effects })
    }

    /// The seeds are a package union, not app-style replacement additions.
    pub fn expand(&self, seeds: Vec<String>) -> Result<FeatureExpansion, FeatureError> {
        let resolved = self
            .model
            .resolve(&FeatureInput {
                base: FeatureBase::Selected(seeds),
                additions: vec![],
                disabled: vec![],
            })
            .map_err(evaluation_error)?;
        let mut out = FeatureExpansion {
            active_features: resolved.selected.iter().cloned().collect(),
            ..FeatureExpansion::default()
        };
        // Local implication work is finished. This visits selected values once
        // to project the existing non-local algebra, never walks feature edges.
        for name in resolved.selected {
            if let Some(effects) = self.effects.get(&name) {
                for effect in effects {
                    project(&mut out, effect);
                }
            }
        }
        Ok(out)
    }
}

fn project(out: &mut FeatureExpansion, effect: &FeatureValue) {
    match effect {
        FeatureValue::Feature(_) => {} // Local edges are held only by FeatureModel.
        FeatureValue::Dep { dep_name } => {
            out.active_deps.insert(dep_name.clone());
        }
        FeatureValue::DepFeature {
            dep_name,
            dep_feature,
            weak,
        } => {
            let target = if *weak {
                &mut out.weak_dep_features
            } else {
                &mut out.dep_features
            };
            target
                .entry(dep_name.clone())
                .or_default()
                .insert(dep_feature.clone());
            if !weak {
                out.active_deps.insert(dep_name.clone());
            }
        }
        FeatureValue::Subskill { path } => {
            out.active_subskills.insert(path.clone());
        }
    }
}

fn definition_error(error: DefinitionError) -> FeatureError {
    FeatureError::Malformed(format!("local feature definition: {error}"))
}

fn evaluation_error(error: EvaluationError) -> FeatureError {
    match error {
        EvaluationError::UnknownFeature { id } => FeatureError::UnknownFeature(id),
        EvaluationError::Cardinality { group, members, .. } => FeatureError::ExclusiveViolation {
            group,
            active: members,
        },
        other => FeatureError::Malformed(format!("local feature request: {other}")),
    }
}
