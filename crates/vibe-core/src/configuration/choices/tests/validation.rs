//! Validation is independent of whether a guard will be active in a request.
use super::*;

fn kernel() -> FeatureModel {
    FeatureModel::new(
        vec![feature("a"), feature("b"), feature("ordinary")],
        vec![],
    )
    .unwrap()
}

#[test]
fn group_and_member_guard_cycles_include_implicit_ownership() {
    let mut own = group("g", &["a"], Cardinality::ZeroOrOne);
    own.guards = vec![ChoiceGuard::Member("a".into())];
    let error = ChoiceModel::new(kernel(), vec![own], vec![]).unwrap_err();
    assert_eq!(
        error,
        ChoiceDefinitionError::GuardCycle {
            blocked: vec![
                ChoiceGuard::Group("g".into()),
                ChoiceGuard::Member("a".into())
            ]
        }
    );
    let mut first = group("first", &["a"], Cardinality::ZeroOrOne);
    first.guards = vec![ChoiceGuard::Group("second".into())];
    let mut second = group("second", &["b"], Cardinality::ZeroOrOne);
    second.guards = vec![ChoiceGuard::Group("first".into())];
    assert!(matches!(
        ChoiceModel::new(kernel(), vec![first, second], vec![]),
        Err(ChoiceDefinitionError::GuardCycle { .. })
    ));
    let guards = vec![
        MemberGuards {
            member: "a".into(),
            guards: vec![ChoiceGuard::Member("b".into())],
        },
        MemberGuards {
            member: "b".into(),
            guards: vec![ChoiceGuard::Member("a".into())],
        },
    ];
    assert!(matches!(
        ChoiceModel::new(
            kernel(),
            vec![group("g", &["a", "b"], Cardinality::ZeroOrMore)],
            guards
        ),
        Err(ChoiceDefinitionError::GuardCycle { .. })
    ));
}

#[test]
fn definitions_validate_ownership_defaults_and_guard_references() {
    let valid = group("g", &["a"], Cardinality::ExactlyOne);
    let mut bad_default = valid.clone();
    bad_default.defaults = Some(vec![]);
    assert_eq!(
        ChoiceModel::new(kernel(), vec![bad_default], vec![]).unwrap_err(),
        ChoiceDefinitionError::DefaultCardinality {
            group: "g".into(),
            cardinality: Cardinality::ExactlyOne,
            members: vec![]
        }
    );
    let duplicate = group("g", &["a", "a"], Cardinality::ZeroOrMore);
    assert!(matches!(
        ChoiceModel::new(kernel(), vec![duplicate], vec![]),
        Err(ChoiceDefinitionError::DuplicateMember { .. })
    ));
    assert!(matches!(
        ChoiceModel::new(kernel(), vec![valid.clone(), valid.clone()], vec![]),
        Err(ChoiceDefinitionError::DuplicateGroup { .. })
    ));
    assert!(matches!(
        ChoiceModel::new(
            kernel(),
            vec![
                valid.clone(),
                group("other", &["a"], Cardinality::ZeroOrOne)
            ],
            vec![]
        ),
        Err(ChoiceDefinitionError::MultipleOwners { .. })
    ));
    let mut unknown = valid.clone();
    unknown.guards = vec![ChoiceGuard::Member("ordinary".into())];
    assert_eq!(
        ChoiceModel::new(kernel(), vec![unknown], vec![]).unwrap_err(),
        ChoiceDefinitionError::UnknownGuard {
            owner: ChoiceGuard::Group("g".into()),
            guard: ChoiceGuard::Member("ordinary".into())
        }
    );
    let mut duplicate_default = valid.clone();
    duplicate_default.defaults = Some(vec!["a".into(), "a".into()]);
    assert!(matches!(
        ChoiceModel::new(kernel(), vec![duplicate_default], vec![]),
        Err(ChoiceDefinitionError::DuplicateDefault { .. })
    ));
    let mut foreign_default = valid.clone();
    foreign_default.defaults = Some(vec!["b".into()]);
    assert!(matches!(
        ChoiceModel::new(kernel(), vec![foreign_default], vec![]),
        Err(ChoiceDefinitionError::UnknownDefault { .. })
    ));
    assert!(matches!(
        ChoiceModel::new(
            kernel(),
            vec![group("g", &["missing"], Cardinality::ZeroOrOne)],
            vec![]
        ),
        Err(ChoiceDefinitionError::UnknownMember { .. })
    ));
    assert!(matches!(
        ChoiceModel::new(
            kernel(),
            vec![group("g", &[], Cardinality::ZeroOrMore)],
            vec![]
        ),
        Err(ChoiceDefinitionError::EmptyGroup { .. })
    ));
    assert!(matches!(
        ChoiceModel::new(
            kernel(),
            vec![group(" ", &["a"], Cardinality::ZeroOrOne)],
            vec![]
        ),
        Err(ChoiceDefinitionError::InvalidGroupId { .. })
    ));
    let duplicate_guard = vec![MemberGuards {
        member: "a".into(),
        guards: vec![
            ChoiceGuard::Group("g".into()),
            ChoiceGuard::Group("g".into()),
        ],
    }];
    assert!(matches!(
        ChoiceModel::new(kernel(), vec![valid.clone()], duplicate_guard),
        Err(ChoiceDefinitionError::DuplicateGuard { .. })
    ));
    let duplicate_row = vec![
        MemberGuards {
            member: "a".into(),
            guards: vec![],
        },
        MemberGuards {
            member: "a".into(),
            guards: vec![],
        },
    ];
    assert!(matches!(
        ChoiceModel::new(kernel(), vec![valid.clone()], duplicate_row),
        Err(ChoiceDefinitionError::DuplicateMemberGuards { .. })
    ));
    assert!(matches!(
        ChoiceModel::new(
            kernel(),
            vec![valid],
            vec![MemberGuards {
                member: "ordinary".into(),
                guards: vec![]
            }]
        ),
        Err(ChoiceDefinitionError::UnknownMemberGuards { .. })
    ));
}

