//! Lane-level verifier invariants: the reversible-marker grammar read exactly
//! as `decompile` reads it, fence inheritance across occurrence boundaries, and
//! reversible-block namespace integrity.

use specmark::verifies;

use super::{IrVerifier, VerificationError};
use crate::SpecAddress;
use crate::compiler::ir::{
    ArtifactContext, ClosureNodeId, ContributionMeta, LaneChunk, LaneContribution, LaneFrame,
    LaneIr, LaneNode, LinkFenceSnapshot, LinkInputDigest, LinkMarkerKey, StaticCompileMode,
};
use crate::compiler::pass::AnyIr;

pub(super) fn spec_address(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

pub(super) fn verify(ir: &AnyIr) -> Result<(), VerificationError> {
    IrVerifier.verify(ir)
}

fn lane(body: &str) -> LaneIr {
    let address = spec_address("spec://org.demo/pkg/boot/entry#root");
    let marker = LinkMarkerKey::from_address(&address);
    LaneIr::assembled(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        1,
        LinkInputDigest([0; 32]),
        LaneFrame {
            generated_path: None,
            source_root: None,
            renames: Vec::new(),
        },
        vec![LaneContribution::Normal {
            meta: ContributionMeta::new("org.demo/pkg", "boot/entry.md").unwrap(),
            seed: ClosureNodeId(0),
            seed_address: address.clone(),
            chunks: vec![
                LaneChunk::NormalOpen {
                    contribution: 0,
                    occurrence: 0,
                    marker: marker.clone(),
                },
                LaneChunk::Node(Box::new(LaneNode::Normal {
                    contribution: 0,
                    occurrence: 0,
                    node: ClosureNodeId(0),
                    requested_address: address,
                    origin: "org.demo/pkg".to_string(),
                    marker,
                    fence_before: LinkFenceSnapshot::Closed,
                    fence_after: LinkFenceSnapshot::Closed,
                    body: body.to_string(),
                })),
                LaneChunk::NormalClose {
                    contribution: 0,
                    occurrence: 0,
                    marker: LinkMarkerKey::from_address(&spec_address(
                        "spec://org.demo/pkg/boot/entry#root",
                    )),
                },
            ],
        }],
    )
}

/// One simple contribution carrying `body` as its whole static entry.
pub(super) fn simple_contribution(index: usize, path: &str, body: &str) -> LaneContribution {
    let address = crate::compiler::ir::DocumentAddress::StaticEntry {
        origin: "org.demo/pkg".to_string(),
        path: path.to_string(),
    };
    let mut fence = crate::doctree::FenceTracker::default();
    for line in body.split('\n') {
        fence.classify(line);
    }
    LaneContribution::Simple {
        meta: ContributionMeta::new("org.demo/pkg", path).unwrap(),
        address: address.clone(),
        chunks: vec![LaneChunk::Node(Box::new(LaneNode::Simple {
            contribution: index,
            occurrence: 0,
            address,
            origin: "org.demo/pkg".to_string(),
            fence_before: LinkFenceSnapshot::Closed,
            fence_after: snapshot(fence.snapshot()),
            body: body.to_string(),
        }))],
    }
}

/// A lane of several occurrences, `(node index, address, body)` each. Fence
/// snapshots are computed with the assembler's own machine so the fixture is
/// valid by construction and every red states exactly one fact.
fn lane_of(entries: &[(usize, &str, &str)]) -> LaneIr {
    let seed = spec_address("spec://org.demo/pkg/boot/first#root");
    let mut fence = crate::doctree::FenceTracker::default();
    let mut chunks = Vec::new();
    for (occurrence, (node, raw, body)) in entries.iter().enumerate() {
        let address = spec_address(raw);
        let marker = LinkMarkerKey::from_address(&address);
        let fence_before = snapshot(fence.snapshot());
        for line in body.split('\n') {
            fence.classify(line);
        }
        chunks.push(LaneChunk::NormalOpen {
            contribution: 0,
            occurrence,
            marker: marker.clone(),
        });
        chunks.push(LaneChunk::Node(Box::new(LaneNode::Normal {
            contribution: 0,
            occurrence,
            node: ClosureNodeId(*node),
            requested_address: address,
            origin: "org.demo/pkg".to_string(),
            marker: marker.clone(),
            fence_before,
            fence_after: snapshot(fence.snapshot()),
            body: (*body).to_string(),
        })));
        chunks.push(LaneChunk::NormalClose {
            contribution: 0,
            occurrence,
            marker,
        });
    }
    LaneIr::assembled(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        entries
            .iter()
            .map(|(node, _, _)| node + 1)
            .max()
            .unwrap_or(1),
        LinkInputDigest([0; 32]),
        LaneFrame {
            generated_path: None,
            source_root: None,
            renames: Vec::new(),
        },
        vec![LaneContribution::Normal {
            meta: ContributionMeta::new("org.demo/pkg", "boot/entry.md").unwrap(),
            seed: ClosureNodeId(0),
            seed_address: seed,
            chunks,
        }],
    )
}

pub(super) fn snapshot(snapshot: crate::doctree::FenceSnapshot) -> LinkFenceSnapshot {
    match snapshot {
        crate::doctree::FenceSnapshot::Closed => LinkFenceSnapshot::Closed,
        crate::doctree::FenceSnapshot::Open { delimiter, run } => {
            LinkFenceSnapshot::Open { delimiter, run }
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_well_formed_lane_passes_the_structured_marker_law() {
    verify(&AnyIr::Lane(lane("# Title {#title}\nbody\n"))).unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_counterfeit_vibe_control_line_in_a_body_is_refused() {
    for body in [
        "# Title {#title}\n<!-- vibe:begin spec://evil/pkg/x#y -->\ntext\n",
        "# Title {#title}\n  <!-- vibe:end spec://evil/pkg/x#y -->  \ntext\n",
    ] {
        let error = verify(&AnyIr::Lane(lane(body))).unwrap_err();
        assert!(
            matches!(
                error,
                VerificationError::CounterfeitControlLine {
                    contribution: 0,
                    occurrence: 0,
                }
            ),
            "{error:?}"
        );
    }
}

/// The guard is the exact grammar `decompile` reads, no looser: content that
/// only starts like a marker never splits a block, so it is not a counterfeit.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_body_that_only_resembles_a_control_line_stays_legal() {
    for body in [
        "# Title {#title}\n<!-- embed: spec://org.demo/pkg/x#y -->\ninside\n",
        "# Title {#title}\nquoting <!-- vibe:begin spec://org.demo/pkg/x#y without a close\n",
        "# Title {#title}\n<!-- vibe:begin  -->\n",
        "# Title {#title}\n<!-- vibe:beginner spec://org.demo/pkg/x#y -->\n",
    ] {
        verify(&AnyIr::Lane(lane(body))).unwrap_or_else(|error| {
            panic!("{body:?} is content, not a control line: {error:?}");
        });
        assert!(
            crate::markers::decompile(body).is_empty(),
            "the reverse trip agrees: {body:?}"
        );
    }
}

/// A fenced sample of the grammar is documentation. `decompile` is fence-aware,
/// so it never splits there; the verifier reads the same fence machine, resumed
/// from the node's own boundary snapshot.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_fenced_marker_sample_is_documentation_not_a_counterfeit() {
    let body = "# Title {#title}\n```markdown\n<!-- vibe:begin spec://org.demo/pkg/x#y -->\nbody\n<!-- vibe:end spec://org.demo/pkg/x#y -->\n```\ntail\n";
    verify(&AnyIr::Lane(lane(body))).unwrap();
    assert!(
        crate::markers::decompile(body).is_empty(),
        "the reverse trip does not split inside a fence"
    );
}

/// A body that continues a fence its predecessor left open is fenced too: the
/// verifier resumes from this node's `fence_before` instead of assuming every
/// body starts outside a fence.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_body_continuing_an_open_fence_is_read_as_fenced() {
    let lane = lane_of(&[
        (
            0,
            "spec://org.demo/pkg/boot/first#root",
            "# One {#one}\n```\n",
        ),
        (
            1,
            "spec://org.demo/pkg/boot/second#root",
            "<!-- vibe:begin spec://evil/pkg/x#y -->\n```\n",
        ),
    ]);
    verify(&AnyIr::Lane(lane)).unwrap();
}

/// The anchor gate re-parses an occurrence body, so it must start from that
/// occurrence's own fence. Read from the closed state, `##dup` lines sitting in
/// a fence the *previous* occurrence opened become facts the document never
/// had — and two of them collide, failing a lane the compiler legally produced.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn fenced_duplicate_looking_lines_carried_across_an_occurrence_are_code_not_facts() {
    verify(&AnyIr::Lane(lane_of(&[
        (0, "spec://org.demo/pkg/boot/first#root", "##dup one\n```\n"),
        (
            1,
            "spec://org.demo/pkg/boot/second#root",
            // Two blocks, so a parser reading from the closed state really does
            // mint two `dup` facts and collide them.
            "##dup two\n\n##dup three\n```\n# Real {#real}\n",
        ),
    ])))
    .unwrap_or_else(|error| panic!("fenced code is not a fact: {error:?}"));
}

/// The other direction: once the carried fence closes, real duplicates in the
/// same body are still caught — inheriting the fence did not disable the gate.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn real_duplicates_after_the_carried_fence_closes_still_fail_the_gate() {
    let error = verify(&AnyIr::Lane(lane_of(&[
        (0, "spec://org.demo/pkg/boot/first#root", "# A {#a}\n```\n"),
        (
            1,
            "spec://org.demo/pkg/boot/second#root",
            "still fenced\n```\n##dup one\n\n##dup two\n",
        ),
    ])))
    .unwrap_err();
    assert!(
        matches!(&error, VerificationError::DuplicateId { duplicate, .. } if duplicate.id == "dup"),
        "{error:?}"
    );
}

