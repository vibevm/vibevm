use super::test_support::*;
use super::*;

#[test]
fn unclosed_fence_makes_shared_occurrences_contextual_and_positional() {
    let nodes = vec![
        normal_node(
            "spec://org.demo/a/boot/open#root",
            "org.demo/a",
            "##org-demo--a--X definition\n```",
        ),
        normal_node(
            "spec://org.demo/a/boot/shared#root",
            "org.demo/a",
            "(#X)\n```\n(#X)",
        ),
    ];
    let input = closure(
        StaticCompileMode::QualifyPerNode,
        nodes,
        vec![normal("org.demo/a", "boot/open", 0, &[0, 1, 1])],
        vec![rename("org.demo/a", "X", "org-demo--a--X")],
    );

    let output = LinkPass::new().run(input).unwrap();
    let occurrences: Vec<_> = linked_result(&output)
        .occurrences
        .iter()
        .map(|occurrence| match occurrence {
            LinkOccurrence::Normal {
                contribution,
                occurrence,
                fence_before,
                fence_after,
                body,
                ..
            } => (
                *contribution,
                *occurrence,
                fence_before,
                fence_after,
                body.as_str(),
            ),
            LinkOccurrence::Simple { .. } => unreachable!(),
        })
        .collect();

    assert_eq!(occurrences.len(), 3);
    assert_eq!(
        occurrences
            .iter()
            .map(|(contribution, occurrence, ..)| (*contribution, *occurrence))
            .collect::<Vec<_>>(),
        [(0, 0), (0, 1), (0, 2)]
    );
    assert!(matches!(occurrences[1].2, LinkFenceSnapshot::Open { .. }));
    assert_eq!(occurrences[1].4, "(#X)\n```\n(#org-demo--a--X)");
    assert!(matches!(occurrences[1].3, LinkFenceSnapshot::Closed));
    assert!(matches!(occurrences[2].2, LinkFenceSnapshot::Closed));
    assert_eq!(occurrences[2].4, "(#org-demo--a--X)\n```\n(#X)");
    assert!(matches!(occurrences[2].3, LinkFenceSnapshot::Open { .. }));
    assert_ne!(occurrences[1].4, occurrences[2].4);

    let mut collapsed = output;
    let LinkState::Linked(result) = &mut collapsed.link else {
        unreachable!()
    };
    let duplicate = result
        .occurrences
        .iter()
        .rposition(|occurrence| matches!(occurrence, LinkOccurrence::Normal { .. }))
        .unwrap();
    result.occurrences.remove(duplicate);
    assert!(matches!(
        validate_linked(&collapsed),
        Err(LinkPassError::ReplayMismatch {
            field: "linked occurrences"
        })
    ));
}
