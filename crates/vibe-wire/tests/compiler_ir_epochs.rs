//! Structural/transport epoch boundaries, NOT compiler-domain or invocation proof.
//! Historical corpus bytes are read only. Synthetic e2 values below test DTO
//! requirements; their digest/origin/placement truth belongs to compiler gates.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-EMBED-PROVENANCE-EPOCH");

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf};
use vibe_wire::behaviour::native_compile::{validate_exchange, validate_reply, validate_request};
use vibe_wire::generated::{
    native::{e1 as native1, e2 as native2},
    shared,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap()
}
fn historical(name: &str) -> Value {
    serde_json::from_str(&read(&format!(
        "formats/corpora/compiler_ir/e1/valid/{name}.json"
    )))
    .unwrap()
}
fn roundtrip<T: DeserializeOwned + Serialize>(value: &Value) {
    let typed: T = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), *value);
}
fn source2() -> Value {
    let mut value = historical("source_document");
    value["ir_schema"] = json!(2);
    value
}
fn request2(payload: Value) -> Value {
    let mut value: Value = serde_json::from_str(&read(
        "formats/corpora/native/e1/compile_request.valid.json",
    ))
    .unwrap();
    value["envelope"] = json!(2);
    value["payload"] = payload;
    value
}
fn duplicate(raw: &str, member: &str) -> String {
    assert!(
        raw.contains(member),
        "duplicate probe must hit an actual member"
    );
    raw.replacen(member, &format!("{member},{member}"), 1)
}

#[test]
fn historical_e1_corpus_stays_strict_and_structurally_lossless() {
    let path = root().join("formats/corpora/compiler_ir/e1/valid");
    let mut files: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut shapes = BTreeSet::new();
    assert!(!files.is_empty());
    for file in files {
        let bytes = std::fs::read(&file).unwrap();
        let typed: shared::Ir = serde_json::from_slice(&bytes).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            serde_json::to_value(&typed).unwrap(),
            value,
            "{}",
            file.display()
        );
        shapes.insert(value["shape"].as_str().unwrap().to_owned());
        let raw = String::from_utf8(bytes.clone()).unwrap();
        let unknown = raw.replacen('{', "{\"future_epoch_probe\":true,", 1);
        assert!(serde_json::from_str::<shared::Ir>(&unknown).is_err());
        assert_eq!(
            std::fs::read(file).unwrap(),
            bytes,
            "inspection never writes historical corpus"
        );
    }
    assert_eq!(
        shapes,
        [
            "source-document",
            "document-document",
            "documents-artifact",
            "closure-artifact",
            "lane-artifact",
            "emitted-artifact"
        ]
        .map(str::to_owned)
        .into()
    );
    // Repeated BTreeMap keys are checked by vibe-spec's strict byte reader
    // before DTO decoding (compiler/wire/json.rs), not by generated serde here.
    // Keep raw known-struct-member duplicate refusal on this actual DTO owner.
    for file in [
        "unknown_shape",
        "unknown_field",
        "level_mismatch",
        "cardinality_mismatch",
        "duplicate_key_struct_member",
    ] {
        let raw = read(&format!(
            "formats/corpora/compiler_ir/e1/invalid/{file}.json"
        ));
        assert!(
            serde_json::from_str::<shared::Ir>(&raw).is_err(),
            "raw historical structural refusal: {file}"
        );
    }
}

fn required_tape<T: DeserializeOwned + Serialize>(mut value: Value, nullable: bool) {
    assert!(
        serde_json::from_value::<T>(value.clone()).is_err(),
        "missing required tape is not historical null"
    );
    value["embeds"] = Value::Null;
    if nullable {
        roundtrip::<T>(&value);
    } else {
        assert!(
            serde_json::from_value::<T>(value.clone()).is_err(),
            "ready tape cannot be null"
        );
    }
    value["embeds"] =
        json!({"body_digest":"structural-placeholder-not-a-validated-digest","occurrences":[]});
    roundtrip::<T>(&value);
    for missing in ["body_digest", "occurrences"] {
        let mut bad = value.clone();
        bad["embeds"].as_object_mut().unwrap().remove(missing);
        assert!(
            serde_json::from_value::<T>(bad).is_err(),
            "required tape member: {missing}"
        );
    }
    let mut bad = value.clone();
    bad["embeds"]["future"] = json!(true);
    assert!(serde_json::from_value::<T>(bad).is_err());
    let raw = serde_json::to_string(&value).unwrap();
    assert!(serde_json::from_str::<T>(&duplicate(&raw, "\"occurrences\":[]")).is_err());
}

