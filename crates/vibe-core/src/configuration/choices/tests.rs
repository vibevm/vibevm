//! Named finite discriminators for local intent and its resolved-only boundary.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE");

use super::super::{
    Cardinality, ClosureGroup, FeatureBase, FeatureInput, FeatureModel, FeatureNode,
    ImplicationReason,
};
use super::*;
use std::collections::BTreeMap;

fn feature(id: &str) -> FeatureNode {
    FeatureNode {
        id: id.into(),
        default: false,
        parent: None,
        required: false,
        requires: vec![],
    }
}
fn group(id: &str, members: &[&str], cardinality: Cardinality) -> ChoiceGroup {
    ChoiceGroup {
        id: id.into(),
        members: members.iter().map(|m| (*m).into()).collect(),
        cardinality,
        defaults: None,
        guards: vec![],
    }
}
fn model(groups: Vec<ChoiceGroup>, guards: Vec<MemberGuards>) -> ChoiceModel {
    let names: std::collections::BTreeSet<_> = groups.iter().flat_map(|g| &g.members).collect();
    ChoiceModel::new(
        FeatureModel::new(names.into_iter().map(|id| feature(id)).collect(), vec![]).unwrap(),
        groups,
        guards,
    )
    .unwrap()
}
fn answer(members: &[&str], intent: MergeIntent) -> ChoiceAnswer {
    ChoiceAnswer {
        selection: Selection::Explicit(members.iter().map(|m| (*m).into()).collect()),
        intent,
    }
}
fn consumer(key: &str, policy: ChoicePolicy, answers: &[(&str, ChoiceAnswer)]) -> ConsumerChoices {
    ConsumerChoices {
        key: key.into(),
        policy,
        answers: answers
            .iter()
            .map(|(id, answer)| ((*id).into(), answer.clone()))
            .collect(),
    }
}
fn ordinary() -> FeatureInput {
    FeatureInput {
        base: FeatureBase::Selected(vec![]),
        additions: vec![],
        disabled: vec![],
    }
}
fn selected(result: &ChoiceEvaluation) -> &[String] {
    match &result.outcome {
        ChoiceOutcome::Resolved { features } => &features.selected,
        _ => panic!("unresolved: {result:?}"),
    }
}
fn unresolved(result: &ChoiceEvaluation) {
    assert_eq!(result.outcome, ChoiceOutcome::Unresolved);
}

#[test]
fn cardinalities_distinguish_questions_overflow_and_resolved_controls() {
    for (cardinality, accepts) in [
        (Cardinality::ZeroOrMore, [true, true, true]),
        (Cardinality::OneOrMore, [false, true, true]),
        (Cardinality::ZeroOrOne, [true, true, false]),
        (Cardinality::ExactlyOne, [false, true, false]),
    ] {
        let model = model(vec![group("g", &["a", "b"], cardinality)], vec![]);
        for (index, members) in [&[][..], &["a"][..], &["a", "b"][..]]
            .into_iter()
            .enumerate()
        {
            let c = consumer(
                "app",
                ChoicePolicy::RequireSelection,
                &[("g", answer(members, MergeIntent::Additive))],
            );
            let r = model.evaluate(&ordinary(), std::slice::from_ref(&c));
            assert_eq!(r.original, [c]);
            assert_eq!(
                r.controls[0].selection,
                Some(members.iter().map(|m| (*m).into()).collect())
            );
            if accepts[index] {
                assert_eq!(selected(&r), members);
            } else {
                unresolved(&r);
                if members.is_empty() {
                    assert_eq!(
                        r.questions,
                        [ChoiceQuestion {
                            group: "g".into(),
                            cardinality,
                            eligible_members: vec!["a".into(), "b".into()],
                            consumers: vec!["app".into()]
                        }]
                    );
                } else {
                    assert_eq!(
                        r.conflicts,
                        [ChoiceConflict::Cardinality {
                            group: "g".into(),
                            cardinality,
                            members: vec!["a".into(), "b".into()],
                            consumers: vec!["app".into()]
                        }]
                    );
                }
            }
        }
    }
}

