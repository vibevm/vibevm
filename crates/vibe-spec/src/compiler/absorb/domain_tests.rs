use specmark::verifies;

use super::*;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, ArtifactId, ClosureDocument, ClosureNodeId,
    StaticCompileMode,
};
use crate::compiler::pass::{AnyIr, PassSegment, PassSegmentError};
use crate::compiler::qualify::{QualifyPass, QualifyPassError};
use crate::{DocTree, SpecAddress};

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn spec_node(raw: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::Spec(spec(raw)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse("body"),
        aliases: Default::default(),
    }
}

fn static_node(path: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::StaticEntry {
            origin: "host".to_string(),
            path: path.to_string(),
        },
        origin: "host".to_string(),
        tree: DocTree::parse("body"),
        aliases: Default::default(),
    }
}

fn meta() -> ContributionMeta {
    ContributionMeta {
        origin: "org.demo/pkg".to_string(),
        path: "boot/entry".to_string(),
    }
}

fn normal(seed: usize, order: &[usize]) -> ClosureContribution {
    ClosureContribution::Normal {
        meta: meta(),
        seed: ClosureNodeId(seed),
        emission_order: order.iter().copied().map(ClosureNodeId).collect(),
    }
}

fn node_address(nodes: &[ClosureDocument], node: usize) -> SpecAddress {
    let DocumentAddress::Spec(address) = &nodes[node].address else {
        panic!("valid normal fixture must be spec-addressed")
    };
    address.clone()
}

