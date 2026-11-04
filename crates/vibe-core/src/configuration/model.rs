//! Compile local references once. Private indices always refer to this model's
//! nodes; parent cycles are refused before the evaluator can traverse them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL");

use std::collections::{BTreeMap, BTreeSet};

use super::{
    ClosureGroup, DefinitionError, DefinitionProblem as Problem, FeatureNode, ReferenceKind,
};

#[derive(Debug, Clone)]
pub(super) struct Node {
    pub declaration: FeatureNode,
    pub parent: Option<usize>,
    pub requires: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(super) struct Group {
    pub declaration: ClosureGroup,
    pub members: Vec<usize>,
    pub fallback: Vec<usize>,
    pub activation: Option<Vec<usize>>,
}

/// A validated model whose compiled state cannot be mutated by callers.
///
/// ```
/// use vibe_core::configuration::{FeatureModel, FeatureNode, FeatureBase, FeatureInput};
/// let model = FeatureModel::new(vec![FeatureNode {
///     id: "core".into(), default: false, parent: None, required: true, requires: vec![],
/// }], vec![])?;
/// let result = model.resolve(&FeatureInput {
///     base: FeatureBase::Selected(vec![]), additions: vec![], disabled: vec![],
/// })?;
/// assert_eq!(result.selected, ["core"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct FeatureModel {
    pub(super) nodes: Vec<Node>,
    pub(super) groups: Vec<Group>,
    pub(super) ids: BTreeMap<String, usize>,
}

fn invalid(problem: Problem) -> DefinitionError {
    DefinitionError { problem }
}

/// Local identifiers are literal data. Validate without rewriting their spelling.
pub(super) fn valid_identifier(id: &str) -> bool {
    !id.trim().is_empty() && !id.chars().any(char::is_control)
}

fn identifier(id: &str) -> Result<(), DefinitionError> {
    if !valid_identifier(id) {
        return Err(invalid(Problem::InvalidIdentifier { id: id.into() }));
    }
    Ok(())
}

fn references(
    ids: &BTreeMap<String, usize>,
    owner: &str,
    values: &[String],
    kind: ReferenceKind,
) -> Result<Vec<usize>, DefinitionError> {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|id| {
            if !seen.insert(id) {
                return Err(invalid(Problem::DuplicateReference {
                    owner: owner.into(),
                    id: id.clone(),
                    kind,
                }));
            }
            ids.get(id).copied().ok_or_else(|| {
                invalid(Problem::UnknownReference {
                    owner: owner.into(),
                    id: id.clone(),
                    kind,
                })
            })
        })
        .collect()
}

impl FeatureModel {
    /// Local validation query for sibling configuration components.
    pub(super) fn contains_feature(&self, id: &str) -> bool {
        self.ids.contains_key(id)
    }
    /// Validate all supplied definitions, including inactive groups. Positive
    /// `requires` cycles are finite closures; parent cycles are invalid trees.
    /// Feature and group IDs occupy separate local namespaces.
    ///
    /// ```
    /// use vibe_core::configuration::FeatureModel;
    /// let empty = FeatureModel::new(vec![], vec![])?;
    /// # let _ = empty;
    /// # Ok::<(), vibe_core::configuration::DefinitionError>(())
    /// ```
    #[track_caller]
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL")]
    pub fn new(
        features: Vec<FeatureNode>,
        groups: Vec<ClosureGroup>,
    ) -> Result<Self, DefinitionError> {
        let mut ids = BTreeMap::new();
        for (index, feature) in features.iter().enumerate() {
            identifier(&feature.id)?;
            if ids.insert(feature.id.clone(), index).is_some() {
                return Err(invalid(Problem::DuplicateFeature {
                    id: feature.id.clone(),
                }));
            }
        }
        let mut nodes = Vec::with_capacity(features.len());
        for declaration in features {
            let parent = declaration
                .parent
                .as_ref()
                .map(|id| {
                    ids.get(id).copied().ok_or_else(|| {
                        invalid(Problem::UnknownReference {
                            owner: declaration.id.clone(),
                            id: id.clone(),
                            kind: ReferenceKind::Parent,
                        })
                    })
                })
                .transpose()?;
            let requires = references(
                &ids,
                &declaration.id,
                &declaration.requires,
                ReferenceKind::Requires,
            )?;
            nodes.push(Node {
                declaration,
                parent,
                requires,
            });
        }
        check_parents(&nodes)?;
        let mut group_ids = BTreeSet::new();
        let mut compiled = Vec::with_capacity(groups.len());
        for declaration in groups {
            identifier(&declaration.id)?;
            if !group_ids.insert(declaration.id.clone()) {
                return Err(invalid(Problem::DuplicateGroup { id: declaration.id }));
            }
            let members = references(
                &ids,
                &declaration.id,
                &declaration.members,
                ReferenceKind::Member,
            )?;
            if members.is_empty() {
                return Err(invalid(Problem::EmptyGroup { id: declaration.id }));
            }
            let fallback = match &declaration.fallback {
                None => Vec::new(),
                Some(values) => {
                    let indices =
                        references(&ids, &declaration.id, values, ReferenceKind::Fallback)?;
                    for (&index, feature) in indices.iter().zip(values) {
                        if !members.contains(&index) {
                            return Err(invalid(Problem::ForeignFallback {
                                group: declaration.id.clone(),
                                feature: feature.clone(),
                            }));
                        }
                    }
                    if !declaration.cardinality.accepts(values.len()) {
                        return Err(invalid(Problem::FallbackCardinality {
                            group: declaration.id.clone(),
                            cardinality: declaration.cardinality,
                            count: values.len(),
                        }));
                    }
                    indices
                }
            };
            let activation = if members.iter().any(|&id| nodes[id].parent.is_none()) {
                None
            } else {
                Some(
                    members
                        .iter()
                        .filter_map(|&id| nodes[id].parent)
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                )
            };
            compiled.push(Group {
                declaration,
                members,
                fallback,
                activation,
            });
        }
        Ok(Self {
            nodes,
            groups: compiled,
            ids,
        })
    }

    pub(super) fn descendant(&self, index: usize, ancestor: usize) -> bool {
        let mut current = self.nodes[index].parent;
        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }
            current = self.nodes[parent].parent;
        }
        false
    }
}

fn check_parents(nodes: &[Node]) -> Result<(), DefinitionError> {
    let mut checked = BTreeSet::new();
    for first in 0..nodes.len() {
        let mut path = Vec::new();
        let mut current = Some(first);
        while let Some(index) = current {
            if checked.contains(&index) {
                break;
            }
            if let Some(start) = path.iter().position(|prior| *prior == index) {
                let features = path[start..]
                    .iter()
                    .map(|&i: &usize| nodes[i].declaration.id.clone())
                    .collect();
                return Err(invalid(Problem::ParentCycle { features }));
            }
            path.push(index);
            current = nodes[index].parent;
        }
        checked.extend(path);
    }
    Ok(())
}
