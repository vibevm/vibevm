//! The corpus is a PRODUCER CHAIN, not four independently authored JSON
//! levels. These tests re-run the real production phases over the persisted
//! documents and demand byte/field equality, so a value that no built-in run
//! could have produced is red here rather than plausible-looking.

use specmark::verifies;
use std::path::PathBuf;

use super::super::decode;
use crate::DocTree;
use crate::compiler::assemble::AssemblePass;
use crate::compiler::ir::{
    ClosureContribution, ClosureIr, LaneChunk, LaneContribution, LaneIr, LaneNode, LinkOccurrence,
    LinkState,
};
use crate::compiler::pass::{AnyIr, Pass};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn carrier(name: &str) -> AnyIr {
    let bytes = std::fs::read(corpus().join("valid").join(name)).unwrap();
    decode(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn closure() -> ClosureIr {
    let AnyIr::Closure(closure) = carrier("closure_artifact.json") else {
        panic!("closure_artifact.json is the closure carrier");
    };
    closure
}

fn lane() -> LaneIr {
    let AnyIr::Lane(lane) = carrier("lane_artifact.json") else {
        panic!("lane_artifact.json is the lane carrier");
    };
    lane
}

/// Every closure node's tree is what the production parser makes of that
/// node's own lines — the persisted arena is a projection of authored text,
/// never hand-shaped structure.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn every_closure_node_tree_is_the_production_parse_of_its_own_lines() {
    for name in ["closure_artifact.json", "closure_artifact_compat.json"] {
        let AnyIr::Closure(closure) = carrier(name) else {
            panic!("{name} is a closure carrier");
        };
        check_trees(name, &closure);
    }
}

fn check_trees(name: &str, closure: &ClosureIr) {
    let mut trees = 0usize;
    let mut check = |label: String, tree: &DocTree| {
        let text = tree.parts().3.join("\n");
        assert_eq!(
            *tree,
            DocTree::parse(&text),
            "{name}/{label}: the persisted tree is not `DocTree::parse` of its own lines"
        );
        trees += 1;
    };
    for (index, node) in closure.nodes.iter().enumerate() {
        check(format!("node {index}"), &node.tree);
    }
    for (index, contribution) in closure.contributions.iter().enumerate() {
        if let ClosureContribution::Simple { document, .. } = contribution {
            check(format!("simple contribution {index}"), &document.tree);
        }
    }
    assert!(
        trees >= 2,
        "{name}: every carried tree is checked, saw {trees}"
    );
}

/// The Lane the corpus persists is EXACTLY what the production `AssemblePass`
/// projects from the corpus closure. Assemble copies each linked occurrence's
/// body and fence state verbatim, so a lane that "closes" a fence the linked
/// occurrence left open is fabricated — and this equality is what says so.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn the_corpus_lane_is_the_production_assemble_of_the_corpus_closure() {
    let projected = AssemblePass::new()
        .run(closure())
        .expect("the corpus closure is linked and assembles");
    assert_eq!(
        projected,
        lane(),
        "the persisted lane is not the projection"
    );
}

/// Field by field, every lane normal/simple node equals the linked occurrence
/// `AssemblePass` projects it from: body, both fence snapshots, requested
/// address, node id and stream order. The equality above proves it wholesale;
/// this one names the field that drifted when it ever fails.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn every_lane_node_equals_the_linked_occurrence_it_projects() {
    let closure = closure();
    let lane = lane();
    let LinkState::Linked(link) = &closure.link else {
        panic!("the corpus closure is linked");
    };
    let mut cursor = 0usize;
    for contribution in &lane.contributions {
        let chunks = match contribution {
            LaneContribution::Normal { chunks, .. } | LaneContribution::Simple { chunks, .. } => {
                chunks
            }
            LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => continue,
        };
        for chunk in chunks {
            let LaneChunk::Node(node) = chunk else {
                continue;
            };
            let occurrence = link
                .occurrences
                .get(cursor)
                .unwrap_or_else(|| panic!("the lane carries more nodes than the link: {cursor}"));
            assert_node(cursor, node, occurrence);
            cursor += 1;
        }
    }
    assert_eq!(
        cursor,
        link.occurrences.len(),
        "every linked occurrence reaches the lane"
    );
}

fn assert_node(index: usize, node: &LaneNode, occurrence: &LinkOccurrence) {
    match (node, occurrence) {
        (
            LaneNode::Normal {
                node,
                requested_address,
                marker,
                fence_before,
                fence_after,
                body,
                ..
            },
            LinkOccurrence::Normal {
                node: linked_node,
                address,
                marker: linked_marker,
                fence_before: before,
                fence_after: after,
                body: linked_body,
                ..
            },
        ) => {
            assert_eq!(node, linked_node, "occurrence {index} node id");
            assert_eq!(requested_address, address, "occurrence {index} address");
            assert_eq!(marker, linked_marker, "occurrence {index} marker");
            assert_eq!(fence_before, before, "occurrence {index} fence before");
            assert_eq!(fence_after, after, "occurrence {index} fence after");
            assert_eq!(body, linked_body, "occurrence {index} body");
        }
        (
            LaneNode::Simple {
                address,
                fence_before,
                fence_after,
                body,
                ..
            },
            LinkOccurrence::Simple {
                address: linked_address,
                fence_before: before,
                fence_after: after,
                body: linked_body,
                ..
            },
        ) => {
            assert_eq!(address, linked_address, "occurrence {index} address");
            assert_eq!(fence_before, before, "occurrence {index} fence before");
            assert_eq!(fence_after, after, "occurrence {index} fence after");
            assert_eq!(body, linked_body, "occurrence {index} body");
        }
        _ => panic!("occurrence {index}: lane node kind and link occurrence kind differ"),
    }
}
