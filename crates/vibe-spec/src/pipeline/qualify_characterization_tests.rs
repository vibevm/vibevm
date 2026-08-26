//! Output-sensitive characterizations of the named qualify boundary.

use specmark::verifies;

use super::tests::MockSource;
use super::*;
use crate::DocTree;
use crate::compiler::ir::{
    ArtifactId, ClosureContribution, ClosureDocument, ClosureIr, ClosureNodeId, ContributionMeta,
    DocumentAddress, QualificationState, StaticCompileMode,
};
use crate::compiler::pass::Pass;
use crate::compiler::qualify::{
    QualifyPass, QualifyPassError, qualify_invocations, reset_qualify_invocations,
    validate_absorption,
};

fn ir_node(raw: &str, origin: &str, text: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::Spec(SpecAddress::parse(raw).unwrap()),
        origin: origin.to_string(),
        tree: DocTree::parse(text),
        aliases: Default::default(),
    }
}

fn ir_meta(origin: &str, path: &str) -> ContributionMeta {
    ContributionMeta {
        origin: origin.to_string(),
        path: path.to_string(),
    }
}

fn ir_normal(meta: ContributionMeta, seed: usize, order: &[usize]) -> ClosureContribution {
    ClosureContribution::Normal {
        meta,
        seed: ClosureNodeId(seed),
        emission_order: order.iter().copied().map(ClosureNodeId).collect(),
    }
}

