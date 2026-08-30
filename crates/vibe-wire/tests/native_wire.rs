//! Focused epoch-1 native wire proofs: registry policy, authored corpus,
//! generated sharing, exact schema field sets, forward/strict object readers,
//! open points, and the deliberately distinct context/reply artifact rows.

use std::any::TypeId;
use std::collections::BTreeSet;
use std::path::PathBuf;

use vibe_wire::generated::format_id::{ForeignParsers, FormatId};
use vibe_wire::generated::lifecycle::e1::{context as lifecycle_context, reply as lifecycle_reply};
use vibe_wire::generated::native::e1::{context, manifest, reply};
use vibe_wire::generated::shared;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn read_json(relative: &str) -> serde_json::Value {
    serde_json::from_str(&read(relative))
        .unwrap_or_else(|error| panic!("{relative} parses as JSON: {error}"))
}

fn keys(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("the selected schema node is an object")
        .keys()
        .cloned()
        .collect()
}

fn strings(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn registry_block(id: &str) -> String {
    let registry = read("formats/REGISTRY.toml");
    let marker = format!("[format.{id}]");
    let tail = registry
        .split_once(&marker)
        .unwrap_or_else(|| panic!("registry contains {marker}"))
        .1;
    tail.split("\n[format.").next().unwrap().to_string()
}

#[test]
fn registry_registers_the_three_exact_native_roots() {
    for (format, id, role, schema) in [
        (
            FormatId::NativeContext,
            "native-context",
            ForeignParsers::Many,
            "schemas/native/e1/context.jtd.json",
        ),
        (
            FormatId::NativeReply,
            "native-reply",
            ForeignParsers::None,
            "schemas/native/e1/reply.jtd.json",
        ),
        (
            FormatId::NativeManifest,
            "native-manifest",
            ForeignParsers::Many,
            "schemas/native/e1/manifest.jtd.json",
        ),
    ] {
        assert_eq!(format.id(), id);
        assert_eq!(format.epoch(), 1);
        assert!(format.recoverable());
        assert_eq!(format.foreign_parsers(), role);

        let block = registry_block(id);
        assert!(block.contains(&format!("schema = \"{schema}\"")), "{id}");
        let role = match role {
            ForeignParsers::None => "none",
            ForeignParsers::Ours => "ours",
            ForeignParsers::Many => "many",
        };
        assert!(
            block.contains(&format!("foreign_parsers = \"{role}\"")),
            "{id}"
        );
        assert!(
            block.contains("corpus = \"formats/corpora/native/e1\""),
            "{id}"
        );
        assert!(block.contains("sunset = \"none\""), "{id}");
    }
}

#[test]
fn schemas_pin_exact_roots_and_shared_fragment_closures() {
    let lifecycle = read_json("schemas/lifecycle/e1/context.jtd.json");
    let native = read_json("schemas/native/e1/context.jtd.json");
    let context_properties = strings(&[
        "envelope",
        "point",
        "execution",
        "project",
        "world",
        "run",
        "artifacts",
        "io",
    ]);
    assert_eq!(keys(&lifecycle["properties"]), context_properties);
    assert_eq!(keys(&native["properties"]), context_properties);
    assert_eq!(
        keys(&lifecycle["optionalProperties"]),
        strings(&["slot_target"])
    );
    assert_eq!(
        keys(&native["optionalProperties"]),
        strings(&["slot_target"])
    );
    assert!(lifecycle.get("definitions").is_none());
    assert!(native.get("definitions").is_none());
    assert_eq!(
        lifecycle["metadata"]["x-vocabularies"],
        native["metadata"]["x-vocabularies"]
    );
    assert_eq!(
        keys(&serde_json::json!({
            "execution": null,
            "project": null,
            "world": null,
            "run": null,
            "artifact": null,
            "io": null,
            "slot_target": null
        })),
        lifecycle["metadata"]["x-vocabularies"]
            .as_array()
            .expect("context declares its vocabulary closure")
            .iter()
            .map(|value| value.as_str().expect("a vocabulary name").to_string())
            .collect()
    );

    let lifecycle = read_json("schemas/lifecycle/e1/reply.jtd.json");
    let native = read_json("schemas/native/e1/reply.jtd.json");
    assert_eq!(
        keys(&lifecycle["properties"]),
        strings(&["envelope", "status", "artifacts", "tasks"])
    );
    assert_eq!(
        keys(&native["properties"]),
        strings(&["envelope", "status", "artifacts"])
    );
    assert_eq!(keys(&native["optionalProperties"]), strings(&["message"]));
    assert!(native.get("definitions").is_none());
    assert_eq!(
        lifecycle["metadata"]["x-vocabularies"],
        serde_json::json!(["reply_status", "reply_artifact"])
    );
    assert_eq!(
        native["metadata"]["x-vocabularies"],
        lifecycle["metadata"]["x-vocabularies"]
    );

    let manifest = read_json("schemas/native/e1/manifest.jtd.json");
    assert_eq!(keys(&manifest["properties"]), strings(&["extensions"]));
    assert!(manifest.get("optionalProperties").is_none());
    let extension = &manifest["definitions"]["manifest_extension"];
    assert_eq!(keys(&extension["properties"]), strings(&["id", "point"]));
    assert_eq!(extension["properties"]["point"]["type"], "string");
    assert!(extension["properties"]["point"].get("enum").is_none());
    assert!(extension["properties"]["point"].get("ref").is_none());
    assert_eq!(
        keys(&extension["optionalProperties"]),
        strings(&["ir_schema"])
    );
    assert!(!keys(&manifest["properties"]).contains("abi"));
    assert!(!keys(&extension["properties"]).contains("abi"));
    assert!(!keys(&extension["optionalProperties"]).contains("abi"));
}

#[test]
fn generated_nested_types_are_the_one_shared_types() {
    for (left, right) in [
        (
            TypeId::of::<lifecycle_context::Execution>(),
            TypeId::of::<context::Execution>(),
        ),
        (
            TypeId::of::<lifecycle_context::Project>(),
            TypeId::of::<context::Project>(),
        ),
        (
            TypeId::of::<lifecycle_context::World>(),
            TypeId::of::<context::World>(),
        ),
        (
            TypeId::of::<lifecycle_context::WorldPackage>(),
            TypeId::of::<context::WorldPackage>(),
        ),
        (
            TypeId::of::<lifecycle_context::Run>(),
            TypeId::of::<context::Run>(),
        ),
        (
            TypeId::of::<lifecycle_context::RunAgentMode>(),
            TypeId::of::<context::RunAgentMode>(),
        ),
        (
            TypeId::of::<lifecycle_context::Artifact>(),
            TypeId::of::<context::Artifact>(),
        ),
        (
            TypeId::of::<lifecycle_context::SlotTarget>(),
            TypeId::of::<context::SlotTarget>(),
        ),
        (
            TypeId::of::<lifecycle_context::Io>(),
            TypeId::of::<context::Io>(),
        ),
        (
            TypeId::of::<lifecycle_reply::ReplyArtifact>(),
            TypeId::of::<reply::ReplyArtifact>(),
        ),
        (
            TypeId::of::<lifecycle_reply::ReplyStatus>(),
            TypeId::of::<reply::ReplyStatus>(),
        ),
    ] {
        assert_eq!(left, right);
    }
    assert_eq!(
        TypeId::of::<context::Execution>(),
        TypeId::of::<shared::Execution>()
    );
    assert_eq!(
        TypeId::of::<reply::ReplyArtifact>(),
        TypeId::of::<shared::ReplyArtifact>()
    );
    assert_ne!(
        TypeId::of::<lifecycle_context::Context>(),
        TypeId::of::<context::Context>()
    );
    assert_ne!(
        TypeId::of::<lifecycle_reply::Reply>(),
        TypeId::of::<reply::Reply>()
    );
}

#[test]
fn authored_native_corpus_has_valid_and_invalid_documents_for_each_root() {
    let context_valid = read_json("formats/corpora/native/e1/context.valid.json");
    let parsed: context::Context = serde_json::from_value(context_valid).unwrap();
    assert_eq!(parsed.point, "vendor:future-point");
    assert!(
        serde_json::from_value::<context::Context>(read_json(
            "formats/corpora/native/e1/context.invalid.json"
        ))
        .is_err()
    );

    let reply_valid = read_json("formats/corpora/native/e1/reply.valid.json");
    let parsed: reply::Reply = serde_json::from_value(reply_valid).unwrap();
    assert_eq!(parsed.artifacts.len(), 1);
    assert!(
        serde_json::from_value::<reply::Reply>(read_json(
            "formats/corpora/native/e1/reply.invalid.json"
        ))
        .is_err()
    );

    let manifest_valid = read_json("formats/corpora/native/e1/manifest.valid.json");
    let parsed: manifest::Manifest = serde_json::from_value(manifest_valid).unwrap();
    assert_eq!(parsed.extensions.len(), 2);
    assert!(
        serde_json::from_value::<manifest::Manifest>(read_json(
            "formats/corpora/native/e1/manifest.invalid.json"
        ))
        .is_err()
    );
}

#[test]
fn context_and_manifest_accept_forward_members_but_reply_refuses_them() {
    let context_doc = read_json("formats/corpora/native/e1/context.valid.json");
    assert!(context_doc.get("future_context_member").is_some());
    assert!(
        context_doc["execution"]
            .get("future_execution_member")
            .is_some()
    );
    serde_json::from_value::<context::Context>(context_doc)
        .expect("context root and shared nested records are permissive");

    let manifest_doc = read_json("formats/corpora/native/e1/manifest.valid.json");
    assert!(manifest_doc.get("future_manifest_member").is_some());
    assert!(
        manifest_doc["extensions"][1]
            .get("future_extension_member")
            .is_some()
    );
    serde_json::from_value::<manifest::Manifest>(manifest_doc)
        .expect("manifest root and extension rows are permissive");

    serde_json::from_value::<reply::Reply>(serde_json::json!({
        "envelope": 1,
        "status": "ok",
        "artifacts": [],
        "tasks": []
    }))
    .expect_err("native reply refuses an unknown root member");
    serde_json::from_value::<reply::Reply>(serde_json::json!({
        "envelope": 1,
        "status": "ok",
        "artifacts": [{
            "id": "out",
            "path": "out.bin",
            "kind": "file",
            "phase": "build"
        }]
    }))
    .expect_err("the unanimous-strict shared reply artifact refuses phase");
}

#[test]
fn open_points_round_trip_without_normalization() {
    let context: context::Context =
        serde_json::from_value(read_json("formats/corpora/native/e1/context.valid.json")).unwrap();
    let rendered = serde_json::to_value(&context).unwrap();
    assert_eq!(rendered["point"], "vendor:future-point");

    let manifest: manifest::Manifest =
        serde_json::from_value(read_json("formats/corpora/native/e1/manifest.valid.json")).unwrap();
    let rendered = serde_json::to_value(&manifest).unwrap();
    assert_eq!(rendered["extensions"][1]["point"], "future:compiler-point");
}

#[test]
fn context_and_reply_artifacts_are_distinct_exact_rows() {
    assert_ne!(
        TypeId::of::<shared::Artifact>(),
        TypeId::of::<shared::ReplyArtifact>()
    );

    let context_row = shared::Artifact {
        id: "input".to_string(),
        phase: "generate".to_string(),
        path: "generated/input.rs".to_string(),
        kind: "file".to_string(),
    };
    let reply_row = shared::ReplyArtifact {
        id: "output".to_string(),
        path: "target/output.bin".to_string(),
        kind: "file".to_string(),
    };
    assert_eq!(
        keys(&serde_json::to_value(context_row).unwrap()),
        strings(&["id", "phase", "path", "kind"])
    );
    assert_eq!(
        keys(&serde_json::to_value(reply_row).unwrap()),
        strings(&["id", "path", "kind"])
    );

    let vocabularies = read_json("formats/vocabularies.json");
    assert_eq!(
        keys(&vocabularies["artifact"]["properties"]),
        strings(&["id", "phase", "path", "kind"])
    );
    assert_eq!(
        keys(&vocabularies["reply_artifact"]["properties"]),
        strings(&["id", "path", "kind"])
    );
    assert_eq!(
        vocabularies["reply_status"]["enum"],
        serde_json::json!(["ok", "fail", "skip"])
    );
}
