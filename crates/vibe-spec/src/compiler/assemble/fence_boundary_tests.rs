//! The intrinsic validator's side of the contribution fence boundary: it
//! observes the closing fence of every contribution and reports it in
//! [`LaneShape`], but it never rules on it. The target policy is the inter-pass
//! verifier's — see `compiler::verify::markdown_boundary`.

use specmark::verifies;

use super::tests::{applied_closure, full_context};
use super::{AssemblePass, validate_lane, validate_shape};
use crate::compiler::ir::{ArtifactContext, LaneIr, LinkFenceSnapshot, StaticCompileMode};
use crate::compiler::pass::Pass;

/// This fixture's final occurrence deliberately leaves a ```` ```text ```` fence
/// open. The intrinsic validator accepts it — for StaticXml *and* retargeted at
/// StaticMarkdown — because whether an open boundary is legal is a target
/// policy, not a lane shape fact. What it does do is *report* the state, so the
/// verifier can rule without a second walk.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn the_intrinsic_validator_reports_the_closing_fence_without_ruling_on_it() {
    let closure = applied_closure(full_context());
    let lane = AssemblePass::new().run(closure).unwrap();
    assert!(
        validate_lane(&lane).is_ok(),
        "the StaticXml characterization keeps its open final occurrence"
    );

    let shape = validate_shape(&lane).expect("the fixture lane is well shaped");
    assert_eq!(
        shape.closing_fences[0],
        LinkFenceSnapshot::Open {
            delimiter: '`',
            run: 3,
        },
        "the summary carries the exact final state: {:?}",
        shape.closing_fences
    );
    assert!(
        shape.closing_fences[1..]
            .iter()
            .all(|closing| *closing == LinkFenceSnapshot::Closed),
        "every other contribution closes: {:?}",
        shape.closing_fences
    );

    // The very same contributions retargeted at StaticMarkdown: still shape-
    // valid, because `validate_lane` is what production runs and R3.3 must not
    // change a single production verdict.
    let markdown = markdown_retarget(&lane);
    assert!(
        validate_lane(&markdown).is_ok(),
        "production keeps accepting exactly what it accepted before R3.3"
    );
    assert_eq!(
        validate_shape(&markdown)
            .expect("shape is target-independent")
            .closing_fences,
        shape.closing_fences,
        "the summary does not depend on the target either"
    );
}

/// The verifier's side of the same fixture: StaticXml keeps its open final
/// occurrence, and only retargeting the identical contributions at
/// StaticMarkdown makes the boundary law fire. This is what keeps the
/// structured-target exemption non-vacuous.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn the_verifier_exempts_structured_targets_and_refuses_the_markdown_retarget() {
    use crate::compiler::pass::AnyIr;
    use crate::compiler::verify::{IrVerifier, VerificationError};

    let closure = applied_closure(full_context());
    let lane = AssemblePass::new().run(closure).unwrap();
    IrVerifier
        .verify(&AnyIr::Lane(lane.clone()))
        .expect("StaticXml renders lane nodes, so an open final fence is legal");

    let error = IrVerifier
        .verify(&AnyIr::Lane(markdown_retarget(&lane)))
        .unwrap_err();
    assert!(
        matches!(
            error,
            VerificationError::ContributionFenceOpen {
                contribution: 0,
                delimiter: '`',
                run: 3,
            }
        ),
        "{error:?}"
    );
}

/// The same contributions under a StaticMarkdown compatibility context.
pub(super) fn markdown_retarget(lane: &LaneIr) -> LaneIr {
    let mut frame = lane.frame.clone();
    frame.generated_path = None;
    frame.source_root = None;
    LaneIr::assembled(
        ArtifactContext::compatibility(StaticCompileMode::QualifyPerNode),
        lane.source_node_count,
        lane.source_link_digest.clone(),
        frame,
        lane.contributions.clone(),
    )
}
