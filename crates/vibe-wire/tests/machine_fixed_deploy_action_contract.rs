//! Generated shape checks for the single fixed Deploy value exchange.
use std::path::PathBuf;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use vibe_wire::behaviour::deploy_records::validate_receipt;
use vibe_wire::generated::{
    deploy_receipt::DeployReceipt,
    format_id::{FormatId, UnknownFields},
    machine::e1::{
        fixed_deploy_action_error::FixedDeployActionError,
        fixed_deploy_action_reply::FixedDeployActionReply,
        fixed_deploy_action_request::FixedDeployActionRequest,
        operation_envelope::OperationEnvelope,
    },
};

#[path = "wire_support/mod.rs"]
mod wire_support;

fn cases(family: &str, category: &str) -> Vec<PathBuf> {
    let directory = wire_support::repo_root().join(format!(
        "formats/corpora/machine-fixed-deploy-action-{family}/e1/{category}"
    ));
    let mut paths: Vec<_> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "{} has no cases", directory.display());
    paths
}

fn read(path: &std::path::Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn corpus<T: DeserializeOwned + Serialize>(family: &str, id: FormatId, counts: (usize, usize)) {
    assert_eq!(id.id(), format!("machine-fixed-deploy-action-{family}"));
    assert_eq!(id.epoch(), 1);
    assert_eq!(id.unknown_fields(), UnknownFields::Deny);
    assert!(FormatId::ALL.contains(&id));
    let valid = cases(family, "valid");
    let invalid = cases(family, "structural-invalid");
    assert_eq!((valid.len(), invalid.len()), counts);
    for path in valid {
        let bytes = std::fs::read_to_string(&path).unwrap();
        let value: T = serde_json::from_str(&bytes)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            read(&path),
            "{}",
            path.display()
        );
    }
    for path in invalid {
        let bytes = std::fs::read_to_string(&path).unwrap();
        assert!(
            serde_json::from_str::<T>(&bytes).is_err(),
            "{}",
            path.display()
        );
    }
}

#[test]
fn request_corpus() {
    corpus::<FixedDeployActionRequest>(
        "request",
        FormatId::MachineFixedDeployActionRequest,
        (2, 11),
    );
}

#[test]
fn reply_corpus() {
    corpus::<FixedDeployActionReply>("reply", FormatId::MachineFixedDeployActionReply, (6, 8));
}

#[test]
fn error_corpus() {
    corpus::<FixedDeployActionError>("error", FormatId::MachineFixedDeployActionError, (4, 6));
}

fn object_paths(value: &Value, path: &mut Vec<String>, found: &mut Vec<Vec<String>>) {
    if let Some(object) = value.as_object() {
        found.push(path.clone());
        for (key, value) in object {
            path.push(key.clone());
            object_paths(value, path, found);
            path.pop();
        }
    }
}

fn object_at<'a>(value: &'a mut Value, path: &[String]) -> &'a mut serde_json::Map<String, Value> {
    let mut current = value;
    for key in path {
        current = current.get_mut(key).unwrap();
    }
    current.as_object_mut().unwrap()
}

fn closed_and_required<T: DeserializeOwned>(family: &str) {
    let mut probes = 0;
    for file in cases(family, "valid") {
        let base = read(&file);
        let mut paths = Vec::new();
        object_paths(&base, &mut Vec::new(), &mut paths);
        for path in paths {
            let mut extra = base.clone();
            object_at(&mut extra, &path).insert("grant".into(), json!(true));
            assert!(
                serde_json::from_value::<T>(extra).is_err(),
                "{} {path:?} extra",
                file.display()
            );
            probes += 1;
            let keys: Vec<_> = object_at(&mut base.clone(), &path)
                .keys()
                .cloned()
                .collect();
            for key in keys {
                let mut missing = base.clone();
                object_at(&mut missing, &path).remove(&key);
                assert!(
                    serde_json::from_value::<T>(missing).is_err(),
                    "{} {path:?}.{key} missing",
                    file.display()
                );
                probes += 1;
            }
        }
    }
    assert!(probes > 0);
    eprintln!("{family}: {probes} unknown-field/missing-field probes");
}

#[test]
fn nested_shapes_are_closed_and_required() {
    closed_and_required::<FixedDeployActionRequest>("request");
    closed_and_required::<FixedDeployActionReply>("reply");
    closed_and_required::<FixedDeployActionError>("error");
}

#[test]
fn nullable_evidence_is_exact() {
    let base = wire_support::read_json(
        "formats/corpora/machine-fixed-deploy-action-error/e1/valid/effect_uncertain.json",
    );
    for key in [
        "requested_action",
        "requested_phase",
        "requested_scope",
        "scope_unavailable_reason",
        "known_run_id",
        "view",
    ] {
        let mut row = base.clone();
        row[key] = Value::Null;
        let parsed: FixedDeployActionError = serde_json::from_value(row.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), row);
        row.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<FixedDeployActionError>(row).is_err());
        for wrong in [json!(42), json!(true), json!([])] {
            let mut row = base.clone();
            row[key] = wrong;
            assert!(serde_json::from_value::<FixedDeployActionError>(row).is_err());
        }
    }
}

