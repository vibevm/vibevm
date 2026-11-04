//! Validate complete choice ownership and the positive guard dependency DAG.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE");

use super::super::FeatureModel;
use super::super::model::valid_identifier;
use super::{ChoiceDefinitionError as Error, ChoiceGroup, ChoiceGuard, MemberGuards};
use std::collections::{BTreeMap, BTreeSet};

/// Validated local choices over one validated feature model. This owns no
/// source, execution, candidate or consumer-instance authority.
///
/// ```
/// use vibe_core::configuration::{ChoiceModel, FeatureModel};
/// let model = ChoiceModel::new(FeatureModel::new(vec![], vec![])?, vec![], vec![])?;
/// # let _ = model;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct ChoiceModel {
    pub(super) features: FeatureModel,
    pub(super) groups: Vec<ChoiceGroup>,
    pub(super) owners: BTreeMap<String, usize>,
    pub(super) group_ids: BTreeMap<String, usize>,
    pub(super) dependencies: BTreeMap<ChoiceGuard, Vec<ChoiceGuard>>,
    pub(super) order: Vec<ChoiceGuard>,
}

impl ChoiceModel {
    /// Validate references/defaults and every guard cycle, including implicit
    /// owning-group dependencies. Ordinary feature-requires cycles are unchanged.
    ///
    /// ```
    /// use vibe_core::configuration::{ChoiceModel, FeatureModel};
    /// let empty = ChoiceModel::new(FeatureModel::new(vec![], vec![])?, vec![], vec![])?;
    /// # let _ = empty;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[track_caller]
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE")]
    pub fn new(
        features: FeatureModel,
        groups: Vec<ChoiceGroup>,
        members: Vec<MemberGuards>,
    ) -> Result<Self, Error> {
        let mut group_ids = BTreeMap::new();
        let mut owners = BTreeMap::new();
        for (index, group) in groups.iter().enumerate() {
            if !valid_identifier(&group.id) {
                return Err(Error::InvalidGroupId {
                    group: group.id.clone(),
                });
            }
            if group_ids.insert(group.id.clone(), index).is_some() {
                return Err(Error::DuplicateGroup {
                    group: group.id.clone(),
                });
            }
            if group.members.is_empty() {
                return Err(Error::EmptyGroup {
                    group: group.id.clone(),
                });
            }
            let mut seen = BTreeSet::new();
            for member in &group.members {
                if !seen.insert(member) {
                    return Err(Error::DuplicateMember {
                        group: group.id.clone(),
                        member: member.clone(),
                    });
                }
                if !features.contains_feature(member) {
                    return Err(Error::UnknownMember {
                        group: group.id.clone(),
                        member: member.clone(),
                    });
                }
                if let Some(first) = owners.insert(member.clone(), index) {
                    return Err(Error::MultipleOwners {
                        member: member.clone(),
                        first: groups[first].id.clone(),
                        second: group.id.clone(),
                    });
                }
            }
            if let Some(defaults) = &group.defaults {
                let mut seen_defaults = BTreeSet::new();
                for member in defaults {
                    if !seen_defaults.insert(member) {
                        return Err(Error::DuplicateDefault {
                            group: group.id.clone(),
                            member: member.clone(),
                        });
                    }
                    if !seen.contains(member) {
                        return Err(Error::UnknownDefault {
                            group: group.id.clone(),
                            member: member.clone(),
                        });
                    }
                }
                if !group.cardinality.accepts(defaults.len()) {
                    return Err(Error::DefaultCardinality {
                        group: group.id.clone(),
                        cardinality: group.cardinality,
                        members: defaults.clone(),
                    });
                }
            }
        }
        let mut dependencies = BTreeMap::new();
        for group in &groups {
            dependencies.insert(ChoiceGuard::Group(group.id.clone()), group.guards.clone());
            for member in &group.members {
                dependencies.insert(
                    ChoiceGuard::Member(member.clone()),
                    vec![ChoiceGuard::Group(group.id.clone())],
                );
            }
        }
        let mut guarded = BTreeSet::new();
        for declaration in members {
            if !guarded.insert(declaration.member.clone()) {
                return Err(Error::DuplicateMemberGuards {
                    member: declaration.member,
                });
            }
            let owner = ChoiceGuard::Member(declaration.member.clone());
            if !owners.contains_key(&declaration.member) {
                return Err(Error::UnknownMemberGuards {
                    member: declaration.member,
                });
            }
            validate_guards(&owner, &declaration.guards, &dependencies)?;
            if let Some(guards) = dependencies.get_mut(&owner) {
                // Explicitly spelling the already implicit group guard is
                // redundant, not a duplicate authored reference.
                for guard in declaration.guards {
                    if !guards.contains(&guard) {
                        guards.push(guard);
                    }
                }
            }
        }
        for group in &groups {
            validate_guards(
                &ChoiceGuard::Group(group.id.clone()),
                &group.guards,
                &dependencies,
            )?;
        }
        let order = order(&dependencies)?;
        Ok(Self {
            features,
            groups,
            owners,
            group_ids,
            dependencies,
            order,
        })
    }
}

fn validate_guards(
    owner: &ChoiceGuard,
    guards: &[ChoiceGuard],
    graph: &BTreeMap<ChoiceGuard, Vec<ChoiceGuard>>,
) -> Result<(), Error> {
    let mut seen = BTreeSet::new();
    for guard in guards {
        if !seen.insert(guard) {
            return Err(Error::DuplicateGuard {
                owner: owner.clone(),
                guard: guard.clone(),
            });
        }
        if !graph.contains_key(guard) {
            return Err(Error::UnknownGuard {
                owner: owner.clone(),
                guard: guard.clone(),
            });
        }
    }
    Ok(())
}

fn order(graph: &BTreeMap<ChoiceGuard, Vec<ChoiceGuard>>) -> Result<Vec<ChoiceGuard>, Error> {
    let mut done = BTreeSet::new();
    let mut result = Vec::new();
    while result.len() < graph.len() {
        let next = graph.iter().find(|(node, deps)| {
            !done.contains(*node) && deps.iter().all(|dep| done.contains(dep))
        });
        let Some((node, _)) = next else {
            return Err(Error::GuardCycle {
                blocked: graph
                    .keys()
                    .filter(|node| !done.contains(*node))
                    .cloned()
                    .collect(),
            });
        };
        done.insert(node.clone());
        result.push(node.clone());
    }
    Ok(result)
}
