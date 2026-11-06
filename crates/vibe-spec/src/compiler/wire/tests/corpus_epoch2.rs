//! Current corpus production. Historical source inputs are re-parsed; ready
//! Closure/Lane/Emitted values come from the current named owners. The separate
//! held custom-target fixture stays explicitly unlinked with pending work.

use super::super::{decode, encode_pretty, history::inspect_historical_compiler_ir};
use crate::compiler::{
    builtin::{compile_artifact, compile_artifact_lane, compile_artifact_prefix},
    ir::{ArtifactInput, ArtifactPlan, ArtifactTarget, DocumentIr, Documents},
    pass::{AnyIr, PassName},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_wire::generated::compiler_ir::e1::ir::Ir as Historical;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formats/corpora/compiler_ir")
}
fn old(name: &str) -> Historical {
    inspect_historical_compiler_ir(&std::fs::read(root().join("e1/valid").join(name)).unwrap())
        .unwrap()
}

fn ready_plan() -> ArtifactPlan {
    let Historical::ClosureArtifact(old) = old("closure_artifact.json") else {
        panic!("closure fixture")
    };
    let context = super::super::closure::decode_context(&old.closure.context).unwrap();
    ArtifactPlan::new(
        context,
        vec![
            ArtifactInput::normal(
                "org.demo/lib",
                "manual/guide.md",
                crate::SpecAddress::parse("spec://org.demo/lib/manual/guide.md#root").unwrap(),
            )
            .unwrap(),
            ArtifactInput::simple(
                "__host__/demo",
                "vibevm/vibespecs/boot/00-core.md",
                "# Core boot rules\n\nRead this first.\n",
            )
            .unwrap(),
            ArtifactInput::elided("org.demo/legacy", "legacy/old.md").unwrap(),
            ArtifactInput::hoisted(
                "org.demo/lib",
                "manual/part.md",
                crate::SpecAddress::parse("spec://org.demo/lib/manual/part.md").unwrap(),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn ready_source() -> super::fixture::Map {
    let guide = "# Guide {#root}\n\n##FACT-A One exact fact.\n\n#use spec://org.demo/lib/manual/part.md\n#use spec://org.demo/lib/manual/part.md#api~r2 as Part\n";
    let part = "# Part {#api}\n\nReference body.\n\n```text\nliteral sample\n```\n";
    super::fixture::Map(BTreeMap::from([
        (
            "spec://org.demo/lib/manual/guide.md#root".into(),
            guide.into(),
        ),
        ("spec://org.demo/lib/manual/part.md".into(), part.into()),
        ("spec://org.demo/lib/manual/part.md#api".into(), part.into()),
    ]))
}

fn pending_custom_fixture() -> AnyIr {
    let historical = old("closure_artifact_compat.json");
    let Historical::ClosureArtifact(arm) = &historical else {
        panic!("closure fixture")
    };
    assert!(matches!(
        arm.closure.link,
        vibe_wire::generated::shared::LinkState::Unlinked(_)
    ));
    assert!(arm.closure.pending_sources.is_some() && arm.closure.pending_embeds.is_some());
    // An authored current fixture adapter, not a product migration: no ready
    // witness or annotation is invented for this still-pending input.
    let mut value = serde_json::to_value(historical).unwrap();
    value["ir_schema"] = 2.into();
    for node in value["closure"]["nodes"].as_array_mut().unwrap() {
        node["embeds"] = serde_json::Value::Null;
    }
    for contribution in value["closure"]["contributions"].as_array_mut().unwrap() {
        if contribution["kind"] == "simple" {
            contribution["document"]["embeds"] = serde_json::Value::Null;
        }
    }
    decode(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn produced() -> BTreeMap<String, AnyIr> {
    let mut values = BTreeMap::new();
    for name in ["source_document.json", "source_document_reached.json"] {
        let Historical::SourceDocument(value) = old(name) else {
            panic!("source fixture")
        };
        values.insert(
            name.into(),
            AnyIr::Source(super::super::address::decode_source_doc(&value.doc).unwrap()),
        );
    }
    let Historical::DocumentDocument(value) = old("document_document.json") else {
        panic!("document fixture")
    };
    let source = super::super::address::decode_source_doc(&value.doc.source).unwrap();
    values.insert(
        "document_document.json".into(),
        AnyIr::Document(DocumentIr::new(
            source.clone(),
            crate::DocTree::parse(source.text()),
        )),
    );
    let Historical::DocumentsArtifact(value) = old("documents_artifact.json") else {
        panic!("documents fixture")
    };
    values.insert(
        "documents_artifact.json".into(),
        AnyIr::Documents(Documents::new(
            value
                .documents
                .iter()
                .map(|document| {
                    let source =
                        super::super::address::decode_source_doc(&document.source).unwrap();
                    DocumentIr::new(source.clone(), crate::DocTree::parse(source.text()))
                })
                .collect(),
        )),
    );
    let plan = ready_plan();
    let source = ready_source();
    values.insert(
        "closure_artifact.json".into(),
        AnyIr::Closure(compile_artifact_prefix(plan.clone(), &source).unwrap()),
    );
    values.insert(
        "lane_artifact.json".into(),
        AnyIr::Lane(compile_artifact_lane(plan.clone(), &source).unwrap()),
    );
    let emitted = compile_artifact(plan, &source).unwrap();
    values.insert(
        "emitted_artifact.json".into(),
        AnyIr::Emitted(emitted.clone()),
    );
    let mut bytes = emitted.bytes().to_vec();
    bytes.extend_from_slice(b"\nfirst explicit opaque rewrite\n");
    let transformed = crate::compiler::transform::emitted_reconstruction::reconstruct(
        emitted,
        bytes,
        &PassName::new("transform:z-first").unwrap(),
    );
    let mut bytes = transformed.bytes().to_vec();
    bytes.extend_from_slice(b"second explicit opaque rewrite\n");
    let transformed = crate::compiler::transform::emitted_reconstruction::reconstruct(
        transformed,
        bytes,
        &PassName::new("transform:a-second").unwrap(),
    );
    values.insert(
        "emitted_artifact_transformed.json".into(),
        AnyIr::Emitted(transformed),
    );
    values.insert(
        "closure_artifact_compat.json".into(),
        pending_custom_fixture(),
    );

    let mut embedded = super::fixture::world();
    embedded.0.insert(
        "spec://org.demo/alpha/boot/entry#root".into(),
        "# Alpha {#root}\nbefore\n#embed spec://org.demo/shared/outer\nafter\n".into(),
    );
    embedded.0.insert(
        "spec://org.demo/shared/outer".into(),
        "OUTER\n#embed spec://org.demo/shared/inner\nEND\n".into(),
    );
    embedded
        .0
        .insert("spec://org.demo/shared/inner".into(), "INNER\n".into());
    let plan = super::fixture::plan_for(ArtifactTarget::StaticXml);
    values.insert(
        "closure_artifact_embedded.json".into(),
        AnyIr::Closure(compile_artifact_prefix(plan.clone(), &embedded).unwrap()),
    );
    values.insert(
        "lane_artifact_embedded.json".into(),
        AnyIr::Lane(compile_artifact_lane(plan.clone(), &embedded).unwrap()),
    );
    values.insert(
        "emitted_artifact_embedded.json".into(),
        AnyIr::Emitted(compile_artifact(plan, &embedded).unwrap()),
    );
    values
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-054#IR-EMBED-PROVENANCE-EPOCH"
)]
fn current_epoch_corpus_is_exact_current_owner_output() {
    let values = produced();
    assert_eq!(values.len(), 12);
    let encoded = values
        .into_iter()
        .map(|(name, value)| {
            let bytes = encode_pretty(&value).unwrap();
            let back = decode(&bytes).unwrap();
            assert_eq!(encode_pretty(&back).unwrap(), bytes, "{name}");
            (name, bytes)
        })
        .collect::<BTreeMap<_, _>>();
    let update = std::env::var_os("VIBE_REGENERATE_COMPILER_IR_E2").is_some();
    let directory = root().join("e2/valid");
    if update {
        std::fs::create_dir_all(&directory).unwrap();
    }
    for (name, bytes) in encoded {
        let path = directory.join(&name);
        if update {
            std::fs::write(&path, &bytes).unwrap();
        }
        assert_eq!(
            std::fs::read(path).unwrap(),
            bytes,
            "{name}: rerun this owner with VIBE_REGENERATE_COMPILER_IR_E2=1 after reviewing the changed semantic contract"
        );
    }
}
