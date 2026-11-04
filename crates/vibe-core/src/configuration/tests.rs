//! Finite semantic matrix, including the former premature-fallback defect.

use super::*;

#[test]
fn literal_identifiers_preserve_spaces_private_names_and_exact_matching() {
    let mut public = node("lint level");
    public.requires = vec!["_internal helper".into()];
    let group = ClosureGroup {
        id: " build mode ".into(),
        members: vec!["lint level".into(), "lint".into()],
        cardinality: Cardinality::ZeroOrOne,
        fallback: None,
    };
    let model = FeatureModel::new(
        vec![
            public,
            node("_internal helper"),
            node("lint"),
            node(" padded "),
        ],
        vec![group],
    )
    .unwrap();
    assert_eq!(
        model.resolve(&input(&["lint level"])).unwrap().selected,
        ["lint level", "_internal helper"]
    );
    assert_eq!(
        model.resolve(&input(&[" padded "])).unwrap().selected,
        [" padded "]
    );
    assert!(matches!(
        model.resolve(&input(&["padded"])),
        Err(EvaluationError::UnknownFeature { .. })
    ));
    assert_eq!(
        model.resolve(&input(&["lint level", "lint"])),
        Err(EvaluationError::Cardinality {
            group: " build mode ".into(),
            cardinality: Cardinality::ZeroOrOne,
            members: vec!["lint level".into(), "lint".into()]
        })
    );
}

#[test]
fn blank_and_control_identifiers_still_refuse_for_features_and_closure_groups() {
    for id in ["", " \u{a0} ", "line\nfeed", "tab\there", "zero\0byte"] {
        assert_eq!(
            FeatureModel::new(vec![node(id)], vec![])
                .unwrap_err()
                .problem,
            DefinitionProblem::InvalidIdentifier { id: id.into() }
        );
        let group = ClosureGroup {
            id: id.into(),
            members: vec!["a".into()],
            cardinality: Cardinality::ZeroOrOne,
            fallback: None,
        };
        assert_eq!(
            FeatureModel::new(vec![node("a")], vec![group])
                .unwrap_err()
                .problem,
            DefinitionProblem::InvalidIdentifier { id: id.into() }
        );
    }
}

fn node(id: &str) -> FeatureNode {
    FeatureNode {
        id: id.into(),
        default: false,
        parent: None,
        required: false,
        requires: vec![],
    }
}

fn group(id: &str, cardinality: Cardinality, fallback: Option<&[&str]>) -> ClosureGroup {
    ClosureGroup {
        id: id.into(),
        members: vec!["a".into(), "b".into()],
        cardinality,
        fallback: fallback.map(|ids| ids.iter().map(|id| (*id).into()).collect()),
    }
}

fn input(selected: &[&str]) -> FeatureInput {
    FeatureInput {
        base: FeatureBase::Selected(selected.iter().map(|id| (*id).into()).collect()),
        additions: vec![],
        disabled: vec![],
    }
}

#[test]
fn all_four_cardinalities_have_distinct_empty_single_multiple_results() {
    let rows = [
        (Cardinality::ZeroOrMore, [true, true, true]),
        (Cardinality::OneOrMore, [false, true, true]),
        (Cardinality::ZeroOrOne, [true, true, false]),
        (Cardinality::ExactlyOne, [false, true, false]),
    ];
    for (cardinality, expected) in rows {
        let model = FeatureModel::new(
            vec![node("a"), node("b")],
            vec![group("g", cardinality, None)],
        )
        .unwrap();
        for (selected, valid) in [
            (&[][..], expected[0]),
            (&["a"][..], expected[1]),
            (&["a", "b"][..], expected[2]),
        ] {
            let result = model.resolve(&input(selected));
            assert_eq!(
                result.is_ok(),
                valid,
                "{cardinality:?} {selected:?}: {result:?}"
            );
            if !valid {
                assert!(matches!(result, Err(EvaluationError::Cardinality { .. })));
            }
        }
    }
}