fn pending_closure(
    nodes: Vec<ClosureDocument>,
    contributions: Vec<ClosureContribution>,
) -> ClosureIr {
    ClosureIr {
        artifact: ArtifactId::new("static-stale-plan-test").unwrap(),
        nodes,
        edges: Vec::new(),
        contributions,
        renames: Vec::new(),
        qualification: QualificationState::Pending(StaticCompileMode::QualifyPerNode),
        absorption: None,
        pending_sources: None,
        pending_embeds: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn both_public_modes_invoke_the_named_qualify_pass_once() {
    let key = "spec://org.demo/pkg/boot/entry#root";
    let source = MockSource::new(&[(key, "# Entry {#root}\n")]);
    let seed = SpecAddress::parse(key).unwrap();

    reset_qualify_invocations();
    compile_static(&seed, &source).unwrap();
    assert_eq!(qualify_invocations(), 1);

    reset_qualify_invocations();
    compile_static_qualified(&seed, &source).unwrap();
    assert_eq!(qualify_invocations(), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn alias_rewrite_cannot_change_the_prequalified_absorption_judgment() {
    let entry = "spec://org.a/a/boot/entry#root";
    let root = "spec://org.b/b/common/shared#root";
    let sub = "spec://org.b/b/common/shared#sub";
    let dep = "spec://org.c/c/boot/dep#root";
    let source = MockSource::new(&[
        (
            entry,
            &format!("# Entry {{#root}}\n#use {root}\n#use {sub}\n"),
        ),
        (
            root,
            &format!("# Shared {{#root}}\n#use {dep} as D\n## Sub {{#sub}}\nSees @!D.\n"),
        ),
        (sub, "## Sub {#sub}\nSees @!D.\n"),
        (dep, "# Dependency {#root}\n"),
    ]);

    let (output, renames) =
        compile_static_qualified(&SpecAddress::parse(entry).unwrap(), &source).unwrap();

    assert!(output.contains("@spec://org.c/c/boot/dep#root"), "{output}");
    assert!(!output.contains("@!D"), "{output}");
    assert!(output.contains("vibe:begin spec://org.b/b/common/shared#root"));
    assert!(!output.contains("vibe:begin spec://org.b/b/common/shared#sub"));
    assert_eq!(
        output.matches("{#org-b--b--common-shared--sub}").count(),
        1,
        "{output}"
    );
    assert_eq!(
        renames
            .iter()
            .filter(|(_, rename)| rename.original == "sub")
            .count(),
        1
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn cross_package_source_text_qualifies_under_its_emitted_contract_owner() {
    let contract = "spec://org.a/a/contract/api#root";
    let source_address = "spec://org.c/c/source/impl#root";
    let source = MockSource::new(&[
        (
            contract,
            &format!("# API {{#root}}\n#source {source_address}\n"),
        ),
        (
            source_address,
            "## Extension {#extension}\nSOURCE-CONTENT\n",
        ),
    ]);

    let (output, _) =
        compile_static_qualified(&SpecAddress::parse(contract).unwrap(), &source).unwrap();

    assert!(output.contains("{#org-a--a--extension}"), "{output}");
    assert!(!output.contains("org-c--c--extension"), "{output}");
    assert!(!output.contains("vibe:begin spec://org.c/c/source/impl#root"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn legacy_tail_rejects_a_bypassed_qualify_pass() {
    let address = SpecAddress::parse("spec://org.demo/pkg/boot/entry#root").unwrap();
    let closure = ClosureIr {
        artifact: ArtifactId::new("static-fragment").unwrap(),
        nodes: vec![ClosureDocument {
            address: DocumentAddress::Spec(address),
            origin: "org.demo/pkg".to_string(),
            tree: DocTree::parse("# Entry {#root}\n"),
            aliases: Default::default(),
        }],
        edges: Vec::new(),
        contributions: vec![ClosureContribution::Normal {
            meta: ContributionMeta {
                origin: "org.demo/pkg".to_string(),
                path: "boot/entry".to_string(),
            },
            seed: ClosureNodeId(0),
            emission_order: vec![ClosureNodeId(0)],
        }],
        renames: Vec::new(),
        qualification: QualificationState::Pending(StaticCompileMode::Plain),
        absorption: None,
        pending_sources: None,
        pending_embeds: None,
    };

    let panic = std::panic::catch_unwind(|| compile_static_continuation(closure));
    assert!(
        panic.is_err(),
        "the legacy tail must not hide a pass bypass"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn same_length_emission_reorder_is_rejected_before_legacy_emission() {
    let ancestor = ir_node(
        "spec://org.demo/pkg/common/doc#root",
        "org.demo/pkg",
        "# Root {#root}\n## Sub {#sub}\nSUB\n",
    );
    let sub = ir_node(
        "spec://org.demo/pkg/common/doc#sub",
        "org.demo/pkg",
        "## Sub {#sub}\nSUB\n",
    );
    let mut qualified = QualifyPass::new()
        .run(pending_closure(
            vec![ancestor, sub],
            vec![ir_normal(ir_meta("org.demo/pkg", "common/doc"), 0, &[0, 1])],
        ))
        .unwrap();

    let ClosureContribution::Normal { emission_order, .. } = &mut qualified.contributions[0] else {
        panic!("fixture contribution remains normal")
    };
    emission_order.swap(0, 1);

    let error = validate_absorption(qualified.absorption.as_ref().unwrap(), &qualified)
        .expect_err("stale occurrence order must fail");
    assert!(matches!(
        error,
        QualifyPassError::AbsorptionOccurrence {
            contribution: 0,
            occurrence: 0,
            expected: 0,
            actual: 1,
        }
    ));
    let consumer = std::panic::catch_unwind(|| compile_static_continuation(qualified));
    assert!(
        consumer.is_err(),
        "legacy absorb must reject before returning any emitted artifact"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn top_level_reorder_is_bound_by_seed_meta_and_simple_address() {
    let a = ir_node(
        "spec://org.demo/pkg/boot/a#root",
        "org.demo/pkg",
        "# A {#root}\n",
    );
    let b = ir_node(
        "spec://org.demo/pkg/boot/b#root",
        "org.demo/pkg",
        "# B {#root}\n",
    );
    let shared_meta = ir_meta("org.demo/pkg", "boot/shared");
    let mut seed_swap = QualifyPass::new()
        .run(pending_closure(
            vec![a.clone(), b.clone()],
            vec![
                ir_normal(shared_meta.clone(), 0, &[0]),
                ir_normal(shared_meta.clone(), 1, &[1]),
            ],
        ))
        .unwrap();
    seed_swap.contributions.swap(0, 1);
    assert!(matches!(
        validate_absorption(seed_swap.absorption.as_ref().unwrap(), &seed_swap),
        Err(QualifyPassError::AbsorptionSeed {
            contribution: 0,
            expected: 0,
            actual: 1,
        })
    ));

    let mut meta_swap = QualifyPass::new()
        .run(pending_closure(
            vec![a, b],
            vec![
                ir_normal(ir_meta("org.demo/a", "boot/a"), 0, &[0]),
                ir_normal(ir_meta("org.demo/b", "boot/b"), 0, &[0]),
            ],
        ))
        .unwrap();
    meta_swap.contributions.swap(0, 1);
    assert!(matches!(
        validate_absorption(meta_swap.absorption.as_ref().unwrap(), &meta_swap),
        Err(QualifyPassError::AbsorptionContributionIdentity {
            contribution: 0,
            ..
        })
    ));

    let simple = |path: &str| ClosureContribution::Simple {
        meta: ir_meta("host", "boot/local"),
        document: Box::new(ClosureDocument {
            address: DocumentAddress::StaticEntry {
                origin: "host".to_string(),
                path: path.to_string(),
            },
            origin: "host".to_string(),
            tree: DocTree::parse("plain simple body"),
            aliases: Default::default(),
        }),
    };
    let mut simple_swap = QualifyPass::new()
        .run(pending_closure(
            Vec::new(),
            vec![simple("boot/a.md"), simple("boot/b.md")],
        ))
        .unwrap();
    simple_swap.contributions.swap(0, 1);
    assert!(matches!(
        validate_absorption(simple_swap.absorption.as_ref().unwrap(), &simple_swap),
        Err(QualifyPassError::AbsorptionContributionIdentity {
            contribution: 0,
            ..
        })
    ));
}
