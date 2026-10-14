use super::*;

#[test]
fn snapshot_reader_preserves_source_identity_and_positions() {
    let source = SourceIr::reached(
        DocumentAddress::StaticEntry {
            origin: "host".into(),
            path: "boot/a.md".into(),
        },
        SourceFormatId::canonical_markdown(),
        "# Heading {#h}\nbody\n",
    );
    let ir = AnyIr::Document(DocumentIr::new(
        source.clone(),
        DocTree::parse(source.text()),
    ));
    let bytes = super::super::wire::encode_pretty(&ir).unwrap();
    let graph = inspect_boot_snapshot(&bytes).unwrap();
    assert_eq!(graph.sources[0].identity, identity(source.address()));
    assert_eq!(
        graph.carrier.as_ref().unwrap(),
        &super::super::wire::encode_generated(&ir).unwrap()
    );
    let tree = graph.sources[0].tree.as_ref().unwrap();
    assert_eq!(tree.node(tree.root()).span, 0..2);
    assert!(inspect_boot_snapshot(b"{}").is_err());
}

#[test]
fn repeated_occurrences_share_one_source_without_becoming_unresolved() {
    let mut graph = BootGraph::default();
    let id = SourceIdentity::Spec("spec://org.test/pkg/doc#root".into());
    assert_eq!(graph.source(id.clone(), None), graph.source(id, None));
    assert_ne!(ReferenceState::Unknown, ReferenceState::Unresolved);
}

#[test]
fn lane_traversal_preserves_all_contribution_kinds_and_repeated_node_identity() {
    use crate::SpecAddress;
    let address = SpecAddress::parse("spec://org.demo/pkg/boot/a#root").unwrap();
    let meta = ContributionMeta::new("org.demo/pkg", "boot/a.md").unwrap();
    let chunks = (0..2)
        .map(|occurrence| {
            LaneChunk::Node(Box::new(LaneNode::Normal {
                contribution: 0,
                occurrence,
                node: ClosureNodeId(7),
                requested_address: address.clone(),
                origin: "org.demo/pkg".into(),
                marker: LinkMarkerKey::from_address(&address),
                fence_before: LinkFenceSnapshot::Closed,
                fence_after: LinkFenceSnapshot::Closed,
                body: "body\n".into(),
            }))
        })
        .collect();
    let rows = vec![
        LaneContribution::Normal {
            meta: meta.clone(),
            seed: ClosureNodeId(7),
            seed_address: address.clone(),
            chunks,
        },
        LaneContribution::Simple {
            meta: meta.clone(),
            address: DocumentAddress::StaticEntry {
                origin: "host".into(),
                path: "simple.md".into(),
            },
            chunks: vec![],
        },
        LaneContribution::Elided { meta: meta.clone() },
        LaneContribution::Hoisted {
            meta,
            target: address,
        },
    ];
    let mut graph = BootGraph::default();
    project_lane(&mut graph, &rows);
    assert_eq!(
        graph
            .contributions
            .iter()
            .map(|c| c.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["normal", "simple", "elided", "hoisted"]
    );
    let row = &graph.contributions[0];
    assert_eq!(row.occurrences.len(), 2);
    assert_eq!(row.seed, Some(row.occurrences[0].source));
    assert_eq!(row.occurrences[0].source, row.occurrences[1].source);
    assert_eq!(
        graph.sources[row.occurrences[0].source].identity,
        SourceIdentity::CapturedNode(7)
    );
    assert_eq!(row.occurrences[1].parent, InclusionParent::Unknown);
    assert_eq!(graph.contributions[2].seed, None);
    assert!(graph.contributions[3].occurrences.is_empty());
}