#[test]
fn e2_closure_link_and_lane_require_explicit_tape_presence() {
    let closure = historical("closure_artifact");
    let doc = closure["closure"]["nodes"][0].clone();
    roundtrip::<shared::ClosureDocument>(&doc);
    let mut extra = doc.clone();
    extra["embeds"] = Value::Null;
    assert!(
        serde_json::from_value::<shared::ClosureDocument>(extra).is_err(),
        "e1 is not widened for e2 fields"
    );
    required_tape::<shared::ClosureDocumentV2>(doc, true);
    required_tape::<shared::LinkOccurrenceV2>(
        closure["closure"]["link"]["result"]["occurrences"][0].clone(),
        false,
    );
    let lane = historical("lane_artifact");
    let node = lane["lane"]["contributions"][0]["chunks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "node")
        .unwrap()["node"]
        .clone();
    required_tape::<shared::LaneNodeV2>(node, false);
}

#[test]
fn typed_e2_occurrence_members_are_closed_but_not_domain_validated_here() {
    let target = historical("source_document")["doc"]["address"]["address"].clone();
    let value = json!({"path":[0,1],"target":target,"source":"source locator","directive_line":3,"start":1,"end":4,"intrinsic_empty":false,"owner":{"kind":"preamble","start":0,"end":5},"suppressed":false});
    roundtrip::<shared::EmbedOccurrence>(&value);
    for field in value.as_object().unwrap().keys() {
        let mut bad = value.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<shared::EmbedOccurrence>(bad).is_err(),
            "missing occurrence field {field}"
        );
    }
    let mut bad = value.clone();
    bad["owner"]["forged"] = json!(1);
    assert!(serde_json::from_value::<shared::EmbedOccurrence>(bad).is_err());
    let mut reversed = value;
    reversed["start"] = json!(99);
    assert!(
        serde_json::from_value::<shared::EmbedOccurrence>(reversed).is_ok(),
        "range/origin truth is a compiler-domain gate, not u32 DTO decoding"
    );
}

#[test]
fn current_native2_exchange_admits_ir2_and_refuses_old_epochs_behaviorally() {
    let payload = source2();
    roundtrip::<shared::IrV2>(&payload);
    roundtrip::<vibe_wire::generated::compiler_ir::e2::ir::Ir>(&payload);
    roundtrip::<vibe_wire::generated::compiler_ir::e1::ir::Ir>(&historical("source_document"));
    let request: native2::compile_request::CompileRequest =
        serde_json::from_value(request2(payload.clone())).unwrap();
    let reply_value = json!({"status":"ok","envelope":2,"payload":payload});
    let reply: native2::compile_reply::CompileReply =
        serde_json::from_value(reply_value.clone()).unwrap();
    validate_exchange(&request, &reply).unwrap();
    for epoch in [1, 3] {
        let mut bad_request = serde_json::to_value(&request).unwrap();
        bad_request["envelope"] = json!(epoch);
        let decoded = serde_json::from_value(bad_request).unwrap();
        assert!(validate_request(&decoded).is_err());
        let mut bad_reply = reply_value.clone();
        bad_reply["envelope"] = json!(epoch);
        assert!(validate_reply(&serde_json::from_value(bad_reply).unwrap()).is_err());
    }
    let old_ir = historical("source_document");
    // Numeric epoch admission is behavioral: source carrier shape is shared.
    let decoded: native2::compile_request::CompileRequest =
        serde_json::from_value(request2(old_ir.clone())).unwrap();
    assert!(validate_request(&decoded).is_err());
    let old_reply: native2::compile_reply::CompileReply =
        serde_json::from_value(json!({"status":"ok","envelope":2,"payload":old_ir})).unwrap();
    assert!(validate_reply(&old_reply).is_err());
    let retained: native1::compile_request::CompileRequest = serde_json::from_str(&read(
        "formats/corpora/native/e1/compile_request.valid.json",
    ))
    .unwrap();
    assert_eq!(retained.envelope, 1);
    assert!(
        serde_json::from_str::<native1::compile_reply::CompileReply>(&read(
            "formats/corpora/native/e1/compile_reply.valid.json"
        ))
        .is_ok()
    );
}

