//! Epoch-1 compiler-native wire roots and transport-neutral admission.

use std::any::TypeId;
use std::collections::BTreeSet;
use std::path::PathBuf;

use vibe_wire::behaviour::native_compile::{
    IrCardinality, IrCarrier, IrLevel, IrShape, NativeCompileError, validate_exchange,
    validate_reply, validate_request,
};
use vibe_wire::generated::format_id::{ForeignParsers, FormatId};
use vibe_wire::generated::native::e1::{compile_reply, compile_request};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn json(relative: &str) -> serde_json::Value {
    serde_json::from_str(&read(relative))
        .unwrap_or_else(|error| panic!("{relative} parses as JSON: {error}"))
}

fn keys(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("selected value is an object")
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
    registry
        .split_once(&marker)
        .unwrap_or_else(|| panic!("registry contains {marker}"))
        .1
        .split("\n[format.")
        .next()
        .unwrap()
        .to_string()
}

fn payload(name: &str) -> serde_json::Value {
    json(&format!("formats/corpora/compiler_ir/e1/valid/{name}.json"))
}

fn request(point: &str, payload: serde_json::Value) -> compile_request::CompileRequest {
    let mut document = json("formats/corpora/native/e1/compile_request.valid.json");
    document["point"] = point.into();
    document["payload"] = payload;
    serde_json::from_value(document).expect("request structurally decodes")
}

fn ok(payload: serde_json::Value) -> compile_reply::CompileReply {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "envelope": 1,
        "payload": payload,
    }))
    .expect("ok reply structurally decodes")
}

#[test]
fn registry_and_schemas_pin_the_two_asymmetric_roots() {
    for (format, id, role, schema) in [
        (
            FormatId::NativeCompileRequest,
            "native-compile-request",
            ForeignParsers::Many,
            "schemas/native/e1/compile_request.jtd.json",
        ),
        (
            FormatId::NativeCompileReply,
            "native-compile-reply",
            ForeignParsers::None,
            "schemas/native/e1/compile_reply.jtd.json",
        ),
    ] {
        assert_eq!(format.id(), id);
        assert_eq!(format.epoch(), 1);
        assert!(format.recoverable());
        assert_eq!(format.foreign_parsers(), role);
        let block = registry_block(id);
        assert!(block.contains(&format!("schema = \"{schema}\"")));
        assert!(block.contains("corpus = \"formats/corpora/native/e1\""));
    }

    let request = json("schemas/native/e1/compile_request.jtd.json");
    assert_eq!(
        keys(&request["properties"]),
        strings(&[
            "envelope",
            "point",
            "execution",
            "project",
            "world",
            "io",
            "payload",
        ])
    );
    assert_eq!(
        request["metadata"]["x-vocabularies"],
        serde_json::json!(["execution", "project", "world", "io", "ir"])
    );
    assert_eq!(request["properties"]["payload"]["ref"], "ir");
    assert_eq!(
        request["properties"]["payload"]["metadata"]["x-reader-projection"],
        "permissive"
    );
    assert!(request.get("optionalProperties").is_none());
    assert!(request.get("definitions").is_none());
    for forbidden in ["run", "artifacts", "slot_target", "tasks"] {
        assert!(!keys(&request["properties"]).contains(forbidden));
    }

    let reply = json("schemas/native/e1/compile_reply.jtd.json");
    assert_eq!(reply["discriminator"], "status");
    assert_eq!(keys(&reply["mapping"]), strings(&["ok", "skip", "fail"]));
    assert_eq!(
        reply["metadata"]["x-vocabularies"],
        serde_json::json!(["ir"])
    );
    assert_eq!(
        keys(&reply["mapping"]["ok"]["properties"]),
        strings(&["envelope", "payload"])
    );
    assert_eq!(reply["mapping"]["ok"]["properties"]["payload"]["ref"], "ir");
    assert!(
        reply["mapping"]["ok"]["properties"]["payload"]
            .get("metadata")
            .is_none(),
        "the strict reply payload is never projected"
    );
    for status in ["skip", "fail"] {
        assert_eq!(
            keys(&reply["mapping"][status]["properties"]),
            strings(&["envelope"])
        );
        assert!(
            reply["mapping"][status]["properties"]
                .get("payload")
                .is_none()
        );
    }
}

