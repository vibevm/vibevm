use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Value, json};
use vibe_wire::behaviour::native_deploy::{
    DIAGNOSTIC_CAP_BYTES, DeployOperation, NativeDeployError, RELATIONAL_LAWS, validate_exchange,
    validate_manifest, validate_reply, validate_request,
};
use vibe_wire::generated::format_id::{ForeignParsers, FormatId};
use vibe_wire::generated::native::e1::{deploy_reply, deploy_request, mechanism_manifest};

const OPERATIONS: [&str; 6] = [
    "plan",
    "fingerprint",
    "apply",
    "verify",
    "remove",
    "recover",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema(name: &str) -> Value {
    let path = root()
        .join("schemas/native/e1")
        .join(format!("{name}.jtd.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(root().join(relative)).unwrap()
}

fn strings(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("schema node is an object")
        .keys()
        .cloned()
        .collect()
}

fn identity() -> Value {
    json!({
        "provider": "org.example/tools",
        "mechanism": "deploy-test",
        "target": "tool",
        "profile": "default"
    })
}

fn artifact() -> Value {
    json!({
        "id": "tool",
        "kind": "executable",
        "shape": "file",
        "digest": "sha256:artifact",
        "path_absolute": "C:/stage/tool.exe",
        "path_relative": "bin/tool.exe"
    })
}

fn plan() -> Value {
    json!({
        "summary": "install tool",
        "resources": [{"resource": "bin/tool.exe", "desired_digest": "sha256:tool"}],
        "lock_resources": ["bin/tool.exe"],
        "config_digest": "sha256:config",
        "reversible": true
    })
}

fn authority() -> Value {
    json!({
        "project_root": "C:/project",
        "settings_root": "C:/settings",
        "user_home": "C:/Users/example",
        "clients": {
            "claude": {"state": "missing"},
            "codex": {"state": "resolved", "command": "codex", "path": "C:/bin/codex.exe"},
            "opencode": {"state": "missing"}
        }
    })
}

fn manifest_value() -> Value {
    json!({
        "mechanisms": [{
            "id": "deploy-test",
            "role": "deploy",
            "name": "Test deploy provider",
            "protocol": 1,
            "operations": OPERATIONS,
            "artifact_kinds": ["executable", "directory"],
            "effect": "user",
            "network": "never",
            "privilege": "none",
            "reversibility": "reversible",
            "atomic_replacement": true,
            "reference_ownership": true
        }]
    })
}

fn request(operation: &str) -> Value {
    let mut value = match operation {
        "plan" => json!({"artifact": artifact(), "authority": authority()}),
        "fingerprint" => json!({"artifact": artifact(), "plan": plan()}),
        "apply" => json!({"artifact": artifact(), "plan": plan()}),
        "verify" => {
            json!({"resources": [{"resource": "bin/tool.exe", "desired_digest": "sha256:tool"}]})
        }
        "remove" => json!({"resources": ["bin/tool.exe"]}),
        "recover" => json!({
            "artifact": artifact(),
            "plan": plan(),
            "observed": [{"resource": "bin/tool.exe", "digest": "sha256:old"}]
        }),
        _ => unreachable!(),
    };
    let object = value.as_object_mut().unwrap();
    object.insert("operation".into(), operation.into());
    object.insert("envelope".into(), 1.into());
    object.insert("protocol".into(), 1.into());
    object.insert("identity".into(), identity());
    value
}

fn reply(operation: &str) -> Value {
    let result = match operation {
        "plan" => {
            let mut value = plan();
            value
                .as_object_mut()
                .unwrap()
                .insert("status".into(), "ok".into());
            value
        }
        "fingerprint" => {
            json!({"status": "ok", "digest": "sha256:fingerprint", "summary": "stable"})
        }
        "apply" | "recover" => json!({
            "status": "ok",
            "completed": ["bin/tool.exe"],
            "evidence": "sha256:applied",
            "prior_state_handle": "opaque:1"
        }),
        "verify" => json!({
            "status": "ok",
            "observed": [{"resource": "bin/tool.exe", "digest": "sha256:tool"}]
        }),
        "remove" => json!({
            "status": "ok",
            "removed": ["bin/tool.exe"],
            "expected_remaining": [],
            "evidence": "sha256:removed"
        }),
        _ => unreachable!(),
    };
    json!({"operation": operation, "envelope": 1, "protocol": 1, "result": result})
}

#[test]
fn registry_pins_the_reader_asymmetry() {
    for (format, id, role) in [
        (
            FormatId::NativeMechanismManifest,
            "native-mechanism-manifest",
            ForeignParsers::Many,
        ),
        (
            FormatId::NativeDeployRequest,
            "native-deploy-request",
            ForeignParsers::Many,
        ),
        (
            FormatId::NativeDeployReply,
            "native-deploy-reply",
            ForeignParsers::None,
        ),
    ] {
        assert_eq!(format.id(), id);
        assert_eq!(format.epoch(), 1);
        assert!(format.recoverable());
        assert_eq!(format.foreign_parsers(), role);
    }
}

#[test]
fn manifest_has_exact_admission_fields_and_permissive_objects() {
    let root = schema("mechanism_manifest");
    let descriptor = &root["definitions"]["mechanism_descriptor"];
    assert_eq!(
        keys(&descriptor["properties"]),
        strings(&[
            "id",
            "role",
            "name",
            "protocol",
            "operations",
            "artifact_kinds",
            "effect",
            "network",
            "privilege",
            "reversibility",
            "atomic_replacement",
            "reference_ownership",
        ])
    );

    let mut value = manifest_value();
    value["mechanisms"][0]["future_descriptor_member"] =
        json!({"preserved_by_foreign_reader": true});
    value["future_manifest_member"] = true.into();
    let decoded: mechanism_manifest::MechanismManifest =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded.mechanisms[0].id, "deploy-test");
    assert_eq!(
        serde_json::to_value(&decoded).unwrap()["mechanisms"][0]["operations"],
        json!(OPERATIONS)
    );
    assert!(root["metadata"]["x-relational-laws"].as_array().unwrap().contains(&json!(
        "artifact-kinds: artifact kinds are nonempty, unique and appear in shared vocabulary canonical order"
    )));

    let vocabularies: Value = serde_json::from_str(&read("formats/vocabularies.json")).unwrap();
    assert_eq!(
        vocabularies["native_deploy_operation"]["enum"],
        json!(OPERATIONS)
    );
    let artifact_kinds: Value = serde_json::from_str(
        r#"["executable","archive","file","directory","skill","agent-plugin"]"#,
    )
    .unwrap();
    assert_eq!(
        vocabularies["native_deploy_artifact_kind"]["enum"],
        artifact_kinds
    );
    let mut unknown_role = value;
    unknown_role["mechanisms"][0]["role"] = "future-role".into();
    assert!(serde_json::from_value::<mechanism_manifest::MechanismManifest>(unknown_role).is_err());
}

#[test]
fn all_six_requests_are_closed_by_operation_but_permissive_by_object() {
    for operation in OPERATIONS {
        let mut value = request(operation);
        value["future_root_member"] = true.into();
        value["identity"]["future_identity_member"] = true.into();
        if value.get("artifact").is_some() {
            value["artifact"]["future_artifact_member"] = true.into();
        }
        let decoded: deploy_request::DeployRequest = serde_json::from_value(value).unwrap();
        assert_eq!(
            serde_json::to_value(decoded).unwrap()["operation"],
            operation
        );
    }

    let mut future = request("plan");
    future["operation"] = "future".into();
    assert!(serde_json::from_value::<deploy_request::DeployRequest>(future).is_err());
    let mut future_kind = request("plan");
    future_kind["artifact"]["kind"] = "future-kind".into();
    assert!(serde_json::from_value::<deploy_request::DeployRequest>(future_kind).is_err());
}

#[test]
fn canonical_config_preserves_absent_against_present_empty() {
    let absent: deploy_request::DeployRequest = serde_json::from_value(request("plan")).unwrap();
    let mut empty_value = request("plan");
    empty_value["config_toml"] = "".into();
    let empty: deploy_request::DeployRequest = serde_json::from_value(empty_value).unwrap();

    let deploy_request::DeployRequest::Plan(absent) = &absent else {
        unreachable!()
    };
    let deploy_request::DeployRequest::Plan(empty) = &empty else {
        unreachable!()
    };
    assert_eq!(absent.config_toml, None);
    assert_eq!(empty.config_toml.as_deref(), Some(""));
    assert!(
        serde_json::to_value(absent)
            .unwrap()
            .get("config_toml")
            .is_none()
    );
    assert_eq!(serde_json::to_value(empty).unwrap()["config_toml"], "");
}

#[test]
fn all_six_replies_are_strict_at_every_object_boundary() {
    for operation in OPERATIONS {
        let value = reply(operation);
        let decoded: deploy_reply::DeployReply = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(decoded).unwrap()["operation"],
            operation
        );

        let mut unknown_root = value.clone();
        unknown_root["future"] = true.into();
        assert!(serde_json::from_value::<deploy_reply::DeployReply>(unknown_root).is_err());
        let mut unknown_result = value;
        unknown_result["result"]["future"] = true.into();
        assert!(serde_json::from_value::<deploy_reply::DeployReply>(unknown_result).is_err());
    }

    let fail: deploy_reply::DeployReply = serde_json::from_value(json!({
        "operation": "apply",
        "envelope": 1,
        "protocol": 1,
        "result": {"status": "fail", "message": "bounded refusal"}
    }))
    .unwrap();
    let encoded = serde_json::to_value(fail).unwrap();
    assert_eq!(encoded["result"]["message"], "bounded refusal");
    assert_eq!(keys(&encoded["result"]), strings(&["status", "message"]));
}

#[test]
fn generated_object_policy_is_recursive_and_root_specific() {
    for module in ["mechanism_manifest", "deploy_request"] {
        let source = read(&format!(
            "crates/vibe-wire/src/generated/native/e1/{module}/mod.rs"
        ));
        assert!(
            !source.contains("deny_unknown_fields"),
            "{module} is foreign-read and permissive at every local object"
        );
    }
    let reply = read("crates/vibe-wire/src/generated/native/e1/deploy_reply/mod.rs");
    assert_eq!(
        reply.matches("#[serde(deny_unknown_fields)]").count(),
        20,
        "six operation objects and fourteen nested status/resource objects are strict"
    );
}

#[test]
fn reply_schema_names_the_diagnostic_cap_and_has_no_echo_members() {
    let reply = schema("deploy_reply");
    assert_eq!(reply["metadata"]["x-diagnostic-cap-bytes"], 8192);
    let request = schema("deploy_request");
    let manifest = schema("mechanism_manifest");
    for (name, root) in [
        ("manifest", manifest),
        ("request", request),
        ("reply", reply),
    ] {
        let text = serde_json::to_string(&root).unwrap();
        for forbidden in [
            "\"secret\":",
            "\"token\":",
            "\"body\":",
            "\"stdout\":",
            "\"stderr\":",
            "\"headers\":",
        ] {
            assert!(!text.contains(forbidden), "{name} exposes {forbidden}");
        }
    }
}

fn decoded_manifest(value: Value) -> mechanism_manifest::MechanismManifest {
    serde_json::from_value(value).unwrap()
}
fn decoded_request(value: Value) -> deploy_request::DeployRequest {
    serde_json::from_value(value).unwrap()
}

fn decoded_reply(value: Value) -> deploy_reply::DeployReply {
    serde_json::from_value(value).unwrap()
}

fn law<T: std::fmt::Debug>(result: Result<T, NativeDeployError>) -> &'static str {
    result.unwrap_err().law()
}

