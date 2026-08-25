use specmark::verifies;

use super::*;

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn meta(origin: &str) -> ContributionMeta {
    ContributionMeta {
        origin: origin.to_string(),
        path: format!("vibedeps/{origin}/boot/entry.md"),
    }
}

fn node(raw: &str, origin: &str, body: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::Spec(spec(raw)),
        origin: origin.to_string(),
        body: body.to_string(),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn two_addresses_into_one_physical_document_remain_two_document_values() {
    let source_one = SourceIr::new(
        DocumentAddress::Spec(spec("spec://org.demo/pkg/common/shared#one")),
        SourceFormatId::new("markdown").unwrap(),
        "## One {#one}\nONE\n",
    );
    let source_two = SourceIr::new(
        DocumentAddress::Spec(spec("spec://org.demo/pkg/common/shared#two")),
        SourceFormatId::new("markdown").unwrap(),
        "## Two {#two}\nTWO\n",
    );
    let documents = Documents::new(vec![
        DocumentIr::new(source_one.clone(), DocTree::parse(source_one.text())),
        DocumentIr::new(source_two.clone(), DocTree::parse(source_two.text())),
    ]);

    assert_eq!(documents.len(), 2);
    let addresses: Vec<&DocumentAddress> = documents
        .iter()
        .map(|document| document.source().address())
        .collect();
    assert_eq!(addresses, vec![source_one.address(), source_two.address()]);
    assert_eq!(documents.iter().next().unwrap().tree().len(), 2);
    assert!(!documents.is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn artifact_plan_keeps_normal_simple_normal_without_a_fake_simple_address() {
    let simple_source = SourceIr::new(
        DocumentAddress::StaticEntry {
            origin: "ungrouped-host".to_string(),
            path: "vibevm/vibespecs/boot/20-local.md".to_string(),
        },
        SourceFormatId::new("markdown").unwrap(),
        "# Local {#root}\n",
    );
    let plan = ArtifactPlan {
        artifact: ArtifactId::new("static-markdown").unwrap(),
        contributions: vec![
            ArtifactInput::Normal {
                meta: meta("org.demo/alpha"),
                seed: spec("spec://org.demo/alpha/boot/entry"),
            },
            ArtifactInput::Simple {
                meta: meta("ungrouped-host"),
                source: simple_source,
            },
            ArtifactInput::Normal {
                meta: meta("org.demo/omega"),
                seed: spec("spec://org.demo/omega/boot/entry"),
            },
        ],
    };

    assert_eq!(plan.artifact.as_str(), "static-markdown");
    assert!(matches!(
        plan.contributions[0],
        ArtifactInput::Normal { .. }
    ));
    let ArtifactInput::Simple { source, meta } = &plan.contributions[1] else {
        panic!("middle contribution must remain simple")
    };
    assert_eq!(meta.origin, "ungrouped-host");
    assert!(matches!(
        source.address(),
        DocumentAddress::StaticEntry { origin, path }
            if origin == "ungrouped-host" && path.ends_with("20-local.md")
    ));
    assert_eq!(source.format().as_str(), "markdown");
    assert!(matches!(
        plan.contributions[2],
        ArtifactInput::Normal { .. }
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn one_graph_can_preserve_shared_nodes_and_each_roots_emission_order() {
    let shared = ClosureNodeId(0);
    let alpha = ClosureNodeId(1);
    let omega = ClosureNodeId(2);
    let simple_document = ClosureDocument {
        address: DocumentAddress::StaticEntry {
            origin: "host".to_string(),
            path: "vibevm/vibespecs/boot/20-local.md".to_string(),
        },
        origin: "host".to_string(),
        body: "SIMPLE-AFTER-DOCUMENT-PASSES".to_string(),
    };
    let closure = ClosureIr {
        artifact: ArtifactId::new("static-xml").unwrap(),
        nodes: vec![
            node(
                "spec://org.demo/shared/boot/base",
                "org.demo/shared",
                "BASE",
            ),
            node(
                "spec://org.demo/alpha/boot/entry",
                "org.demo/alpha",
                "ALPHA",
            ),
            node(
                "spec://org.demo/omega/boot/entry",
                "org.demo/omega",
                "OMEGA",
            ),
        ],
        edges: vec![
            ClosureEdge {
                from: alpha,
                to: shared,
                kind: ClosureEdgeKind::Use,
            },
            ClosureEdge {
                from: omega,
                to: shared,
                kind: ClosureEdgeKind::Use,
            },
        ],
        contributions: vec![
            ClosureContribution::Normal {
                meta: meta("org.demo/alpha"),
                seed: alpha,
                emission_order: vec![shared, alpha],
            },
            ClosureContribution::Simple {
                meta: meta("host"),
                document: simple_document,
            },
            ClosureContribution::Normal {
                meta: meta("org.demo/omega"),
                seed: omega,
                emission_order: vec![shared, omega],
            },
        ],
        renames: Vec::new(),
    };

    let ClosureContribution::Normal {
        emission_order: first,
        ..
    } = &closure.contributions[0]
    else {
        panic!("first contribution must be normal")
    };
    let ClosureContribution::Normal {
        emission_order: last,
        ..
    } = &closure.contributions[2]
    else {
        panic!("last contribution must be normal")
    };
    assert_eq!(first, &[shared, alpha]);
    assert_eq!(last, &[shared, omega]);
    assert_eq!(closure.nodes.len(), 3);
    assert_eq!(closure.edges.len(), 2);
    assert!(
        closure
            .nodes
            .iter()
            .all(|node| matches!(node.address, DocumentAddress::Spec(_)))
    );
    let ClosureContribution::Simple { document, .. } = &closure.contributions[1] else {
        panic!("middle contribution must remain simple")
    };
    assert!(matches!(
        document.address,
        DocumentAddress::StaticEntry { .. }
    ));
    assert_eq!(document.body, "SIMPLE-AFTER-DOCUMENT-PASSES");
    assert_eq!(closure.artifact.as_str(), "static-xml");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn lane_has_one_frame_around_heterogeneous_contributions() {
    let lane_node = |origin: &str, marker: NodeMarkers| LaneNode {
        address: DocumentAddress::StaticEntry {
            origin: origin.to_string(),
            path: "boot/entry.md".to_string(),
        },
        origin: origin.to_string(),
        body: format!("{origin}\n"),
        markers: marker,
    };
    let lane = LaneIr {
        artifact: ArtifactId::new("static-markdown").unwrap(),
        frame: LaneFrame {
            header: "HEADER\n".to_string(),
            preamble: "PREAMBLE\n".to_string(),
            renames: vec![OriginRename {
                origin: "org.demo/alpha".to_string(),
                rename: RenameEntry {
                    original: "root".to_string(),
                    qualified: "org-demo--alpha--root".to_string(),
                },
            }],
        },
        contributions: vec![
            LaneContribution::Normal {
                meta: meta("org.demo/alpha"),
                nodes: vec![lane_node(
                    "org.demo/alpha",
                    NodeMarkers::Reversible {
                        key: "spec://org.demo/alpha/boot/entry".to_string(),
                    },
                )],
            },
            LaneContribution::Simple {
                meta: meta("host"),
                node: lane_node("host", NodeMarkers::None),
            },
            LaneContribution::Normal {
                meta: meta("org.demo/omega"),
                nodes: vec![lane_node(
                    "org.demo/omega",
                    NodeMarkers::Reversible {
                        key: "spec://org.demo/omega/boot/entry".to_string(),
                    },
                )],
            },
        ],
    };

    assert_eq!(lane.frame.header, "HEADER\n");
    assert_eq!(lane.frame.preamble, "PREAMBLE\n");
    assert_eq!(lane.frame.renames.len(), 1);
    assert_eq!(lane.contributions.len(), 3);
    assert_eq!(lane.artifact.as_str(), "static-markdown");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn owned_transform_apis_preserve_identity_and_make_mutation_deliberate() {
    let mut source = SourceIr::new(
        DocumentAddress::Spec(spec("spec://org.demo/pkg/boot/entry#root")),
        SourceFormatId::new("markdown").unwrap(),
        "# Entry {#root}\n",
    );
    source.text_mut().push_str("SOURCE-PASS\n");
    let (address, format, text) = source.into_parts();
    assert_eq!(format.as_str(), "markdown");

    let source = SourceIr::new(address, format, text);
    let mut document = DocumentIr::new(source.clone(), DocTree::parse(source.text()));
    *document.tree_mut() = DocTree::parse("# Rewritten {#root}\n");
    let (source, tree) = document.into_parts();
    assert_eq!(source.text(), "# Entry {#root}\nSOURCE-PASS\n");
    assert_eq!(tree.text(tree.root()), "# Rewritten {#root}");

    let mut documents = Documents::new(vec![DocumentIr::new(
        source.clone(),
        DocTree::parse(source.text()),
    )]);
    for document in documents.iter_mut() {
        *document.tree_mut() = DocTree::parse("# Batch {#root}\n");
    }
    let vector = documents.into_vec();
    let owned: Vec<DocumentIr> = Documents::new(vector).into_iter().collect();
    assert_eq!(owned.len(), 1);
    assert_eq!(
        owned[0].tree().text(owned[0].tree().root()),
        "# Batch {#root}"
    );
}

#[test]
fn internal_ids_reject_blank_values() {
    assert!(SourceFormatId::new(" \t").is_err());
    assert!(ArtifactId::new("\n").is_err());
}