#[test]
fn active_fallback_is_separate_from_base_defaults_and_inactive_cardinality() {
    let mut a = node("a");
    a.default = true;
    a.parent = Some("on".into());
    let mut b = node("b");
    b.parent = Some("on".into());
    let g = group("g", Cardinality::ExactlyOne, Some(&["b"]));
    let model = FeatureModel::new(vec![a, b, node("on")], vec![g]).unwrap();
    assert!(model.resolve(&input(&[])).unwrap().selected.is_empty());
    let result = model.resolve(&input(&["on"])).unwrap();
    assert_eq!(result.selected, ["b", "on"]);
    assert_eq!(
        result.implied,
        [Implication {
            feature: "b".into(),
            reason: ImplicationReason::GroupDefault { group: "g".into() }
        }]
    );
    assert_eq!(
        model
            .resolve(&FeatureInput {
                base: FeatureBase::Defaults,
                additions: vec![],
                disabled: vec![]
            })
            .unwrap()
            .selected,
        ["a", "on"]
    );
}

#[test]
fn explicit_max_one_replacement_and_disabled_implication_are_structured() {
    let mut child = node("child");
    child.parent = Some("a".into());
    let mut b = node("b");
    b.requires = vec!["needed".into()];
    let model = FeatureModel::new(
        vec![node("a"), child, b, node("needed")],
        vec![group("g", Cardinality::ExactlyOne, Some(&["a"]))],
    )
    .unwrap();
    let request = FeatureInput {
        base: FeatureBase::Selected(vec!["a".into(), "child".into()]),
        additions: vec!["b".into()],
        disabled: vec![],
    };
    assert_eq!(model.resolve(&request).unwrap().selected, ["b", "needed"]);
    let disabled = FeatureInput {
        disabled: vec!["needed".into()],
        ..request
    };
    assert_eq!(
        model.resolve(&disabled),
        Err(EvaluationError::DisabledImplied {
            feature: "needed".into(),
            reason: ImplicationReason::Requires {
                feature: "b".into()
            },
        })
    );
    assert!(matches!(
        model.resolve(&FeatureInput {
            additions: vec!["a".into(), "b".into()],
            ..input(&[])
        }),
        Err(EvaluationError::ExplicitCardinality { .. })
    ));
}

#[test]
fn positive_requires_cycles_close_and_required_children_explain_in_order() {
    let mut a = node("a");
    a.requires = vec!["b".into()];
    let mut b = node("b");
    b.requires = vec!["a".into()];
    let mut child = node("child");
    child.parent = Some("a".into());
    child.required = true;
    let model = FeatureModel::new(vec![child, a, b], vec![]).unwrap();
    let result = model.resolve(&input(&["a"])).unwrap();
    assert_eq!(result.selected, ["child", "a", "b"]);
    assert_eq!(
        result.implied,
        [
            Implication {
                feature: "child".into(),
                reason: ImplicationReason::Required {
                    parent: Some("a".into())
                }
            },
            Implication {
                feature: "b".into(),
                reason: ImplicationReason::Requires {
                    feature: "a".into()
                }
            },
        ]
    );
}

#[test]
fn ordinary_closure_finishes_before_fallback_under_declaration_permutations() {
    let mut root = node("root");
    root.requires = vec!["relay".into()];
    let mut relay = node("relay");
    relay.requires = vec!["b".into()];
    let orders = [
        vec![relay.clone(), node("a"), node("b"), root.clone()],
        vec![root, relay, node("a"), node("b")],
    ];
    for nodes in orders {
        let model = FeatureModel::new(
            nodes,
            vec![group("g", Cardinality::ExactlyOne, Some(&["a"]))],
        )
        .unwrap();
        let result = model.resolve(&input(&["root"])).unwrap();
        let mut selected = result.selected;
        selected.sort();
        assert_eq!(selected, ["b", "relay", "root"]);
        assert!(
            !result
                .implied
                .iter()
                .any(|r| matches!(r.reason, ImplicationReason::GroupDefault { .. }))
        );
    }
}

#[test]
fn fallback_round_uses_one_snapshot_for_all_group_eligibility() {
    let mut trigger = node("trigger");
    trigger.requires = vec!["b".into()];
    let first = ClosureGroup {
        id: "first".into(),
        members: vec!["trigger".into()],
        cardinality: Cardinality::ExactlyOne,
        fallback: Some(vec!["trigger".into()]),
    };
    let second = group("second", Cardinality::ExactlyOne, Some(&["a"]));
    let mut a = node("a");
    a.parent = Some("trigger".into());
    let mut b = node("b");
    b.parent = Some("trigger".into());
    for groups in [vec![first.clone(), second.clone()], vec![second, first]] {
        let model = FeatureModel::new(vec![a.clone(), b.clone(), trigger.clone()], groups).unwrap();
        assert_eq!(
            model.resolve(&input(&[])).unwrap().selected,
            ["b", "trigger"]
        );
    }
}