#[test]
fn admission_refuses_wrong_epochs_and_reply_operation() {
    let mut manifest = manifest_value();
    manifest["mechanisms"][0]["protocol"] = 2.into();
    assert_eq!(
        law(validate_manifest(&decoded_manifest(manifest))),
        "protocol"
    );

    for field in ["envelope", "protocol"] {
        let mut value = request("plan");
        value[field] = 2.into();
        assert_eq!(
            law(validate_request(
                &decoded_request(value),
                "org.example/tools",
                "deploy-test"
            )),
            "envelope-protocol"
        );
    }
    for field in ["envelope", "protocol"] {
        let mut value = reply("plan");
        value[field] = 2.into();
        assert_eq!(
            law(validate_reply(DeployOperation::Plan, &decoded_reply(value))),
            "envelope-protocol"
        );
    }
    assert_eq!(
        law(validate_exchange(
            &decoded_request(request("plan")),
            "org.example/tools",
            "deploy-test",
            &decoded_reply(reply("apply")),
        )),
        "reply-operation"
    );
}

#[test]
fn admission_refuses_blank_control_and_oversize_fail_messages() {
    for (message, expected_fragment) in [
        ("   ".to_string(), "blank"),
        ("line\tbreak".to_string(), "control"),
        ("x".repeat(DIAGNOSTIC_CAP_BYTES + 1), "8192-byte"),
    ] {
        let value = json!({
            "operation": "apply",
            "envelope": 1,
            "protocol": 1,
            "result": {"status": "fail", "message": message}
        });
        let error = validate_reply(DeployOperation::Apply, &decoded_reply(value)).unwrap_err();
        assert_eq!(error.law(), "fail-message");
        assert!(error.to_string().contains(expected_fragment));
        assert!(error.to_string().len() < 256);
    }
}