#[test]
fn foreign_request_is_permissive_but_canonical_ir_and_host_reply_are_strict() {
    let mut extended = source2();
    extended["future"] = json!(true);
    extended["doc"]["future"] = json!(true);
    let raw = serde_json::to_string(&request2(extended.clone())).unwrap();
    let request: native2::compile_request::CompileRequest = serde_json::from_str(&raw).unwrap();
    validate_request(&request).unwrap();
    assert_eq!(
        serde_json::to_value(request.payload).unwrap(),
        source2(),
        "foreign projection discards unknown members only"
    );
    assert!(serde_json::from_value::<shared::IrV2>(extended.clone()).is_err());
    assert!(
        serde_json::from_value::<native2::compile_reply::CompileReply>(
            json!({"status":"ok","envelope":2,"payload":extended})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<native2::compile_reply::CompileReply>(
            json!({"status":"skip","envelope":2,"future":true})
        )
        .is_err()
    );
    for member in [
        "\"envelope\":2",
        "\"ir_schema\":2",
        "\"format\":\"markdown\"",
    ] {
        assert!(
            serde_json::from_str::<native2::compile_request::CompileRequest>(&duplicate(
                &raw, member
            ))
            .is_err(),
            "known duplicates survive projection for refusal"
        );
    }
    let reply =
        serde_json::to_string(&json!({"status":"ok","envelope":2,"payload":source2()})).unwrap();
    assert!(
        serde_json::from_str::<native2::compile_reply::CompileReply>(&duplicate(
            &reply,
            "\"ir_schema\":2"
        ))
        .is_err()
    );
}

#[test]
fn generated_roots_and_registry_keep_historical_and_current_roles_distinct() {
    for (epoch, reference) in [(1, "ir"), (2, "ir_v2")] {
        let schema: Value =
            serde_json::from_str(&read(&format!("schemas/compiler_ir/e{epoch}/ir.jtd.json")))
                .unwrap();
        assert_eq!(schema["ref"], reference);
        let request: Value = serde_json::from_str(&read(&format!(
            "schemas/native/e{epoch}/compile_request.jtd.json"
        )))
        .unwrap();
        assert_eq!(request["properties"]["payload"]["ref"], reference);
        assert_eq!(
            request["properties"]["payload"]["metadata"]["x-reader-projection"],
            "permissive"
        );
        let reply: Value = serde_json::from_str(&read(&format!(
            "schemas/native/e{epoch}/compile_reply.jtd.json"
        )))
        .unwrap();
        assert_eq!(
            reply["mapping"]["ok"]["properties"]["payload"]["ref"],
            reference
        );
    }
    let registry: toml::Value = toml::from_str(&read("formats/REGISTRY.toml")).unwrap();
    assert_eq!(
        registry["format"]["compiler-ir"]["epoch"].as_integer(),
        Some(2)
    );
    assert_eq!(
        registry["format"]["compiler-ir-v1"]["epoch"].as_integer(),
        Some(1)
    );
    assert_eq!(
        registry["format"]["compiler-ir"]["foreign_parsers"].as_str(),
        Some("none")
    );
    assert_eq!(
        registry["format"]["compiler-ir-v1"]["foreign_parsers"].as_str(),
        Some("none")
    );
    for (suffix, epoch) in [("", 2), ("-v1", 1)] {
        for (side, readers) in [("request", "many"), ("reply", "none")] {
            let key = format!("native-compile-{side}{suffix}");
            assert_eq!(registry["format"][&key]["epoch"].as_integer(), Some(epoch));
            assert_eq!(
                registry["format"][&key]["foreign_parsers"].as_str(),
                Some(readers)
            );
        }
    }
}
