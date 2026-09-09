//! Strict generated-wire coverage for the engine-owned deploy restart roots.

use std::path::PathBuf;

use vibe_wire::generated::deploy_checkpoints::DeployCheckpoints;
use vibe_wire::generated::deploy_inverse::DeployInverse;
use vibe_wire::generated::deploy_lock_resources::DeployLockResources;
use vibe_wire::generated::format_id::{ForeignParsers, FormatId};

fn decode_lock_resources(bytes: &[u8]) -> serde_json::Result<DeployLockResources> {
    serde_json::from_slice(bytes)
}

fn encode_lock_resources(value: &DeployLockResources) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec_pretty(value)
}

fn decode_checkpoints(bytes: &[u8]) -> serde_json::Result<DeployCheckpoints> {
    serde_json::from_slice(bytes)
}

fn encode_checkpoints(value: &DeployCheckpoints) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec_pretty(value)
}

fn decode_inverse(bytes: &[u8]) -> serde_json::Result<DeployInverse> {
    serde_json::from_slice(bytes)
}

fn encode_inverse(value: &DeployInverse) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec_pretty(value)
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus(name: &str) -> Vec<u8> {
    std::fs::read(root().join("formats/corpora/deploy/e1").join(name)).unwrap()
}

#[test]
fn every_valid_restart_root_round_trips_through_its_generated_type() {
    for name in ["lock_resources_minimal.json", "lock_resources_full.json"] {
        let value = decode_lock_resources(&corpus(&format!("valid/{name}"))).unwrap();
        let encoded = encode_lock_resources(&value).unwrap();
        assert_eq!(decode_lock_resources(&encoded).unwrap(), value, "{name}");
    }
    let checkpoints = decode_checkpoints(&corpus("valid/checkpoints.json")).unwrap();
    assert_eq!(
        decode_checkpoints(&encode_checkpoints(&checkpoints).unwrap()).unwrap(),
        checkpoints
    );
    let inverse = decode_inverse(&corpus("valid/inverse.json")).unwrap();
    assert_eq!(
        decode_inverse(&encode_inverse(&inverse).unwrap()).unwrap(),
        inverse
    );
}

#[test]
fn each_strict_reader_refuses_and_names_an_unknown_member() {
    let error =
        decode_lock_resources(&corpus("invalid/lock_resources_unknown_field.json")).unwrap_err();
    assert!(error.to_string().contains("unknown field"), "{error}");
    assert!(error.to_string().contains("repair"), "{error}");
    let error = decode_checkpoints(&corpus("invalid/checkpoints_unknown_field.json")).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
    assert!(error.to_string().contains("retry"));
    let error = decode_inverse(&corpus("invalid/inverse_unknown_field.json")).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
    assert!(error.to_string().contains("force"));
}

#[test]
fn lock_reader_retains_the_preexisting_u64_byte_count() {
    let value = decode_lock_resources(&corpus("valid/lock_resources_full.json")).unwrap();
    assert_eq!(
        value.pending.unwrap().native.unwrap().image_bytes,
        4_294_967_297
    );
    let fractional = br#"{"schema":1,"pending":{"generation":3,"plan_hash":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","resources":[],"native":{"target":"tool","mechanism":"deploy:vibe-bin","pin":"org.example/providers#native","descriptor_id":"native","protocol":1,"provider_version":"1","provider_root":"slots/provider","platform":"windows-x86_64","record_root":"project","origin":"prebuilt","image":"image.dll","image_digest":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","image_bytes":1.5}}}"#;
    assert!(decode_lock_resources(fractional).is_err());
    for name in [
        "lock_resources_image_bytes_negative.json",
        "lock_resources_image_bytes_overflow.json",
    ] {
        let error = decode_lock_resources(&corpus(&format!("invalid/{name}")))
            .expect_err("negative and greater-than-u64 counts must refuse");
        assert!(
            error.to_string().contains("expected u64"),
            "{name}: {error}"
        );
    }
}

#[test]
fn canonical_writer_keeps_the_durable_epoch_one_field_order() {
    assert_eq!(
        encode_lock_resources(&DeployLockResources {
            schema: 1,
            committed: None,
            pending: None,
        })
        .unwrap(),
        b"{\n  \"schema\": 1\n}"
    );
    assert_eq!(
        encode_checkpoints(&DeployCheckpoints {
            schema: 1,
            plan_hash: "a".repeat(64),
            completed: vec!["bin/tool".to_owned()],
        })
        .unwrap(),
        b"{\n  \"schema\": 1,\n  \"plan_hash\": \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\n  \"completed\": [\n    \"bin/tool\"\n  ]\n}"
    );
    let inverse = DeployInverse {
        schema: 1,
        generation: 2,
        provider_pin: "org.example/providers#native".to_owned(),
        resource: "bin/tool".to_owned(),
        receipt_post_digest: "b".repeat(64),
        prior_state_handle: "rollback/one".to_owned(),
    };
    let encoded = String::from_utf8(encode_inverse(&inverse).unwrap()).unwrap();
    assert!(encoded.find("\"schema\"").unwrap() < encoded.find("\"generation\"").unwrap());
    assert!(encoded.find("\"generation\"").unwrap() < encoded.find("\"provider_pin\"").unwrap());
    assert!(
        encoded.find("\"receipt_post_digest\"").unwrap()
            < encoded.find("\"prior_state_handle\"").unwrap()
    );
}

#[test]
fn registry_pins_all_three_restart_roots_as_strict_nonrecoverable_facts() {
    for id in [
        "deploy-lock-resources",
        "deploy-checkpoints",
        "deploy-inverse",
    ] {
        let format = FormatId::ALL
            .iter()
            .copied()
            .find(|format| format.id() == id)
            .unwrap();
        assert_eq!(format.epoch(), 1);
        assert!(!format.recoverable());
        assert_eq!(format.foreign_parsers(), ForeignParsers::None);
    }
}