#[test]
fn request_projection_accepts_forward_members_and_refuses_duplicate_known_keys() {
    let raw = read("formats/corpora/native/e1/compile_request.valid.json");
    let parsed: compile_request::CompileRequest = serde_json::from_str(&raw)
        .expect("forward root and recursively nested members are ignored");
    assert_eq!(parsed.point, "compile:source");
    assert_eq!(
        validate_request(&parsed).unwrap().carrier,
        IrCarrier::SourceDocument
    );

    let duplicate = read("formats/corpora/native/e1/compile_request.invalid.json");
    serde_json::from_str::<compile_request::CompileRequest>(&duplicate)
        .expect_err("the projected adapter refuses a duplicate known payload member");

    let nested_duplicate = raw.replace(
        "\"declared_path\": \"boot/10-guide.md\"",
        "\"declared_path\": \"boot/10-guide.md\",\n        \"declared_path\": \"boot/10-guide.md\"",
    );
    assert_ne!(nested_duplicate, raw);
    serde_json::from_str::<compile_request::CompileRequest>(&nested_duplicate)
        .expect_err("the projected adapter refuses duplicates recursively");
}

#[test]
fn all_landed_stage_carriers_and_pass_are_admitted_without_rewrite() {
    let cases = [
        (
            "compile:source",
            "source_document",
            IrShape {
                carrier: IrCarrier::SourceDocument,
                level: IrLevel::Source,
                cardinality: IrCardinality::Document,
            },
        ),
        (
            "compile:document",
            "document_document",
            IrShape {
                carrier: IrCarrier::DocumentDocument,
                level: IrLevel::Document,
                cardinality: IrCardinality::Document,
            },
        ),
        (
            "compile:lane",
            "lane_artifact",
            IrShape {
                carrier: IrCarrier::LaneArtifact,
                level: IrLevel::Lane,
                cardinality: IrCardinality::Artifact,
            },
        ),
        (
            "compile:emitted",
            "emitted_artifact",
            IrShape {
                carrier: IrCarrier::EmittedArtifact,
                level: IrLevel::Emitted,
                cardinality: IrCardinality::Artifact,
            },
        ),
    ];
    for (point, file, expected) in cases {
        let admitted = request(point, payload(file));
        assert_eq!(validate_request(&admitted), Ok(expected));
        assert_eq!(
            admitted.point, point,
            "admission never normalizes the point"
        );

        let pass = request("compile:pass", payload(file));
        assert_eq!(validate_request(&pass), Ok(expected));
    }
    let batch = request(
        "compile:pass",
        serde_json::json!({
            "shape": "documents-artifact",
            "ir_schema": 1,
            "level": "document",
            "cardinality": "artifact",
            "documents": [],
        }),
    );
    assert_eq!(
        validate_request(&batch).unwrap().carrier,
        IrCarrier::DocumentsArtifact
    );
}

#[test]
fn request_structural_and_behavior_invalid_matrix_is_red() {
    let base = json("formats/corpora/native/e1/compile_request.valid.json");

    let mut missing = base.clone();
    missing.as_object_mut().unwrap().remove("payload");
    assert!(serde_json::from_value::<compile_request::CompileRequest>(missing).is_err());

    let mut wrong_type = base.clone();
    wrong_type["envelope"] = "1".into();
    assert!(serde_json::from_value::<compile_request::CompileRequest>(wrong_type).is_err());

    let mut unknown_tag = base.clone();
    unknown_tag["payload"]["shape"] = "future-carrier".into();
    assert!(serde_json::from_value::<compile_request::CompileRequest>(unknown_tag).is_err());

    let mut closed_enum = base.clone();
    closed_enum["payload"]["level"] = "future-level".into();
    assert!(serde_json::from_value::<compile_request::CompileRequest>(closed_enum).is_err());

    let mismatch = request("compile:lane", payload("source_document"));
    assert!(matches!(
        validate_request(&mismatch),
        Err(NativeCompileError::StageCarrier {
            point: _,
            carrier: IrCarrier::SourceDocument
        })
    ));

    let mut wrong_envelope = request("compile:source", payload("source_document"));
    wrong_envelope.envelope = 2;
    assert!(matches!(
        validate_request(&wrong_envelope),
        Err(NativeCompileError::Envelope { found: 2, .. })
    ));

    let mut wrong_schema = payload("source_document");
    wrong_schema["ir_schema"] = 2.into();
    assert!(matches!(
        validate_request(&request("compile:source", wrong_schema)),
        Err(NativeCompileError::IrSchema { found: 2, .. })
    ));

    let point = format!("vendor:{}", "é".repeat(200));
    let unknown = request(&point, payload("source_document"));
    let error = validate_request(&unknown).unwrap_err();
    let NativeCompileError::UnsupportedPoint { point: bounded } = &error else {
        panic!("unknown point returns its typed refusal: {error:?}");
    };
    assert!(bounded.truncated());
    assert!(bounded.as_str().len() <= 128);
    assert_eq!(unknown.point, point, "the decoded point is never rewritten");
    assert!(error.to_string().len() < 220);
    assert!(!error.to_string().contains("payload"));
}

