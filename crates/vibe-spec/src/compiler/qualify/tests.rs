use std::collections::BTreeMap;

use specmark::verifies;

use super::*;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, AbsorptionState, ArtifactId, ClosureContribution,
    ClosureDocument, ClosureEdge, ClosureEdgeKind, ClosureIr, ClosureNodeId,
    ContributionAbsorption, ContributionMeta, DocumentAddress, LinkState, QualificationState,
    StaticCompileMode,
};

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn node(raw: &str, origin: &str, text: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::Spec(spec(raw)),
        origin: origin.to_string(),
        tree: DocTree::parse(text),
        aliases: BTreeMap::new(),
    }
}

fn meta(origin: &str) -> ContributionMeta {
    ContributionMeta {
        origin: origin.to_string(),
        path: format!("vibedeps/{origin}/boot/entry.md"),
    }
}

fn normal(seed: usize, order: &[usize], origin: &str) -> ClosureContribution {
    ClosureContribution::Normal {
        meta: meta(origin),
        seed: ClosureNodeId(seed),
        emission_order: order.iter().copied().map(ClosureNodeId).collect(),
    }
}

fn closure(
    mode: StaticCompileMode,
    nodes: Vec<ClosureDocument>,
    contributions: Vec<ClosureContribution>,
) -> ClosureIr {
    ClosureIr {
        artifact: ArtifactId::new("static-test").unwrap(),
        nodes,
        edges: Vec::new(),
        contributions,
        renames: Vec::new(),
        qualification: QualificationState::Pending(mode),
        absorption: AbsorptionState::Unplanned,
        link: LinkState::Unlinked,
        pending_sources: None,
        pending_embeds: None,
    }
}

fn run(input: ClosureIr) -> ClosureIr {
    QualifyPass::new().run(input).unwrap()
}

fn mask(output: &ClosureIr, contribution: usize) -> Vec<bool> {
    let plan = planned_plan(output);
    let ContributionAbsorption::Normal { occurrences, .. } = &plan.contributions[contribution]
    else {
        panic!("expected normal absorption mask")
    };
    occurrences
        .iter()
        .map(|occurrence| occurrence.absorbed)
        .collect()
}