#[test]
fn separate_namespaces_and_ordinary_requires_cycles_remain_valid() {
    let mut a = feature("a");
    a.requires = vec!["b".into()];
    let mut b = feature("b");
    b.requires = vec!["a".into()];
    let kernel = FeatureModel::new(vec![a, b], vec![]).unwrap();
    let model = ChoiceModel::new(
        kernel,
        vec![group("a", &["a"], Cardinality::ExactlyOne)],
        vec![MemberGuards {
            member: "a".into(),
            guards: vec![ChoiceGuard::Group("a".into())],
        }],
    )
    .unwrap();
    let c = consumer(
        "C",
        ChoicePolicy::RequireSelection,
        &[("a", answer(&["a"], MergeIntent::Exact))],
    );
    assert_eq!(selected(&model.evaluate(&ordinary(), &[c])), ["a", "b"]);
}

#[test]
fn invalid_consumers_and_duplicate_members_cannot_manufacture_success() {
    let model = model(vec![group("g", &["a"], Cardinality::ZeroOrMore)], vec![]);
    let a = consumer(
        "duplicate",
        ChoicePolicy::RequireSelection,
        &[("g", answer(&["a", "a"], MergeIntent::Additive))],
    );
    let inputs = vec![
        a.clone(),
        a,
        ConsumerChoices {
            key: "".into(),
            policy: ChoicePolicy::RequireSelection,
            answers: BTreeMap::new(),
        },
    ];
    let r = model.evaluate(&ordinary(), &inputs);
    unresolved(&r);
    assert_eq!(r.original, inputs);
    assert!(r.invalid_answers.iter().any(
        |e| matches!(e, AnswerError::DuplicateConsumer { consumer } if consumer == "duplicate")
    ));
    assert!(
        r.invalid_answers
            .iter()
            .any(|e| matches!(e, AnswerError::InvalidConsumer { consumer } if consumer.is_empty()))
    );
    assert_eq!(
        r.invalid_answers
            .iter()
            .filter(|e| matches!(e, AnswerError::DuplicateMember { .. }))
            .count(),
        2
    );
}
