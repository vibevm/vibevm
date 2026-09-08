use serde_json::{Value, json};
use vibe_wire::behaviour::native_mechanism::{
    DIAGNOSTIC_CAP_BYTES, NativeMechanismError, validate_manifest,
};
use vibe_wire::generated::native::e1::mechanism_manifest;

const OPERATIONS: [&str; 6] = [
    "plan",
    "fingerprint",
    "apply",
    "verify",
    "remove",
    "recover",
];

fn manifest_value() -> Value {
    json!({"mechanisms": [{
        "id": "selected", "role": "deploy", "name": "selected", "protocol": 1,
        "operations": OPERATIONS, "artifact_kinds": ["executable", "directory"],
        "effect": "user", "network": "never", "privilege": "none",
        "reversibility": "reversible", "atomic_replacement": true,
        "reference_ownership": true
    }]})
}

fn decoded(value: Value) -> mechanism_manifest::MechanismManifest {
    serde_json::from_value(value).unwrap()
}

fn law<T: std::fmt::Debug>(result: Result<T, NativeMechanismError>) -> &'static str {
    result.unwrap_err().law()
}

#[test]
fn manifest_operations_are_exact_for_each_admitted_role() {
    let producing = json!(["plan", "fingerprint", "apply", "verify"]);
    for role in ["build", "package"] {
        let mut value = manifest_value();
        value["mechanisms"][0]["role"] = role.into();
        value["mechanisms"][0]["name"] = "selected".into();
        value["mechanisms"][0]["operations"] = producing.clone();
        validate_manifest(&decoded(value)).expect("producing role is admitted");
    }

    for (role, operations) in [
        ("build", json!(OPERATIONS)),
        ("package", json!(["plan", "fingerprint", "verify", "apply"])),
        ("deploy", producing.clone()),
        ("acquire", json!([])),
    ] {
        let mut value = manifest_value();
        value["mechanisms"][0]["role"] = role.into();
        value["mechanisms"][0]["operations"] = operations;
        let error = validate_manifest(&decoded(value)).unwrap_err();
        assert_eq!(error.law(), "role-operation-set");
        assert!(error.to_string().starts_with("native mechanism "));
    }

    for name in [
        "   ".to_owned(),
        "bad\nname".to_owned(),
        "x".repeat(DIAGNOSTIC_CAP_BYTES + 1),
    ] {
        let mut value = manifest_value();
        value["mechanisms"][0]["name"] = name.into();
        assert_eq!(law(validate_manifest(&decoded(value))), "descriptor-name");
    }
}
