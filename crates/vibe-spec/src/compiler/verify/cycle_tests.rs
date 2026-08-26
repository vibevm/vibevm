//! The per-edge-kind cycle law, deterministic first-failure order, and
//! qualification/absorption typestate, over hand-built closures.

use specmark::verifies;

use super::super::ir::{
    AbsorptionState, ClosureContribution, ClosureEdge, ClosureEdgeKind, ClosureIr, ClosureNodeId,
    QualificationState, StaticCompileMode,
};
use super::super::pass::Pass;
use super::super::qualify::QualifyPass;
use super::VerificationError;
use super::closure_tests::{address, base_nodes, closure, node, occurrence, use_edge, verify};

// --- the per-edge-kind cycle law ---------------------------------------

fn two_node_closure(kind: ClosureEdgeKind, first: &str, second: &str) -> ClosureIr {
    let nodes = vec![
        node(first, "# A {#a}\nalpha\n"),
        node(second, "# B {#b}\nbeta\n"),
    ];
    let edges = vec![
        ClosureEdge {
            from: ClosureNodeId(0),
            to: ClosureNodeId(1),
            kind,
            requested_target: address(second),
        },
        ClosureEdge {
            from: ClosureNodeId(1),
            to: ClosureNodeId(0),
            kind,
            requested_target: address(first),
        },
    ];
    closure(
        nodes,
        edges,
        vec![occurrence(first, 0), occurrence(second, 1)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    )
}

const CONTRACT_A: &str = "spec://org.demo/lib/contract/a#r";
const CONTRACT_B: &str = "spec://org.demo/lib/contract/b#r";
const SOURCE_B: &str = "spec://org.demo/lib/source/b#r";

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_non_contract_use_cycle_is_illegal_with_its_exact_path() {
    let error = verify(&two_node_closure(
        ClosureEdgeKind::Use,
        CONTRACT_A,
        SOURCE_B,
    ))
    .unwrap_err();
    match error {
        VerificationError::IllegalCycle { kind, path } => {
            assert_eq!(kind, ClosureEdgeKind::Use);
            // Anchored on the node that makes the component illegal, so the
            // rendered path shows the reason rather than an admitted sub-loop.
            assert_eq!(path[0], SOURCE_B);
            assert_eq!(path[1], CONTRACT_A);
            assert_eq!(path[2], SOURCE_B, "the path closes on itself");
        }
        other => panic!("expected a use cycle, got {other:?}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_contract_only_use_cycle_is_a_legal_forward_declaration() {
    verify(&two_node_closure(
        ClosureEdgeKind::Use,
        CONTRACT_A,
        CONTRACT_B,
    ))
    .unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn one_non_contract_participant_makes_the_use_cycle_illegal() {
    assert!(matches!(
        verify(&two_node_closure(
            ClosureEdgeKind::Use,
            CONTRACT_B,
            SOURCE_B
        )),
        Err(VerificationError::IllegalCycle { .. })
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn even_an_all_contract_embed_cycle_is_a_hard_error() {
    // A mutation that applied the contract exception to embed turns this red.
    assert!(matches!(
        verify(&two_node_closure(
            ClosureEdgeKind::Embed,
            CONTRACT_A,
            CONTRACT_B
        )),
        Err(VerificationError::IllegalCycle {
            kind: ClosureEdgeKind::Embed,
            ..
        })
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn the_contract_exception_applies_to_source_cycles_independently() {
    verify(&two_node_closure(
        ClosureEdgeKind::Source,
        CONTRACT_A,
        CONTRACT_B,
    ))
    .unwrap();
    assert!(matches!(
        verify(&two_node_closure(
            ClosureEdgeKind::Source,
            CONTRACT_A,
            SOURCE_B
        )),
        Err(VerificationError::IllegalCycle {
            kind: ClosureEdgeKind::Source,
            ..
        })
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_mixed_kind_union_cycle_is_legal_when_each_relation_is_acyclic() {
    // A -Use-> B and B -Embed-> A: neither relation cycles on its own, so the
    // union cycle is retained provenance, not recursive execution. A
    // union-SCC implementation turns this red.
    let nodes = vec![
        node(SOURCE_B, "# A {#a}\nalpha\n"),
        node(CONTRACT_A, "# B {#b}\nbeta\n"),
    ];
    let edges = vec![
        ClosureEdge {
            from: ClosureNodeId(0),
            to: ClosureNodeId(1),
            kind: ClosureEdgeKind::Use,
            requested_target: address(CONTRACT_A),
        },
        ClosureEdge {
            from: ClosureNodeId(1),
            to: ClosureNodeId(0),
            kind: ClosureEdgeKind::Embed,
            requested_target: address(SOURCE_B),
        },
    ];
    verify(&closure(
        nodes,
        edges,
        vec![occurrence(SOURCE_B, 0), occurrence(CONTRACT_A, 1)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap();
}

/// The positive side of the same law: a legal contract-only cycle keeps every
/// request pointing at its own node, so the identity check admits it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn an_honest_contract_cycle_satisfies_the_request_identity_law() {
    let honest = two_node_closure(ClosureEdgeKind::Use, CONTRACT_A, CONTRACT_B);
    verify(&honest).unwrap();
    let ClosureContribution::Normal {
        seed_address,
        emission_order,
        ..
    } = &honest.contributions[0]
    else {
        unreachable!("the fixture holds one normal contribution")
    };
    assert_eq!(seed_address.without_pin(), CONTRACT_A);
    assert_eq!(
        emission_order
            .iter()
            .map(|current| current.requested_address.without_pin())
            .collect::<Vec<_>>(),
        [CONTRACT_A, CONTRACT_B]
    );
    assert_eq!(
        honest
            .edges
            .iter()
            .map(|edge| edge.requested_target.without_pin())
            .collect::<Vec<_>>(),
        [CONTRACT_B, CONTRACT_A]
    );
}

// --- deterministic first-failure order ---------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn bounds_beat_duplicate_ids_and_duplicate_ids_beat_cycles() {
    let mut dirty = vec![
        node(CONTRACT_A, "# A {#a}\n##dup one\n\n##dup two\n"),
        node(SOURCE_B, "# B {#b}\nbeta\n"),
    ];
    // A duplicate fact and an illegal use cycle in one carrier: the semantic
    // gate is judged first.
    let edges = vec![use_edge(0, 1, SOURCE_B), use_edge(1, 0, CONTRACT_A)];
    let error = verify(&closure(
        dirty.clone(),
        edges,
        vec![occurrence(CONTRACT_A, 0), occurrence(SOURCE_B, 1)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap_err();
    assert!(
        matches!(error, VerificationError::DuplicateId { .. }),
        "{error:?}"
    );

    // Add an out-of-range edge endpoint to the same carrier: the structural
    // bounds error now wins over both.
    dirty.push(node(CONTRACT_B, "# C {#c}\n"));
    let error = verify(&closure(
        dirty,
        vec![use_edge(0, 9, SOURCE_B)],
        vec![occurrence(CONTRACT_A, 0)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap_err();
    assert!(
        matches!(error, VerificationError::InvalidNodeId { .. }),
        "{error:?}"
    );
}

// --- typestate ----------------------------------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn pending_with_renames_or_a_plan_and_applied_without_one_are_misaligned() {
    let linked = vec![use_edge(0, 1, "spec://org.demo/pkg/common/contract/b#r")];
    let with_renames = closure(
        base_nodes(),
        linked.clone(),
        vec![occurrence("spec://org.demo/pkg/common/contract/a#r", 0)],
        0,
        vec![super::super::ir::OriginRename {
            origin: "org.demo/pkg".to_string(),
            rename: crate::RenameEntry {
                original: "a".to_string(),
                qualified: "org.demo/pkg--a".to_string(),
            },
        }],
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    );
    assert!(matches!(
        verify(&with_renames),
        Err(VerificationError::PendingRenames { count: 1 })
    ));

    let qualified = QualifyPass::new()
        .run(closure(
            base_nodes(),
            linked.clone(),
            vec![
                occurrence("spec://org.demo/pkg/common/contract/a#r", 0),
                occurrence("spec://org.demo/pkg/common/contract/b#r", 1),
            ],
            0,
            Vec::new(),
            QualificationState::Pending(StaticCompileMode::Plain),
            AbsorptionState::Unplanned,
        ))
        .unwrap();
    let AbsorptionState::Planned(plan) = qualified.absorption.clone() else {
        unreachable!("qualify plans its input")
    };

    let mut regressed = qualified.clone();
    regressed.absorption = AbsorptionState::Unplanned;
    assert!(matches!(
        verify(&regressed),
        Err(VerificationError::MisalignedState { .. })
    ));

    let mut unplanned_qualification = closure(
        base_nodes(),
        linked,
        vec![
            occurrence("spec://org.demo/pkg/common/contract/a#r", 0),
            occurrence("spec://org.demo/pkg/common/contract/b#r", 1),
        ],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    );
    unplanned_qualification.absorption = AbsorptionState::Planned(plan.clone());
    assert!(matches!(
        verify(&unplanned_qualification),
        Err(VerificationError::MisalignedState { .. })
    ));

    let mut live = qualified;
    live.absorption = AbsorptionState::Planned(plan);
    live.pending_sources = Some(Default::default());
    assert!(matches!(
        verify(&live),
        Err(VerificationError::PendingSnapshotsLive { kind: "source" })
    ));
}