fn semantic_gaps<T: DeserializeOwned + Serialize>(family: &str, count: usize) {
    let paths = cases(family, "semantic-gap");
    assert_eq!(paths.len(), count);
    for path in paths {
        let original = read(&path);
        let generated: T = serde_json::from_value(original.clone()).unwrap();
        assert_eq!(serde_json::to_value(generated).unwrap(), original);
    }
}

#[test]
fn semantic_gaps_require_later_adapter() {
    // The generated shape intentionally does not establish epoch, key or diagnostic bounds.
    semantic_gaps::<FixedDeployActionRequest>("request", 3);
    semantic_gaps::<FixedDeployActionReply>("reply", 3);
    semantic_gaps::<FixedDeployActionError>("error", 2);
    let message = wire_support::read_json(
        "formats/corpora/machine-fixed-deploy-action-error/e1/semantic-gap/oversized_message.json",
    );
    assert_eq!(message["message"].as_str().unwrap().len(), 4097);
    let epoch = wire_support::read_json(
        "formats/corpora/machine-fixed-deploy-action-request/e1/semantic-gap/unsupported_epoch.json",
    );
    assert_ne!(epoch["schema"], json!(1));
    let key = wire_support::read_json(
        "formats/corpora/machine-fixed-deploy-action-request/e1/semantic-gap/oversized_key.json",
    );
    assert_eq!(key["action"]["key"].as_str().unwrap().len(), 65536);
}

#[test]
fn completed_nested_receipt_needs_generated_reader_and_behavior() {
    let completed = wire_support::read_json(
        "formats/corpora/machine-fixed-deploy-action-reply/e1/valid/completed.json",
    );
    let outer: FixedDeployActionReply = serde_json::from_value(completed.clone()).unwrap();
    assert_eq!(serde_json::to_value(outer).unwrap(), completed);
    let receipt: DeployReceipt =
        serde_json::from_str(completed["reply"]["receipt_json"].as_str().unwrap()).unwrap();
    validate_receipt(&receipt).unwrap();
    assert_eq!(
        serde_json::to_string(&receipt).unwrap(),
        completed["reply"]["receipt_json"].as_str().unwrap(),
        "completed fixture uses the generated receipt's canonical serialization",
    );
    assert_eq!(completed["reply"]["receipt_format_id"], "deploy-receipt");

    let malformed = wire_support::read_json(
        "formats/corpora/machine-fixed-deploy-action-reply/e1/semantic-gap/malformed_nested_receipt.json",
    );
    assert!(serde_json::from_value::<FixedDeployActionReply>(malformed.clone()).is_ok());
    assert!(serde_json::from_str::<DeployReceipt>(
        malformed["reply"]["receipt_json"].as_str().unwrap(),
    ).is_err());
    let invalid = wire_support::read_json(
        "formats/corpora/machine-fixed-deploy-action-reply/e1/semantic-gap/invalid_nested_receipt_status.json",
    );
    let nested: DeployReceipt =
        serde_json::from_str(invalid["reply"]["receipt_json"].as_str().unwrap()).unwrap();
    assert!(validate_receipt(&nested).is_err());
}

fn envelope<T: DeserializeOwned + Serialize>(family: &str, id: FormatId, stage: &str) {
    for path in cases(family, "valid") {
        let nested = read(&path);
        let payload_json = serde_json::to_string(&nested).unwrap();
        let mut body = json!({"stage": stage, "format_id": id.id(), "payload_json": payload_json});
        if stage != "input" {
            body["effect_state"] = json!("uncertain");
            body["completeness"] = json!("partial");
            body["provenance_id"] = Value::Null;
        }
        let original = json!({"schema": 1, "operation_id": "rust:vibe-lifecycle:deploy-vibe-bin-fixed",
            "domain_owner": "rust:vibe-lifecycle", "source_id": null, "evidence_id": null, "body": body});
        let parsed: OperationEnvelope = serde_json::from_value(original.clone()).unwrap();
        let encoded = serde_json::to_value(parsed).unwrap();
        assert_eq!(encoded, original);
        let recovered: T =
            serde_json::from_str(encoded["body"]["payload_json"].as_str().unwrap()).unwrap();
        assert_eq!(serde_json::to_value(recovered).unwrap(), nested);
    }
}

#[test]
fn envelope_nested_values_roundtrip() {
    // This is transport preservation only; the later adapter decides owner/effect coherence.
    envelope::<FixedDeployActionRequest>(
        "request",
        FormatId::MachineFixedDeployActionRequest,
        "input",
    );
    envelope::<FixedDeployActionReply>("reply", FormatId::MachineFixedDeployActionReply, "result");
    envelope::<FixedDeployActionError>("error", FormatId::MachineFixedDeployActionError, "error");
}
