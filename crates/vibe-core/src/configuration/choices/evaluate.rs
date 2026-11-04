//! Original answers, provisional controls, and the resolved-only feature boundary.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE");

use super::super::FeatureInput;
use super::{
    AnswerError, ChoiceConflict, ChoiceEvaluation, ChoiceGuard, ChoiceModel, ChoiceOutcome,
    ChoicePolicy, ChoiceProjection, ChoiceQuestion, ConsumerChoices, DefaultUse, GroupControl,
    MemberControl, MergeIntent, ProjectionOrigin, Selection,
};
use std::collections::{BTreeMap, BTreeSet};

struct Seed {
    consumer: String,
    intent: MergeIntent,
    origin: ProjectionOrigin,
    members: BTreeSet<String>,
}

struct GuardFacts {
    active: Vec<bool>,
    eligible: BTreeMap<String, bool>,
    blocked: BTreeMap<ChoiceGuard, Vec<ChoiceGuard>>,
}

impl ChoiceModel {
    /// Evaluate already-grouped consumers without authorizing, persisting or
    /// running anything. Ordinary input validation runs alongside choice
    /// diagnosis. Only resolved inputs invoke feature closure, exactly once;
    /// its result must preserve the chosen choice-member set.
    ///
    /// ```
    /// use vibe_core::configuration::{ChoiceModel, FeatureModel, FeatureInput, FeatureBase, ChoiceOutcome};
    /// let model = ChoiceModel::new(FeatureModel::new(vec![], vec![])?, vec![], vec![])?;
    /// let result = model.evaluate(&FeatureInput { base: FeatureBase::Selected(vec![]),
    ///     additions: vec![], disabled: vec![] }, &[]);
    /// assert!(matches!(result.outcome, ChoiceOutcome::Resolved { .. }));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE")]
    pub fn evaluate(
        &self,
        ordinary: &FeatureInput,
        consumers: &[ConsumerChoices],
    ) -> ChoiceEvaluation {
        let mut result = ChoiceEvaluation {
            original: consumers.to_vec(),
            controls: vec![],
            defaults: vec![],
            questions: vec![],
            conflicts: vec![],
            invalid_answers: self.validate_answers(consumers),
            outcome: ChoiceOutcome::Unresolved,
        };
        if let Err(error) = self.features.validate_input(ordinary) {
            result
                .conflicts
                .push(ChoiceConflict::FeatureInput { error });
        }
        let seeds = self.seeds(consumers);
        let guards = self.guards(&seeds);
        for (index, group) in self.groups.iter().enumerate() {
            let projections = seeds[index]
                .iter()
                .map(|seed| ChoiceProjection {
                    consumer: seed.consumer.clone(),
                    intent: seed.intent,
                    origin: seed.origin,
                    eligible: group
                        .members
                        .iter()
                        .filter(|id| {
                            seed.members.contains(*id) && guards.eligible.get(*id) == Some(&true)
                        })
                        .cloned()
                        .collect(),
                    dormant: group
                        .members
                        .iter()
                        .filter(|id| {
                            seed.members.contains(*id) && guards.eligible.get(*id) != Some(&true)
                        })
                        .cloned()
                        .collect(),
                })
                .collect::<Vec<_>>();
            let mut control = GroupControl {
                id: group.id.clone(),
                active: guards.active[index],
                cardinality: group.cardinality,
                members: group
                    .members
                    .iter()
                    .map(|id| MemberControl {
                        id: id.clone(),
                        eligible: guards.eligible.get(id) == Some(&true),
                        blocked_by: guards.blocked[&ChoiceGuard::Member(id.clone())].clone(),
                        requested_by: seeds[index]
                            .iter()
                            .filter(|seed| seed.members.contains(id))
                            .map(|seed| seed.consumer.clone())
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .collect(),
                    })
                    .collect(),
                projections,
                selection: None,
                blocked_by: guards.blocked[&ChoiceGuard::Group(group.id.clone())].clone(),
            };
            if control.active {
                for seed in &seeds[index] {
                    if seed.origin == ProjectionOrigin::DeclaredDefaults
                        && let Some(declared) = &group.defaults
                    {
                        result.defaults.push(DefaultUse {
                            consumer: seed.consumer.clone(),
                            group: group.id.clone(),
                            declared: declared.clone(),
                            intent: seed.intent,
                        });
                    }
                }
                control.selection =
                    super::merge::merge(group, &control.projections, &mut result.conflicts);
                if let Some(selected) = &control.selection
                    && !group.cardinality.accepts(selected.len())
                {
                    let contributors = control
                        .projections
                        .iter()
                        .filter(|p| selected.is_empty() || !p.eligible.is_empty())
                        .map(|p| p.consumer.clone())
                        .collect();
                    if selected.is_empty() {
                        result.questions.push(ChoiceQuestion {
                            group: group.id.clone(),
                            cardinality: group.cardinality,
                            eligible_members: control
                                .members
                                .iter()
                                .filter(|m| m.eligible)
                                .map(|m| m.id.clone())
                                .collect(),
                            consumers: contributors,
                        });
                    } else {
                        result.conflicts.push(ChoiceConflict::Cardinality {
                            group: group.id.clone(),
                            cardinality: group.cardinality,
                            members: selected.clone(),
                            consumers: contributors,
                        });
                    }
                }
            }
            result.controls.push(control);
        }
        if result.invalid_answers.is_empty()
            && result.questions.is_empty()
            && result.conflicts.is_empty()
        {
            self.finish(ordinary, &mut result);
        }
        result
    }