#[test]
fn unsupported_point_diagnostics_are_control_safe_and_multibyte_bounded() {
    let injected = "vendor:\r\n\0\u{001b}\u{0085}\u{009f}safe";
    let injected_request = request(injected, payload("source_document"));
    let error = validate_request(&injected_request).unwrap_err();
    let NativeCompileError::UnsupportedPoint { point } = &error else {
        panic!("control-bearing point returns its typed refusal: {error:?}");
    };
    assert_eq!(
        injected_request.point, injected,
        "decoded request bytes stay untouched"
    );
    assert!(!point.truncated());
    assert_eq!(point.as_str().matches('\u{fffd}').count(), 6);
    assert!(
        point
            .as_str()
            .chars()
            .all(|character| !character.is_control())
    );
    assert!(
        error
            .to_string()
            .chars()
            .all(|character| !character.is_control())
    );

    let multibyte = format!("vendor:{}", "😀".repeat(80));
    let multibyte_request = request(&multibyte, payload("source_document"));
    let error = validate_request(&multibyte_request).unwrap_err();
    let NativeCompileError::UnsupportedPoint { point } = error else {
        panic!("multibyte point returns its typed refusal");
    };
    assert_eq!(multibyte_request.point, multibyte);
    assert!(point.truncated());
    assert!(point.as_str().len() <= 128);
    assert!(point.as_str().is_char_boundary(point.as_str().len()));
    assert!(point.as_str().ends_with('😀'));
}

