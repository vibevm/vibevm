//! Registered generated shape for explicit per-operation Deploy profile data.
use std::path::PathBuf;

use serde_json::Value;
use vibe_wire::generated::{
    format_id::{FormatId, UnknownFields},
    machine::e1::profiled_deploy_action_request::ProfiledDeployActionRequest,
};

fn cases(category: &str) -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/machine-profiled-deploy-action-request/e1")
        .join(category);
    let mut paths: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    paths
}

#[test]
fn registered_profiled_request_corpus_is_strict_and_keeps_legacy_separate() {
    let id = FormatId::MachineProfiledDeployActionRequest;
    assert_eq!(id.id(), "machine-profiled-deploy-action-request");
    assert_eq!(id.epoch(), 1);
    assert_eq!(id.unknown_fields(), UnknownFields::Deny);
    assert_ne!(id, FormatId::MachineFixedDeployActionRequest);
    let valid = cases("valid");
    let invalid = cases("structural-invalid");
    assert_eq!((valid.len(), invalid.len()), (2, 2));
    for path in valid {
        let source = std::fs::read_to_string(&path).unwrap();
        let parsed: ProfiledDeployActionRequest = serde_json::from_str(&source).unwrap();
        let value: Value = serde_json::from_str(&source).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }
    for path in invalid {
        let source = std::fs::read_to_string(&path).unwrap();
        assert!(serde_json::from_str::<ProfiledDeployActionRequest>(&source).is_err());
    }
    let semantic = cases("semantic-gap");
    assert_eq!(semantic.len(), 2);
    for path in semantic {
        let source = std::fs::read_to_string(&path).unwrap();
        assert!(serde_json::from_str::<ProfiledDeployActionRequest>(&source).is_ok());
    }
}
