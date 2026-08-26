//! Red fixtures for the private arena invariant cell: each malformed arena
//! returns its exact typed error without panicking.

use std::collections::HashMap;

use super::super::{DocTree, Node, NodeId, NodeKind};
use super::verify_structure;
use crate::doctree::DocTreeInvariantError;

/// The arena node the parser would have built, for mutation. `span` defaults to
/// the one line at `line`, and `heading_line` agrees with it.
fn heading(id: Option<&str>, level: u8, parent: Option<usize>, line: usize) -> Node {
    Node {
        id: id.map(str::to_string),
        level,
        kind: NodeKind::Heading,
        heading: String::new(),
        trailing: String::new(),
        heading_line: line,
        span: line..line + 1,
        parent: parent.map(NodeId),
        children: Vec::new(),
    }
}

fn fact(id: &str, parent: usize, line: usize) -> Node {
    let mut node = heading(Some(id), 0, Some(parent), line);
    node.kind = NodeKind::Fact;
    node
}

fn root(line_count: usize) -> Node {
    let mut node = heading(None, 0, None, 0);
    node.span = 0..line_count;
    node
}

fn arena(nodes: Vec<Node>, line_count: usize) -> DocTree {
    DocTree::corrupt_for_test(nodes, line_count)
}

/// A legal three-node arena, fully linked: root `0..3`, a level-1 heading owning
/// the whole document, and one fact leaf inside it. Every red below mutates
/// exactly one fact of this arena, so it states one thing.
fn healthy() -> Vec<Node> {
    let mut root = root(3);
    root.children = vec![NodeId(1)];
    let mut section = heading(Some("a"), 1, Some(0), 0);
    section.span = 0..3;
    section.children = vec![NodeId(2)];
    vec![root, section, fact("f", 1, 1)]
}

#[test]
fn a_parsed_tree_satisfies_every_structural_invariant() {
    let tree = DocTree::parse("# A {#a}\ntext\n## B {#b}\nmore\n##fact x\n");
    assert_eq!(verify_structure(&tree), Ok(()));
}

/// Real corpus shapes — repeated anchors, fenced code, an empty document, a
/// preamble before the first heading — all satisfy the strengthened law.
#[test]
fn parsed_corpus_shapes_satisfy_the_strengthened_law() {
    for source in [
        "",
        "no headings at all\n",
        "preamble\n# A {#a}\n##dup one\n\n## B {#dup}\ntail\n",
        "# A {#a}\n```markdown\n# not a heading {#a}\n```\n### deep {#d}\n",
        "# A {#a}\n###### six {#s}\n# B {#b}\n",
    ] {
        let tree = DocTree::parse(source);
        assert_eq!(verify_structure(&tree), Ok(()), "source: {source:?}");
    }
}

#[test]
fn a_healthy_hand_built_arena_passes() {
    assert_eq!(verify_structure(&arena(healthy(), 3)), Ok(()));
}

#[test]
fn an_empty_arena_is_a_typed_error() {
    let error = verify_structure(&DocTree::corrupt_for_test(Vec::new(), 0)).unwrap_err();
    assert_eq!(error, DocTreeInvariantError::EmptyArena);
}

/// One named root field and the mutation that breaks it.
type RootDamage = (&'static str, fn(&mut Node));

/// Every synthetic-root field is load-bearing, one at a time.
#[test]
fn each_synthetic_root_field_is_checked_by_name() {
    let cases: [RootDamage; 8] = [
        ("kind", |node| node.kind = NodeKind::Fact),
        ("level", |node| node.level = 2),
        ("id", |node| node.id = Some("r".to_string())),
        ("heading", |node| node.heading = "Title".to_string()),
        ("trailing", |node| node.trailing = ":add".to_string()),
        ("heading_line", |node| node.heading_line = 1),
        ("parent", |node| node.parent = Some(NodeId(0))),
        ("span", |node| node.span = 0..2),
    ];
    for (field, damage) in cases {
        let mut nodes = vec![root(3)];
        damage(&mut nodes[0]);
        let error = verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err();
        assert_eq!(
            error,
            DocTreeInvariantError::RootNotSynthetic { field },
            "field {field}"
        );
    }
}

#[test]
fn an_illegal_heading_level_is_rejected_with_its_index() {
    let mut nodes = healthy();
    nodes[1].level = 9;
    let error = verify_structure(&arena(nodes, 3)).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::IllegalHeadingLevel { index: 1, level: 9 }
    );
}

