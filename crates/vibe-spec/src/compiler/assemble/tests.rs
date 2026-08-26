use specmark::verifies;

use super::*;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, AbsorptionState, ArtifactContext, ArtifactFrame,
    ArtifactId, ArtifactTarget, ClosureContribution, ClosureDocument, ClosureIr, ClosureNodeId,
    ClosureOccurrence, ContributionAbsorption, ContributionMeta, DocumentAddress, LaneChunk,
    LaneContribution, LaneFrame, LaneIr, LaneNode, LinkFenceSnapshot, LinkMarkerKey,
    LinkOccurrence, LinkState, OriginRename, QualificationState, StaticCompileMode,
};
use crate::compiler::link::LinkPass;
use crate::compiler::pass::Pass;
use crate::{DocTree, RenameEntry, SpecAddress};

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn meta(origin: &str, path: &str) -> ContributionMeta {
    ContributionMeta::new(origin, path).unwrap()
}

fn normal_node(raw: &str, origin: &str, body: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::Spec(spec(raw)),
        origin: origin.to_string(),
        tree: DocTree::parse(body),
        aliases: Default::default(),
    }
}

fn occurrence(node: usize, requested: &str) -> ClosureOccurrence {
    ClosureOccurrence {
        node: ClosureNodeId(node),
        requested_address: spec(requested),
    }
}

