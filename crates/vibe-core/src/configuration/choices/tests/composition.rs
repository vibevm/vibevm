//! The choice boundary delegates closure once and refuses a changed member set.
use super::*;

#[test]
fn resolved_choices_seed_the_base_and_preserve_ordinary_implication_reasons() {
    let mut a = feature("a");
    a.requires = vec!["helper".into()];
    let kernel = FeatureModel::new(vec![a, feature("helper")], vec![]).unwrap();
    let model = ChoiceModel::new(
        kernel,
        vec![group("g", &["a"], Cardinality::ExactlyOne)],
        vec![],
    )
    .unwrap();
    let c = consumer(
        "caller",
        ChoicePolicy::RequireSelection,
        &[("g", answer(&["a"], MergeIntent::Additive))],
    );
    let r = model.evaluate(&ordinary(), &[c]);
    let ChoiceOutcome::Resolved { features } = r.outcome else {
        panic!("{r:?}")
    };
    assert_eq!(features.selected, ["a", "helper"]);
    assert_eq!(features.implied[0].feature, "helper");
    assert_eq!(
        features.implied[0].reason,
        ImplicationReason::Requires {
            feature: "a".into()
        }
    );
}

#[test]
fn ordinary_implications_and_closure_fallbacks_cannot_authorize_choice_members() {
    let mut trigger = feature("trigger");
    trigger.requires = vec!["a".into()];
    let implied = FeatureModel::new(vec![feature("a"), feature("b"), trigger], vec![]).unwrap();
    let fallback = FeatureModel::new(
        vec![feature("a"), feature("b"), feature("trigger")],
        vec![ClosureGroup {
            id: "closure".into(),
            members: vec!["a".into(), "b".into()],
            cardinality: Cardinality::ExactlyOne,
            fallback: Some(vec!["a".into()]),
        }],
    )
    .unwrap();
    for kernel in [implied, fallback] {
        let model = ChoiceModel::new(
            kernel,
            vec![group("choice", &["a", "b"], Cardinality::ZeroOrMore)],
            vec![],
        )
        .unwrap();
        let ordinary = FeatureInput {
            base: FeatureBase::Selected(vec!["trigger".into()]),
            additions: vec![],
            disabled: vec![],
        };
        let r = model.evaluate(
            &ordinary,
            &[consumer("caller", ChoicePolicy::RequireSelection, &[])],
        );
        unresolved(&r);
        assert_eq!(
            r.conflicts,
            [ChoiceConflict::UnchosenActivation {
                members: vec!["a".into()]
            }]
        );
    }
}

#[test]
fn ordinary_defaults_do_not_activate_choice_guards() {
    let mut a = feature("a");
    a.default = true;
    let mut dependent = group("dependent", &["b"], Cardinality::ExactlyOne);
    dependent.defaults = Some(vec!["b".into()]);
    dependent.guards = vec![ChoiceGuard::Member("a".into())];
    let model = ChoiceModel::new(
        FeatureModel::new(vec![a, feature("b")], vec![]).unwrap(),
        vec![group("g", &["a"], Cardinality::ZeroOrOne), dependent],
        vec![],
    )
    .unwrap();
    let r = model.evaluate(
        &FeatureInput {
            base: FeatureBase::Defaults,
            additions: vec![],
            disabled: vec![],
        },
        &[consumer("caller", ChoicePolicy::UseDeclaredDefaults, &[])],
    );
    unresolved(&r);
    assert!(!r.controls[1].active);
    assert!(r.defaults.is_empty());
    assert_eq!(
        r.conflicts,
        [ChoiceConflict::UnchosenActivation {
            members: vec!["a".into()]
        }]
    );
}

#[test]
fn direct_override_and_disable_cannot_erase_the_chosen_set() {
    let kernel = FeatureModel::new(
        vec![feature("a"), feature("b")],
        vec![ClosureGroup {
            id: "closure".into(),
            members: vec!["a".into(), "b".into()],
            cardinality: Cardinality::ZeroOrOne,
            fallback: None,
        }],
    )
    .unwrap();
    let model = ChoiceModel::new(
        kernel,
        vec![group("g", &["a", "b"], Cardinality::ExactlyOne)],
        vec![],
    )
    .unwrap();
    let c = consumer(
        "caller",
        ChoicePolicy::RequireSelection,
        &[("g", answer(&["a"], MergeIntent::Exact))],
    );
    let override_input = FeatureInput {
        additions: vec!["b".into()],
        ..ordinary()
    };
    let overridden = model.evaluate(&override_input, std::slice::from_ref(&c));
    unresolved(&overridden);
    assert_eq!(
        overridden.conflicts,
        [
            ChoiceConflict::UnchosenActivation {
                members: vec!["b".into()]
            },
            ChoiceConflict::ChosenLoss {
                members: vec!["a".into()]
            }
        ]
    );
    let disabled = model.evaluate(
        &FeatureInput {
            disabled: vec!["a".into()],
            ..ordinary()
        },
        &[c],
    );
    unresolved(&disabled);
    assert_eq!(
        disabled.conflicts,
        [ChoiceConflict::ChosenLoss {
            members: vec!["a".into()]
        }]
    );
}

#[test]
fn ordinary_kernel_conflicts_keep_their_exact_cause_without_partial_output() {
    let mut a = feature("a");
    a.requires = vec!["needed".into()];
    let kernel = FeatureModel::new(vec![a, feature("needed")], vec![]).unwrap();
    let model = ChoiceModel::new(
        kernel,
        vec![group("g", &["a"], Cardinality::ExactlyOne)],
        vec![],
    )
    .unwrap();
    let c = consumer(
        "caller",
        ChoicePolicy::RequireSelection,
        &[("g", answer(&["a"], MergeIntent::Additive))],
    );
    let r = model.evaluate(
        &FeatureInput {
            disabled: vec!["needed".into()],
            ..ordinary()
        },
        &[c],
    );
    unresolved(&r);
    assert_eq!(
        r.conflicts,
        [ChoiceConflict::FeatureClosure {
            error: super::super::super::EvaluationError::DisabledImplied {
                feature: "needed".into(),
                reason: ImplicationReason::Requires {
                    feature: "a".into()
                },
            }
        }]
    );
}

#[test]
fn no_choice_groups_preserve_the_existing_kernel_behavior() {
    let mut parent = feature("parent");
    parent.required = true;
    let mut child = feature("child");
    child.parent = Some("parent".into());
    let kernel = FeatureModel::new(vec![parent, child], vec![]).unwrap();
    let request = FeatureInput {
        additions: vec!["child".into()],
        ..ordinary()
    };
    let expected = kernel.resolve(&request).unwrap();
    let model = ChoiceModel::new(kernel, vec![], vec![]).unwrap();
    assert_eq!(
        model.evaluate(&request, &[]).outcome,
        ChoiceOutcome::Resolved { features: expected }
    );
}

#[test]
fn group_guard_observes_activity_even_without_a_selected_member() {
    let mut dependent = group("dependent", &["x"], Cardinality::ExactlyOne);
    dependent.guards = vec![ChoiceGuard::Group("optional".into())];
    let model = model(
        vec![
            group("optional", &["on"], Cardinality::ZeroOrOne),
            dependent,
        ],
        vec![],
    );
    let r = model.evaluate(
        &ordinary(),
        &[consumer("caller", ChoicePolicy::RequireSelection, &[])],
    );
    unresolved(&r);
    assert!(r.controls[1].active);
    assert_eq!(r.questions[0].group, "dependent");
}