/// One named fact field and the mutation that breaks it.
type FactDamage = (&'static str, fn(&mut Vec<Node>));

/// Every field `flush_block` fills on a fact leaf is load-bearing, one at a
/// time. Each corruption is independent, and `arena` rebuilds the derived anchor
/// index from the mutated nodes — so dropping the id is caught by the *shape*
/// check, not by an index that happens to disagree.
#[test]
fn each_parsed_fact_field_is_checked_by_name() {
    let cases: [FactDamage; 6] = [
        ("id", |nodes| nodes[2].id = None),
        ("heading", |nodes| nodes[2].heading = "Title".to_string()),
        ("trailing", |nodes| nodes[2].trailing = ":add".to_string()),
        ("level", |nodes| nodes[2].level = 2),
        ("children", |nodes| {
            nodes.push(heading(Some("z"), 2, Some(2), 2));
            nodes[2].children = vec![NodeId(3)];
        }),
        ("position", |nodes| {
            // The root itself declared a fact: `flush_block` never mints one at
            // arena position 0.
            nodes[0].kind = NodeKind::Fact;
            nodes[0].id = Some("r".to_string());
        }),
    ];
    for (field, damage) in cases {
        let mut nodes = healthy();
        damage(&mut nodes);
        let error = verify_structure(&arena(nodes, 3)).unwrap_err();
        let expected = if field == "position" {
            // The root check runs first and is more specific.
            DocTreeInvariantError::RootNotSynthetic { field: "kind" }
        } else {
            DocTreeInvariantError::FactShape { index: 2, field }
        };
        assert_eq!(error, expected, "field {field}");
    }
}

/// A fact at arena position 0 that survives the root check — the root is a
/// heading, and a *second* arena slot is turned into the root's clone — still
/// fails the fact position law.
#[test]
fn a_fact_at_the_root_position_is_rejected_by_the_shape_law() {
    let mut root = root(3);
    root.kind = NodeKind::Fact;
    root.id = Some("r".to_string());
    root.level = 0;
    // Bypass the root law by asking the shape law directly about position 0.
    let error = super::verify_node_shape(0, &root, 3).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::FactShape {
            index: 0,
            field: "position",
        }
    );
}

#[test]
fn a_heading_line_disagreeing_with_its_span_is_rejected() {
    let mut nodes = healthy();
    nodes[1].heading_line = 2;
    assert_eq!(
        verify_structure(&arena(nodes, 3)).unwrap_err(),
        DocTreeInvariantError::HeadingLineMismatch {
            index: 1,
            heading_line: 2,
            start: 0,
        }
    );
}

#[test]
fn a_span_leaving_the_document_is_rejected_with_its_bounds() {
    let mut nodes = healthy();
    nodes[2].span = 1..9;
    assert_eq!(
        verify_structure(&arena(nodes, 3)).unwrap_err(),
        DocTreeInvariantError::SpanOutOfBounds {
            index: 2,
            start: 1,
            end: 9,
            len: 3,
        }
    );
}

#[test]
fn an_out_of_bounds_child_id_is_rejected_before_any_indexing() {
    let mut nodes = healthy();
    nodes[1].children.push(NodeId(7));
    let error = verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::ChildOutOfBounds {
            parent: 1,
            child: 7,
            len: 3,
        }
    );
}

#[test]
fn a_child_listed_twice_by_one_parent_is_rejected() {
    let mut nodes = healthy();
    nodes[0].children = vec![NodeId(1)];
    nodes[1].children = vec![NodeId(2), NodeId(2)];
    let error = verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::DuplicateChild {
            parent: 1,
            child: 2,
        }
    );
}

/// The incoming-edge count is what rules out adoption by two parents, adoption
/// by none, and an edge into the root.
#[test]
fn the_incoming_edge_count_is_exactly_one_for_every_non_root_node() {
    let mut nodes = healthy();
    nodes[0].children = vec![NodeId(1), NodeId(2)];
    nodes[1].children = vec![NodeId(2)];
    assert_eq!(
        verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err(),
        DocTreeInvariantError::IncomingEdges { index: 2, count: 2 }
    );

    let mut nodes = healthy();
    nodes[0].children = vec![NodeId(1)];
    nodes[1].children = Vec::new();
    assert_eq!(
        verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err(),
        DocTreeInvariantError::IncomingEdges { index: 2, count: 0 }
    );

    let mut nodes = healthy();
    nodes[0].children = vec![NodeId(1)];
    nodes[1].children = vec![NodeId(2), NodeId(0)];
    assert_eq!(
        verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err(),
        DocTreeInvariantError::IncomingEdges { index: 0, count: 1 }
    );
}

#[test]
fn a_child_whose_parent_pointer_disagrees_is_rejected() {
    let mut nodes = healthy();
    nodes[0].children = vec![NodeId(1), NodeId(2)];
    nodes[1].children = Vec::new();
    // Node 2 keeps `parent = 1` while the root adopted it.
    let error = verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::ParentMismatch {
            parent: 0,
            child: 2,
        }
    );
}

