use specmark::verifies;

use super::*;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, ArtifactId, ClosureDocument, ClosureEdge,
    ClosureEdgeKind, ClosureNodeId, LinkState, OriginRename, StaticCompileMode,
};
use crate::compiler::pass::{AnyIr, PassSegment, PassSegmentError};
use crate::{DocTree, RenameEntry, SpecAddress};

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn node(raw: &str, text: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::Spec(spec(raw)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse(text),
        aliases: Default::default(),
    }
}

fn node_spec_address(node: &ClosureDocument) -> SpecAddress {
    let DocumentAddress::Spec(address) = &node.address else {
        panic!("normal test node must be spec-addressed")
    };
    address.clone()
}

fn meta(path: &str) -> ContributionMeta {
    ContributionMeta {
        origin: "org.demo/pkg".to_string(),
        path: path.to_string(),
    }
}

fn normal(path: &str, seed: usize, order: &[usize]) -> ClosureContribution {
    ClosureContribution::Normal {
        meta: meta(path),
        seed: ClosureNodeId(seed),
        emission_order: order.iter().copied().map(ClosureNodeId).collect(),
    }
}

fn normal_plan(
    path: &str,
    seed: usize,
    occurrences: &[(usize, bool)],
    nodes: &[ClosureDocument],
) -> ContributionAbsorption {
    ContributionAbsorption::Normal {
        meta: meta(path),
        seed: ClosureNodeId(seed),
        seed_address: node_spec_address(&nodes[seed]),
        occurrences: occurrences
            .iter()
            .map(|(node, absorbed)| AbsorptionOccurrence {
                node: ClosureNodeId(*node),
                address: node_spec_address(&nodes[*node]),
                absorbed: *absorbed,
            })
            .collect(),
    }
}

fn closure(
    nodes: Vec<ClosureDocument>,
    contributions: Vec<ClosureContribution>,
    plan: Vec<ContributionAbsorption>,
) -> ClosureIr {
    ClosureIr {
        artifact: ArtifactId::new("static-absorb-test").unwrap(),
        nodes,
        edges: Vec::new(),
        contributions,
        renames: Vec::new(),
        qualification: QualificationState::Applied(StaticCompileMode::QualifyPerNode),
        absorption: AbsorptionState::Planned(AbsorptionPlan {
            mode: StaticCompileMode::QualifyPerNode,
            contributions: plan,
        }),
        link: LinkState::Unlinked,
        pending_sources: None,
        pending_embeds: None,
    }
}

fn order(closure: &ClosureIr, contribution: usize) -> Vec<usize> {
    let ClosureContribution::Normal { emission_order, .. } = &closure.contributions[contribution]
    else {
        panic!("expected normal contribution")
    };
    emission_order.iter().map(|node| node.0).collect()
}

fn plan(closure: &ClosureIr) -> &AbsorptionPlan {
    match &closure.absorption {
        AbsorptionState::Planned(plan) | AbsorptionState::Applied(plan) => plan,
        AbsorptionState::Unplanned => panic!("expected a carried absorption plan"),
    }
}

fn mask_projection_and_absorption(mut closure: ClosureIr) -> ClosureIr {
    for contribution in &mut closure.contributions {
        if let ClosureContribution::Normal { emission_order, .. } = contribution {
            emission_order.clear();
        }
    }
    closure.absorption = AbsorptionState::Unplanned;
    closure
}

fn assert_only_projection_and_absorption_changed(before: &ClosureIr, after: &ClosureIr) {
    assert_eq!(plan(before), plan(after), "absorb must carry the same plan");
    assert_eq!(
        mask_projection_and_absorption(before.clone()),
        mask_projection_and_absorption(after.clone()),
        "absorb changed a field outside normal orders and absorption typestate"
    );
}

