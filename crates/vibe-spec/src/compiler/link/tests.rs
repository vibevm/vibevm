use specmark::verifies;

use super::test_support::*;
use super::*;
use crate::compiler::pass::{AnyIr, PassSegment, PassSegmentError};

fn mask_link(mut closure: ClosureIr) -> ClosureIr {
    closure.link = LinkState::Unlinked;
    closure
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn heterogeneous_global_linking_is_once_ordered_and_preserves_the_carrier() {
    let nodes = vec![
        normal_node(
            "spec://org.demo/a/boot/a#root",
            "org.demo/a",
            "A sees (#SIMPLE) and (#MISSING)",
        ),
        normal_node("spec://org.demo/b/boot/b#root", "org.demo/b", "B body"),
    ];
    let contributions = vec![
        normal("org.demo/a", "boot/a", 0, &[0]),
        simple("host", "boot/local.md", "simple sees (#NORMAL)"),
        normal("org.demo/b", "boot/b", 1, &[1]),
    ];
    let input = closure(
        StaticCompileMode::QualifyPerNode,
        nodes,
        contributions,
        vec![
            rename("org.demo/a", "NORMAL", "org-demo--a--NORMAL"),
            rename("host", "SIMPLE", "host--SIMPLE"),
        ],
    );
    let before = input.clone();
    reset_link_invocations();

    let output = LinkPass::new().run(input).unwrap();

    assert_eq!(link_invocations(), 1);
    assert_eq!(
        occurrence_bytes(linked_result(&output)),
        [
            "A sees (#host--SIMPLE) and (#MISSING)",
            "simple sees (#org-demo--a--NORMAL)",
            "B body",
        ]
    );
    assert!(matches!(
        linked_result(&output).contributions.as_slice(),
        [
            LinkContributionWitness::Normal { .. },
            LinkContributionWitness::Simple { .. },
            LinkContributionWitness::Normal { .. },
        ]
    ));
    let positions: Vec<_> = linked_result(&output)
        .chunks
        .iter()
        .filter_map(|chunk| match chunk {
            LinkChunk::NormalOccurrence {
                contribution,
                occurrence,
                node,
                ..
            } => Some((*contribution, *occurrence, Some(node.0))),
            LinkChunk::SimpleOccurrence {
                contribution,
                occurrence,
                ..
            } => Some((*contribution, *occurrence, None)),
            LinkChunk::Literal { .. } => None,
        })
        .collect();
    assert_eq!(positions, [(0, 0, Some(0)), (1, 0, None), (2, 0, Some(1))]);
    assert_eq!(mask_link(before), mask_link(output.clone()));
    validate_linked(&output).unwrap();
}

#[test]
fn plain_is_identity_and_empty_and_double_states_are_explicit() {
    let input = closure(
        StaticCompileMode::Plain,
        vec![normal_node(
            "spec://org.demo/a/boot/a#root",
            "org.demo/a",
            "(#X)",
        )],
        vec![normal("org.demo/a", "boot/a", 0, &[0])],
        vec![rename("one", "X", "one--X"), rename("two", "X", "two--X")],
    );
    let output = LinkPass::new().run(input).unwrap();
    assert_eq!(occurrence_bytes(linked_result(&output)), ["(#X)"]);
    assert!(matches!(
        LinkPass::new().run(output),
        Err(LinkPassError::AlreadyLinked)
    ));

    let empty = closure(StaticCompileMode::Plain, Vec::new(), Vec::new(), Vec::new());
    let empty = LinkPass::new().run(empty).unwrap();
    assert!(linked_result(&empty).contributions.is_empty());
    assert!(linked_result(&empty).chunks.is_empty());

    let empty_normal = closure(
        StaticCompileMode::Plain,
        vec![normal_node(
            "spec://org.demo/a/boot/a#root",
            "org.demo/a",
            "seed-only",
        )],
        vec![normal("org.demo/a", "boot/a", 0, &[])],
        Vec::new(),
    );
    let empty_normal = LinkPass::new().run(empty_normal).unwrap();
    assert!(matches!(
        linked_result(&empty_normal).contributions.as_slice(),
        [LinkContributionWitness::Normal {
            occurrence_count: 0,
            ..
        }]
    ));
    assert!(linked_result(&empty_normal).chunks.is_empty());
}

#[test]
fn ambiguity_keeps_duplicate_candidates_and_manager_attribution() {
    let input = closure(
        StaticCompileMode::QualifyPerNode,
        vec![normal_node(
            "spec://org.demo/a/boot/a#root",
            "org.demo/a",
            "See (#X)",
        )],
        vec![normal("org.demo/a", "boot/a", 0, &[0])],
        vec![
            rename("same", "X", "same--X"),
            rename("same", "X", "same--X"),
        ],
    );
    let mut segment = PassSegment::default();
    segment.push(LinkPass::new()).unwrap();
    let error = segment.run(AnyIr::Closure(input)).unwrap_err();
    let PassSegmentError::PassFailed { pass, source } = error else {
        panic!("ambiguity must remain a named pass failure")
    };
    assert_eq!(pass.as_str(), LINK_PASS_NAME);
    assert!(matches!(
        source.downcast_ref::<LinkPassError>(),
        Some(LinkPassError::AmbiguousShortLink { label, candidates })
            if label == "X"
                && candidates == &["same--X (same)".to_string(), "same--X (same)".to_string()]
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn digest_and_replay_reject_self_asserted_and_post_link_drift() {
    let input = closure(
        StaticCompileMode::QualifyPerNode,
        vec![normal_node(
            "spec://org.demo/a/boot/a#root",
            "org.demo/a",
            "(#X)",
        )],
        vec![normal("org.demo/a", "boot/a", 0, &[0])],
        vec![rename("org.demo/a", "X", "org-demo--a--X")],
    );
    let output = LinkPass::new().run(input).unwrap();

    let mut digest = output.clone();
    let LinkState::Linked(result) = &mut digest.link else {
        unreachable!()
    };
    result.input_digest.0[0] ^= 1;
    assert!(matches!(
        validate_linked(&digest),
        Err(LinkPassError::ReplayMismatch {
            field: "input digest"
        })
    ));

    let mut chunk = output.clone();
    let LinkState::Linked(result) = &mut chunk.link else {
        unreachable!()
    };
    let LinkChunk::NormalOccurrence { bytes, .. } = &mut result.chunks[1] else {
        panic!("the first body follows its open literal")
    };
    bytes.push_str(" MUTATED");
    assert!(matches!(
        validate_linked(&chunk),
        Err(LinkPassError::ReplayMismatch {
            field: "linked chunks"
        })
    ));

    let mut literal = output.clone();
    let LinkState::Linked(result) = &mut literal.link else {
        unreachable!()
    };
    let LinkChunk::Literal { bytes, .. } = &mut result.chunks[0] else {
        panic!("the stream begins with the normal open literal")
    };
    bytes.push(' ');
    assert!(matches!(
        validate_linked(&literal),
        Err(LinkPassError::ReplayMismatch {
            field: "linked chunks"
        })
    ));

    let mut position = output.clone();
    let LinkState::Linked(result) = &mut position.link else {
        unreachable!()
    };
    let LinkChunk::NormalOccurrence { occurrence, .. } = &mut result.chunks[1] else {
        unreachable!()
    };
    *occurrence = 99;
    assert!(matches!(
        validate_linked(&position),
        Err(LinkPassError::ReplayMismatch {
            field: "linked chunks"
        })
    ));

    let mut body = output.clone();
    body.nodes[0].tree = crate::DocTree::parse("changed");
    assert!(matches!(
        validate_linked(&body),
        Err(LinkPassError::ReplayMismatch {
            field: "input digest"
        })
    ));

    let mut renames = output.clone();
    renames.renames[0].rename.qualified.push_str("-changed");
    assert!(matches!(
        validate_linked(&renames),
        Err(LinkPassError::ReplayMismatch {
            field: "input digest"
        })
    ));

    let mut order = output.clone();
    let ClosureContribution::Normal { emission_order, .. } = &mut order.contributions[0] else {
        unreachable!()
    };
    emission_order.clear();
    assert!(validate_linked(&order).is_err());

    let mut origin = output;
    origin.nodes[0].origin = "org.demo/changed".to_string();
    validate_linked(&origin).unwrap();
}

#[test]
fn exact_literals_pin_markers_pins_and_newlines_without_reparse() {
    let address = "spec://org.demo/a/boot/a#root~r7";
    let input = closure(
        StaticCompileMode::Plain,
        vec![normal_node(address, "org.demo/a", "BODY")],
        vec![normal("org.demo/a", "boot/a", 0, &[0])],
        Vec::new(),
    );
    let tree = input.nodes[0].tree.clone();
    let output = LinkPass::new().run(input).unwrap();
    assert_eq!(output.nodes[0].tree, tree);
    assert_eq!(
        linked_text(&output).unwrap(),
        concat!(
            "<!-- vibe:begin spec://org.demo/a/boot/a#root -->\n",
            "BODY\n",
            "<!-- vibe:end spec://org.demo/a/boot/a#root -->\n",
        )
    );
    assert!(matches!(
        linked_result(&output).chunks.as_slice(),
        [
            LinkChunk::Literal {
                kind: LinkLiteralKind::NormalOpen,
                ..
            },
            LinkChunk::NormalOccurrence { .. },
            LinkChunk::Literal {
                kind: LinkLiteralKind::ForcedNewline,
                bytes,
            },
            LinkChunk::Literal {
                kind: LinkLiteralKind::NormalClose,
                ..
            },
        ] if bytes == "\n"
    ));
}

#[test]
fn terminal_newline_variants_have_exact_equal_bytes_but_distinct_literals() {
    let mut outputs = Vec::new();
    let mut forced = Vec::new();
    for body in ["BODY", "BODY\n", "BODY\n\n"] {
        let input = closure(
            StaticCompileMode::Plain,
            vec![normal_node(
                "spec://org.demo/a/boot/a#root",
                "org.demo/a",
                body,
            )],
            vec![normal("org.demo/a", "boot/a", 0, &[0])],
            Vec::new(),
        );
        let output = LinkPass::new().run(input).unwrap();
        outputs.push(linked_text(&output).unwrap());
        forced.push(
            linked_result(&output)
                .chunks
                .iter()
                .filter(|chunk| {
                    matches!(
                        chunk,
                        LinkChunk::Literal {
                            kind: LinkLiteralKind::ForcedNewline,
                            ..
                        }
                    )
                })
                .count(),
        );
    }
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(outputs[1], outputs[2]);
    assert_eq!(forced, [1, 1, 0]);
}

#[test]
fn inline_backticks_are_line_local_and_only_outside_links_rewrite() {
    let input = closure(
        StaticCompileMode::QualifyPerNode,
        vec![normal_node(
            "spec://org.demo/a/boot/a#root",
            "org.demo/a",
            "`(#X)` and (#X)\n`open-only (#X)\n(#X) next-line",
        )],
        vec![normal("org.demo/a", "boot/a", 0, &[0])],
        vec![rename("org.demo/a", "X", "org-demo--a--X")],
    );
    let output = LinkPass::new().run(input).unwrap();
    assert_eq!(
        occurrence_bytes(linked_result(&output)),
        [concat!(
            "`(#X)` and (#org-demo--a--X)\n",
            "`open-only (#X)\n",
            "(#org-demo--a--X) next-line",
        )]
    );
}
