//! The Markdown contribution fence boundary: a fence may carry between the
//! occurrences of one normal contribution, but never past that contribution's
//! end. Structured targets are deliberately exempt — see
//! `assemble::tests::an_open_final_fence_is_legal_for_xml_and_refused_for_markdown`.

use specmark::verifies;

use super::VerificationError;
use super::lane_tests::{simple_contribution, snapshot, spec_address, verify};
use crate::compiler::ir::{
    ArtifactContext, ClosureNodeId, ContributionMeta, LaneChunk, LaneContribution, LaneFrame,
    LaneIr, LaneNode, LinkInputDigest, LinkMarkerKey, StaticCompileMode,
};
use crate::compiler::pass::AnyIr;

/// One normal contribution from `(node index, address, body)` occurrences, with
/// fence snapshots computed by the assembler's own machine.
fn normal_contribution(index: usize, entries: &[(usize, &str, &str)]) -> LaneContribution {
    let seed = spec_address(entries[0].1);
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
            contribution: index,
            occurrence,
            marker: marker.clone(),
        });
        chunks.push(LaneChunk::Node(Box::new(LaneNode::Normal {
            contribution: index,
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
            contribution: index,
            occurrence,
            marker,
        });
    }
    LaneContribution::Normal {
        meta: ContributionMeta::new("org.demo/pkg", format!("boot/c{index}.md")).unwrap(),
        seed: ClosureNodeId(entries[0].0),
        seed_address: seed,
        chunks,
    }
}

/// A Markdown lane of several top-level contributions.
fn markdown_lane(contributions: Vec<LaneContribution>, nodes: usize) -> LaneIr {
    LaneIr::assembled(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        nodes,
        LinkInputDigest([0; 32]),
        LaneFrame {
            generated_path: None,
            source_root: None,
            renames: Vec::new(),
        },
        contributions,
    )
}

/// The contribution index a fence-boundary refusal names, if that is what the
/// verifier returned. The error is the *verifier's* own — the intrinsic
/// validator has no such variant, because it does not rule on the boundary.
fn fence_open(error: &VerificationError) -> Option<usize> {
    match error {
        VerificationError::ContributionFenceOpen { contribution, .. } => Some(*contribution),
        _ => None,
    }
}

/// Render the lane through the real `static-md` backend, so the round trip is
/// asserted against production bytes rather than a test-local concatenation.
fn render_markdown(lane: &LaneIr) -> String {
    use crate::compiler::backend::EmitBackend;
    let backend = crate::compiler::emit::static_md::StaticMarkdownBackend::new();
    let witness = crate::compiler::emit::capture_witness_for_test(lane, backend.id())
        .expect("the fixture lane is emittable");
    let bytes = backend
        .emit(lane, &witness)
        .expect("the static-md backend renders a compatibility fragment");
    String::from_utf8(bytes).expect("static-md emits UTF-8")
}

/// The layering itself. R3.3's verifier is a `#[cfg(test)]` seam, so a lane the
/// boundary law refuses must still travel the *production* passes untouched:
/// `AssemblePass` and the `static-md` backend accept it and emit exactly the
/// bytes they emitted before R3.3, because `validate_lane` — the verdict every
/// production caller runs — never learned this law.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn with_the_verifier_off_an_unclosed_markdown_contribution_still_compiles() {
    let lane = unclosed_markdown_lane();
    crate::compiler::assemble::validate_lane(&lane)
        .expect("the intrinsic production verdict is unchanged by R3.3");

    let AnyIr::Lane(carried) = producing_segment(&lane)
        .run(AnyIr::Closure(valid_closure()))
        .expect("a verifier-off segment carries the lane through unchanged")
    else {
        panic!("the producing pass returns a lane")
    };
    assert_eq!(carried, lane, "production leaves the carrier untouched");

    assert_eq!(
        render_markdown(&lane),
        concat!(
            "<!-- vibe:begin spec://org.demo/pkg/boot/first#root -->\n",
            "# One {#one}\n",
            "```\n",
            "<!-- vibe:end spec://org.demo/pkg/boot/first#root -->\n",
            "plain text\n",
        ),
        "the pre-R3.3 bytes are emitted verbatim, open fence and all"
    );
}

/// The identical carrier under the test-only seam: `run_checked` with a verifier
/// fails, under the producing pass's own name, through a verifier-owned error.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn the_same_carrier_fails_under_the_producing_pass_when_verify_each_is_on() {
    let lane = unclosed_markdown_lane();
    let error = producing_segment(&lane)
        .run_checked(AnyIr::Closure(valid_closure()), Some(super::IrVerifier))
        .unwrap_err();
    let crate::compiler::pass::PassSegmentError::VerificationFailed { pass, source, .. } = error
    else {
        panic!("expected the producing pass to be named, got {error:?}")
    };
    assert_eq!(pass.as_str(), PRODUCER, "the pass that built the lane");
    assert_eq!(fence_open(&source), Some(0), "{source:?}");
}