#[test]
fn reply_discriminator_is_strict_and_behavior_gates_epochs() {
    let valid: compile_reply::CompileReply =
        serde_json::from_value(json("formats/corpora/native/e1/compile_reply.valid.json")).unwrap();
    assert_eq!(
        validate_reply(&valid).unwrap().unwrap().carrier,
        IrCarrier::SourceDocument
    );
    assert!(
        serde_json::from_value::<compile_reply::CompileReply>(json(
            "formats/corpora/native/e1/compile_reply.invalid.json"
        ))
        .is_err()
    );

    for status in ["skip", "fail"] {
        let reply: compile_reply::CompileReply = serde_json::from_value(serde_json::json!({
            "status": status,
            "envelope": 1,
            "message": "bounded status-only result",
        }))
        .unwrap();
        assert_eq!(validate_reply(&reply), Ok(None));

        assert!(
            serde_json::from_value::<compile_reply::CompileReply>(serde_json::json!({
                "status": status,
                "envelope": 1,
                "payload": payload("source_document"),
            }))
            .is_err(),
            "{status} cannot carry a payload"
        );
    }
    assert!(
        serde_json::from_value::<compile_reply::CompileReply>(serde_json::json!({
            "status": "ok",
            "envelope": 1,
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<compile_reply::CompileReply>(serde_json::json!({
            "status": "future",
            "envelope": 1,
        }))
        .is_err()
    );

    let mut unknown_root = json("formats/corpora/native/e1/compile_reply.valid.json");
    unknown_root["future"] = true.into();
    assert!(serde_json::from_value::<compile_reply::CompileReply>(unknown_root).is_err());
    let mut unknown_nested = json("formats/corpora/native/e1/compile_reply.valid.json");
    unknown_nested["payload"]["doc"]["future"] = true.into();
    assert!(serde_json::from_value::<compile_reply::CompileReply>(unknown_nested).is_err());

    let mut wrong_envelope: compile_reply::CompileReply = ok(payload("source_document"));
    let compile_reply::CompileReply::Ok(value) = &mut wrong_envelope else {
        unreachable!()
    };
    value.envelope = 2;
    assert!(matches!(
        validate_reply(&wrong_envelope),
        Err(NativeCompileError::Envelope { found: 2, .. })
    ));

    let mut wrong_schema = payload("source_document");
    wrong_schema["ir_schema"] = 2.into();
    assert!(matches!(
        validate_reply(&ok(wrong_schema)),
        Err(NativeCompileError::IrSchema { found: 2, .. })
    ));
}

#[test]
fn exchange_requires_exact_shape_preservation() {
    let request = request("compile:source", payload("source_document"));
    let matching = ok(payload("source_document"));
    assert_eq!(validate_exchange(&request, &matching), Ok(()));

    let mismatched = ok(payload("document_document"));
    assert!(matches!(
        validate_exchange(&request, &mismatched),
        Err(NativeCompileError::ExchangeShape {
            request: IrShape {
                carrier: IrCarrier::SourceDocument,
                ..
            },
            reply: IrShape {
                carrier: IrCarrier::DocumentDocument,
                ..
            },
        })
    ));

    for status in ["skip", "fail"] {
        let reply: compile_reply::CompileReply = serde_json::from_value(serde_json::json!({
            "status": status,
            "envelope": 1,
        }))
        .unwrap();
        assert_eq!(validate_exchange(&request, &reply), Ok(()));
    }
}

#[test]
fn both_roots_reexport_the_one_canonical_compiler_family() {
    type Legacy = vibe_wire::generated::compiler_ir::e1::ir::Ir;
    type Shared = vibe_wire::generated::shared::Ir;
    assert_eq!(TypeId::of::<Legacy>(), TypeId::of::<Shared>());
    assert_eq!(TypeId::of::<compile_request::Ir>(), TypeId::of::<Shared>());
    assert_eq!(TypeId::of::<compile_reply::Ir>(), TypeId::of::<Shared>());

    let legacy = read("crates/vibe-wire/src/generated/compiler_ir/e1/ir/mod.rs");
    let compiler_names: BTreeSet<&str> = legacy
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub use crate::generated::shared::")
                .and_then(|tail| tail.strip_suffix(';'))
        })
        .collect();
    for module in [
        "crates/vibe-wire/src/generated/native/e1/compile_request/mod.rs",
        "crates/vibe-wire/src/generated/native/e1/compile_reply/mod.rs",
    ] {
        let source = read(module);
        for line in source.lines().map(str::trim) {
            let declaration = ["pub struct ", "pub enum ", "pub type "]
                .into_iter()
                .find_map(|prefix| line.strip_prefix(prefix))
                .and_then(|tail| tail.split([' ', '{', '=']).next());
            if let Some(name) = declaration {
                assert!(
                    !compiler_names.contains(name),
                    "{module} duplicates compiler declaration {name}"
                );
            }
        }
    }
}

#[test]
fn projection_lint_allowance_is_local_to_the_generated_request_helper() {
    let marker = "#[allow(clippy::collapsible_if, unused_variables)]";
    let request = read("crates/vibe-wire/src/generated/native/e1/compile_request/mod.rs");
    assert_eq!(request.matches(marker).count(), 1);
    assert!(request.contains(&format!("{marker}\nmod __reader_projection")));
    for module in [
        "crates/vibe-wire/src/generated/native/e1/compile_reply/mod.rs",
        "crates/vibe-wire/src/generated/native/e1/context/mod.rs",
        "crates/vibe-wire/src/generated/shared/mod.rs",
    ] {
        assert!(!read(module).contains(marker), "unrelated module {module}");
    }
    assert!(!read("crates/vibe-wire/src/lib.rs").contains(marker));
}

#[test]
fn authored_compile_corpus_has_one_valid_and_invalid_document_per_root() {
    let dir = root().join("formats/corpora/native/e1");
    let names: BTreeSet<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("compile_"))
        .collect();
    assert_eq!(
        names,
        strings(&[
            "compile_request.valid.json",
            "compile_request.invalid.json",
            "compile_reply.valid.json",
            "compile_reply.invalid.json",
        ])
    );
}
