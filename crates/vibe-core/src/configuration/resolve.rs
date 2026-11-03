//! Monotone feature closure: exhaust ordinary implications, then select every
//! eligible fallback from the same snapshot. Defaults never race a requires
//! chain or another group's eligibility within a round.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL");

use std::collections::BTreeSet;

use super::model::Group;
use super::{
    EvaluationError as Error, FeatureBase, FeatureInput, FeatureModel, FeatureResolution,
    Implication, ImplicationReason as Reason,
};

struct Selection {
    selected: BTreeSet<usize>,
    reasons: Vec<Option<Reason>>,
}

impl Selection {
    fn add(&mut self, index: usize, reason: Reason) -> bool {
        if self.selected.insert(index) {
            self.reasons[index] = Some(reason);
            true
        } else {
            false
        }
    }

    fn active(&self, group: &Group) -> bool {
        group
            .activation
            .as_ref()
            .is_none_or(|ids| ids.iter().any(|id| self.selected.contains(id)))
    }
}

impl FeatureModel {
    /// Resolve one supplied local selection. No I/O or effect is performed.
    /// Fallbacks are a separate simultaneous round after ordinary closure;
    /// incompatible resulting cardinalities refuse, without default backtracking.
    ///
    /// ```
    /// use vibe_core::configuration::{FeatureModel, FeatureBase, FeatureInput};
    /// let model = FeatureModel::new(vec![], vec![])?;
    /// let result = model.resolve(&FeatureInput {
    ///     base: FeatureBase::Selected(vec![]), additions: vec![], disabled: vec![],
    /// })?;
    /// assert!(result.selected.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL")]
    pub fn resolve(&self, input: &FeatureInput) -> Result<FeatureResolution, Error> {
        let base = match &input.base {
            FeatureBase::Defaults => self
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.declaration.default)
                .map(|(i, _)| i)
                .collect(),
            FeatureBase::Selected(ids) => self.input_ids(ids)?,
        };
        let explicit = self.input_ids(&input.additions)?;
        let disabled = input
            .disabled
            .iter()
            .map(|id| self.input_id(id))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(id) = input
            .additions
            .iter()
            .find(|id| input.disabled.contains(id))
        {
            return Err(Error::EnabledAndDisabled { id: id.clone() });
        }
        let mut selected: BTreeSet<usize> = base.union(&explicit).copied().collect();
        for &index in &disabled {
            let descendant = self.ids.values().copied().find(|&candidate| {
                explicit.contains(&candidate) && self.descendant(candidate, index)
            });
            if let Some(descendant) = descendant {
                return Err(Error::DisabledAncestor {
                    feature: self.name(index),
                    descendant: self.name(descendant),
                });
            }
            self.remove_descendants(&mut selected, index);
        }
        for group in &self.groups {
            if !group.declaration.cardinality.at_most_one() {
                continue;
            }
            let chosen: Vec<_> = group
                .members
                .iter()
                .copied()
                .filter(|id| explicit.contains(id))
                .collect();
            match chosen.as_slice() {
                [] => {}
                [only] => {
                    for &member in group.members.iter().filter(|member| *member != only) {
                        if let Some(descendant) = self.ids.values().copied().find(|&candidate| {
                            explicit.contains(&candidate) && self.descendant(candidate, member)
                        }) {
                            return Err(Error::ExplicitReplacement {
                                group: group.declaration.id.clone(),
                                selected: self.name(*only),
                                member: self.name(member),
                                descendant: self.name(descendant),
                            });
                        }
                        self.remove_descendants(&mut selected, member);
                    }
                }
                _ => {
                    return Err(Error::ExplicitCardinality {
                        group: group.declaration.id.clone(),
                        cardinality: group.declaration.cardinality,
                        members: chosen.into_iter().map(|id| self.name(id)).collect(),
                    });
                }
            }
        }
        let mut state = Selection {
            selected,
            reasons: vec![None; self.nodes.len()],
        };
        self.close(&mut state);
        for group in &self.groups {
            if !state.active(group) {
                continue;
            }
            let members: Vec<_> = group
                .members
                .iter()
                .copied()
                .filter(|id| state.selected.contains(id))
                .collect();
            if !group.declaration.cardinality.accepts(members.len()) {
                return Err(Error::Cardinality {
                    group: group.declaration.id.clone(),
                    cardinality: group.declaration.cardinality,
                    members: members.into_iter().map(|id| self.name(id)).collect(),
                });
            }
        }
        for index in disabled {
            if let Some(reason) = &state.reasons[index] {
                return Err(Error::DisabledImplied {
                    feature: self.name(index),
                    reason: reason.clone(),
                });
            }
        }
        Ok(FeatureResolution {
            selected: self
                .nodes
                .iter()
                .enumerate()
                .filter(|(id, _)| state.selected.contains(id))
                .map(|(_, node)| node.declaration.id.clone())
                .collect(),
            implied: state
                .reasons
                .into_iter()
                .enumerate()
                .filter_map(|(id, reason)| {
                    reason.map(|reason| Implication {
                        feature: self.name(id),
                        reason,
                    })
                })
                .collect(),
        })
    }

    fn input_id(&self, id: &str) -> Result<usize, Error> {
        self.ids
            .get(id)
            .copied()
            .ok_or_else(|| Error::UnknownFeature { id: id.into() })
    }

    fn input_ids(&self, ids: &[String]) -> Result<BTreeSet<usize>, Error> {
        ids.iter().map(|id| self.input_id(id)).collect()
    }

    fn name(&self, index: usize) -> String {
        self.nodes[index].declaration.id.clone()
    }

    fn remove_descendants(&self, selected: &mut BTreeSet<usize>, ancestor: usize) {
        selected.retain(|&index| index != ancestor && !self.descendant(index, ancestor));
    }

    fn ordinary_round(&self, state: &mut Selection) -> bool {
        let mut changed = false;
        for (index, node) in self.nodes.iter().enumerate() {
            let selected = state.selected.contains(&index);
            if selected {
                if let Some(parent) = node.parent {
                    changed |= state.add(
                        parent,
                        Reason::Parent {
                            child: self.name(index),
                        },
                    );
                }
                for &required in &node.requires {
                    changed |= state.add(
                        required,
                        Reason::Requires {
                            feature: self.name(index),
                        },
                    );
                }
            }
            if node.declaration.required
                && !selected
                && node
                    .parent
                    .is_none_or(|parent| state.selected.contains(&parent))
            {
                changed |= state.add(
                    index,
                    Reason::Required {
                        parent: node.parent.map(|id| self.name(id)),
                    },
                );
            }
        }
        changed
    }

    fn close(&self, state: &mut Selection) {
        loop {
            while self.ordinary_round(state) {}
            // Collect without mutating: all eligibility tests see the same
            // completed ordinary closure, independent of group iteration order.
            let defaults: Vec<_> = self
                .groups
                .iter()
                .filter(|group| {
                    state.active(group)
                        && !group.members.iter().any(|id| state.selected.contains(id))
                })
                .flat_map(|group| {
                    group.fallback.iter().map(|&index| {
                        (
                            index,
                            Reason::GroupDefault {
                                group: group.declaration.id.clone(),
                            },
                        )
                    })
                })
                .collect();
            let mut changed = false;
            for (index, reason) in defaults {
                changed |= state.add(index, reason);
            }
            if !changed {
                return;
            }
        }
    }
}
