use std::collections::{BTreeMap, BTreeSet};

use specmark::verifies;

use super::*;
use crate::compiler::embed_snapshot::EmbedResolutionSnapshot;
use crate::compiler::ir::{
    ArtifactId, ContributionMeta, DocumentIr, QualificationState, SourceFormatId, SourceIr,
    StaticCompileMode,
};
use crate::compiler::source_snapshot::DocumentObservation;

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn document(raw: &str, text: &str) -> DocumentIr {
    let source = SourceIr::new(
        DocumentAddress::Spec(spec(raw)),
        SourceFormatId::canonical_markdown(),
        text,
    );
    DocumentIr::new(source.clone(), DocTree::parse(source.text()))
}

fn snapshot(documents: &[(&str, &str)], use_keys: &[&str]) -> EmbedResolutionSnapshot {
    EmbedResolutionSnapshot {
        discovery_order: documents
            .iter()
            .map(|(key, _)| (*key).to_string())
            .collect(),
        documents: documents
            .iter()
            .map(|(key, text)| {
                (
                    (*key).to_string(),
                    DocumentObservation::Resolved(document(key, text)),
                )
            })
            .collect::<BTreeMap<_, _>>(),
        explicit_use_keys: use_keys
            .iter()
            .map(|key| (*key).to_string())
            .collect::<BTreeSet<_>>(),
    }
}

