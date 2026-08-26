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
        tree: DocTree::parse(body),
        aliases: Default::default(),
    }
}

fn node_spec_address(node: &ClosureDocument) -> SpecAddress {
    let DocumentAddress::Spec(address) = &node.address else {
        unreachable!()
    };
    address.clone()
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
    let context = ArtifactContext::new(
        ArtifactId::new("static-md").unwrap(),
        ArtifactTarget::StaticMarkdown,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.md".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();
    let simple_meta = ContributionMeta {
        origin: "ungrouped-host".to_string(),
        path: "vibevm/vibespecs/boot/20-local.md".to_string(),
    };
    let plan = ArtifactPlan::new(
        context,
        vec![
            ArtifactInput::from_kind(ArtifactInputKind::Normal {
                meta: meta("org.demo/alpha"),
                seed: spec("spec://org.demo/alpha/boot/entry"),
            }),
            ArtifactInput::from_kind(ArtifactInputKind::Simple {
                meta: simple_meta,
                source: simple_source,
            }),
            ArtifactInput::from_kind(ArtifactInputKind::Elided {
                meta: meta("org.demo/elided"),
            }),
            ArtifactInput::from_kind(ArtifactInputKind::Hoisted {
                meta: meta("org.demo/hoisted"),
                target: spec("spec://org.demo/hoisted/boot/entry"),
            }),
            ArtifactInput::from_kind(ArtifactInputKind::Normal {
                meta: meta("org.demo/omega"),
                seed: spec("spec://org.demo/omega/boot/entry"),
            }),
        ],
    )
    .unwrap();

    assert_eq!(plan.context().artifact().as_str(), "static-md");
    assert_eq!(plan.context().mode(), StaticCompileMode::QualifyPerNode);
    assert!(matches!(
        plan.contributions()[0].kind(),
        ArtifactInputKind::Normal { .. }
    ));
    let ArtifactInputKind::Simple { source, meta } = plan.contributions()[1].kind() else {
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
        plan.contributions()[2].kind(),
        ArtifactInputKind::Elided { .. }
    ));
    assert!(matches!(
        plan.contributions()[3].kind(),
        ArtifactInputKind::Hoisted { .. }
    ));
    assert!(matches!(
        plan.contributions()[4].kind(),
        ArtifactInputKind::Normal { .. }
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
        tree: DocTree::parse("SIMPLE-AFTER-DOCUMENT-PASSES"),
        aliases: Default::default(),
    };
    let nodes = vec![
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
    ];
    let shared_address = node_spec_address(&nodes[0]);
    let alpha_address = node_spec_address(&nodes[1]);
    let omega_address = node_spec_address(&nodes[2]);
    let context = ArtifactContext::new(
        ArtifactId::new("static-xml").unwrap(),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();
    let closure = ClosureIr::testing(
        context,
        nodes,
        vec![
            ClosureEdge {
                from: alpha,
                to: shared,
                kind: ClosureEdgeKind::Use,
                requested_target: shared_address.clone(),
            },
            ClosureEdge {
                from: omega,
                to: shared,
                kind: ClosureEdgeKind::Use,
                requested_target: shared_address.clone(),
            },
        ],
        vec![
            ClosureContribution::Normal {
                meta: meta("org.demo/alpha"),
                seed: alpha,
                seed_address: alpha_address.clone(),
                emission_order: vec![
                    ClosureOccurrence {
                        node: shared,
                        requested_address: shared_address.clone(),
                    },
                    ClosureOccurrence {
                        node: alpha,
                        requested_address: alpha_address,
                    },
                ],
            },
            ClosureContribution::Simple {
                meta: meta("host"),
                document: Box::new(simple_document),
            },
            ClosureContribution::Normal {
                meta: meta("org.demo/omega"),
                seed: omega,
                seed_address: omega_address.clone(),
                emission_order: vec![
                    ClosureOccurrence {
                        node: shared,
                        requested_address: shared_address,
                    },
                    ClosureOccurrence {
                        node: omega,
                        requested_address: omega_address,
                    },
                ],
            },
        ],
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::QualifyPerNode),
        AbsorptionState::Unplanned,
        LinkState::Unlinked,
        None,
        None,
    );

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
    assert_eq!(
        first.iter().map(|entry| entry.node).collect::<Vec<_>>(),
        [shared, alpha]
    );
    assert_eq!(
        last.iter().map(|entry| entry.node).collect::<Vec<_>>(),
        [shared, omega]
    );
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
    assert_eq!(
        document.tree.text(document.tree.root()),
        "SIMPLE-AFTER-DOCUMENT-PASSES"
    );
    assert_eq!(closure.context().artifact().as_str(), "static-xml");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn lane_has_one_frame_around_heterogeneous_contributions() {
    let address = spec("spec://org.demo/alpha/boot/entry#root~r7");
    let marker = LinkMarkerKey::from_address(&address);
    let lane = LaneIr::assembled(
        ArtifactContext::new(
            ArtifactId::new("static-md").unwrap(),
            ArtifactTarget::StaticMarkdown,
            ArtifactFrame::StaticLane {
                generated_path: "vibevm/vibespecs/boot/STATIC.md".to_string(),
                source_root: "vibevm/vibedeps".to_string(),
            },
            StaticCompileMode::QualifyPerNode,
        )
        .unwrap(),
        1,
        LinkInputDigest([7; 32]),
        LaneFrame {
            generated_path: Some("vibevm/vibespecs/boot/STATIC.md".to_string()),
            source_root: Some("vibevm/vibedeps".to_string()),
            renames: vec![OriginRename {
                origin: "org.demo/alpha".to_string(),
                rename: RenameEntry {
                    original: "root".to_string(),
                    qualified: "org-demo--alpha--root".to_string(),
                },
            }],
        },
        vec![
            LaneContribution::Normal {
                meta: meta("org.demo/alpha"),
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
                        origin: "org.demo/alpha".to_string(),
                        marker: marker.clone(),
                        fence_before: LinkFenceSnapshot::Closed,
                        fence_after: LinkFenceSnapshot::Closed,
                        body: "ALPHA\n".to_string(),
                    })),
                    LaneChunk::NormalClose {
                        contribution: 0,
                        occurrence: 0,
                        marker,
                    },
                ],
            },
            LaneContribution::Simple {
                meta: meta("host"),
                address: DocumentAddress::StaticEntry {
                    origin: "host".to_string(),
                    path: "boot/entry.md".to_string(),
                },
                chunks: Vec::new(),
            },
            LaneContribution::Elided {
                meta: meta("org.demo/elided"),
            },
            LaneContribution::Hoisted {
                meta: meta("org.demo/hoisted"),
                target: spec("spec://org.demo/hoisted/boot/entry#root"),
            },
        ],
    );

    assert_eq!(
        lane.frame.generated_path.as_deref(),
        Some("vibevm/vibespecs/boot/STATIC.md")
    );
    assert_eq!(lane.frame.source_root.as_deref(), Some("vibevm/vibedeps"));
    assert_eq!(lane.frame.renames.len(), 1);
    assert_eq!(lane.contributions.len(), 4);
    assert_eq!(lane.context().artifact().as_str(), "static-md");
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