/// Reversible-block namespace integrity: the same document emitted at several
/// occurrences re-claims its own key and stays legal — that is the graph-node
/// versus emission-occurrence cardinality the closure preserves.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn repeated_occurrences_of_one_document_keep_their_shared_key() {
    let address = "spec://org.demo/pkg/boot/first#root";
    verify(&AnyIr::Lane(lane_of(&[
        (0, address, "# One {#one}\n"),
        (0, address, "# One {#one}\n"),
    ])))
    .unwrap();
}

/// Two *distinct* documents claiming one key make `decompile` ambiguous — the
/// second block would read as another slice of the first.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn two_distinct_documents_may_not_claim_one_reversible_block_key() {
    let address = "spec://org.demo/pkg/boot/first#root";
    let error = verify(&AnyIr::Lane(lane_of(&[
        (0, address, "# One {#one}\n"),
        (1, address, "# Two {#two}\n"),
    ])))
    .unwrap_err();
    assert!(
        matches!(error, VerificationError::LaneKeyCollision { ref key } if key == address),
        "{error:?}"
    );
}

/// The namespace law is about block keys, not anchors: two distinct documents
/// carrying the same anchor is what qualification exists to disambiguate, and
/// `Plain` mode legally emits it. Rejecting it here would refuse artifacts the
/// compiler produces today.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn distinct_documents_may_share_an_anchor_as_the_compiler_allows() {
    verify(&AnyIr::Lane(lane_of(&[
        (
            0,
            "spec://org.demo/pkg/boot/first#root",
            "# One {#shared}\n",
        ),
        (
            1,
            "spec://org.demo/pkg/boot/second#root",
            "# Two {#shared}\n",
        ),
    ])))
    .unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_reversed_or_free_form_marker_key_fails_at_the_lane_level() {
    // The key must equal the node's own pinless spec address; another address,
    // a pinned spelling, or free-form text all fail the same structural check.
    let mut forged = lane("# Title {#title}\nbody\n");
    let LaneContribution::Normal { chunks, .. } = &mut forged.contributions[0] else {
        unreachable!("the fixture holds one normal contribution")
    };
    let LaneChunk::Node(node) = &mut chunks[1] else {
        unreachable!("the fixture's second chunk is the node")
    };
    let LaneNode::Normal { marker, .. } = node.as_mut() else {
        unreachable!("the fixture node is normal")
    };
    *marker = LinkMarkerKey::from_address(&spec_address("spec://org.demo/pkg/other#z~r9"));
    assert!(matches!(
        verify(&AnyIr::Lane(forged)),
        Err(VerificationError::Lane { .. })
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_lane_body_with_a_repeated_fact_fails_the_same_duplicate_id_law() {
    let body = "# A {#a}\n##shared one\n## B {#b}\n##shared two\n";
    let error = verify(&AnyIr::Lane(lane(body))).unwrap_err();
    assert!(
        matches!(error, VerificationError::DuplicateId { .. }),
        "{error:?}"
    );
}