    fn finish(&self, ordinary: &FeatureInput, result: &mut ChoiceEvaluation) {
        let selected: Vec<_> = result
            .controls
            .iter()
            .filter_map(|g| g.selection.as_ref())
            .flat_map(|members| members.iter().cloned())
            .collect();
        let features = match self.features.resolve_with_seeds(ordinary, &selected) {
            Ok(features) => features,
            Err(error) => {
                result
                    .conflicts
                    .push(ChoiceConflict::FeatureClosure { error });
                return;
            }
        };
        let chosen: BTreeSet<_> = selected.iter().collect();
        let actual: BTreeSet<_> = features
            .selected
            .iter()
            .filter(|id| self.owners.contains_key(*id))
            .collect();
        let unchosen: Vec<_> = features
            .selected
            .iter()
            .filter(|id| actual.contains(id) && !chosen.contains(id))
            .cloned()
            .collect();
        let lost: Vec<_> = selected
            .iter()
            .filter(|id| !actual.contains(id))
            .cloned()
            .collect();
        if !unchosen.is_empty() {
            result
                .conflicts
                .push(ChoiceConflict::UnchosenActivation { members: unchosen });
        }
        if !lost.is_empty() {
            result
                .conflicts
                .push(ChoiceConflict::ChosenLoss { members: lost });
        }
        if result.conflicts.is_empty() {
            result.outcome = ChoiceOutcome::Resolved { features };
        }
    }

    fn validate_answers(&self, consumers: &[ConsumerChoices]) -> Vec<AnswerError> {
        let mut errors = Vec::new();
        let mut keys = BTreeSet::new();
        for consumer in consumers {
            if consumer.key.trim().is_empty() || consumer.key.chars().any(char::is_control) {
                errors.push(AnswerError::InvalidConsumer {
                    consumer: consumer.key.clone(),
                });
            }
            if !keys.insert(&consumer.key) {
                errors.push(AnswerError::DuplicateConsumer {
                    consumer: consumer.key.clone(),
                });
            }
            for (id, answer) in &consumer.answers {
                let Some(&index) = self.group_ids.get(id) else {
                    errors.push(AnswerError::UnknownGroup {
                        consumer: consumer.key.clone(),
                        group: id.clone(),
                    });
                    continue;
                };
                if let Selection::Explicit(members) = &answer.selection {
                    let mut seen = BTreeSet::new();
                    for member in members {
                        if !seen.insert(member) {
                            errors.push(AnswerError::DuplicateMember {
                                consumer: consumer.key.clone(),
                                group: id.clone(),
                                member: member.clone(),
                            });
                        } else if !self.groups[index].members.contains(member) {
                            errors.push(AnswerError::UnknownMember {
                                consumer: consumer.key.clone(),
                                group: id.clone(),
                                member: member.clone(),
                            });
                        }
                    }
                }
            }
        }
        errors
    }

    fn seeds(&self, consumers: &[ConsumerChoices]) -> Vec<Vec<Seed>> {
        let mut ordered: Vec<_> = consumers.iter().collect();
        ordered.sort_by(|a, b| a.key.cmp(&b.key));
        self.groups
            .iter()
            .map(|group| {
                ordered
                    .iter()
                    .map(|consumer| {
                        let answer = consumer.answers.get(&group.id);
                        let intent = answer.map_or(MergeIntent::Additive, |a| a.intent);
                        let (origin, members) = match answer.map(|a| &a.selection) {
                            Some(Selection::Explicit(members)) => {
                                (ProjectionOrigin::Explicit, members.as_slice())
                            }
                            _ if consumer.policy == ChoicePolicy::UseDeclaredDefaults => {
                                match &group.defaults {
                                    Some(members) => {
                                        (ProjectionOrigin::DeclaredDefaults, members.as_slice())
                                    }
                                    None => (ProjectionOrigin::Unanswered, &[][..]),
                                }
                            }
                            _ => (ProjectionOrigin::Unanswered, &[][..]),
                        };
                        Seed {
                            consumer: consumer.key.clone(),
                            intent,
                            origin,
                            members: members
                                .iter()
                                .filter(|id| group.members.contains(id))
                                .cloned()
                                .collect(),
                        }
                    })
                    .collect()
            })
            .collect()
    }

    fn guards(&self, seeds: &[Vec<Seed>]) -> GuardFacts {
        let mut truth = BTreeMap::new();
        let mut active = vec![false; self.groups.len()];
        let mut eligible = BTreeMap::new();
        let mut blocked = BTreeMap::new();
        for node in &self.order {
            let unmet: Vec<_> = self.dependencies[node]
                .iter()
                .filter(|guard| truth.get(*guard) != Some(&true))
                .cloned()
                .collect();
            let holds = unmet.is_empty();
            let value = match node {
                ChoiceGuard::Group(id) => {
                    active[self.group_ids[id]] = holds;
                    holds
                }
                ChoiceGuard::Member(id) => {
                    eligible.insert(id.clone(), holds);
                    holds
                        && seeds[self.owners[id]]
                            .iter()
                            .any(|seed| seed.members.contains(id))
                }
            };
            truth.insert(node.clone(), value);
            blocked.insert(node.clone(), unmet);
        }
        GuardFacts {
            active,
            eligible,
            blocked,
        }
    }
}
