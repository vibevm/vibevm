//! Preserve the existing evaluator's result, controls and diagnostic structure.
use super::vocabulary::{cardinality, guard, intent, policy};
use vibe_core::{configuration as k, manifest as m};
use vibe_wire::generated::shared as w;

fn choice_guard(g: &k::ChoiceGuard) -> w::ConfigurationGuard {
    guard(&match g {
        k::ChoiceGuard::Group(v) => m::ConfigurationGuard::Group(v.clone()),
        k::ChoiceGuard::Member(v) => m::ConfigurationGuard::Member(v.clone()),
    })
}
pub(super) fn implication(r: &k::ImplicationReason) -> w::ConfigurationImplicationReason {
    match r {
        k::ImplicationReason::Parent { child } => w::ConfigurationImplicationReason::Parent(
            Box::new(w::ConfigurationImplicationReasonParent {
                child: child.clone(),
            }),
        ),
        k::ImplicationReason::Requires { feature } => w::ConfigurationImplicationReason::Requires(
            Box::new(w::ConfigurationImplicationReasonRequires {
                feature: feature.clone(),
            }),
        ),
        k::ImplicationReason::Required { parent } => w::ConfigurationImplicationReason::Required(
            Box::new(w::ConfigurationImplicationReasonRequired {
                parent: parent.clone(),
            }),
        ),
        k::ImplicationReason::GroupDefault { group } => {
            w::ConfigurationImplicationReason::GroupDefault(Box::new(
                w::ConfigurationImplicationReasonGroupDefault {
                    group: group.clone(),
                },
            ))
        }
    }
}
pub(super) fn features(f: &k::FeatureResolution) -> w::ConfigurationFeatureResult {
    w::ConfigurationFeatureResult {
        selected: f.selected.clone(),
        implied: f
            .implied
            .iter()
            .map(|i| w::ConfigurationImplication {
                feature: i.feature.clone(),
                reason: implication(&i.reason),
            })
            .collect(),
    }
}
pub(super) fn project(
    e: k::ChoiceEvaluation,
    out: &mut w::ConfigurationEvaluation,
) -> Option<k::FeatureResolution> {
    out.normalized_consumers = e
        .original
        .iter()
        .map(|c| w::ConfigurationConsumer {
            key: c.key.clone(),
            policy: policy(&c.policy),
            answers: c
                .answers
                .iter()
                .map(|(id, a)| {
                    (
                        id.clone(),
                        w::ConfigurationSelection {
                            intent: intent(&a.intent),
                            members: match &a.selection {
                                k::Selection::Omitted => None,
                                k::Selection::Explicit(v) => Some(v.clone()),
                            },
                        },
                    )
                })
                .collect(),
        })
        .collect();
    out.controls = e
        .controls
        .iter()
        .map(|c| w::ConfigurationGroupControl {
            id: c.id.clone(),
            active: c.active,
            cardinality: cardinality(&c.cardinality),
            selection: c.selection.clone(),
            blocked_by: c.blocked_by.iter().map(choice_guard).collect(),
            members: c
                .members
                .iter()
                .map(|m| w::ConfigurationMemberControl {
                    id: m.id.clone(),
                    eligible: m.eligible,
                    requested_by: m.requested_by.clone(),
                    blocked_by: m.blocked_by.iter().map(choice_guard).collect(),
                })
                .collect(),
            projections: c
                .projections
                .iter()
                .map(|p| w::ConfigurationProjection {
                    consumer: p.consumer.clone(),
                    intent: intent(&p.intent),
                    eligible: p.eligible.clone(),
                    dormant: p.dormant.clone(),
                    origin: match p.origin {
                        k::ProjectionOrigin::Explicit => w::ConfigurationProjectionOrigin::Explicit,
                        k::ProjectionOrigin::Unanswered => {
                            w::ConfigurationProjectionOrigin::Unanswered
                        }
                        k::ProjectionOrigin::DeclaredDefaults => {
                            w::ConfigurationProjectionOrigin::DeclaredDefaults
                        }
                    },
                })
                .collect(),
        })
        .collect();
    out.defaults = e
        .defaults
        .iter()
        .map(|d| w::ConfigurationDefaultUse {
            consumer: d.consumer.clone(),
            group: d.group.clone(),
            declared: d.declared.clone(),
            intent: intent(&d.intent),
        })
        .collect();
    out.questions.extend(e.questions.iter().map(|q| {
        w::ConfigurationQuestion::Choice(Box::new(w::ConfigurationQuestionChoice {
            group: q.group.clone(),
            cardinality: cardinality(&q.cardinality),
            consumers: q.consumers.clone(),
            eligible_members: q.eligible_members.clone(),
        }))
    }));
    out.problems
        .extend(e.invalid_answers.iter().map(answer_error));
    out.problems.extend(e.conflicts.iter().map(conflict));
    match e.outcome {
        k::ChoiceOutcome::Resolved { features } => Some(features),
        k::ChoiceOutcome::Unresolved => None,
    }
}
fn answer_error(e: &k::AnswerError) -> w::ConfigurationProblem {
    match e {
        k::AnswerError::InvalidConsumer { consumer } => w::ConfigurationProblem::InvalidConsumer(
            Box::new(w::ConfigurationProblemInvalidConsumer {
                consumer: consumer.clone(),
            }),
        ),
        k::AnswerError::DuplicateConsumer { consumer } => {
            w::ConfigurationProblem::DuplicateConsumer(Box::new(
                w::ConfigurationProblemDuplicateConsumer {
                    consumer: consumer.clone(),
                },
            ))
        }
        k::AnswerError::UnknownGroup { consumer, group } => {
            w::ConfigurationProblem::UnknownGroup(Box::new(w::ConfigurationProblemUnknownGroup {
                consumer: consumer.clone(),
                group: group.clone(),
            }))
        }
        k::AnswerError::UnknownMember {
            consumer,
            group,
            member,
        } => {
            w::ConfigurationProblem::UnknownMember(Box::new(w::ConfigurationProblemUnknownMember {
                consumer: consumer.clone(),
                group: group.clone(),
                member: member.clone(),
            }))
        }
        k::AnswerError::DuplicateMember {
            consumer,
            group,
            member,
        } => w::ConfigurationProblem::DuplicateMember(Box::new(
            w::ConfigurationProblemDuplicateMember {
                consumer: consumer.clone(),
                group: group.clone(),
                member: member.clone(),
            },
        )),
    }
}
fn conflict(e: &k::ChoiceConflict) -> w::ConfigurationProblem {
    match e {
        k::ChoiceConflict::FeatureInput { error } => {
            feature_error(error, w::ConfigurationProblemFeatureStage::Input)
        }
        k::ChoiceConflict::FeatureClosure { error } => {
            feature_error(error, w::ConfigurationProblemFeatureStage::Closure)
        }
        k::ChoiceConflict::ExactDisagreement {
            group,
            first_consumer,
            first_members,
            second_consumer,
            second_members,
        } => w::ConfigurationProblem::ExactDisagreement(Box::new(
            w::ConfigurationProblemExactDisagreement {
                group: group.clone(),
                first_consumer: first_consumer.clone(),
                first_members: first_members.clone(),
                second_consumer: second_consumer.clone(),
                second_members: second_members.clone(),
            },
        )),
        k::ChoiceConflict::AdditiveOutsideExact {
            group,
            exact_consumer,
            exact_members,
            additive_consumer,
            outside,
        } => w::ConfigurationProblem::AdditiveOutsideExact(Box::new(
            w::ConfigurationProblemAdditiveOutsideExact {
                group: group.clone(),
                exact_consumer: exact_consumer.clone(),
                exact_members: exact_members.clone(),
                additive_consumer: additive_consumer.clone(),
                outside: outside.clone(),
            },
        )),
        k::ChoiceConflict::Cardinality {
            group,
            cardinality: c,
            members,
            consumers,
        } => w::ConfigurationProblem::Cardinality(Box::new(w::ConfigurationProblemCardinality {
            group: group.clone(),
            cardinality: cardinality(c),
            members: members.clone(),
            consumers: consumers.clone(),
        })),
        k::ChoiceConflict::ChosenLoss { members } => {
            w::ConfigurationProblem::ChosenLoss(Box::new(w::ConfigurationProblemChosenLoss {
                members: members.clone(),
            }))
        }
        k::ChoiceConflict::UnchosenActivation { members } => {
            w::ConfigurationProblem::UnchosenActivation(Box::new(
                w::ConfigurationProblemUnchosenActivation {
                    members: members.clone(),
                },
            ))
        }
    }
}
fn feature_error(
    e: &k::EvaluationError,
    stage: w::ConfigurationProblemFeatureStage,
) -> w::ConfigurationProblem {
    use w::ConfigurationProblemFeatureCode as Code;
    let mut p = w::ConfigurationProblemFeature {
        stage,
        code: Code::Unknown,
        feature: String::new(),
        members: vec![],
        reason: None,
        group: None,
        related: None,
    };
    match e {
        k::EvaluationError::UnknownFeature { id } => p.feature = id.clone(),
        k::EvaluationError::EnabledAndDisabled { id } => {
            p.code = Code::EnabledAndDisabled;
            p.feature = id.clone();
        }
        k::EvaluationError::DisabledAncestor {
            feature,
            descendant,
        } => {
            p.code = Code::DisabledAncestor;
            p.feature = feature.clone();
            p.related = Some(descendant.clone());
        }
        k::EvaluationError::DisabledImplied { feature, reason } => {
            p.code = Code::DisabledImplied;
            p.feature = feature.clone();
            p.reason = Some(implication(reason));
        }
        k::EvaluationError::ExplicitReplacement {
            group,
            selected,
            member,
            descendant,
        } => {
            p.code = Code::Replacement;
            p.group = Some(group.clone());
            p.feature = selected.clone();
            p.members = vec![member.clone()];
            p.related = Some(descendant.clone());
        }
        k::EvaluationError::ExplicitCardinality { group, members, .. } => {
            p.code = Code::ExplicitCardinality;
            p.group = Some(group.clone());
            p.members = members.clone();
        }
        k::EvaluationError::Cardinality { group, members, .. } => {
            p.code = Code::Cardinality;
            p.group = Some(group.clone());
            p.members = members.clone();
        }
    }
    w::ConfigurationProblem::Feature(Box::new(p))
}