fn planned_closure(nodes: Vec<ClosureDocument>, seed: usize, order: &[usize]) -> ClosureIr {
    let occurrences = order
        .iter()
        .map(|node| AbsorptionOccurrence {
            node: ClosureNodeId(*node),
            address: node_address(&nodes, *node),
            absorbed: false,
        })
        .collect();
    let plan = AbsorptionPlan {
        mode: StaticCompileMode::QualifyPerNode,
        contributions: vec![ContributionAbsorption::Normal {
            meta: meta(),
            seed: ClosureNodeId(seed),
            seed_address: node_address(&nodes, seed),
            occurrences,
        }],
    };
    ClosureIr {
        artifact: ArtifactId::new("domain-test").unwrap(),
        nodes,
        edges: Vec::new(),
        contributions: vec![normal(seed, order)],
        renames: Vec::new(),
        qualification: QualificationState::Applied(StaticCompileMode::QualifyPerNode),
        absorption: AbsorptionState::Planned(plan),
        pending_sources: None,
        pending_embeds: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn empty_normal_static_seed_fails_analysis_and_planned_validation() {
    let analyze = ClosureIr {
        artifact: ArtifactId::new("domain-test").unwrap(),
        nodes: vec![static_node("boot/local.md")],
        edges: Vec::new(),
        contributions: vec![normal(0, &[])],
        renames: Vec::new(),
        qualification: QualificationState::Pending(StaticCompileMode::QualifyPerNode),
        absorption: AbsorptionState::Unplanned,
        pending_sources: None,
        pending_embeds: None,
    };
    assert!(matches!(
        QualifyPass::new().run(analyze),
        Err(QualifyPassError::NonSpecSeedGraphNode {
            contribution: 0,
            node: 0,
        })
    ));

    let mut planned = planned_closure(
        vec![spec_node("spec://org.demo/pkg/boot/entry#root")],
        0,
        &[],
    );
    planned.nodes[0] = static_node("boot/replaced.md");
    assert!(matches!(
        AbsorbPass::new().run(planned),
        Err(AbsorbPassError::InvalidPlan(
            QualifyPassError::NonSpecSeedGraphNode {
                contribution: 0,
                node: 0,
            }
        ))
    ));
}

#[test]
fn named_absorb_rejects_a_live_normal_static_entry() {
    let mut invalid = planned_closure(
        vec![
            spec_node("spec://org.demo/pkg/boot/a#root"),
            spec_node("spec://org.demo/pkg/boot/b#root"),
        ],
        0,
        &[0, 1],
    );
    invalid.nodes[1] = static_node("boot/replaced.md");
    let mut segment = PassSegment::default();
    segment.push(AbsorbPass::new()).unwrap();

    let error = segment.run(AnyIr::Closure(invalid)).unwrap_err();
    let PassSegmentError::PassFailed { pass, source } = error else {
        panic!("normal-domain failure must retain manager attribution")
    };
    assert_eq!(pass.as_str(), ABSORB_PASS_NAME);
    assert!(matches!(
        source.downcast_ref::<AbsorbPassError>(),
        Some(AbsorbPassError::InvalidPlan(
            QualifyPassError::NonSpecGraphNode {
                contribution: 0,
                occurrence: 1,
            }
        ))
    ));
}

#[test]
fn applied_verifier_rejects_static_seed_and_live_nodes() {
    let applied = AbsorbPass::new()
        .run(planned_closure(
            vec![
                spec_node("spec://org.demo/pkg/boot/a#root"),
                spec_node("spec://org.demo/pkg/boot/b#root"),
            ],
            0,
            &[0, 1],
        ))
        .unwrap();

    let mut seed = applied.clone();
    seed.nodes[0] = static_node("boot/seed.md");
    assert!(matches!(
        validate_applied_absorption(&seed),
        Err(AbsorbPassError::AppliedNonSpecSeedNode {
            contribution: 0,
            node: 0,
        })
    ));

    let mut live = applied;
    live.nodes[1] = static_node("boot/live.md");
    assert!(matches!(
        validate_applied_absorption(&live),
        Err(AbsorbPassError::AppliedNonSpecNode {
            contribution: 0,
            occurrence: 1,
            node: 1,
        })
    ));
}

#[test]
fn empty_normal_seed_bounds_are_independently_red() {
    let mut planned = planned_closure(
        vec![spec_node("spec://org.demo/pkg/boot/entry#root")],
        0,
        &[],
    );
    let ClosureContribution::Normal { seed, .. } = &mut planned.contributions[0] else {
        unreachable!()
    };
    *seed = ClosureNodeId(1);
    let AbsorptionState::Planned(plan) = &mut planned.absorption else {
        unreachable!()
    };
    let ContributionAbsorption::Normal { seed, .. } = &mut plan.contributions[0] else {
        unreachable!()
    };
    *seed = ClosureNodeId(1);
    assert!(matches!(
        AbsorbPass::new().run(planned),
        Err(AbsorbPassError::InvalidPlan(
            QualifyPassError::InvalidSeedNodeId {
                contribution: 0,
                node: 1,
            }
        ))
    ));

    let mut applied = AbsorbPass::new()
        .run(planned_closure(
            vec![spec_node("spec://org.demo/pkg/boot/entry#root")],
            0,
            &[],
        ))
        .unwrap();
    applied.nodes.clear();
    assert!(matches!(
        validate_applied_absorption(&applied),
        Err(AbsorbPassError::AppliedMissingSeedNode {
            contribution: 0,
            node: 0,
        })
    ));
}

#[test]
fn revision_pin_is_part_of_the_exact_normal_witness() {
    let mut planned = planned_closure(
        vec![spec_node("spec://org.demo/pkg/boot/entry#root")],
        0,
        &[0],
    );
    planned.nodes[0].address =
        DocumentAddress::Spec(spec("spec://org.demo/pkg/boot/entry#root~r7"));
    assert!(matches!(
        AbsorbPass::new().run(planned),
        Err(AbsorbPassError::InvalidPlan(
            QualifyPassError::AbsorptionSeedAddress {
                contribution: 0,
                node: 0,
                expected,
                actual,
            }
        )) if expected.pinned_r.is_none() && actual.pinned_r == Some(7)
    ));

    let mut applied = AbsorbPass::new()
        .run(planned_closure(
            vec![
                spec_node("spec://org.demo/pkg/boot/a#root"),
                spec_node("spec://org.demo/pkg/boot/b#root"),
            ],
            0,
            &[0, 1],
        ))
        .unwrap();
    applied.nodes[1].address = DocumentAddress::Spec(spec("spec://org.demo/pkg/boot/b#root~r9"));
    assert!(matches!(
        validate_applied_absorption(&applied),
        Err(AbsorbPassError::AppliedOccurrenceAddress {
            contribution: 0,
            occurrence: 1,
            node: 1,
            expected,
            actual,
        }) if expected.pinned_r.is_none() && actual.pinned_r == Some(9)
    ));
}
