use super::*;
use crate::compiler::ir::{LaneChunk, LaneContribution, LaneNode, LinkFenceSnapshot};
use crate::compiler::pass::Pass;

#[test]
fn independent_expected_lane_proves_projector_and_transition_separately() {
    let closure = super::tests::fixture();
    let expected = super::tests::independent_expected_lane(&closure);
    validate_assembled_transition(&closure, &expected).unwrap();
    let actual = AssemblePass::new().run(closure).unwrap();
    assert_eq!(actual, expected);

    let LaneContribution::Normal {
        seed_address,
        chunks,
        ..
    } = &expected.contributions[0]
    else {
        unreachable!()
    };
    assert!(seed_address.to_string().ends_with("#root~r7"));
    assert_eq!(
        super::tests::normal_nodes(chunks).len(),
        3,
        "shared duplicate survives"
    );
    assert!(
        super::tests::normal_nodes(chunks)
            .iter()
            .any(|node| matches!(
                node,
                LaneNode::Normal {
                    fence_after: LinkFenceSnapshot::Open { .. },
                    ..
                }
            ))
    );
    let LaneContribution::Simple { chunks, .. } = &expected.contributions[1] else {
        unreachable!()
    };
    assert!(matches!(
        chunks.last(),
        Some(LaneChunk::ForcedNewline { .. })
    ));
    let LaneContribution::Normal { chunks, .. } = &expected.contributions[4] else {
        unreachable!()
    };
    assert!(chunks.is_empty(), "empty normal stays present");
}

#[test]
fn transition_cells_have_no_production_projector_dependency() {
    for source in [
        include_str!("transition.rs"),
        include_str!("transition/contributions.rs"),
    ] {
        assert!(!source.contains("project_lane"));
        assert!(!source.contains("super::project"));
        assert!(!source.contains("::project"));
    }
}