#[test]
fn omitted_defaults_keep_strength_and_explicit_empty_never_defaults() {
    let mut g = group("plugins", &["a", "b"], Cardinality::ZeroOrMore);
    g.defaults = Some(vec!["a".into()]);
    let model = model(vec![g], vec![]);
    let b = consumer(
        "B",
        ChoicePolicy::RequireSelection,
        &[("plugins", answer(&["b"], MergeIntent::Additive))],
    );
    let a = consumer("A", ChoicePolicy::UseDeclaredDefaults, &[]);
    let r = model.evaluate(&ordinary(), &[b.clone(), a.clone()]);
    assert_eq!(selected(&r), ["a", "b"]);
    assert_eq!(
        r.defaults,
        [DefaultUse {
            consumer: "A".into(),
            group: "plugins".into(),
            declared: vec!["a".into()],
            intent: MergeIntent::Additive
        }]
    );
    let exact = consumer(
        "A",
        ChoicePolicy::UseDeclaredDefaults,
        &[(
            "plugins",
            ChoiceAnswer {
                selection: Selection::Omitted,
                intent: MergeIntent::Exact,
            },
        )],
    );
    let r = model.evaluate(&ordinary(), &[exact, b.clone()]);
    unresolved(&r);
    assert_eq!(
        r.conflicts,
        [ChoiceConflict::AdditiveOutsideExact {
            group: "plugins".into(),
            exact_consumer: "A".into(),
            exact_members: vec!["a".into()],
            additive_consumer: "B".into(),
            outside: vec!["b".into()]
        }]
    );
    assert_eq!(r.defaults[0].intent, MergeIntent::Exact);
    for intent in [MergeIntent::Additive, MergeIntent::Exact] {
        let explicit = consumer(
            "A",
            ChoicePolicy::UseDeclaredDefaults,
            &[("plugins", answer(&[], intent))],
        );
        let r = model.evaluate(&ordinary(), &[explicit]);
        assert!(selected(&r).is_empty());
        assert!(r.defaults.is_empty());
    }
    let exact_empty = consumer(
        "A",
        ChoicePolicy::UseDeclaredDefaults,
        &[("plugins", answer(&[], MergeIntent::Exact))],
    );
    let r = model.evaluate(&ordinary(), &[exact_empty, b]);
    unresolved(&r);
    assert!(r.defaults.is_empty());
}

#[test]
fn absent_default_is_unanswered_but_empty_default_is_an_exact_bound() {
    for defaults in [None, Some(vec![])] {
        let mut g = group("g", &["a"], Cardinality::ZeroOrMore);
        g.defaults = defaults.clone();
        let model = model(vec![g], vec![]);
        let a = consumer(
            "A",
            ChoicePolicy::UseDeclaredDefaults,
            &[(
                "g",
                ChoiceAnswer {
                    selection: Selection::Omitted,
                    intent: MergeIntent::Exact,
                },
            )],
        );
        let b = consumer(
            "B",
            ChoicePolicy::RequireSelection,
            &[("g", answer(&["a"], MergeIntent::Additive))],
        );
        let r = model.evaluate(&ordinary(), &[a, b]);
        if defaults.is_none() {
            assert_eq!(selected(&r), ["a"]);
            assert!(r.defaults.is_empty());
        } else {
            unresolved(&r);
            assert_eq!(r.defaults[0].declared, Vec::<String>::new());
            assert!(matches!(
                r.conflicts[0],
                ChoiceConflict::AdditiveOutsideExact { .. }
            ));
        }
    }
    let mut g = group("g", &["a", "b"], Cardinality::ZeroOrMore);
    g.defaults = Some(vec!["a".into()]);
    let model = model(vec![g], vec![]);
    let a = consumer(
        "A",
        ChoicePolicy::RequireSelection,
        &[(
            "g",
            ChoiceAnswer {
                selection: Selection::Omitted,
                intent: MergeIntent::Exact,
            },
        )],
    );
    let b = consumer(
        "B",
        ChoicePolicy::RequireSelection,
        &[("g", answer(&["b"], MergeIntent::Additive))],
    );
    assert_eq!(selected(&model.evaluate(&ordinary(), &[a, b])), ["b"]);
}

#[test]
fn exact_union_bounds_are_set_based_and_consumer_order_independent() {
    let model = model(
        vec![group("g", &["a", "b", "c"], Cardinality::ZeroOrMore)],
        vec![],
    );
    let consumers = [
        consumer(
            "A",
            ChoicePolicy::RequireSelection,
            &[("g", answer(&["b", "a"], MergeIntent::Exact))],
        ),
        consumer(
            "B",
            ChoicePolicy::RequireSelection,
            &[("g", answer(&["a"], MergeIntent::Additive))],
        ),
        consumer(
            "C",
            ChoicePolicy::RequireSelection,
            &[("g", answer(&["a", "b"], MergeIntent::Exact))],
        ),
    ];
    let expected = model.evaluate(&ordinary(), &consumers);
    assert_eq!(selected(&expected), ["a", "b"]);
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let input: Vec<_> = order.iter().map(|i| consumers[*i].clone()).collect();
        let r = model.evaluate(&ordinary(), &input);
        assert_eq!(r.original, input);
        assert_eq!(r.outcome, expected.outcome);
        assert_eq!(r.controls, expected.controls);
    }
    let conflicting = [
        consumer(
            "B",
            ChoicePolicy::RequireSelection,
            &[("g", answer(&["b"], MergeIntent::Exact))],
        ),
        consumer(
            "A",
            ChoicePolicy::RequireSelection,
            &[("g", answer(&["a"], MergeIntent::Exact))],
        ),
    ];
    let first = model.evaluate(&ordinary(), &conflicting);
    let second = model.evaluate(
        &ordinary(),
        &[conflicting[1].clone(), conflicting[0].clone()],
    );
    assert_eq!(first.conflicts, second.conflicts);
    unresolved(&first);
    assert_eq!(
        first.conflicts[0],
        ChoiceConflict::ExactDisagreement {
            group: "g".into(),
            first_consumer: "A".into(),
            first_members: vec!["a".into()],
            second_consumer: "B".into(),
            second_members: vec!["b".into()]
        }
    );
    assert!(first.conflicts[0].to_string().contains("A"));
}