#[test]
fn public_normal_and_hoisted_inputs_bind_origin_to_typed_package_authority() {
    let valid = ArtifactInput::hoisted(
        "org.demo/pkg [shared by org.demo/a]",
        "vibevm/vibedeps/org.demo.pkg/1.0.0/boot/entry.md",
        spec("spec://org.demo/pkg/boot/entry"),
    );
    assert!(valid.is_ok());
    for (origin, target) in [
        ("org.demo/pkg", "spec://org.other/pkg/boot/entry"),
        ("org.demo/pkg", "spec://host/boot-entry"),
        ("org.demo/pkg", "spec://org.demo/pkg@1.0.0/boot/entry"),
        ("org.demo/pkg", "spec://org.demo/pkg/boot/entry#root"),
    ] {
        assert!(
            ArtifactInput::hoisted(origin, "boot/entry.md", spec(target)).is_err(),
            "accepted {origin} -> {target}"
        );
    }
    assert!(
        ArtifactInput::hoisted(
            "org.demo/pkg\nforged",
            "boot/entry.md",
            spec("spec://org.demo/pkg/boot/entry"),
        )
        .is_err()
    );
    assert!(
        ArtifactInput::hoisted(
            "org.demo/pkg forged",
            "boot/entry.md",
            spec("spec://org.demo/pkg/boot/entry"),
        )
        .is_err()
    );
    assert!(
        ArtifactInput::normal(
            "org.demo/a",
            "boot/entry.md",
            spec("spec://org.demo/b/boot/entry"),
        )
        .is_err()
    );
}