fn planned_plan(output: &ClosureIr) -> &AbsorptionPlan {
    let AbsorptionState::Planned(plan) = &output.absorption else {
        panic!("expected planned absorption state")
    };
    plan
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn plain_alias_lowering_is_load_bearing_and_fences_stay_exact() {
    let mut root = node(
        "spec://org.demo/root/boot/entry#root",
        "org.demo/root",
        "# Entry {#root}\nOutside @!dep.\n```\n@!dep\n```\n",
    );
    root.aliases.insert(
        "dep".to_string(),
        spec("spec://org.demo/dep/boot/entry#root~r7"),
    );
    reset_qualify_invocations();

    let output = run(closure(
        StaticCompileMode::Plain,
        vec![root],
        vec![normal(0, &[0], "org.demo/root")],
    ));
    let text = output.nodes[0].tree.text(output.nodes[0].tree.root());

    assert_eq!(qualify_invocations(), 1);
    assert!(text.contains("Outside @spec://org.demo/dep/boot/entry#root."));
    assert!(text.contains("```\n@!dep\n```"));
    assert!(text.contains("{#root}"));
    assert!(output.nodes[0].aliases.is_empty());
    assert!(output.renames.is_empty());
    assert_eq!(
        output.qualification,
        QualificationState::Applied(StaticCompileMode::Plain)
    );
    assert_eq!(mask(&output, 0), [false]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn qualification_uses_each_node_origin_and_doc_path_but_leaves_cross_node_link() {
    let dependency = node(
        "spec://org.b/b/boot/dep#root",
        "org.b/b",
        "# Dependency {#root}\n##OTHER other\n",
    );
    let entry = node(
        "spec://org.a/a/common/entry#root",
        "org.a/a",
        "# Entry {#root}\nLocal (#root), cross (#OTHER).\n",
    );

    let output = run(closure(
        StaticCompileMode::QualifyPerNode,
        vec![dependency, entry],
        vec![normal(1, &[0, 1], "org.a/a")],
    ));
    let dep = output.nodes[0].tree.text(output.nodes[0].tree.root());
    let entry = output.nodes[1].tree.text(output.nodes[1].tree.root());

    assert!(dep.contains("{#org-b--b--root}"), "{dep}");
    assert!(dep.contains("##org-b--b--OTHER"), "{dep}");
    assert!(entry.contains("{#org-a--a--common-entry--root}"), "{entry}");
    assert!(
        entry.contains("Local (#org-a--a--common-entry--root), cross (#OTHER)."),
        "{entry}"
    );
    assert_eq!(
        output
            .renames
            .iter()
            .map(|entry| entry.origin.as_str())
            .collect::<Vec<_>>(),
        ["org.b/b", "org.b/b", "org.a/a"]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn absorption_is_planned_before_alias_rewrite() {
    let target = spec("spec://org.demo/dep/boot/entry#root");
    let sub_text = "## Sub {#sub}\nSees @!D.\n";
    let mut root = node(
        "spec://org.demo/pkg/common/doc#root",
        "org.demo/pkg",
        &format!("# Root {{#root}}\n{sub_text}"),
    );
    root.aliases.insert("D".to_string(), target);
    let sub = node(
        "spec://org.demo/pkg/common/doc#sub",
        "org.demo/pkg",
        sub_text,
    );

    let output = run(closure(
        StaticCompileMode::QualifyPerNode,
        vec![root, sub],
        vec![normal(0, &[0, 1], "org.demo/pkg")],
    ));

    assert_eq!(mask(&output, 0), [false, true]);
    let root = output.nodes[0].tree.text(output.nodes[0].tree.root());
    assert!(root.contains("@spec://org.demo/dep/boot/entry#root"));
    assert!(root.contains("{#org-demo--pkg--common-doc--sub}"));
    assert_eq!(
        output.nodes[1].tree.text(output.nodes[1].tree.root()),
        sub_text.trim_end()
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn equal_text_keeps_the_first_occurrence_in_either_order() {
    let first = node(
        "spec://org.demo/pkg/common/doc#one",
        "org.demo/pkg",
        "## Same {#same}\n",
    );
    let second = node(
        "spec://org.demo/pkg/common/doc#two",
        "org.demo/pkg",
        "## Same {#same}\n",
    );

    let forward = run(closure(
        StaticCompileMode::QualifyPerNode,
        vec![first.clone(), second.clone()],
        vec![normal(0, &[0, 1], "org.demo/pkg")],
    ));
    assert_eq!(mask(&forward, 0), [false, true]);
    assert!(
        forward.nodes[0]
            .tree
            .text(forward.nodes[0].tree.root())
            .contains("org-demo")
    );
    assert_eq!(
        forward.nodes[1].tree.text(forward.nodes[1].tree.root()),
        "## Same {#same}"
    );

    let reverse = run(closure(
        StaticCompileMode::QualifyPerNode,
        vec![first, second],
        vec![normal(1, &[1, 0], "org.demo/pkg")],
    ));
    assert_eq!(mask(&reverse, 0), [false, true]);
    assert!(
        reverse.nodes[1]
            .tree
            .text(reverse.nodes[1].tree.root())
            .contains("org-demo")
    );
    assert_eq!(
        reverse.nodes[0].tree.text(reverse.nodes[0].tree.root()),
        "## Same {#same}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn duplicate_node_occurrences_keep_an_aligned_mask_and_one_live_transform() {
    let shared = node(
        "spec://org.demo/pkg/boot/shared#root",
        "org.demo/pkg",
        "# Shared {#root}\n",
    );
    let input = closure(
        StaticCompileMode::QualifyPerNode,
        vec![shared],
        vec![normal(0, &[0, 0], "org.demo/pkg")],
    );
    let original_order = input.contributions.clone();
    let output = run(input);

    assert_eq!(mask(&output, 0), [false, true]);
    assert_eq!(output.contributions, original_order);
    assert_eq!(
        planned_plan(&output).mode,
        StaticCompileMode::QualifyPerNode
    );
    let ContributionAbsorption::Normal {
        seed_address,
        occurrences,
        ..
    } = &planned_plan(&output).contributions[0]
    else {
        unreachable!()
    };
    assert!(matches!(
        &output.nodes[0].address,
        DocumentAddress::Spec(address) if seed_address == address
    ));
    assert!(occurrences.iter().all(|occurrence| matches!(
        &output.nodes[occurrence.node.0].address,
        DocumentAddress::Spec(address) if occurrence.address == *address
    )));
    assert!(
        output.nodes[0]
            .tree
            .text(output.nodes[0].tree.root())
            .contains("org-demo--pkg--root")
    );
    assert_eq!(output.renames.len(), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn absorption_is_per_contribution_and_shared_live_node_keeps_graph_identity() {
    let ancestor = node(
        "spec://org.demo/pkg/common/doc#root",
        "org.demo/pkg",
        "# Ancestor {#root}\nshared @!D\n",
    );
    let mut shared = node(
        "spec://org.demo/pkg/common/doc#sub",
        "org.demo/pkg",
        "shared @!D\n",
    );
    shared
        .aliases
        .insert("D".to_string(), spec("spec://org.demo/dep/boot/entry#root"));
    let edges = vec![ClosureEdge {
        from: ClosureNodeId(0),
        to: ClosureNodeId(1),
        kind: ClosureEdgeKind::Use,
    }];
    let mut input = closure(
        StaticCompileMode::QualifyPerNode,
        vec![ancestor, shared],
        vec![
            normal(0, &[0, 1], "org.demo/pkg"),
            normal(1, &[1], "org.demo/pkg"),
        ],
    );
    input.edges = edges.clone();
    let orders = input.contributions.clone();
    let output = run(input);

    assert_eq!(mask(&output, 0), [false, true]);
    assert_eq!(mask(&output, 1), [false]);
    assert_eq!(output.edges, edges);
    assert_eq!(output.contributions, orders);
    assert_eq!(output.nodes.len(), 2);
    let shared = output.nodes[1].tree.text(output.nodes[1].tree.root());
    assert!(shared.contains("@spec://org.demo/dep/boot/entry#root"));
    assert!(output.nodes[1].aliases.is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn node_dead_in_every_contribution_stays_authored_and_adds_no_rename() {
    let dead_text = "plain @!D\n";
    let ancestor = node(
        "spec://org.demo/pkg/common/doc#root",
        "org.demo/pkg",
        &format!("ancestor\n{dead_text}"),
    );
    let mut dead = node(
        "spec://org.demo/pkg/common/doc#sub",
        "org.demo/pkg",
        dead_text,
    );
    dead.aliases
        .insert("D".to_string(), spec("spec://org.demo/dep/boot/entry#root"));
    let output = run(closure(
        StaticCompileMode::QualifyPerNode,
        vec![ancestor, dead],
        vec![normal(0, &[0, 1], "org.demo/pkg")],
    ));

    assert_eq!(mask(&output, 0), [false, true]);
    assert_eq!(
        output.nodes[1].tree.text(output.nodes[1].tree.root()),
        dead_text.trim_end()
    );
    assert!(output.nodes[1].aliases.contains_key("D"));
    assert!(output.renames.is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn source_or_embed_only_graph_node_stays_entirely_untouched() {
    let root = node(
        "spec://org.demo/pkg/boot/entry#root",
        "org.demo/pkg",
        "# Root {#root}\n",
    );
    let mut graph_only = node(
        "spec://org.other/dep/common/piece#root",
        "org.other/dep",
        "# Graph only {#orphan}\n@!D\n",
    );
    graph_only.aliases.insert(
        "D".to_string(),
        spec("spec://org.demo/target/boot/entry#root"),
    );
    let mut input = closure(
        StaticCompileMode::QualifyPerNode,
        vec![root, graph_only.clone()],
        vec![normal(0, &[0], "org.demo/pkg")],
    );
    input.edges.push(ClosureEdge {
        from: ClosureNodeId(0),
        to: ClosureNodeId(1),
        kind: ClosureEdgeKind::Source,
    });
    let output = run(input);

    assert_eq!(output.nodes[1], graph_only);
    assert!(
        output
            .renames
            .iter()
            .all(|entry| entry.rename.original != "orphan")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn normal_simple_normal_order_and_simple_origin_are_preserved() {
    let alpha = node(
        "spec://org.demo/alpha/boot/entry#root",
        "org.demo/alpha",
        "# Alpha {#root}\n",
    );
    let omega = node(
        "spec://org.demo/omega/boot/entry#root",
        "org.demo/omega",
        "# Omega {#root}\n",
    );
    let simple = ClosureDocument {
        address: DocumentAddress::StaticEntry {
            origin: "host".to_string(),
            path: "boot/local.md".to_string(),
        },
        origin: "stale-document-origin".to_string(),
        tree: DocTree::parse("# Local {#root}\n"),
        aliases: BTreeMap::new(),
    };
    let contributions = vec![
        normal(0, &[0], "org.demo/alpha"),
        ClosureContribution::Simple {
            meta: meta("host"),
            document: Box::new(simple),
        },
        normal(1, &[1], "org.demo/omega"),
    ];
    let output = run(closure(
        StaticCompileMode::QualifyPerNode,
        vec![alpha, omega],
        contributions,
    ));

    assert!(matches!(
        output.contributions[0],
        ClosureContribution::Normal { .. }
    ));
    let ClosureContribution::Simple { document, .. } = &output.contributions[1] else {
        panic!("middle contribution must remain simple")
    };
    assert!(
        document
            .tree
            .text(document.tree.root())
            .contains("{#host--root}")
    );
    assert!(matches!(
        output.contributions[2],
        ClosureContribution::Normal { .. }
    ));
    assert_eq!(
        output
            .renames
            .iter()
            .map(|entry| entry.origin.as_str())
            .collect::<Vec<_>>(),
        ["org.demo/alpha", "host", "org.demo/omega"]
    );
    assert!(matches!(
        planned_plan(&output).contributions.as_slice(),
        [
            ContributionAbsorption::Normal { .. },
            ContributionAbsorption::Simple { .. },
            ContributionAbsorption::Normal { .. }
        ]
    ));
}

#[test]
fn absorption_alignment_is_a_private_invariant() {
    let input = closure(
        StaticCompileMode::Plain,
        vec![node(
            "spec://org.demo/pkg/boot/entry#root",
            "org.demo/pkg",
            "body",
        )],
        vec![normal(0, &[0, 0], "org.demo/pkg")],
    );
    let DocumentAddress::Spec(address) = &input.nodes[0].address else {
        unreachable!()
    };
    let address = address.clone();
    let invalid = AbsorptionPlan {
        mode: StaticCompileMode::Plain,
        contributions: vec![ContributionAbsorption::Normal {
            meta: meta("org.demo/pkg"),
            seed: ClosureNodeId(0),
            seed_address: address.clone(),
            occurrences: vec![AbsorptionOccurrence {
                node: ClosureNodeId(0),
                address,
                absorbed: false,
            }],
        }],
    };
    assert!(matches!(
        absorption::validate(&invalid, &input),
        Err(QualifyPassError::AbsorptionAlignment {
            contribution: Some(0),
            expected: 2,
            actual: 1,
        })
    ));
}

#[test]
fn invalid_simple_alias_state_is_rejected_transactionally() {
    let mut simple = ClosureDocument {
        address: DocumentAddress::StaticEntry {
            origin: "host".to_string(),
            path: "boot/local.md".to_string(),
        },
        origin: "host".to_string(),
        tree: DocTree::parse("# Local {#root}\n@!D\n"),
        aliases: BTreeMap::new(),
    };
    simple
        .aliases
        .insert("D".to_string(), spec("spec://org.demo/dep/boot/entry#root"));
    let input = closure(
        StaticCompileMode::QualifyPerNode,
        Vec::new(),
        vec![ClosureContribution::Simple {
            meta: meta("host"),
            document: Box::new(simple),
        }],
    );
    let before = input.clone();

    assert!(matches!(
        QualifyPass::new().run(input),
        Err(QualifyPassError::SimpleAliases { contribution: 0 })
    ));
    let ClosureContribution::Simple { document, .. } = &before.contributions[0] else {
        panic!("fixture remains simple")
    };
    assert!(document.tree.text(document.tree.root()).contains("@!D"));
    assert!(document.aliases.contains_key("D"));
}