#[test]
fn dormant_exact_disagreement_survives_switch_away_and_back() {
    let gate = group("gate", &["off", "on"], Cardinality::ExactlyOne);
    let plugins = group("plugins", &["x"], Cardinality::ZeroOrMore);
    let model = model(
        vec![gate, plugins],
        vec![MemberGuards {
            member: "x".into(),
            guards: vec![ChoiceGuard::Member("on".into())],
        }],
    );
    let a = consumer(
        "A",
        ChoicePolicy::RequireSelection,
        &[
            ("gate", answer(&["off"], MergeIntent::Additive)),
            ("plugins", answer(&["x"], MergeIntent::Exact)),
        ],
    );
    let b = consumer(
        "B",
        ChoicePolicy::RequireSelection,
        &[("plugins", answer(&[], MergeIntent::Exact))],
    );
    let away = model.evaluate(&ordinary(), &[a.clone(), b.clone()]);
    assert_eq!(selected(&away), ["off"]);
    assert_eq!(away.controls[1].projections[0].dormant, ["x"]);
    assert_eq!(
        away.controls[1].members[0].blocked_by,
        [ChoiceGuard::Member("on".into())]
    );
    let mut on = a.clone();
    on.answers
        .insert("gate".into(), answer(&["on"], MergeIntent::Additive));
    let back = model.evaluate(&ordinary(), &[on, b.clone()]);
    unresolved(&back);
    assert!(matches!(
        back.conflicts[0],
        ChoiceConflict::ExactDisagreement { .. }
    ));
    assert_eq!(model.evaluate(&ordinary(), &[a, b]), away);
}

#[test]
fn inactive_groups_do_not_ask_but_unknown_dormant_answers_stay_invalid() {
    let gate = group("gate", &["on"], Cardinality::ZeroOrOne);
    let mut plugin = group("plugin", &["x"], Cardinality::ExactlyOne);
    plugin.guards = vec![ChoiceGuard::Member("on".into())];
    let model = model(vec![gate, plugin], vec![]);
    let dormant = consumer(
        "A",
        ChoicePolicy::RequireSelection,
        &[("plugin", answer(&["x"], MergeIntent::Exact))],
    );
    let r = model.evaluate(&ordinary(), std::slice::from_ref(&dormant));
    assert!(selected(&r).is_empty());
    assert!(r.questions.is_empty());
    assert!(!r.controls[1].active);
    assert_eq!(r.controls[1].blocked_by, [ChoiceGuard::Member("on".into())]);
    let invalid = consumer(
        "A",
        ChoicePolicy::RequireSelection,
        &[
            ("plugin", answer(&["removed"], MergeIntent::Exact)),
            ("removed-group", answer(&[], MergeIntent::Exact)),
        ],
    );
    let r = model.evaluate(&ordinary(), std::slice::from_ref(&invalid));
    unresolved(&r);
    assert_eq!(r.original, [invalid]);
    assert_eq!(r.invalid_answers.len(), 2);
    assert!(
        matches!(&r.invalid_answers[0], AnswerError::UnknownMember { member, .. } if member == "removed")
    );
}

#[test]
fn defaults_report_full_declared_set_while_member_eligibility_projects_it() {
    let gate = group("gate", &["off", "on"], Cardinality::ExactlyOne);
    let mut plugin = group("plugin", &["x", "y"], Cardinality::ZeroOrMore);
    plugin.defaults = Some(vec!["x".into(), "y".into()]);
    let model = model(
        vec![gate, plugin],
        vec![MemberGuards {
            member: "x".into(),
            guards: vec![ChoiceGuard::Member("on".into())],
        }],
    );
    let a = consumer(
        "A",
        ChoicePolicy::UseDeclaredDefaults,
        &[("gate", answer(&["off"], MergeIntent::Additive))],
    );
    let r = model.evaluate(&ordinary(), &[a]);
    assert_eq!(selected(&r), ["off", "y"]);
    assert_eq!(r.defaults[0].declared, ["x", "y"]);
    assert_eq!(r.controls[1].projections[0].eligible, ["y"]);
    assert_eq!(r.controls[1].projections[0].dormant, ["x"]);
}