fn full_context() -> ArtifactContext {
    ArtifactContext::new(
        ArtifactId::new("static-xml").unwrap(),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap()
}

fn applied_closure(context: ArtifactContext) -> ClosureIr {
    let a_request = "spec://org.demo/a/boot/a#root~r7";
    let shared_request = "spec://org.demo/shared/boot/shared#root";
    let empty_request = "spec://org.demo/empty/boot/empty#root";
    let nodes = vec![
        normal_node("spec://org.demo/a/boot/a#root", "document/a", "```text\n"),
        normal_node(
            shared_request,
            "document/shared",
            "See (#x).\n```\n# Shared {#document-shared--x}\n",
        ),
        normal_node(empty_request, "document/empty", ""),
    ];
    let first_order = vec![
        occurrence(0, a_request),
        occurrence(1, shared_request),
        occurrence(1, shared_request),
    ];
    let contributions = vec![
        ClosureContribution::Normal {
            meta: meta("meta/a", "boot/a.md"),
            seed: ClosureNodeId(0),
            seed_address: spec(a_request),
            emission_order: first_order.clone(),
        },
        ClosureContribution::Simple {
            meta: meta("meta/simple", "boot/simple.md"),
            document: Box::new(ClosureDocument {
                address: DocumentAddress::StaticEntry {
                    origin: "address/simple".to_string(),
                    path: "boot/simple.md".to_string(),
                },
                origin: "document/simple".to_string(),
                tree: DocTree::parse("SIMPLE"),
                aliases: Default::default(),
            }),
        },
        ClosureContribution::Elided {
            meta: meta("meta/elided", "boot/elided.md"),
        },
        ClosureContribution::Hoisted {
            meta: meta("meta/hoisted", "boot/hoisted.md"),
            target: spec("spec://org.demo/hoisted/boot/entry#root"),
        },
        ClosureContribution::Normal {
            meta: meta("meta/empty", "boot/empty.md"),
            seed: ClosureNodeId(2),
            seed_address: spec(empty_request),
            emission_order: Vec::new(),
        },
    ];
    let absorption = AbsorptionPlan {
        mode: StaticCompileMode::QualifyPerNode,
        contributions: vec![
            ContributionAbsorption::Normal {
                meta: meta("meta/a", "boot/a.md"),
                seed: ClosureNodeId(0),
                seed_address: spec(a_request),
                occurrences: first_order
                    .iter()
                    .map(|entry| AbsorptionOccurrence {
                        node: entry.node,
                        requested_address: entry.requested_address.clone(),
                        absorbed: false,
                    })
                    .collect(),
            },
            ContributionAbsorption::Simple {
                meta: meta("meta/simple", "boot/simple.md"),
                address: DocumentAddress::StaticEntry {
                    origin: "address/simple".to_string(),
                    path: "boot/simple.md".to_string(),
                },
            },
            ContributionAbsorption::Elided {
                meta: meta("meta/elided", "boot/elided.md"),
            },
            ContributionAbsorption::Hoisted {
                meta: meta("meta/hoisted", "boot/hoisted.md"),
                target: spec("spec://org.demo/hoisted/boot/entry#root"),
            },
            ContributionAbsorption::Normal {
                meta: meta("meta/empty", "boot/empty.md"),
                seed: ClosureNodeId(2),
                seed_address: spec(empty_request),
                occurrences: Vec::new(),
            },
        ],
    };
    let closure = ClosureIr::testing(
        context,
        nodes,
        Vec::new(),
        contributions,
        vec![OriginRename {
            origin: "document/shared".to_string(),
            rename: RenameEntry {
                original: "x".to_string(),
                qualified: "document-shared--x".to_string(),
            },
        }],
        QualificationState::Applied(StaticCompileMode::QualifyPerNode),
        AbsorptionState::Applied(absorption),
        LinkState::Unlinked,
        None,
        None,
    );
    LinkPass::new().run(closure).unwrap()
}

pub(super) fn fixture() -> ClosureIr {
    applied_closure(full_context())
}

fn lane_fixture() -> (ClosureIr, LaneIr) {
    let closure = fixture();
    let lane = AssemblePass::new().run(closure.clone()).unwrap();
    (closure, lane)
}

pub(super) fn independent_expected_lane(closure: &ClosureIr) -> LaneIr {
    let LinkState::Linked(link) = &closure.link else {
        panic!("test fixture must be linked")
    };
    let mut normal_chunks = Vec::new();
    for linked in &link.occurrences[..3] {
        let LinkOccurrence::Normal {
            contribution,
            occurrence,
            node,
            address,
            marker,
            fence_before,
            fence_after,
            body,
            trailing_newline_required,
        } = linked
        else {
            panic!("first three fixture occurrences must be normal")
        };
        let document = &closure.nodes[node.0];
        normal_chunks.push(LaneChunk::NormalOpen {
            contribution: *contribution,
            occurrence: *occurrence,
            marker: marker.clone(),
        });
        normal_chunks.push(LaneChunk::Node(Box::new(LaneNode::Normal {
            contribution: *contribution,
            occurrence: *occurrence,
            node: *node,
            requested_address: address.clone(),
            origin: document.origin.clone(),
            marker: marker.clone(),
            fence_before: fence_before.clone(),
            fence_after: fence_after.clone(),
            body: body.clone(),
        })));
        if *trailing_newline_required {
            normal_chunks.push(LaneChunk::ForcedNewline {
                contribution: *contribution,
                occurrence: *occurrence,
            });
        }
        normal_chunks.push(LaneChunk::NormalClose {
            contribution: *contribution,
            occurrence: *occurrence,
            marker: marker.clone(),
        });
    }
    let LinkOccurrence::Simple {
        contribution,
        occurrence,
        address,
        fence_before,
        fence_after,
        body,
        trailing_newline_required,
    } = &link.occurrences[3]
    else {
        panic!("fourth fixture occurrence must be simple")
    };
    let ClosureContribution::Simple { document, .. } = &closure.contributions[1] else {
        panic!("second fixture contribution must be simple")
    };
    let mut simple_chunks = vec![LaneChunk::Node(Box::new(LaneNode::Simple {
        contribution: *contribution,
        occurrence: *occurrence,
        address: address.clone(),
        origin: document.origin.clone(),
        fence_before: fence_before.clone(),
        fence_after: fence_after.clone(),
        body: body.clone(),
    }))];
    if *trailing_newline_required {
        simple_chunks.push(LaneChunk::ForcedNewline {
            contribution: *contribution,
            occurrence: *occurrence,
        });
    }

    let ClosureContribution::Normal {
        meta: first_meta,
        seed: first_seed,
        seed_address: first_address,
        ..
    } = &closure.contributions[0]
    else {
        panic!("first fixture contribution must be normal")
    };
    let ClosureContribution::Simple {
        meta: simple_meta, ..
    } = &closure.contributions[1]
    else {
        panic!("second fixture contribution must be simple")
    };
    let ClosureContribution::Elided { meta: elided_meta } = &closure.contributions[2] else {
        panic!("third fixture contribution must be elided")
    };
    let ClosureContribution::Hoisted {
        meta: hoisted_meta,
        target,
    } = &closure.contributions[3]
    else {
        panic!("fourth fixture contribution must be hoisted")
    };
    let ClosureContribution::Normal {
        meta: empty_meta,
        seed: empty_seed,
        seed_address: empty_address,
        ..
    } = &closure.contributions[4]
    else {
        panic!("fifth fixture contribution must be normal")
    };
    LaneIr::assembled(
        closure.context().clone(),
        closure.nodes.len(),
        link.input_digest.clone(),
        LaneFrame {
            generated_path: Some("vibevm/vibespecs/boot/STATIC.xml".to_string()),
            source_root: Some("vibevm/vibedeps".to_string()),
            renames: closure.renames.clone(),
        },
        vec![
            LaneContribution::Normal {
                meta: first_meta.clone(),
                seed: *first_seed,
                seed_address: first_address.clone(),
                chunks: normal_chunks,
            },
            LaneContribution::Simple {
                meta: simple_meta.clone(),
                address: document.address.clone(),
                chunks: simple_chunks,
            },
            LaneContribution::Elided {
                meta: elided_meta.clone(),
            },
            LaneContribution::Hoisted {
                meta: hoisted_meta.clone(),
                target: target.clone(),
            },
            LaneContribution::Normal {
                meta: empty_meta.clone(),
                seed: *empty_seed,
                seed_address: empty_address.clone(),
                chunks: Vec::new(),
            },
        ],
    )
}

pub(super) fn normal_nodes(chunks: &[LaneChunk]) -> Vec<&LaneNode> {
    chunks
        .iter()
        .filter_map(|chunk| match chunk {
            LaneChunk::Node(node) if matches!(node.as_ref(), LaneNode::Normal { .. }) => {
                Some(node.as_ref())
            }
            _ => None,
        })
        .collect()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn heterogeneous_shared_duplicate_and_empty_projection_is_exact() {
    let closure = fixture();
    reset_assemble_invocations();
    let lane = AssemblePass::new().run(closure.clone()).unwrap();
    assert_eq!(assemble_invocations(), 1);
    assert_eq!(lane.context(), closure.context());
    assert_eq!(lane.source_node_count, 3);
    assert_eq!(lane.frame.renames, closure.renames);
    assert_eq!(
        lane.frame.generated_path.as_deref(),
        Some("vibevm/vibespecs/boot/STATIC.xml")
    );
    assert_eq!(lane.frame.source_root.as_deref(), Some("vibevm/vibedeps"));
    assert!(matches!(
        lane.contributions.as_slice(),
        [
            LaneContribution::Normal { .. },
            LaneContribution::Simple { .. },
            LaneContribution::Elided { .. },
            LaneContribution::Hoisted { .. },
            LaneContribution::Normal { .. },
        ]
    ));
    let LaneContribution::Normal {
        seed_address,
        chunks,
        ..
    } = &lane.contributions[0]
    else {
        unreachable!()
    };
    assert!(seed_address.to_string().ends_with("#root~r7"));
    let nodes = normal_nodes(chunks);
    assert_eq!(nodes.len(), 3);
    assert!(matches!(
        nodes.as_slice(),
        [
            LaneNode::Normal {
                node: ClosureNodeId(0),
                occurrence: 0,
                ..
            },
            LaneNode::Normal {
                node: ClosureNodeId(1),
                occurrence: 1,
                ..
            },
            LaneNode::Normal {
                node: ClosureNodeId(1),
                occurrence: 2,
                ..
            },
        ]
    ));
    let LinkState::Linked(link) = &closure.link else {
        unreachable!()
    };
    let linked_bodies = link
        .occurrences
        .iter()
        .filter_map(|entry| match entry {
            crate::compiler::ir::LinkOccurrence::Normal {
                node: ClosureNodeId(1),
                body,
                ..
            } => Some(body.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let lane_bodies = nodes[1..]
        .iter()
        .map(|node| match node {
            LaneNode::Normal { body, .. } => body.as_str(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(lane_bodies, linked_bodies, "linked bodies copy exactly");
    let LaneNode::Normal {
        requested_address,
        marker,
        ..
    } = nodes[0]
    else {
        unreachable!()
    };
    assert!(requested_address.to_string().ends_with("#root~r7"));
    assert_eq!(marker.as_str(), "spec://org.demo/a/boot/a#root");
    let LaneContribution::Simple { chunks, .. } = &lane.contributions[1] else {
        unreachable!()
    };
    assert!(matches!(
        chunks.as_slice(),
        [
            LaneChunk::Node(node),
            LaneChunk::ForcedNewline { .. }
        ] if matches!(node.as_ref(), LaneNode::Simple { origin, .. } if origin == "document/simple")
    ));
    let LaneContribution::Normal { chunks, .. } = &lane.contributions[4] else {
        unreachable!()
    };
    assert!(chunks.is_empty(), "empty normal contribution survives");
    validate_assembled_transition(&closure, &lane).unwrap();
}

#[test]
fn compatibility_and_empty_frames_are_semantic_and_valid() {
    let mut closure = applied_closure(ArtifactContext::compatibility(
        StaticCompileMode::QualifyPerNode,
    ));
    closure.contributions.clear();
    closure.nodes.clear();
    closure.renames.clear();
    closure.absorption = AbsorptionState::Applied(AbsorptionPlan {
        mode: StaticCompileMode::QualifyPerNode,
        contributions: Vec::new(),
    });
    closure.link = LinkState::Unlinked;
    let closure = LinkPass::new().run(closure).unwrap();
    let lane = AssemblePass::new().run(closure).unwrap();
    assert!(lane.frame.generated_path.is_none());
    assert!(lane.frame.source_root.is_none());
    assert!(lane.contributions.is_empty());
    validate_lane(&lane).unwrap();
}

#[test]
fn intrinsic_validator_rejects_marker_newline_chunk_fence_and_frame_mutations() {
    let (_, lane) = lane_fixture();
    let mut marker_lane = lane.clone();
    let LaneContribution::Normal { chunks, .. } = &mut marker_lane.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::NormalOpen { marker, .. } = &mut chunks[0] else {
        unreachable!()
    };
    *marker = LinkMarkerKey::from_address(&spec("spec://org.demo/wrong/boot/x#root"));
    assert!(validate_lane(&marker_lane).is_err());

    let mut newline = lane.clone();
    let LaneContribution::Simple { chunks, .. } = &mut newline.contributions[1] else {
        unreachable!()
    };
    chunks.pop();
    assert!(validate_lane(&newline).is_err());

    let mut order = lane.clone();
    let LaneContribution::Normal { chunks, .. } = &mut order.contributions[0] else {
        unreachable!()
    };
    chunks.swap(0, 1);
    assert!(validate_lane(&order).is_err());

    let mut fence = lane.clone();
    let LaneContribution::Normal { chunks, .. } = &mut fence.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[1] else {
        unreachable!()
    };
    let LaneNode::Normal { fence_after, .. } = node.as_mut() else {
        unreachable!()
    };
    *fence_after = LinkFenceSnapshot::Closed;
    assert!(validate_lane(&fence).is_err());

    let mut frame = lane;
    frame.frame.generated_path = None;
    assert!(validate_lane(&frame).is_err());
}

#[test]
fn transition_rejects_context_provenance_rename_request_and_simple_origin_mutations() {
    let (closure, lane) = lane_fixture();
    let mut provenance = lane.clone();
    let LaneContribution::Elided { meta } = &mut provenance.contributions[2] else {
        unreachable!()
    };
    meta.origin = "changed/elided".to_string();
    assert!(validate_assembled_transition(&closure, &provenance).is_err());

    let mut renames = lane.clone();
    renames.frame.renames.clear();
    assert!(validate_assembled_transition(&closure, &renames).is_err());

    let mut request = lane.clone();
    let LaneContribution::Normal { chunks, .. } = &mut request.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[1] else {
        unreachable!()
    };
    let LaneNode::Normal {
        requested_address, ..
    } = node.as_mut()
    else {
        unreachable!()
    };
    *requested_address = spec("spec://org.demo/a/boot/a#root~r8");
    assert!(validate_lane(&request).is_ok());
    assert!(validate_assembled_transition(&closure, &request).is_err());

    let mut simple_origin = lane.clone();
    let LaneContribution::Simple { chunks, .. } = &mut simple_origin.contributions[1] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[0] else {
        unreachable!()
    };
    let LaneNode::Simple {
        origin: node_origin,
        ..
    } = node.as_mut()
    else {
        unreachable!()
    };
    *node_origin = "meta/simple".to_string();
    assert!(validate_lane(&simple_origin).is_ok());
    assert!(validate_assembled_transition(&closure, &simple_origin).is_err());

    let mut body = lane.clone();
    let LaneContribution::Simple { chunks, .. } = &mut body.contributions[1] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[0] else {
        unreachable!()
    };
    let LaneNode::Simple {
        body: linked_body, ..
    } = node.as_mut()
    else {
        unreachable!()
    };
    linked_body.push('X');
    assert!(validate_lane(&body).is_ok());
    assert!(validate_assembled_transition(&closure, &body).is_err());

    let changed_context = ArtifactContext::new(
        ArtifactId::new("static-md").unwrap(),
        ArtifactTarget::StaticMarkdown,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.md".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();
    let mut changed_frame = lane.frame.clone();
    changed_frame.generated_path = Some("vibevm/vibespecs/boot/STATIC.md".to_string());
    let context = LaneIr::assembled(
        changed_context,
        lane.source_node_count,
        lane.source_link_digest.clone(),
        changed_frame,
        lane.contributions.clone(),
    );
    assert!(validate_lane(&context).is_ok());
    assert!(validate_assembled_transition(&closure, &context).is_err());
}
