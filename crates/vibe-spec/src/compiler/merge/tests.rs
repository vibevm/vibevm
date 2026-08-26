use std::collections::{BTreeMap, BTreeSet};

use specmark::verifies;

use super::*;
use crate::compiler::embed_snapshot::EmbedResolutionSnapshot;
use crate::compiler::ir::{
    AbsorptionState, ArtifactId, ContributionMeta, QualificationState, SourceFormatId, SourceIr,
    StaticCompileMode,
};

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn document(raw: &str, raw_text: &str, parsed_text: &str) -> super::super::ir::DocumentIr {
    super::super::ir::DocumentIr::new(
        SourceIr::new(
            DocumentAddress::Spec(spec(raw)),
            SourceFormatId::canonical_markdown(),
            raw_text,
        ),
        DocTree::parse(parsed_text),
    )
}

fn snapshot(
    documents: Vec<(&str, super::super::ir::DocumentIr)>,
    patterns: Vec<(&str, Vec<SpecAddress>)>,
    use_keys: &[&str],
) -> SourceResolutionSnapshot {
    SourceResolutionSnapshot {
        discovery_order: documents
            .iter()
            .map(|(key, _)| (*key).to_string())
            .collect(),
        documents: documents
            .into_iter()
            .map(|(key, document)| (key.to_string(), DocumentObservation::Resolved(document)))
            .collect::<BTreeMap<_, _>>(),
        expansions: patterns
            .into_iter()
            .map(|(pattern, targets)| {
                (
                    pattern.to_string(),
                    ExpansionObservation::Resolved {
                        requested: spec(pattern),
                        targets,
                    },
                )
            })
            .collect(),
        explicit_use_keys: use_keys
            .iter()
            .map(|key| (*key).to_string())
            .collect::<BTreeSet<_>>(),
    }
}