#[test]
fn incompatible_simultaneous_defaults_refuse_without_backtracking() {
    let mut trigger = node("trigger");
    trigger.requires = vec!["b".into()];
    let first = ClosureGroup {
        id: "first".into(),
        members: vec!["trigger".into()],
        cardinality: Cardinality::ExactlyOne,
        fallback: Some(vec!["trigger".into()]),
    };
    let second = group("second", Cardinality::ExactlyOne, Some(&["a"]));
    for groups in [vec![first.clone(), second.clone()], vec![second, first]] {
        let model = FeatureModel::new(vec![node("a"), node("b"), trigger.clone()], groups).unwrap();
        assert_eq!(
            model.resolve(&input(&[])),
            Err(EvaluationError::Cardinality {
                group: "second".into(),
                cardinality: Cardinality::ExactlyOne,
                members: vec!["a".into(), "b".into()],
            })
        );
    }
}

#[test]
fn model_rejects_unknown_duplicate_cycles_and_invalid_fallbacks() {
    let mut missing = node("a");
    missing.requires = vec!["absent".into()];
    let mut repeated = node("a");
    repeated.requires = vec!["b".into(), "b".into()];
    let mut cyclic_a = node("a");
    cyclic_a.parent = Some("b".into());
    let mut cyclic_b = node("b");
    cyclic_b.parent = Some("a".into());
    let g = group("g", Cardinality::ExactlyOne, Some(&["a"]));
    let mut wrong_fallback = g.clone();
    wrong_fallback.fallback = Some(vec!["c".into()]);
    let rows = [
        ("invalid ID", vec![node("")], vec![]),
        ("duplicate feature", vec![node("a"), node("a")], vec![]),
        ("unknown requires", vec![missing], vec![]),
        ("duplicate requires", vec![repeated, node("b")], vec![]),
        ("parent cycle", vec![cyclic_a, cyclic_b], vec![]),
        (
            "duplicate group",
            vec![node("a"), node("b")],
            vec![g.clone(), g.clone()],
        ),
        (
            "foreign fallback",
            vec![node("a"), node("b"), node("c")],
            vec![wrong_fallback],
        ),
        (
            "fallback cardinality",
            vec![node("a"), node("b")],
            vec![group("g", Cardinality::ExactlyOne, Some(&[]))],
        ),
    ];
    for (name, nodes, groups) in rows {
        let first = FeatureModel::new(nodes.clone(), groups.clone()).unwrap_err();
        let again = FeatureModel::new(nodes, groups).unwrap_err();
        assert_eq!(first, again, "{name}");
        assert!(first.to_string().contains("PROP-069#FEATURE-KERNEL"));
    }
}

#[test]
fn inactive_definition_is_validated_and_private_spelling_is_opaque() {
    let mut a = node("a");
    a.parent = Some("off".into());
    let mut b = node("b");
    b.parent = Some("off".into());
    let g = group("g", Cardinality::ZeroOrMore, Some(&["absent"]));
    assert_eq!(
        FeatureModel::new(vec![a, b, node("off")], vec![g])
            .unwrap_err()
            .problem,
        DefinitionProblem::UnknownReference {
            owner: "g".into(),
            id: "absent".into(),
            kind: ReferenceKind::Fallback
        }
    );
    let model = FeatureModel::new(vec![node("_private")], vec![]).unwrap();
    assert_eq!(
        model.resolve(&input(&["_private"])).unwrap().selected,
        ["_private"]
    );
    assert_eq!(
        model.resolve(&input(&["missing"])),
        Err(EvaluationError::UnknownFeature {
            id: "missing".into()
        })
    );
}

