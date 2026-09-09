//! Generated epoch-2 scrape transaction journal corpus and registry pins.

use std::path::PathBuf;

use vibe_wire::generated::format_id::{ForeignParsers, FormatId, UnknownFields};
use vibe_wire::generated::scrape::e2::transaction_journal as wire;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus(kind: &str, name: &str) -> Vec<u8> {
    std::fs::read(
        root()
            .join("formats/corpora/scrape-transaction-journal/e2")
            .join(kind)
            .join(name),
    )
    .unwrap()
}

#[test]
fn registry_pins_the_internal_epoch_two_root() {
    assert_eq!(FormatId::ScrapeTransactionJournal.epoch(), 2);
    assert!(!FormatId::ScrapeTransactionJournal.recoverable());
    assert_eq!(
        FormatId::ScrapeTransactionJournal.foreign_parsers(),
        ForeignParsers::None
    );
    assert_eq!(
        FormatId::ScrapeTransactionJournal.unknown_fields(),
        UnknownFields::Deny
    );
}

#[test]
fn valid_minimal_and_full_documents_round_trip_canonically() {
    for name in ["minimal-export.json", "full-in-place.json"] {
        let value: wire::TransactionJournal =
            serde_json::from_slice(&corpus("valid", name)).unwrap();
        assert_eq!(value.schema, 2, "{name}");
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            serde_json::from_slice::<wire::TransactionJournal>(&bytes).unwrap(),
            value
        );
    }
}

#[test]
fn invalid_root_nested_duplicate_missing_type_and_legacy_shapes_refuse() {
    for name in [
        "unknown-root.json",
        "unknown-nested.json",
        "duplicate-schema.json",
        "missing-required.json",
        "missing-required-null.json",
        "wrong-type.json",
        "legacy-epoch1-shape.json",
    ] {
        assert!(
            serde_json::from_slice::<wire::TransactionJournal>(&corpus("invalid", name)).is_err(),
            "{name}"
        );
    }
}

#[test]
fn every_transaction_state_discriminator_is_a_closed_flat_object() {
    let states = vec![
        wire::TransactionState::Preparing(Box::new(wire::TransactionStatePreparing {})),
        wire::TransactionState::Prepared(Box::new(wire::TransactionStatePrepared {})),
        wire::TransactionState::BeforePassed(Box::new(wire::TransactionStateBeforePassed {})),
        wire::TransactionState::Candidate(Box::new(wire::TransactionStateCandidate {})),
        wire::TransactionState::PublishedPendingVerify(Box::new(
            wire::TransactionStatePublishedPendingVerify {},
        )),
        wire::TransactionState::Mutating(Box::new(wire::TransactionStateMutating {})),
        wire::TransactionState::ContractBoundary(Box::new(
            wire::TransactionStateContractBoundary {
                action: wire::ContractBoundaryAction::DeleteLastMoved,
            },
        )),
        wire::TransactionState::Verified(Box::new(wire::TransactionStateVerified {})),
        wire::TransactionState::CleanupPending(Box::new(wire::TransactionStateCleanupPending {})),
        wire::TransactionState::Complete(Box::new(wire::TransactionStateComplete {})),
        wire::TransactionState::RollingBack(Box::new(wire::TransactionStateRollingBack {})),
        wire::TransactionState::RolledBack(Box::new(wire::TransactionStateRolledBack {})),
        wire::TransactionState::RollbackFailed(Box::new(wire::TransactionStateRollbackFailed {})),
    ];
    for state in states {
        let bytes = serde_json::to_vec(&state).unwrap();
        let object = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
        assert!(
            object["kind"].is_string(),
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(
            serde_json::from_slice::<wire::TransactionState>(&bytes).unwrap(),
            state
        );
    }
    assert!(serde_json::from_slice::<wire::TransactionState>(br#"{"kind":"future"}"#).is_err());
}

#[test]
fn remaining_discriminator_arms_are_strict_flat_objects() {
    for payload in [
        r#"{"kind":"prepared-after","snapshot_name":"prepared/one"}"#,
        r#"{"kind":"source","before":{"sha256":"sha256:source","bytes":1,"mode":null},"source_path":"input.txt"}"#,
    ] {
        assert!(serde_json::from_str::<wire::ExportPayload>(payload).is_ok());
    }
    for commit in [
        r#"{"kind":"delete-last","empty_ancestors":[],"path":"vibe.toml"}"#,
        r#"{"kind":"external-preserve"}"#,
    ] {
        assert!(serde_json::from_str::<wire::ContractCommit>(commit).is_ok());
    }
    assert!(
        serde_json::from_str::<wire::ExportPayload>(
            r#"{"kind":"prepared-after","snapshot_name":"prepared/one","extra":true}"#,
        )
        .is_err()
    );
    assert!(serde_json::from_str::<wire::ContractCommit>(r#"{"kind":"future"}"#).is_err());
}