fn closure(root: &str, tree: &str, pending: EmbedResolutionSnapshot) -> ClosureIr {
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
        absorption: None,
        pending_sources: None,
        pending_embeds: Some(pending),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn repeated_embeds_splice_and_edge_twice_but_append_one_node() {
    let root = "spec://org.demo/pkg/boot/root#root";
    let target = "spec://org.demo/pkg/common/piece#root";
    let input = closure(
        root,
        &format!(
            "# Root {{#root}}\n#use spec://org.demo/pkg/boot/dep#root as dep\n#embed {target}\n#embed {target}\n@!dep\n"
        ),
        snapshot(&[(target, "# Piece {#piece}\nPIECE\n")], &[root]),
    );

    assert!(input.pending_embeds.is_some());
    let bypassed = input.nodes[0].tree.text(input.nodes[0].tree.root());
    let output = embed_closure(input).unwrap();

    assert!(output.pending_embeds.is_none());
    assert_eq!(output.nodes.len(), 2);
    assert_eq!(output.edges.len(), 2);
    assert!(
        output
            .edges
            .iter()
            .all(|edge| edge.kind == ClosureEdgeKind::Embed)
    );
    let text = output.nodes[0].tree.text(output.nodes[0].tree.root());
    assert_ne!(text, bypassed);
    assert_eq!(text.matches("PIECE").count(), 2, "{text}");
    assert!(!text.contains("#use"));
    assert!(!text.contains("#embed"));
    assert_eq!(
        output.nodes[0].aliases["dep"].without_pin(),
        "spec://org.demo/pkg/boot/dep#root"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn accepted_replay_order_not_scheduler_discovery_orders_nodes_and_edges() {
    let a = "spec://org.demo/pkg/boot/a#root";
    let b = "spec://org.demo/pkg/boot/b#root";
    let a_target = "spec://org.demo/pkg/common/a-piece#root";
    let b_target = "spec://org.demo/pkg/common/b-piece#root";
    let pending = snapshot(&[(a_target, "A-TARGET"), (b_target, "B-TARGET")], &[a, b]);
    assert_eq!(pending.discovery_order, vec![a_target, b_target]);
    let mut input = closure(a, &format!("# A {{#root}}\n#embed {a_target}\n"), pending);
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(b)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse(&format!("# B {{#root}}\n#embed {b_target}\n")),
        aliases: Default::default(),
    });
    input.contributions[0] = ClosureContribution::Normal {
        meta: ContributionMeta {
            origin: "org.demo/pkg".to_string(),
            path: "boot/a".to_string(),
        },
        seed: ClosureNodeId(0),
        emission_order: vec![ClosureNodeId(1), ClosureNodeId(0)],
    };

    let output = embed_closure(input).unwrap();
    let keys: Vec<String> = output
        .nodes
        .iter()
        .map(|node| match &node.address {
            DocumentAddress::Spec(address) => address.without_pin(),
            DocumentAddress::StaticEntry { .. } => panic!("unexpected static node"),
        })
        .collect();

    assert_eq!(keys, vec![a, b, b_target, a_target]);
    assert_eq!(
        output.edges,
        vec![
            ClosureEdge {
                from: ClosureNodeId(1),
                to: ClosureNodeId(2),
                kind: ClosureEdgeKind::Embed,
            },
            ClosureEdge {
                from: ClosureNodeId(0),
                to: ClosureNodeId(3),
                kind: ClosureEdgeKind::Embed,
            },
        ]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn shared_node_across_contributions_is_expanded_once_at_first_emission_encounter() {
    let shared = "spec://org.demo/pkg/boot/shared#root";
    let alpha = "spec://org.demo/pkg/boot/alpha#root";
    let omega = "spec://org.demo/pkg/boot/omega#root";
    let target = "spec://org.demo/pkg/common/piece#root";
    let mut input = closure(
        shared,
        &format!("# Shared {{#root}}\n#embed {target}\n"),
        snapshot(&[(target, "PIECE")], &[shared, alpha, omega]),
    );
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(alpha)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse("ALPHA"),
        aliases: Default::default(),
    });
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(omega)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse("OMEGA"),
        aliases: Default::default(),
    });
    input.contributions = vec![
        ClosureContribution::Normal {
            meta: ContributionMeta {
                origin: "org.demo/pkg".to_string(),
                path: "boot/alpha".to_string(),
            },
            seed: ClosureNodeId(1),
            emission_order: vec![ClosureNodeId(0), ClosureNodeId(1)],
        },
        ClosureContribution::Normal {
            meta: ContributionMeta {
                origin: "org.demo/pkg".to_string(),
                path: "boot/omega".to_string(),
            },
            seed: ClosureNodeId(2),
            emission_order: vec![ClosureNodeId(0), ClosureNodeId(2)],
        },
    ];
    let contributions = input.contributions.clone();

    let output = embed_closure(input).unwrap();
    let shared_text = output.nodes[0].tree.text(output.nodes[0].tree.root());

    assert_eq!(shared_text.matches("PIECE").count(), 1, "{shared_text}");
    assert_eq!(output.nodes.len(), 4);
    assert_eq!(
        output.edges,
        vec![ClosureEdge {
            from: ClosureNodeId(0),
            to: ClosureNodeId(3),
            kind: ClosureEdgeKind::Embed,
        }]
    );
    assert_eq!(output.contributions, contributions);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn current_root_directives_override_stale_snapshot_directives() {
    let root = "spec://org.demo/pkg/boot/root#root";
    let stale = "spec://org.demo/pkg/common/stale#root";
    let current = "spec://org.demo/pkg/common/current#root";
    let pending = snapshot(
        &[
            (root, &format!("# Snapshot {{#root}}\n#embed {stale}\n")),
            (stale, "STALE"),
            (current, "CURRENT"),
        ],
        &[root],
    );
    let input = closure(
        root,
        &format!("# Current {{#root}}\n#embed {current}\n"),
        pending,
    );

    let output = embed_closure(input).unwrap();

    let text = output.nodes[0].tree.text(output.nodes[0].tree.root());
    assert!(text.contains("CURRENT"), "{text}");
    assert!(!text.contains("STALE"), "{text}");
    assert_eq!(output.nodes.len(), 2);
    let DocumentAddress::Spec(address) = &output.nodes[1].address else {
        panic!("expected spec node")
    };
    assert_eq!(address.without_pin(), current);
    assert_eq!(
        output.edges,
        vec![ClosureEdge {
            from: ClosureNodeId(0),
            to: ClosureNodeId(1),
            kind: ClosureEdgeKind::Embed,
        }]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn embedded_explicit_use_target_reuses_its_id_and_current_tree() {
    let root = "spec://org.demo/pkg/boot/root#root";
    let target = "spec://org.demo/pkg/boot/target#root";
    let pending = snapshot(
        &[(target, "# Snapshot {#root}\nSTALE-TARGET\n")],
        &[root, target],
    );
    let mut input = closure(
        root,
        &format!("# Root {{#root}}\n#use {target}\n#embed {target}\n"),
        pending,
    );
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(target)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse("# Current {#root}\nCURRENT-TARGET\n"),
        aliases: Default::default(),
    });
    input.edges.push(ClosureEdge {
        from: ClosureNodeId(0),
        to: ClosureNodeId(1),
        kind: ClosureEdgeKind::Use,
    });
    input.contributions[0] = ClosureContribution::Normal {
        meta: ContributionMeta {
            origin: "org.demo/pkg".to_string(),
            path: "boot/root".to_string(),
        },
        seed: ClosureNodeId(0),
        emission_order: vec![ClosureNodeId(1), ClosureNodeId(0)],
    };

    let output = embed_closure(input).unwrap();
    let root_text = output.nodes[0].tree.text(output.nodes[0].tree.root());

    assert!(root_text.contains("CURRENT-TARGET"), "{root_text}");
    assert!(!root_text.contains("STALE-TARGET"), "{root_text}");
    assert_eq!(output.nodes.len(), 2);
    assert_eq!(
        output.edges,
        vec![
            ClosureEdge {
                from: ClosureNodeId(0),
                to: ClosureNodeId(1),
                kind: ClosureEdgeKind::Use,
            },
            ClosureEdge {
                from: ClosureNodeId(0),
                to: ClosureNodeId(1),
                kind: ClosureEdgeKind::Embed,
            },
        ]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn nested_explicit_use_replay_keeps_one_edge_per_authored_occurrence() {
    let a = "spec://org.demo/pkg/boot/a#root";
    let b = "spec://org.demo/pkg/boot/b#root";
    let c = "spec://org.demo/pkg/common/c#root";
    let pending = snapshot(&[(c, "# C {#root}\nC-BODY\n")], &[a, b]);
    let mut input = closure(
        a,
        &format!("# A {{#root}}\n#use {b}\n#embed {b}\n#embed {b}\n"),
        pending,
    );
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(b)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse(&format!("# B {{#root}}\nCURRENT-B\n#embed {c}\n")),
        aliases: Default::default(),
    });
    input.edges.push(ClosureEdge {
        from: ClosureNodeId(0),
        to: ClosureNodeId(1),
        kind: ClosureEdgeKind::Use,
    });
    input.contributions[0] = ClosureContribution::Normal {
        meta: ContributionMeta {
            origin: "org.demo/pkg".to_string(),
            path: "boot/a".to_string(),
        },
        seed: ClosureNodeId(0),
        emission_order: vec![ClosureNodeId(1), ClosureNodeId(0)],
    };

    let output = embed_closure(input).unwrap();
    let a_text = output.nodes[0].tree.text(output.nodes[0].tree.root());
    let b_text = output.nodes[1].tree.text(output.nodes[1].tree.root());

    assert_eq!(a_text.matches("CURRENT-B").count(), 2, "{a_text}");
    assert_eq!(a_text.matches("C-BODY").count(), 2, "{a_text}");
    assert_eq!(b_text.matches("C-BODY").count(), 1, "{b_text}");
    assert_eq!(output.nodes.len(), 3);
    assert_eq!(
        output.edges,
        vec![
            ClosureEdge {
                from: ClosureNodeId(0),
                to: ClosureNodeId(1),
                kind: ClosureEdgeKind::Use,
            },
            ClosureEdge {
                from: ClosureNodeId(1),
                to: ClosureNodeId(2),
                kind: ClosureEdgeKind::Embed,
            },
            ClosureEdge {
                from: ClosureNodeId(0),
                to: ClosureNodeId(1),
                kind: ClosureEdgeKind::Embed,
            },
            ClosureEdge {
                from: ClosureNodeId(0),
                to: ClosureNodeId(1),
                kind: ClosureEdgeKind::Embed,
            },
        ]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn embedded_source_only_target_reuses_its_id_but_reads_the_snapshot_tree() {
    let root = "spec://org.demo/pkg/contract/root#root";
    let target = "spec://org.demo/pkg/source/target#root";
    let pending = snapshot(
        &[(target, "# Authored {#root}\nSNAPSHOT-TARGET\n")],
        &[root],
    );
    let mut input = closure(
        root,
        &format!("# Root {{#root}}\n#embed {target}\n"),
        pending,
    );
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(target)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse("# Mutated {#root}\nSOURCE-NODE-TREE\n"),
        aliases: Default::default(),
    });
    input.edges.push(ClosureEdge {
        from: ClosureNodeId(0),
        to: ClosureNodeId(1),
        kind: ClosureEdgeKind::Source,
    });

    let output = embed_closure(input).unwrap();
    let root_text = output.nodes[0].tree.text(output.nodes[0].tree.root());

    assert!(root_text.contains("SNAPSHOT-TARGET"), "{root_text}");
    assert!(!root_text.contains("SOURCE-NODE-TREE"), "{root_text}");
    assert_eq!(output.nodes.len(), 2);
    assert_eq!(
        output.edges.last(),
        Some(&ClosureEdge {
            from: ClosureNodeId(0),
            to: ClosureNodeId(1),
            kind: ClosureEdgeKind::Embed,
        })
    );
}