/// A registered custom backend consumes lane nodes exactly as StaticXml does,
/// so the same carrier that Markdown refuses is legal for it. Only the target
/// changes between the two verdicts.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_custom_target_is_exempt_from_the_markdown_boundary_law() {
    let markdown = unclosed_markdown_lane();
    assert_eq!(
        fence_open(&verify(&AnyIr::Lane(markdown.clone())).unwrap_err()),
        Some(0),
        "the Markdown target refuses it"
    );

    let custom = LaneIr::assembled(
        crate::compiler::ir::ArtifactContext::new(
            crate::compiler::ir::ArtifactId::new("opaque").unwrap(),
            crate::compiler::ir::ArtifactTarget::custom("opaque")
                .expect("opaque is a valid backend id"),
            crate::compiler::ir::ArtifactFrame::CompatibilityFragment,
            StaticCompileMode::Plain,
        )
        .expect("a custom backend pairs with the compatibility fragment"),
        markdown.source_node_count,
        markdown.source_link_digest.clone(),
        markdown.frame.clone(),
        markdown.contributions.clone(),
    );
    verify(&AnyIr::Lane(custom)).expect("a structured custom backend is exempt");
}

/// One Markdown contribution ending in an unclosed fence, followed by another.
fn unclosed_markdown_lane() -> LaneIr {
    markdown_lane(
        vec![
            normal_contribution(
                0,
                &[(
                    0,
                    "spec://org.demo/pkg/boot/first#root",
                    "# One {#one}\n```\n",
                )],
            ),
            simple_contribution(1, "boot/simple.md", "plain text\n"),
        ],
        1,
    )
}

const PRODUCER: &str = "lane-producer";

/// A valid closure to feed the segment, so the only thing under test is the
/// lane the pass *produces*.
fn valid_closure() -> crate::compiler::ir::ClosureIr {
    super::closure_tests::minimal_closure()
}

/// A one-pass `Closure -> Lane` segment returning the fixture lane, so the
/// carrier crosses a real manager boundary as a pass **output** — which is what
/// makes the failure nameable by its producing pass — with and without the
/// verifier and nothing else about it changed.
fn producing_segment(lane: &LaneIr) -> crate::compiler::pass::PassSegment {
    struct Producer {
        name: crate::compiler::pass::PassName,
        lane: LaneIr,
    }
    impl crate::compiler::pass::Pass for Producer {
        type Input = crate::compiler::ir::ClosureIr;
        type Output = LaneIr;
        type Error = std::convert::Infallible;

        fn name(&self) -> &crate::compiler::pass::PassName {
            &self.name
        }

        fn run(&self, _input: Self::Input) -> Result<LaneIr, Self::Error> {
            Ok(self.lane.clone())
        }
    }

    let mut segment = crate::compiler::pass::PassSegment::default();
    segment
        .push(Producer {
            name: crate::compiler::pass::PassName::new(PRODUCER)
                .expect("a static test pass name is non-blank"),
            lane: lane.clone(),
        })
        .expect("a closure-to-lane pass is a valid artifact segment");
    segment
}

/// A Markdown contribution may not leave a fence open behind it: the next
/// contribution's body, the generated framing and the markers themselves would
/// all be read as code. The verifier refuses it before any backend runs, naming
/// the contribution that failed to close.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_markdown_normal_contribution_may_not_leave_a_fence_open_for_the_next_one() {
    let lane = markdown_lane(
        vec![
            normal_contribution(
                0,
                &[(
                    0,
                    "spec://org.demo/pkg/boot/first#root",
                    "# One {#one}\n```\n",
                )],
            ),
            simple_contribution(1, "boot/simple.md", "plain text\n"),
        ],
        1,
    );
    let error = verify(&AnyIr::Lane(lane)).unwrap_err();
    assert_eq!(fence_open(&error), Some(0), "{error:?}");
}

/// The same law for a simple contribution: its static entry may not escape
/// either.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_markdown_simple_contribution_may_not_leave_a_fence_open_for_the_next_one() {
    let lane = markdown_lane(
        vec![
            simple_contribution(0, "boot/simple.md", "```\n"),
            normal_contribution(
                1,
                &[(0, "spec://org.demo/pkg/boot/second#root", "# Two {#two}\n")],
            ),
        ],
        1,
    );
    let error = verify(&AnyIr::Lane(lane)).unwrap_err();
    assert_eq!(fence_open(&error), Some(0), "{error:?}");
}

/// The carried-fence support the law must not weaken: a fence opened in one
/// occurrence and closed in the next, inside a single contribution, is legal —
/// a further contribution may follow it, and the rendered Markdown decompiles
/// into exactly the blocks the lane declared.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_fence_carried_between_occurrences_of_one_contribution_stays_legal() {
    let first = "spec://org.demo/pkg/boot/first#root";
    let second = "spec://org.demo/pkg/boot/second#root";
    let lane = markdown_lane(
        vec![
            normal_contribution(
                0,
                &[
                    (0, first, "# One {#one}\n```\n"),
                    (1, second, "still code\n```\n"),
                ],
            ),
            simple_contribution(1, "boot/simple.md", "after\n"),
        ],
        2,
    );
    verify(&AnyIr::Lane(lane.clone())).unwrap();

    let rendered = render_markdown(&lane);
    let blocks = crate::markers::decompile(&rendered);
    let keys: Vec<&str> = blocks.iter().map(|block| block.key.as_str()).collect();
    assert_eq!(keys, [first, second], "exact blocks from {rendered:?}");
    assert_eq!(blocks[0].body, "# One {#one}\n```");
    assert_eq!(blocks[1].body, "still code\n```");
    assert!(
        rendered.ends_with("after\n"),
        "the following contribution is plain text, not code: {rendered:?}"
    );
}