#[test]
fn a_two_node_child_cycle_is_detected_as_unreachable_not_as_a_panic() {
    // 1.parent = 2 and 2.parent = 1 with matching child lists: parent pointers
    // agree both ways and each has one incoming edge, but the component never
    // touches the root.
    let mut a = heading(Some("a"), 1, Some(2), 0);
    a.children = vec![NodeId(2)];
    let mut b = heading(Some("b"), 1, Some(1), 1);
    b.children = vec![NodeId(1)];
    let error = verify_structure(&DocTree::corrupt_for_test(vec![root(3), a, b], 3)).unwrap_err();
    assert_eq!(error, DocTreeInvariantError::UnreachableNode { index: 1 });
}

#[test]
fn a_child_span_escaping_its_parent_is_rejected() {
    let mut nodes = healthy();
    nodes[1].span = 1..2;
    nodes[1].heading_line = 1;
    nodes[2].span = 0..1;
    nodes[2].heading_line = 0;
    let error = verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::SpanNotNested {
            parent: 1,
            child: 2,
        }
    );
}

#[test]
fn siblings_out_of_document_order_or_overlapping_are_rejected() {
    let mut nodes = healthy();
    nodes[1].span = 0..3;
    nodes.push(fact("g", 1, 2));
    // Two facts under one heading, listed with the later one first.
    nodes[1].children = vec![NodeId(3), NodeId(2)];
    nodes[0].children = vec![NodeId(1)];
    let error = verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::ChildrenOutOfOrder {
            parent: 1,
            previous: 3,
            child: 2,
        }
    );
}

#[test]
fn a_heading_no_deeper_than_its_owner_is_rejected() {
    let mut nodes = healthy();
    nodes[1].span = 0..3;
    nodes[2] = heading(Some("b"), 1, Some(1), 1);
    let error = verify_structure(&arena(nodes, 3)).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::HeadingNotDeeper {
            parent: 1,
            child: 2,
        }
    );
}

#[test]
fn arena_order_that_is_not_document_order_is_rejected() {
    let mut nodes = healthy();
    nodes[1].span = 0..3;
    nodes[2] = fact("f", 1, 2);
    nodes.push(fact("g", 1, 1));
    nodes[1].children = vec![NodeId(3), NodeId(2)];
    nodes[0].children = vec![NodeId(1)];
    // Children are now in document order, but the arena mints them backwards.
    let error = verify_structure(&DocTree::corrupt_for_test(nodes, 3)).unwrap_err();
    assert_eq!(
        error,
        DocTreeInvariantError::ArenaOrderBroken {
            previous: 2,
            index: 3,
        }
    );
}

/// The anchor index is a derived view: it must name the FIRST arena occurrence
/// and nothing else.
#[test]
fn an_anchor_index_that_is_not_the_arena_first_occurrence_is_rejected() {
    let nodes = vec![root(3), heading(Some("a"), 1, Some(0), 0), fact("a", 1, 1)];
    let mut linked = nodes.clone();
    linked[0].children = vec![NodeId(1)];
    linked[1].children = vec![NodeId(2)];
    linked[1].span = 0..3;

    // Pointing at the second occurrence is the classic silent divergence.
    let anchors = HashMap::from([("a".to_string(), NodeId(2))]);
    let tree = DocTree::corrupt_index_for_test(linked.clone(), 3, anchors, vec!["a".to_string()]);
    assert_eq!(
        verify_structure(&tree).unwrap_err(),
        DocTreeInvariantError::AnchorIndex {
            anchor: "a".to_string(),
            expected: Some(1),
            actual: Some(2),
        }
    );

    // An anchor no node carries must not be indexed at all.
    let anchors = HashMap::from([
        ("a".to_string(), NodeId(1)),
        ("ghost".to_string(), NodeId(2)),
    ]);
    let tree = DocTree::corrupt_index_for_test(linked.clone(), 3, anchors, vec!["a".to_string()]);
    assert_eq!(
        verify_structure(&tree).unwrap_err(),
        DocTreeInvariantError::AnchorIndex {
            anchor: "ghost".to_string(),
            expected: None,
            actual: Some(2),
        }
    );

    // The repeats record must be exactly the arena's own repeats.
    let anchors = HashMap::from([("a".to_string(), NodeId(1))]);
    let tree = DocTree::corrupt_index_for_test(linked, 3, anchors, Vec::new());
    assert_eq!(
        verify_structure(&tree).unwrap_err(),
        DocTreeInvariantError::DuplicateAnchors {
            expected: vec!["a".to_string()],
            actual: Vec::new(),
        }
    );
}