#[test]
fn direct_additions_conflict_with_disable_but_a_base_choice_can_be_replaced() {
    let model = FeatureModel::new(
        vec![node("a"), node("b")],
        vec![group("g", Cardinality::ZeroOrOne, None)],
    )
    .unwrap();
    let changed = FeatureInput {
        base: FeatureBase::Selected(vec!["a".into()]),
        additions: vec!["b".into()],
        disabled: vec![],
    };
    assert_eq!(model.resolve(&changed).unwrap().selected, ["b"]);
    assert_eq!(
        model.resolve(&FeatureInput {
            disabled: vec!["b".into()],
            ..changed
        }),
        Err(EvaluationError::EnabledAndDisabled { id: "b".into() })
    );
    let removed = FeatureInput {
        base: FeatureBase::Selected(vec!["a".into()]),
        additions: vec![],
        disabled: vec!["a".into()],
    };
    assert!(model.resolve(&removed).unwrap().selected.is_empty());
}

#[test]
fn definition_errors_identify_the_exact_reference_kind_and_owner() {
    let mut missing_parent = node("child");
    missing_parent.parent = Some("absent".into());
    assert_eq!(
        FeatureModel::new(vec![missing_parent], vec![])
            .unwrap_err()
            .problem,
        DefinitionProblem::UnknownReference {
            owner: "child".into(),
            id: "absent".into(),
            kind: ReferenceKind::Parent
        }
    );
    let mut repeated_members = group("g", Cardinality::ZeroOrMore, None);
    repeated_members.members = vec!["a".into(), "a".into()];
    assert_eq!(
        FeatureModel::new(vec![node("a")], vec![repeated_members])
            .unwrap_err()
            .problem,
        DefinitionProblem::DuplicateReference {
            owner: "g".into(),
            id: "a".into(),
            kind: ReferenceKind::Member
        }
    );
    let repeated_fallback = group("g", Cardinality::OneOrMore, Some(&["a", "a"]));
    assert_eq!(
        FeatureModel::new(vec![node("a"), node("b")], vec![repeated_fallback])
            .unwrap_err()
            .problem,
        DefinitionProblem::DuplicateReference {
            owner: "g".into(),
            id: "a".into(),
            kind: ReferenceKind::Fallback
        }
    );
    let mut empty = group("g", Cardinality::ZeroOrMore, None);
    empty.members.clear();
    assert_eq!(
        FeatureModel::new(vec![], vec![empty]).unwrap_err().problem,
        DefinitionProblem::EmptyGroup { id: "g".into() }
    );
}

#[test]
fn exclusive_replacement_preserves_direct_descendants_by_refusing_the_conflict() {
    let mut child = node("child");
    child.parent = Some("a".into());
    let model = FeatureModel::new(
        vec![node("a"), node("b"), child],
        vec![group("g", Cardinality::ExactlyOne, Some(&["a"]))],
    )
    .unwrap();
    let request = FeatureInput {
        base: FeatureBase::Defaults,
        additions: vec!["b".into(), "child".into()],
        disabled: vec![],
    };
    assert_eq!(
        model.resolve(&request),
        Err(EvaluationError::ExplicitReplacement {
            group: "g".into(),
            selected: "b".into(),
            member: "a".into(),
            descendant: "child".into(),
        })
    );
    assert_eq!(
        model
            .resolve(&FeatureInput {
                base: FeatureBase::Selected(vec!["child".into()]),
                additions: vec!["b".into()],
                disabled: vec![]
            })
            .unwrap()
            .selected,
        ["b"]
    );
}

#[test]
fn overlapping_closure_constraints_preserve_union_and_direct_inputs() {
    let first = group("first", Cardinality::ZeroOrOne, None);
    let mut second = group("second", Cardinality::ZeroOrOne, None);
    second.members = vec!["b".into(), "c".into()];
    for groups in [vec![first.clone(), second.clone()], vec![second, first]] {
        let model = FeatureModel::new(vec![node("a"), node("b"), node("c")], groups).unwrap();
        assert_eq!(
            model.resolve(&input(&["a", "c"])).unwrap().selected,
            ["a", "c"]
        );
        assert!(matches!(
            model.resolve(&input(&["a", "b"])),
            Err(EvaluationError::Cardinality { .. })
        ));
        assert_eq!(
            model
                .resolve(&FeatureInput {
                    base: FeatureBase::Selected(vec!["b".into()]),
                    additions: vec!["a".into(), "c".into()],
                    disabled: vec![]
                })
                .unwrap()
                .selected,
            ["a", "c"]
        );
        assert!(matches!(
            model.resolve(&FeatureInput {
                additions: vec!["a".into(), "b".into()],
                ..input(&[])
            }),
            Err(EvaluationError::ExplicitCardinality { .. })
        ));
    }
}