#[test]
fn invented_unobserved_embed_fails_without_fallback() {
    let root = "spec://org.demo/pkg/boot/root#root";
    let missing = "spec://org.demo/pkg/common/invented#root";
    let input = closure(
        root,
        &format!("# Root {{#root}}\n#embed {missing}\n"),
        snapshot(&[], &[root]),
    );

    let error = embed_closure(input).unwrap_err();

    assert!(matches!(
        error,
        EmbedPassError::MissingObservation { addr } if addr == missing
    ));
}

#[test]
fn observed_failure_uses_the_current_semantic_occurrence_address() {
    let root = "spec://org.demo/pkg/boot/root#root";
    let target = "spec://org.demo/pkg/common/missing#root";
    let observed_request = format!("{target}~r1");
    let replay_request = format!("{target}~r2");
    let mut pending = snapshot(&[], &[root]);
    pending.documents.insert(
        target.to_string(),
        DocumentObservation::Failed {
            requested: spec(&observed_request),
            reason: "first observation".to_string(),
        },
    );
    let input = closure(
        root,
        &format!("# Root {{#root}}\n#embed {replay_request}\n"),
        pending,
    );

    let error = embed_closure(input).unwrap_err();

    assert!(matches!(
        error,
        EmbedPassError::Embed(EmbedError::Unresolved { addr, reason })
            if addr == replay_request && reason == "first observation"
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn failure_in_a_later_root_leaves_the_entire_closure_unmodified() {
    let first = "spec://org.demo/pkg/boot/first#root";
    let second = "spec://org.demo/pkg/boot/second#root";
    let good = "spec://org.demo/pkg/common/good#root";
    let missing = "spec://org.demo/pkg/common/missing#root";
    let mut pending = snapshot(&[(good, "GOOD")], &[first, second]);
    pending.documents.insert(
        missing.to_string(),
        DocumentObservation::Failed {
            requested: spec(missing),
            reason: "late failure".to_string(),
        },
    );
    let mut input = closure(
        first,
        &format!("# First {{#root}}\n#embed {good}\n"),
        pending,
    );
    input.nodes.push(ClosureDocument {
        address: DocumentAddress::Spec(spec(second)),
        origin: "org.demo/pkg".to_string(),
        tree: DocTree::parse(&format!("# Second {{#root}}\n#embed {missing}\n")),
        aliases: Default::default(),
    });
    input.contributions[0] = ClosureContribution::Normal {
        meta: ContributionMeta {
            origin: "org.demo/pkg".to_string(),
            path: "boot/first".to_string(),
        },
        seed: ClosureNodeId(0),
        emission_order: vec![ClosureNodeId(0), ClosureNodeId(1)],
    };
    let before = input.clone();

    let error = embed_closure_in_place(&mut input).unwrap_err();

    assert!(matches!(
        error,
        EmbedPassError::Embed(EmbedError::Unresolved { addr, reason })
            if addr == missing && reason == "late failure"
    ));
    assert_eq!(input, before);
}