fn closure(root: &str, tree: &str, snapshot: SourceResolutionSnapshot) -> ClosureIr {
    ClosureIr {
        artifact: ArtifactId::new("static-fragment").unwrap(),
        nodes: vec![ClosureDocument {
            address: DocumentAddress::Spec(spec(root)),
            origin: "org.demo/pkg".to_string(),
            tree: DocTree::parse(tree),
            aliases: Default::default(),
        }],
        edges: Vec::new(),
        contributions: vec![ClosureContribution::Normal {
            meta: ContributionMeta {
                origin: "org.demo/pkg".to_string(),
                path: "contract/api".to_string(),
            },
            seed: ClosureNodeId(0),
            emission_order: vec![ClosureNodeId(0)],
        }],
        renames: Vec::new(),
        qualification: QualificationState::Pending(StaticCompileMode::Plain),
        absorption: AbsorptionState::Unplanned,
        pending_sources: Some(snapshot),
        pending_embeds: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn merge_consumes_pending_snapshot_then_adds_source_membership() {
    let root = "spec://org.demo/pkg/contract/api#root";
    let source = "spec://org.demo/pkg/source/impl#root";
    let pending = snapshot(
        vec![
            (
                root,
                document(root, "RAW", &format!("# API {{#root}}\n#source {source}\n")),
            ),
            (
                source,
                document(source, "RAW-SOURCE", "# Impl {#impl}\nSOURCE\n"),
            ),
        ],
        vec![(source, vec![spec(source)])],
        &[root],
    );
    let mut before = closure(
        root,
        &format!("# API {{#root}}\n#source {source}\n"),
        pending,
    );
    let pending_embeds = EmbedResolutionSnapshot {
        discovery_order: vec!["spec://org.demo/pkg/common/piece#root".to_string()],
        ..Default::default()
    };
    before.pending_embeds = Some(pending_embeds.clone());
    assert_eq!(before.nodes.len(), 1);
    assert!(before.edges.is_empty());
    assert!(before.pending_sources.is_some());

    let after = merge_closure(before).unwrap();

    assert!(after.pending_sources.is_none());
    assert_eq!(after.pending_embeds, Some(pending_embeds));
    assert_eq!(after.nodes.len(), 2);
    assert_eq!(after.edges.len(), 1);
    assert_eq!(after.edges[0].kind, ClosureEdgeKind::Source);
    assert!(
        after
            .edges
            .iter()
            .all(|edge| edge.kind != ClosureEdgeKind::Embed)
    );
    assert_eq!(
        after.nodes[0].tree.text(after.nodes[0].tree.root()),
        "# API {#root}\n#source spec://org.demo/pkg/source/impl#root\n# Impl {#impl}\nSOURCE"
    );
    assert_eq!(
        after.contributions[0],
        ClosureContribution::Normal {
            meta: ContributionMeta {
                origin: "org.demo/pkg".to_string(),
                path: "contract/api".to_string(),
            },
            seed: ClosureNodeId(0),
            emission_order: vec![ClosureNodeId(0)],
        }
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn source_target_that_is_already_a_use_node_reuses_its_id() {
    let root = "spec://org.demo/pkg/contract/api#root";
    let source = "spec://org.demo/pkg/contract/shared#root";
    let mut pending = snapshot(
        vec![
            (
                root,
                document(root, "RAW", &format!("# API {{#root}}\n#source {source}\n")),
            ),
            (
                source,
                document(source, "RAW-SOURCE", "# Shared {#shared}\nSHARED\n"),
            ),
        ],
        vec![(source, vec![spec(source)])],
        &[root, source],
    );
    pending.explicit_use_keys.insert(source.to_string());
    let mut input = closure(
        root,
        &format!("# API {{#root}}\n#source {source}\n"),
        pending,
    );
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(source)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse("# Shared {#shared}\nSHARED\n"),
        aliases: Default::default(),
    });

    let output = merge_closure(input).unwrap();

    assert_eq!(output.nodes.len(), 2);
    assert_eq!(output.edges[0].to, ClosureNodeId(1));
}

fn node_keys(closure: &ClosureIr) -> Vec<String> {
    closure
        .nodes
        .iter()
        .map(|node| match &node.address {
            DocumentAddress::Spec(address) => address.without_pin(),
            DocumentAddress::StaticEntry { .. } => panic!("unexpected static node"),
        })
        .collect()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn final_membership_follows_close_replay_not_scheduler_preorder() {
    let a = "spec://org.demo/pkg/contract/a#root";
    let b = "spec://org.demo/pkg/contract/b#root";
    let source_a = "spec://org.demo/pkg/source/a#root";
    let source_b = "spec://org.demo/pkg/source/b#root";
    let pending = snapshot(
        vec![
            (
                a,
                document(a, "RAW-A", &format!("# A {{#root}}\n#source {source_a}\n")),
            ),
            (
                b,
                document(b, "RAW-B", &format!("# B {{#root}}\n#source {source_b}\n")),
            ),
            (
                source_a,
                document(source_a, "RAW-SA", "# SA {#sa}\nSOURCE-A\n"),
            ),
            (
                source_b,
                document(source_b, "RAW-SB", "# SB {#sb}\nSOURCE-B\n"),
            ),
        ],
        vec![
            (source_a, vec![spec(source_a)]),
            (source_b, vec![spec(source_b)]),
        ],
        &[a, b],
    );
    let mut input = closure(b, &format!("# B {{#root}}\n#source {source_b}\n"), pending);
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(a)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse(&format!("# A {{#root}}\n#source {source_a}\n")),
        aliases: Default::default(),
    });
    input.contributions[0] = ClosureContribution::Normal {
        meta: ContributionMeta {
            origin: "org.demo/pkg".to_string(),
            path: "contract/a".to_string(),
        },
        seed: ClosureNodeId(1),
        emission_order: vec![ClosureNodeId(0), ClosureNodeId(1)],
    };

    let output = merge_closure(input).unwrap();

    assert_eq!(
        node_keys(&output),
        vec![b, a, source_b, source_a],
        "source-only nodes follow accepted B-before-A replay"
    );
    let source_edges: Vec<(ClosureNodeId, ClosureNodeId)> = output
        .edges
        .iter()
        .filter(|edge| edge.kind == ClosureEdgeKind::Source)
        .map(|edge| (edge.from, edge.to))
        .collect();
    assert_eq!(
        source_edges,
        vec![
            (ClosureNodeId(0), ClosureNodeId(2)),
            (ClosureNodeId(1), ClosureNodeId(3)),
        ]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn current_closure_tree_drives_both_fold_and_final_edges() {
    let root = "spec://org.demo/pkg/contract/api#root";
    let stale = "spec://org.demo/pkg/source/stale#root";
    let current = "spec://org.demo/pkg/source/current#root";
    let pending = snapshot(
        vec![
            (
                root,
                document(
                    root,
                    "RAW",
                    &format!("# Snapshot {{#root}}\n#source {stale}\n"),
                ),
            ),
            (
                stale,
                document(stale, "RAW-S", "# Stale {#stale}\nSTALE-BODY\n"),
            ),
            (
                current,
                document(current, "RAW-C", "# Current {#current}\nCURRENT-BODY\n"),
            ),
        ],
        vec![(stale, vec![spec(stale)]), (current, vec![spec(current)])],
        &[root],
    );
    let input = closure(
        root,
        &format!("# Current root {{#root}}\n#source {current}\n"),
        pending,
    );

    let output = merge_closure(input).unwrap();

    assert_eq!(node_keys(&output), vec![root, current]);
    assert_eq!(output.edges.len(), 1);
    assert_eq!(output.edges[0].from, ClosureNodeId(0));
    assert_eq!(output.edges[0].to, ClosureNodeId(1));
    let merged = output.nodes[0].tree.text(output.nodes[0].tree.root());
    assert!(merged.contains("CURRENT-BODY"), "{merged}");
    assert!(!merged.contains("STALE-BODY"), "{merged}");
}
