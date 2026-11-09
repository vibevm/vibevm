//! The user-application index (`vibe-user-applications/3`) reads and
//! round-trips its corpus and keeps a strict reader (PROP-059 `#index`).

use std::path::PathBuf;

use serde_json::Value;
use vibe_wire::generated::application::e2::index::ApplicationIndex;
use vibe_wire::generated::format_id::{FormatId, UnknownFields};

fn corpus() -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/application/e2");
    std::fs::read(root.join("index.json")).unwrap()
}

#[test]
fn application_index_reads_and_round_trips_its_corpus() {
    let bytes = corpus();
    let authored: Value = serde_json::from_slice(&bytes).unwrap();
    let decoded: ApplicationIndex = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.protocol, "vibe-user-applications/3");
    assert_eq!(serde_json::to_value(decoded).unwrap(), authored);
}

#[test]
fn application_index_rejects_unknown_members() {
    let mut value: Value = serde_json::from_slice(&corpus()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("future".into(), Value::Bool(true));
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(serde_json::from_slice::<ApplicationIndex>(&bytes).is_err());
}

#[test]
fn registry_exposes_distinct_current_and_legacy_index_epochs() {
    assert_eq!(FormatId::ApplicationIndex.epoch(), 5);
    assert_eq!(FormatId::ApplicationIndexV5.epoch(), 4);
    assert_eq!(FormatId::ApplicationIndexV4.epoch(), 3);
    assert_eq!(FormatId::ApplicationIndexV3.epoch(), 2);
    assert_eq!(
        FormatId::ApplicationIndex.unknown_fields(),
        UnknownFields::Deny
    );
}

/// Recovery means deletion and reprojection without human input, not readability.
#[test]
fn registry_preserves_authoritative_input_and_continuation_state() {
    assert!(!FormatId::ConfigurationInput.recoverable());
    assert!(FormatId::ConfigurationDescription.recoverable());
    assert!(FormatId::ConfigurationEvaluation.recoverable());
    assert!(FormatId::ConfigurationAppPlan.recoverable());
    assert!(!FormatId::ApplicationSteps.recoverable());
    assert!(FormatId::ApplicationStepsV1.recoverable());
    assert!(!FormatId::ApplicationImportWork.recoverable());
    assert!(!FormatId::ApplicationIndex.recoverable());
    assert!(!FormatId::ApplicationIndexV5.recoverable());
    assert!(!FormatId::ApplicationIndexV3.recoverable());
}

/// `vibe app outdated --json` (`vibe-app-outdated/1`, PROP-065
/// `#outdated`) reads and round-trips its corpus, `latest: null` included.
#[test]
fn application_outdated_reads_and_round_trips_its_corpus() {
    use vibe_wire::generated::application_outdated::{ApplicationOutdated, Status};
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("formats/corpora/application-outdated/e1/outdated.json"),
    )
    .unwrap();
    let authored: Value = serde_json::from_slice(&bytes).unwrap();
    let decoded: ApplicationOutdated = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.protocol, "vibe-app-outdated/1");
    assert_eq!(decoded.installations[1].status, Status::Held);
    assert_eq!(decoded.installations[2].latest, None);
    assert_eq!(serde_json::to_value(decoded).unwrap(), authored);
    assert_eq!(FormatId::CliApplicationOutdated.epoch(), 1);
}