#[test]
fn independent_questions_conflicts_and_stale_answers_are_collected_without_features() {
    let model = model(
        vec![
            group("missing", &["m"], Cardinality::ExactlyOne),
            group("overflow", &["a", "b"], Cardinality::ZeroOrOne),
            group("stale", &["s"], Cardinality::ZeroOrMore),
        ],
        vec![],
    );
    let c = consumer(
        "caller",
        ChoicePolicy::RequireSelection,
        &[
            ("overflow", answer(&["a", "b"], MergeIntent::Additive)),
            ("stale", answer(&["removed"], MergeIntent::Exact)),
        ],
    );
    let r = model.evaluate(&ordinary(), std::slice::from_ref(&c));
    unresolved(&r);
    assert_eq!(r.original, [c]);
    assert_eq!(r.questions[0].group, "missing");
    assert!(
        matches!(&r.conflicts[0], ChoiceConflict::Cardinality { group, consumers, .. } if group == "overflow" && consumers == &["caller"])
    );
    assert!(
        matches!(&r.invalid_answers[0], AnswerError::UnknownMember { member, .. } if member == "removed")
    );
}

#[test]
fn conflicting_upstream_requests_can_explain_downstream_defaults_but_never_resolve() {
    let gate = group("gate", &["a", "b"], Cardinality::ExactlyOne);
    let mut child = group("child", &["x"], Cardinality::ExactlyOne);
    child.guards = vec![ChoiceGuard::Member("a".into())];
    child.defaults = Some(vec!["x".into()]);
    let model = model(vec![gate, child], vec![]);
    let a = consumer(
        "A",
        ChoicePolicy::UseDeclaredDefaults,
        &[("gate", answer(&["a"], MergeIntent::Exact))],
    );
    let b = consumer(
        "B",
        ChoicePolicy::UseDeclaredDefaults,
        &[("gate", answer(&["b"], MergeIntent::Exact))],
    );
    let r = model.evaluate(&ordinary(), &[a, b]);
    unresolved(&r);
    assert!(r.controls[1].active);
    assert_eq!(r.defaults.len(), 2);
    assert!(
        r.defaults
            .iter()
            .all(|d| d.group == "child" && d.declared == ["x"])
    );
}

#[cfg(test)]
mod composition;
#[cfg(test)]
mod validation;

#[test]
fn review_ordinary_input_errors_are_visible_beside_choice_questions() {
    let model = model(vec![group("g", &["a"], Cardinality::ExactlyOne)], vec![]);
    let unanswered = [consumer("caller", ChoicePolicy::RequireSelection, &[])];
    for (input, expected) in [
        (
            FeatureInput {
                additions: vec!["unknown".into()],
                ..ordinary()
            },
            super::super::EvaluationError::UnknownFeature {
                id: "unknown".into(),
            },
        ),
        (
            FeatureInput {
                additions: vec!["a".into()],
                disabled: vec!["a".into()],
                ..ordinary()
            },
            super::super::EvaluationError::EnabledAndDisabled { id: "a".into() },
        ),
    ] {
        let r = model.evaluate(&input, &unanswered);
        unresolved(&r);
        assert_eq!(r.questions[0].group, "g");
        assert!(
            r.conflicts.iter().any(|conflict| matches!(conflict,
            ChoiceConflict::FeatureInput { error } if error == &expected)),
            "{r:?}"
        );
    }
}

#[test]
fn review_overflow_names_only_nonempty_contributors() {
    let model = model(
        vec![group("g", &["a", "b"], Cardinality::ZeroOrOne)],
        vec![],
    );
    let consumers = [
        consumer(
            "A",
            ChoicePolicy::RequireSelection,
            &[("g", answer(&["a"], MergeIntent::Additive))],
        ),
        consumer(
            "B",
            ChoicePolicy::RequireSelection,
            &[("g", answer(&["b"], MergeIntent::Additive))],
        ),
        consumer("uninvolved", ChoicePolicy::RequireSelection, &[]),
    ];
    let r = model.evaluate(&ordinary(), &consumers);
    unresolved(&r);
    assert_eq!(
        r.conflicts,
        [ChoiceConflict::Cardinality {
            group: "g".into(),
            cardinality: Cardinality::ZeroOrOne,
            members: vec!["a".into(), "b".into()],
            consumers: vec!["A".into(), "B".into()]
        }]
    );
}
