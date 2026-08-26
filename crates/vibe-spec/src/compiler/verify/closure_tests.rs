//! Closure-level verifier invariants: bounds, identity, reachability, the
//! per-edge-kind cycle law, qualification/absorption typestate, and the
//! pre-pass transition witness (including the absorbed-bit flip reds).

use specmark::verifies;

use super::super::close::document_origin;
use super::super::ir::{
    AbsorptionState, ArtifactContext, ClosureContribution, ClosureDocument, ClosureEdge,
    ClosureEdgeKind, ClosureIr, ClosureNodeId, ClosureOccurrence, ContributionMeta,
    DocumentAddress, LinkState, QualificationState, StaticCompileMode,
};
use super::{IrVerifier, VerificationError};
use crate::compiler::pass::AnyIr;
use crate::{DocTree, SpecAddress};

pub(super) fn address(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

pub(super) fn node(raw: &str, text: &str) -> ClosureDocument {
    let spec = address(raw);
    ClosureDocument {
        address: DocumentAddress::Spec(spec.clone()),
        origin: document_origin(&spec),
        tree: DocTree::parse(text),
        aliases: Default::default(),
    }
}

pub(super) fn occurrence(raw: &str, node: usize) -> ClosureOccurrence {
    ClosureOccurrence {
        node: ClosureNodeId(node),
        requested_address: address(raw),
    }
}

pub(super) fn use_edge(from: usize, to: usize, target: &str) -> ClosureEdge {
    ClosureEdge {
        from: ClosureNodeId(from),
        to: ClosureNodeId(to),
        kind: ClosureEdgeKind::Use,
        requested_target: address(target),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn closure(
    nodes: Vec<ClosureDocument>,
    edges: Vec<ClosureEdge>,
    emission: Vec<ClosureOccurrence>,
    seed: usize,
    renames: Vec<super::super::ir::OriginRename>,
    qualification: QualificationState,
    absorption: AbsorptionState,
) -> ClosureIr {
    let seed_address = match nodes.get(seed).map(|node| &node.address) {
        Some(DocumentAddress::Spec(spec)) => spec.clone(),
        Some(DocumentAddress::StaticEntry { .. }) => unreachable!("seeds are spec-addressed"),
        // An out-of-range seed is the mutation under test; the bounds check
        // fires long before the seed address is ever consumed.
        None => match &nodes
            .first()
            .expect("fixtures carry at least one node")
            .address
        {
            DocumentAddress::Spec(spec) => spec.clone(),
            DocumentAddress::StaticEntry { .. } => unreachable!("fixtures are spec-addressed"),
        },
    };
    ClosureIr::testing(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        nodes,
        edges,
        vec![ClosureContribution::Normal {
            meta: ContributionMeta::new("org.demo/pkg", "boot/entry.md").unwrap(),
            seed: ClosureNodeId(seed),
            seed_address,
            emission_order: emission,
        }],
        renames,
        qualification,
        absorption,
        LinkState::Unlinked,
        None,
        None,
    )
}

pub(super) fn base_nodes() -> Vec<ClosureDocument> {
    vec![
        node(
            "spec://org.demo/pkg/common/contract/a#r",
            "# A {#a}\ncontract a\n",
        ),
        node(
            "spec://org.demo/pkg/common/contract/b#r",
            "# B {#b}\ncontract b\n",
        ),
    ]
}

pub(super) fn verify(closure: &ClosureIr) -> Result<(), VerificationError> {
    IrVerifier.verify(&AnyIr::Closure(closure.clone()))
}

/// The minimal valid closure, shared with the manager tests' counting pass.
pub(super) fn minimal_closure() -> ClosureIr {
    closure(
        base_nodes(),
        vec![use_edge(1, 0, "spec://org.demo/pkg/common/contract/a#r")],
        vec![
            occurrence("spec://org.demo/pkg/common/contract/a#r", 0),
            occurrence("spec://org.demo/pkg/common/contract/b#r", 1),
        ],
        1,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    )
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_minimal_pending_unplanned_closure_passes() {
    verify(&minimal_closure()).unwrap();
}

/// Every recorded request must still name the node it resolved to. A pass that
/// retargets a request while keeping the node id leaves a carrier whose
/// provenance lies; downstream that is only a `debug_assert_eq!` in qualify's
/// absorption analysis, so without this law the compiler behaves one way in
/// debug and another in release.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_request_that_no_longer_names_its_node_is_a_typed_error_in_every_profile() {
    let elsewhere = address("spec://org.evil/pkg/common/contract/z#r");

    let mut retargeted_edge = minimal_closure();
    retargeted_edge.edges[0].requested_target = elsewhere.clone();
    let error = verify(&retargeted_edge).unwrap_err();
    assert!(
        matches!(
            &error,
            VerificationError::EdgeTargetMismatch { edge: 0, expected, actual }
                if expected == "spec://org.evil/pkg/common/contract/z#r"
                    && actual == "spec://org.demo/pkg/common/contract/a#r"
        ),
        "{error:?}"
    );

    let mut retargeted_seed = minimal_closure();
    let ClosureContribution::Normal { seed_address, .. } = &mut retargeted_seed.contributions[0]
    else {
        unreachable!("the fixture holds one normal contribution")
    };
    *seed_address = elsewhere.clone();
    let error = verify(&retargeted_seed).unwrap_err();
    assert!(
        matches!(
            &error,
            VerificationError::SeedAddressMismatch { contribution: 0, actual, .. }
                if actual == "spec://org.demo/pkg/common/contract/b#r"
        ),
        "{error:?}"
    );

    let mut retargeted_occurrence = minimal_closure();
    let ClosureContribution::Normal { emission_order, .. } =
        &mut retargeted_occurrence.contributions[0]
    else {
        unreachable!("the fixture holds one normal contribution")
    };
    emission_order[1].requested_address = elsewhere;
    let error = verify(&retargeted_occurrence).unwrap_err();
    assert!(
        matches!(
            &error,
            VerificationError::OccurrenceAddressMismatch {
                contribution: 0,
                occurrence: 1,
                actual,
                ..
            } if actual == "spec://org.demo/pkg/common/contract/b#r"
        ),
        "{error:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn out_of_range_edge_seed_and_occurrence_ids_fail_before_any_indexing() {
    let over = vec![
        node("spec://org.demo/pkg/common/a#r", "# A {#a}\n"),
        node("spec://org.demo/pkg/common/b#r", "# B {#b}\n"),
    ];
    let edges = vec![use_edge(0, 7, "spec://org.demo/pkg/common/b#r")];
    let error = verify(&closure(
        over.clone(),
        edges,
        vec![occurrence("spec://org.demo/pkg/common/a#r", 0)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap_err();
    assert!(
        matches!(
            error,
            VerificationError::InvalidNodeId {
                site: "edge target",
                index: 7,
                len: 2,
            }
        ),
        "{error:?}"
    );

    let error = verify(&closure(
        over,
        Vec::new(),
        vec![occurrence("spec://org.demo/pkg/common/a#r", 0)],
        9,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap_err();
    assert!(
        matches!(
            error,
            VerificationError::InvalidNodeId {
                site: "normal seed",
                index: 9,
                ..
            }
        ),
        "{error:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn duplicate_node_keys_wrong_origins_and_static_graph_nodes_are_typed_errors() {
    let duplicated = vec![
        node("spec://org.demo/pkg/common/a#r", "# A {#a}\n"),
        node("spec://org.demo/pkg/common/a#r", "# A2 {#a}\n"),
    ];
    let error = verify(&closure(
        duplicated,
        Vec::new(),
        vec![occurrence("spec://org.demo/pkg/common/a#r", 0)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap_err();
    assert!(
        matches!(
            error,
            VerificationError::DuplicateNodeAddress {
                first: 0,
                second: 1,
                ..
            }
        ),
        "{error:?}"
    );

    let mut mismatched = vec![
        node("spec://org.demo/pkg/common/a#r", "# A {#a}\n"),
        node("spec://org.demo/pkg/common/b#r", "# B {#b}\n"),
    ];
    mismatched[0].origin = "org.evil/other".to_string();
    let error = verify(&closure(
        mismatched,
        Vec::new(),
        vec![occurrence("spec://org.demo/pkg/common/a#r", 0)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap_err();
    assert!(
        matches!(
            error,
            VerificationError::NodeOriginMismatch { index: 0, .. }
        ),
        "{error:?}"
    );

    let mut smuggled = vec![
        node("spec://org.demo/pkg/common/a#r", "# A {#a}\n"),
        node("spec://org.demo/pkg/common/b#r", "# B {#b}\n"),
    ];
    smuggled[1].address = DocumentAddress::StaticEntry {
        origin: "host".to_string(),
        path: "boot/x.md".to_string(),
    };
    let error = verify(&closure(
        smuggled,
        Vec::new(),
        vec![occurrence("spec://org.demo/pkg/common/a#r", 0)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap_err();
    assert!(
        matches!(error, VerificationError::NodeAddressKind { index: 1 }),
        "{error:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn an_unconnected_node_is_unreachable_but_a_source_only_satellite_is_valid() {
    let stranded = vec![
        node("spec://org.demo/pkg/common/a#r", "# A {#a}\n"),
        node("spec://org.demo/pkg/common/b#r", "# B {#b}\n"),
        node("spec://org.demo/pkg/common/c#r", "# C {#c}\n"),
    ];
    let error = verify(&closure(
        stranded,
        vec![use_edge(0, 1, "spec://org.demo/pkg/common/b#r")],
        vec![occurrence("spec://org.demo/pkg/common/a#r", 0)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap_err();
    assert!(
        matches!(error, VerificationError::UnreachableNode { index: 2 }),
        "{error:?}"
    );

    // Node C is reached only by a source edge and never listed in an emission
    // order: retained provenance, not orphaning.
    let satellite = vec![
        node("spec://org.demo/pkg/common/a#r", "# A {#a}\n"),
        node("spec://org.demo/pkg/common/c#r", "# C {#c}\n"),
    ];
    let source_edge = ClosureEdge {
        from: ClosureNodeId(0),
        to: ClosureNodeId(1),
        kind: ClosureEdgeKind::Source,
        requested_target: address("spec://org.demo/pkg/common/c#r"),
    };
    verify(&closure(
        satellite,
        vec![source_edge],
        vec![occurrence("spec://org.demo/pkg/common/a#r", 0)],
        0,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    ))
    .unwrap();
}