fn two_node_closure() -> ClosureIr {
    let nodes = vec![
        node("spec://org.demo/pkg/boot/a#root", "a"),
        node("spec://org.demo/pkg/boot/b#root", "b"),
    ];
    closure(
        nodes.clone(),
        vec![normal("boot/a", 0, &[0, 1])],
        vec![normal_plan("boot/a", 0, &[(0, false), (1, false)], &nodes)],
    )
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn duplicate_live_occurrences_preserve_multiplicity_and_the_whole_carrier() {
    let nodes = vec![
        node("spec://org.demo/pkg/common/doc#root", "root"),
        node("spec://org.demo/pkg/common/doc#sub", "sub"),
    ];
    let mut input = closure(
        nodes.clone(),
        vec![normal("common/doc", 0, &[0, 0, 1])],
        vec![normal_plan(
            "common/doc",
            0,
            &[(0, false), (0, false), (1, true)],
            &nodes,
        )],
    );
    input.edges.push(ClosureEdge {
        from: ClosureNodeId(0),
        to: ClosureNodeId(1),
        kind: ClosureEdgeKind::Use,
    });
    input.renames.push(OriginRename {
        origin: "org.demo/pkg".to_string(),
        rename: RenameEntry {
            original: "root".to_string(),
            qualified: "org-demo--pkg--root".to_string(),
        },
    });
    let before = input.clone();
    reset_absorb_invocations();

    let output = AbsorbPass::new().run(input).unwrap();

    assert_eq!(absorb_invocations(), 1);
    assert_eq!(order(&output, 0), [0, 0]);
    assert_only_projection_and_absorption_changed(&before, &output);
    assert!(matches!(output.absorption, AbsorptionState::Applied(_)));
    validate_applied_absorption(&output).unwrap();

    let mut deduplicated = output;
    let ClosureContribution::Normal { emission_order, .. } = &mut deduplicated.contributions[0]
    else {
        unreachable!()
    };
    emission_order.dedup();
    assert!(matches!(
        validate_applied_absorption(&deduplicated),
        Err(AbsorbPassError::AppliedAlignment {
            contribution: Some(0),
            expected: 2,
            actual: 1,
        })
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn shared_all_dead_empty_and_simple_contributions_keep_their_exact_identity() {
    let simple_address = DocumentAddress::StaticEntry {
        origin: "host".to_string(),
        path: "boot/local.md".to_string(),
    };
    let simple_meta = ContributionMeta {
        origin: "host".to_string(),
        path: "boot/local.md".to_string(),
    };
    let simple_document = ClosureDocument {
        address: simple_address.clone(),
        origin: "host".to_string(),
        tree: DocTree::parse("simple body"),
        aliases: Default::default(),
    };
    let contributions = vec![
        normal("root-a", 0, &[0, 1, 2]),
        normal("root-b", 1, &[1, 2]),
        normal("all-dead", 2, &[2]),
        normal("empty", 0, &[]),
        ClosureContribution::Simple {
            meta: simple_meta.clone(),
            document: Box::new(simple_document.clone()),
        },
    ];
    let nodes = vec![
        node("spec://org.demo/pkg/common/doc#root", "root and shared"),
        node("spec://org.demo/pkg/common/doc#shared", "shared"),
        node("spec://org.demo/pkg/common/doc#dead", "dead"),
    ];
    let plan = vec![
        normal_plan("root-a", 0, &[(0, false), (1, true), (2, true)], &nodes),
        normal_plan("root-b", 1, &[(1, false), (2, true)], &nodes),
        normal_plan("all-dead", 2, &[(2, true)], &nodes),
        normal_plan("empty", 0, &[], &nodes),
        ContributionAbsorption::Simple {
            meta: simple_meta,
            address: simple_address,
        },
    ];
    let mut input = closure(nodes, contributions, plan);
    input.edges = vec![
        ClosureEdge {
            from: ClosureNodeId(0),
            to: ClosureNodeId(1),
            kind: ClosureEdgeKind::Use,
        },
        ClosureEdge {
            from: ClosureNodeId(1),
            to: ClosureNodeId(2),
            kind: ClosureEdgeKind::Embed,
        },
    ];
    let before = input.clone();

    reset_absorb_invocations();
    let output = AbsorbPass::new().run(input).unwrap();

    assert_eq!(absorb_invocations(), 1);
    assert_eq!(order(&output, 0), [0]);
    assert_eq!(order(&output, 1), [1]);
    assert!(order(&output, 2).is_empty());
    assert!(order(&output, 3).is_empty());
    let ClosureContribution::Simple { document, .. } = &output.contributions[4] else {
        panic!("simple contribution kept its top-level position")
    };
    assert_eq!(document.as_ref(), &simple_document);
    assert_only_projection_and_absorption_changed(&before, &output);
    validate_applied_absorption(&output).unwrap();
}

#[test]
fn stale_identity_is_rejected_before_any_projection_is_consumed() {
    let nodes = vec![
        node("spec://org.demo/pkg/boot/a#root", "a"),
        node("spec://org.demo/pkg/boot/b#root", "b"),
    ];
    let mut stale = closure(
        nodes.clone(),
        vec![normal("boot/a", 0, &[0, 1])],
        vec![normal_plan("boot/a", 0, &[(0, false), (1, false)], &nodes)],
    );
    let ClosureContribution::Normal { emission_order, .. } = &mut stale.contributions[0] else {
        unreachable!()
    };
    emission_order.swap(0, 1);

    assert!(matches!(
        AbsorbPass::new().run(stale),
        Err(AbsorbPassError::InvalidPlan(
            QualifyPassError::AbsorptionOccurrence {
                contribution: 0,
                occurrence: 0,
                expected: 0,
                actual: 1,
            }
        ))
    ));
}

#[test]
fn planned_identity_binds_node_order_addresses_and_empty_seed() {
    let base = two_node_closure();
    let expected_a = node_spec_address(&base.nodes[0]);
    let expected_b = node_spec_address(&base.nodes[1]);

    let mut swapped = base.clone();
    swapped.nodes.swap(0, 1);
    assert!(matches!(
        AbsorbPass::new().run(swapped),
        Err(AbsorbPassError::InvalidPlan(
            QualifyPassError::AbsorptionSeedAddress {
                contribution: 0,
                node: 0,
                expected,
                actual,
            }
        )) if *expected == expected_a && *actual == expected_b
    ));

    let replacement = spec("spec://org.demo/pkg/boot/c#root");
    let mut replaced = base.clone();
    replaced.nodes[1].address = DocumentAddress::Spec(replacement.clone());
    assert!(matches!(
        AbsorbPass::new().run(replaced),
        Err(AbsorbPassError::InvalidPlan(
            QualifyPassError::AbsorptionOccurrenceAddress {
                contribution: 0,
                occurrence: 1,
                node: 1,
                expected,
                actual,
            }
        )) if *expected == expected_b && *actual == replacement
    ));

    let nodes = vec![node("spec://org.demo/pkg/boot/empty#root", "empty")];
    let mut empty = closure(
        nodes.clone(),
        vec![normal("boot/empty", 0, &[])],
        vec![normal_plan("boot/empty", 0, &[], &nodes)],
    );
    let expected = node_spec_address(&empty.nodes[0]);
    let actual = spec("spec://org.demo/pkg/boot/replaced#root");
    empty.nodes[0].address = DocumentAddress::Spec(actual.clone());
    assert!(matches!(
        AbsorbPass::new().run(empty),
        Err(AbsorbPassError::InvalidPlan(
            QualifyPassError::AbsorptionSeedAddress {
                contribution: 0,
                node: 0,
                expected: error_expected,
                actual: error_actual,
            }
        )) if *error_expected == expected && *error_actual == actual
    ));

    let mut body_only = base;
    body_only.nodes[1].tree = DocTree::parse("rewritten body after qualify");
    body_only.nodes[1].origin = "org.demo/rewritten-provider".to_string();
    assert!(AbsorbPass::new().run(body_only).is_ok());
}

#[test]
fn planned_and_applied_mode_and_address_drift_are_precise() {
    let base = two_node_closure();
    let mut planned_mode = base.clone();
    planned_mode.qualification = QualificationState::Applied(StaticCompileMode::Plain);
    assert!(matches!(
        AbsorbPass::new().run(planned_mode),
        Err(AbsorbPassError::InvalidPlan(
            QualifyPassError::AbsorptionMode {
                expected: StaticCompileMode::QualifyPerNode,
                actual: StaticCompileMode::Plain,
            }
        ))
    ));

    let applied = AbsorbPass::new().run(base).unwrap();
    let expected_seed = node_spec_address(&applied.nodes[0]);
    let replaced_seed = spec("spec://org.demo/pkg/boot/seed-replaced#root");
    let mut seed_drift = applied.clone();
    seed_drift.nodes[0].address = DocumentAddress::Spec(replaced_seed.clone());
    assert!(matches!(
        validate_applied_absorption(&seed_drift),
        Err(AbsorbPassError::AppliedSeedAddress {
            contribution: 0,
            node: 0,
            expected,
            actual,
        }) if *expected == expected_seed && *actual == replaced_seed
    ));

    let expected = node_spec_address(&applied.nodes[1]);
    let actual = spec("spec://org.demo/pkg/boot/replaced#root");
    let mut address_drift = applied.clone();
    address_drift.nodes[1].address = DocumentAddress::Spec(actual.clone());
    assert!(matches!(
        validate_applied_absorption(&address_drift),
        Err(AbsorbPassError::AppliedOccurrenceAddress {
            contribution: 0,
            occurrence: 1,
            node: 1,
            expected: error_expected,
            actual: error_actual,
        }) if *error_expected == expected && *error_actual == actual
    ));

    let mut mode_drift = applied;
    mode_drift.qualification = QualificationState::Applied(StaticCompileMode::Plain);
    assert!(matches!(
        validate_applied_absorption(&mode_drift),
        Err(AbsorbPassError::AppliedMode {
            expected: StaticCompileMode::QualifyPerNode,
            actual: StaticCompileMode::Plain,
        })
    ));

    let coherent_address = spec("spec://org.demo/pkg/boot/coherent#root");
    let mut coherent = two_node_closure();
    coherent.qualification = QualificationState::Applied(StaticCompileMode::Plain);
    coherent.nodes[1].address = DocumentAddress::Spec(coherent_address.clone());
    let AbsorptionState::Planned(plan) = &mut coherent.absorption else {
        unreachable!()
    };
    plan.mode = StaticCompileMode::Plain;
    let ContributionAbsorption::Normal { occurrences, .. } = &mut plan.contributions[0] else {
        unreachable!()
    };
    occurrences[1].address = coherent_address;
    assert!(AbsorbPass::new().run(coherent).is_ok());
}

#[test]
fn state_preconditions_reject_bypass_pending_and_double_application() {
    let nodes = vec![node("spec://org.demo/pkg/boot/entry#root", "body")];
    let base = closure(
        nodes.clone(),
        vec![normal("boot/entry", 0, &[0])],
        vec![normal_plan("boot/entry", 0, &[(0, false)], &nodes)],
    );
    let mut pending = base.clone();
    pending.qualification = QualificationState::Pending(StaticCompileMode::QualifyPerNode);
    assert!(matches!(
        AbsorbPass::new().run(pending),
        Err(AbsorbPassError::QualificationPending)
    ));

    let mut unplanned = base.clone();
    unplanned.absorption = AbsorptionState::Unplanned;
    assert!(matches!(
        AbsorbPass::new().run(unplanned),
        Err(AbsorbPassError::Unplanned)
    ));

    let applied = AbsorbPass::new().run(base).unwrap();
    assert!(matches!(
        AbsorbPass::new().run(applied),
        Err(AbsorbPassError::AlreadyApplied)
    ));
}

#[test]
fn manager_attributes_invalid_state_to_absorb_with_typed_source() {
    let mut invalid = two_node_closure();
    invalid.absorption = AbsorptionState::Unplanned;
    let mut segment = PassSegment::default();
    segment.push(AbsorbPass::new()).unwrap();

    let error = segment.run(AnyIr::Closure(invalid)).unwrap_err();
    let PassSegmentError::PassFailed { pass, source } = error else {
        panic!("invalid absorb input must remain a named pass failure")
    };
    assert_eq!(pass.as_str(), ABSORB_PASS_NAME);
    assert!(matches!(
        source.downcast_ref::<AbsorbPassError>(),
        Some(AbsorbPassError::Unplanned)
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn applied_verifier_rejects_projection_and_retained_graph_drift() {
    let nodes = vec![
        node("spec://org.demo/pkg/boot/a#root", "a"),
        node("spec://org.demo/pkg/boot/b#root", "b"),
    ];
    let base = closure(
        nodes.clone(),
        vec![normal("boot/a", 0, &[0, 1])],
        vec![normal_plan("boot/a", 0, &[(0, false), (1, false)], &nodes)],
    );
    let output = AbsorbPass::new().run(base).unwrap();

    let mut reordered = output.clone();
    let ClosureContribution::Normal { emission_order, .. } = &mut reordered.contributions[0] else {
        unreachable!()
    };
    emission_order.swap(0, 1);
    assert!(matches!(
        validate_applied_absorption(&reordered),
        Err(AbsorbPassError::AppliedOccurrence {
            contribution: 0,
            occurrence: 0,
            expected: 0,
            actual: 1,
        })
    ));

    let mut missing = output;
    missing.nodes.pop();
    assert!(matches!(
        validate_applied_absorption(&missing),
        Err(AbsorbPassError::AppliedMissingNode {
            contribution: 0,
            occurrence: 1,
            node: 1,
        })
    ));
}