#[test]
fn manifest_admission_refuses_duplicate_ids_and_noncanonical_sets() {
    let mut duplicate_id = manifest_value();
    let descriptor = duplicate_id["mechanisms"][0].clone();
    duplicate_id["mechanisms"]
        .as_array_mut()
        .unwrap()
        .push(descriptor);
    assert_eq!(
        law(validate_manifest(&decoded_manifest(duplicate_id))),
        "descriptor-id"
    );
    for id in ["".to_owned(), "x".repeat(DIAGNOSTIC_CAP_BYTES + 1)] {
        let mut value = manifest_value();
        value["mechanisms"][0]["id"] = id.into();
        assert_eq!(
            law(validate_manifest(&decoded_manifest(value))),
            "descriptor-id"
        );
    }

    for operations in [
        json!(["plan", "fingerprint"]),
        json!(["plan", "fingerprint", "apply", "verify", "remove", "remove"]),
        json!([
            "fingerprint",
            "plan",
            "apply",
            "verify",
            "remove",
            "recover"
        ]),
    ] {
        let mut value = manifest_value();
        value["mechanisms"][0]["operations"] = operations;
        assert_eq!(
            law(validate_manifest(&decoded_manifest(value))),
            "deploy-operation-set"
        );
    }

    for artifacts in [
        json!([]),
        json!(["executable", "executable"]),
        json!(["directory", "executable"]),
    ] {
        let mut value = manifest_value();
        value["mechanisms"][0]["artifact_kinds"] = artifacts;
        assert_eq!(
            law(validate_manifest(&decoded_manifest(value))),
            "artifact-kinds"
        );
    }
}

