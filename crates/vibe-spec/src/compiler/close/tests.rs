use specmark::verifies;

use super::*;
use crate::DocTree;
use crate::compiler::embed_snapshot::EmbedResolutionSnapshot;
use crate::compiler::ir::{SourceFormatId, SourceIr};
use crate::compiler::pass::IrPayload;
use crate::compiler::source_snapshot::{ExpansionObservation, SourceResolutionSnapshot};

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn document(raw: &str, text: &str) -> DocumentIr {
    let source = SourceIr::new(
        DocumentAddress::Spec(spec(raw)),
        SourceFormatId::canonical_markdown(),
        text,
    );
    let tree = DocTree::parse(source.text());
    DocumentIr::new(source, tree)
}

fn close(
    seed: &SpecAddress,
    documents: Documents,
    state: &CloseState,
) -> Result<ClosureIr, UseGraphError> {
    close_documents(
        &ArtifactId::new("static-fragment").unwrap(),
        &ContributionMeta {
            origin: document_origin(seed),
            path: seed.doc_path.clone(),
        },
        StaticCompileMode::Plain,
        seed,
        documents,
        state,
    )
}

fn node_keys(closure: &ClosureIr) -> Vec<String> {
    closure
        .nodes
        .iter()
        .map(|node| match &node.address {
            DocumentAddress::Spec(address) => address.without_pin(),
            DocumentAddress::StaticEntry { .. } => panic!("unexpected static entry"),
        })
        .collect()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn diamond_is_dependency_first_deduplicated_and_exact() {
    let a = "spec://org.demo/pkg/boot/a#root";
    let b = "spec://org.demo/pkg/boot/b#root";
    let c = "spec://org.demo/pkg/boot/c#root";
    let d = "spec://org.demo/pkg/boot/d#root";
    let documents = Documents::new(vec![
        document(a, &format!("# A {{#root}}\n#use {b}\n#use {c}\n")),
        document(b, &format!("# B {{#root}}\n#use {d}\n")),
        document(d, "# D {#root}\n"),
        document(c, &format!("# C {{#root}}\n#use {d}\n")),
    ]);

    let closure = close(&spec(a), documents, &CloseState::default()).unwrap();

    assert_eq!(node_keys(&closure), vec![d, b, c, a]);
    assert_eq!(
        closure.nodes.len(),
        4,
        "the shared diamond node closes once"
    );
    assert_eq!(closure.edges.len(), 4);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn implementation_cycle_keeps_the_exact_public_path() {
    let a = "spec://org.demo/pkg/source/a#root";
    let b = "spec://org.demo/pkg/source/b#root";
    let missing = spec("spec://org.demo/pkg/source/missing#root");
    let documents = Documents::new(vec![
        document(
            a,
            &format!("# A {{#root}}\n#use {b}\n#use {}\n", missing.without_pin()),
        ),
        document(b, &format!("# B {{#root}}\n#use {a}\n")),
    ]);
    let state = CloseState::default();
    state.record_failure(&missing, "later missing".to_string());

    let error = close(&spec(a), documents, &state).unwrap_err();
    assert_eq!(
        error,
        UseGraphError::Cycle(vec![a.to_string(), b.to_string(), a.to_string()])
    );
    assert_eq!(error.to_string(), format!("use cycle: {a} -> {b} -> {a}"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn contract_cycle_is_admitted_with_dependency_first_legacy_order() {
    let a = "spec://org.demo/pkg/contract/a#root";
    let b = "spec://org.demo/pkg/contract/b#root";
    let documents = Documents::new(vec![
        document(a, &format!("# A {{#root}}\n#use {b}\n")),
        document(b, &format!("# B {{#root}}\n#use {a}\n")),
    ]);

    let closure = close(&spec(a), documents, &CloseState::default()).unwrap();
    assert_eq!(node_keys(&closure), vec![b, a]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn unresolved_load_is_replayed_at_its_declared_graph_position() {
    let a = "spec://org.demo/pkg/boot/a#root";
    let missing = spec("spec://org.demo/pkg/boot/missing#root");
    let later = "spec://org.demo/pkg/boot/later#root";
    let documents = Documents::new(vec![
        document(
            a,
            &format!(
                "# A {{#root}}\n#use {}\n#use {later}\n",
                missing.without_pin()
            ),
        ),
        document(later, "# Later {#root}\n"),
    ]);
    let state = CloseState::default();
    state.record_failure(&missing, "not in mock".to_string());

    let error = close(&spec(a), documents, &state).unwrap_err();
    assert_eq!(
        error,
        UseGraphError::Unresolved {
            addr: missing.to_string(),
            reason: "not in mock".to_string(),
        }
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn close_is_one_artifact_and_its_parsed_tree_body_is_load_bearing() {
    let key = "spec://org.demo/pkg/boot/a#root";
    let raw = SourceIr::new(
        DocumentAddress::Spec(spec(key)),
        SourceFormatId::canonical_markdown(),
        "# Raw {#raw}\nRAW\n",
    );
    let document = DocumentIr::new(raw, DocTree::parse("# Parsed {#parsed}\nPARSED\n"));

    let closure = close(
        &spec(key),
        Documents::new(vec![document]),
        &CloseState::default(),
    )
    .unwrap();

    assert_eq!(
        ClosureIr::SHAPE.cardinality,
        super::super::ir::IrCardinality::Artifact
    );
    assert_eq!(
        closure.nodes[0].tree.text(closure.nodes[0].tree.root()),
        "# Parsed {#parsed}\nPARSED"
    );
    assert!(
        !closure.nodes[0]
            .tree
            .text(closure.nodes[0].tree.root())
            .contains("RAW")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn close_transports_invalid_pending_sources_without_judging_membership() {
    let key = "spec://org.demo/pkg/contract/api#root";
    let pattern = spec("spec://org.demo/plugin-*/source/impl#root");
    let source = SourceIr::new(
        DocumentAddress::Spec(spec(key)),
        SourceFormatId::canonical_markdown(),
        format!("# API {{#root}}\n#source {}\n", pattern.without_pin()),
    );
    let document = DocumentIr::new(source.clone(), DocTree::parse(source.text()));
    let mut pending = SourceResolutionSnapshot::default();
    pending.expansions.insert(
        pattern.without_pin(),
        ExpansionObservation::Failed {
            requested: pattern,
            reason: "must be judged by merge".to_string(),
        },
    );
    let state = CloseState::default();
    state.set_pending_sources(pending.clone());

    let closure = close(&spec(key), Documents::new(vec![document]), &state).unwrap();

    assert_eq!(closure.nodes.len(), 1);
    assert!(closure.edges.is_empty());
    assert_eq!(closure.pending_sources, Some(pending));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn close_transports_pending_embeds_without_judging_membership() {
    let key = "spec://org.demo/pkg/boot/root#root";
    let pending = EmbedResolutionSnapshot {
        discovery_order: vec!["spec://org.demo/pkg/common/piece#root".to_string()],
        ..Default::default()
    };
    let state = CloseState::default();
    state.set_pending_embeds(pending.clone());

    let closure = close(
        &spec(key),
        Documents::new(vec![document(key, "# Root {#root}\n")]),
        &state,
    )
    .unwrap();

    assert_eq!(closure.nodes.len(), 1);
    assert!(closure.edges.is_empty());
    assert_eq!(closure.pending_embeds, Some(pending));
    assert_eq!(
        closure.qualification,
        QualificationState::Pending(StaticCompileMode::Plain)
    );
    assert!(matches!(closure.absorption, AbsorptionState::Unplanned));
}