#[test]
fn request_admission_enforces_pin_config_and_path_laws() {
    let valid = decoded_request(request("plan"));
    assert_eq!(
        validate_request(&valid, "org.example/tools", "deploy-test").unwrap(),
        DeployOperation::Plan
    );
    assert_eq!(
        law(validate_request(&valid, "org.other/tools", "deploy-test")),
        "exact-pin"
    );

    for identity in ["", "bad\nprovider"] {
        let mut value = request("plan");
        value["identity"]["provider"] = identity.into();
        assert_eq!(
            law(validate_request(
                &decoded_request(value),
                "org.example/tools",
                "deploy-test"
            )),
            "exact-pin"
        );
    }
    let mut overlong = request("plan");
    overlong["identity"]["mechanism"] = "x".repeat(DIAGNOSTIC_CAP_BYTES + 1).into();
    assert_eq!(
        law(validate_request(
            &decoded_request(overlong),
            "org.example/tools",
            "deploy-test"
        )),
        "exact-pin"
    );

    let mut canonical = request("plan");
    canonical["config_toml"] = "a = 1\n".into();
    validate_request(
        &decoded_request(canonical),
        "org.example/tools",
        "deploy-test",
    )
    .unwrap();
    let mut noncanonical = request("plan");
    noncanonical["config_toml"] = "a=1".into();
    assert_eq!(
        law(validate_request(
            &decoded_request(noncanonical),
            "org.example/tools",
            "deploy-test"
        )),
        "canonical-config"
    );
    let mut unsafe_path = request("plan");
    unsafe_path["artifact"]["path_relative"] = "../tool.exe".into();
    assert_eq!(
        law(validate_request(
            &decoded_request(unsafe_path),
            "org.example/tools",
            "deploy-test"
        )),
        "paths"
    );
}

#[test]
fn reply_resource_identity_and_schema_law_inventory_are_exact() {
    let mut duplicate = reply("plan");
    duplicate["result"]["resources"] = json!([
        {"resource": "bin/tool.exe", "desired_digest": "sha256:a"},
        {"resource": "bin/tool.exe", "desired_digest": "sha256:b"}
    ]);
    assert_eq!(
        law(validate_reply(
            DeployOperation::Plan,
            &decoded_reply(duplicate)
        )),
        "resource-identity"
    );

    let mut schema_laws = BTreeSet::new();
    for name in ["mechanism_manifest", "deploy_request", "deploy_reply"] {
        let document = schema(name);
        for law in document["metadata"]["x-relational-laws"]
            .as_array()
            .unwrap()
        {
            schema_laws.insert(law.as_str().unwrap().split_once(':').unwrap().0.to_owned());
        }
    }
    assert_eq!(
        schema_laws,
        RELATIONAL_LAWS
            .iter()
            .map(|law| (*law).to_owned())
            .collect()
    );
}
