//! Authored golden documents for the epoch-1 deploy journal pair
//! (`schemas/deploy_intent.jtd.json` / `schemas/deploy_receipt.jtd.json`,
//! R8A2 — §7.2 of the packages-2026-09 architecture: the intent
//! written before apply, the receipt written after). Both formats are
//! registered `foreign_parsers = "none"`, so the generated readers are
//! STRICT: an unknown member is a wire bug, and every refusal below
//! names the member it refused. The pair shares this corpus home the
//! same way the five lifecycle records share `lifecycle/e1` — one
//! exchange, one home.
//!
//! Two kinds of check sit beside each other, and they are not the same
//! kind. READER checks prove the strict generated reader refuses an
//! unknown member while naming it; CELL checks prove the hand-written
//! validator (`behaviour::deploy_records`) refuses a broken scalar
//! with the typed error naming the member — the corpus documents are
//! the shared fixtures both halves cite.

use std::path::PathBuf;

use vibe_wire::behaviour::deploy_records::{
    DeployIntentError, DeployReceiptError, validate_intent, validate_receipt,
};
use vibe_wire::generated::deploy_intent::DeployIntent;
use vibe_wire::generated::deploy_receipt::{DeployReceipt, ReceiptStatus};
use vibe_wire::generated::format_id::{ForeignParsers, FormatId};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus() -> PathBuf {
    repo_root().join("formats/corpora/deploy/e1")
}

fn read_corpus(name: &str) -> serde_json::Value {
    let path = corpus().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} readable: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} parses: {e}"))
}

fn parse_intent(doc: serde_json::Value) -> DeployIntent {
    serde_json::from_value(doc).expect("parses through the generated reader")
}

fn parse_receipt(doc: serde_json::Value) -> DeployReceipt {
    serde_json::from_value(doc).expect("parses through the generated reader")
}

#[test]
fn every_valid_document_round_trips_and_validates() {
    for name in [
        "intent_minimal.json",
        "intent_full.json",
        "receipt_minimal.json",
        "receipt_full.json",
    ] {
        let authored = read_corpus(&format!("valid/{name}"));
        if name.starts_with("intent") {
            let intent = parse_intent(authored.clone());
            validate_intent(&intent)
                .unwrap_or_else(|error| panic!("{name} violates a law: {error}"));
            assert_eq!(
                serde_json::to_value(&intent).unwrap(),
                authored,
                "{name} loses data on generated round-trip"
            );
        } else {
            let receipt = parse_receipt(authored.clone());
            validate_receipt(&receipt)
                .unwrap_or_else(|error| panic!("{name} violates a law: {error}"));
            assert_eq!(
                serde_json::to_value(&receipt).unwrap(),
                authored,
                "{name} loses data on generated round-trip"
            );
        }
    }
}

#[test]
fn the_minimal_intent_and_receipt_carry_exactly_the_wire_minimum() {
    let intent = parse_intent(read_corpus("valid/intent_minimal.json"));
    assert_eq!(intent.schema, 1);
    assert_eq!(intent.target.project, "demo");
    assert_eq!(intent.target.package, None);
    assert_eq!(intent.prior_generation, None);
    assert_eq!(intent.resources.len(), 1);
    assert!(intent.resources[0].prior_digest.is_none());

    let receipt = parse_receipt(read_corpus("valid/receipt_minimal.json"));
    assert_eq!(receipt.schema, 1);
    assert_eq!(receipt.status, ReceiptStatus::Applied);
    assert!(
        receipt.finalized_at.is_none(),
        "mid-flight is not finalised"
    );
    assert!(receipt.evidence.is_none());
    assert!(receipt.prior_state_handle.is_none());
    // An `applied` receipt carries no finalized_at on the wire either.
    let wire = serde_json::to_value(&receipt).unwrap();
    assert!(wire.get("finalized_at").is_none());
}

#[test]
fn the_full_documents_carry_every_optional_member() {
    let intent = parse_intent(read_corpus("valid/intent_full.json"));
    assert_eq!(intent.target.package.as_deref(), Some("org.demo/tools"));
    assert_eq!(intent.prior_generation, Some(2));
    assert_eq!(intent.resources.len(), 2);
    assert!(intent.resources[0].prior_digest.is_some());
    assert!(intent.resources[1].prior_digest.is_none());

    let receipt = parse_receipt(read_corpus("valid/receipt_full.json"));
    assert_eq!(receipt.status, ReceiptStatus::Verified);
    assert!(receipt.finalized_at.is_some());
    assert!(receipt.provider.version.is_some());
    assert!(receipt.provider.content_hash.is_some());
    assert!(receipt.evidence.is_some());
    assert!(receipt.prior_state_handle.is_some());
    assert_eq!(receipt.resources.len(), 2);
}

#[test]
fn the_readers_refuse_unknown_members_and_name_them() {
    for (name, field) in [
        ("invalid/intent_unknown_field.json", "dry_run"),
        ("invalid/receipt_unknown_field.json", "api_token"),
    ] {
        let doc = read_corpus(name);
        let error = if name.contains("intent") {
            serde_json::from_value::<DeployIntent>(doc)
                .expect_err("an unknown member must be refused")
                .to_string()
        } else {
            serde_json::from_value::<DeployReceipt>(doc)
                .expect_err("an unknown member must be refused")
                .to_string()
        };
        assert!(
            error.contains("unknown field"),
            "{name}: reader says: {error}"
        );
        assert!(
            error.contains(field),
            "{name}: reader names the member: {error}"
        );
    }
}

#[test]
fn a_bad_plan_hash_is_a_typed_refusal_naming_the_member() {
    // The corpus document carries a `sha256:`-prefixed, 71-character
    // plan hash: both the prefix (the record's digests are bare hex)
    // and the length are wrong, and ONE typed refusal names the member.
    let intent = parse_intent(read_corpus("invalid/intent_digest_bad.json"));
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::PlanHashNotHex { .. })
    ));
}

#[test]
fn a_blank_project_is_a_typed_refusal_naming_the_member() {
    let intent = parse_intent(read_corpus("invalid/intent_blank_project.json"));
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::UnsafeScalar { field, .. }) if field == "target.project"
    ));
}

#[test]
fn a_bad_receipt_digest_is_a_typed_refusal_naming_the_member() {
    // The corpus document carries an UPPERCASE-hex artifact digest: the
    // law is lowercase, and the refusal names `artifact_digest`.
    let receipt = parse_receipt(read_corpus("invalid/receipt_digest_bad.json"));
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::DigestNotHex { member, .. }) if member == "artifact_digest"
    ));
}

#[test]
fn a_blank_owned_resource_is_a_typed_refusal_naming_the_row() {
    let receipt = parse_receipt(read_corpus("invalid/receipt_blank_resource.json"));
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::UnsafeResource { row: 0, .. })
    ));
}

#[test]
fn the_finalisation_matrix_is_cited_from_the_corpus() {
    // A verified receipt without finalized_at claims a terminal state
    // it never reached; an applied one with it is mid-flight wearing a
    // terminal timestamp. Both directions refuse.
    let base = read_corpus("valid/receipt_minimal.json");

    let mut unfinalised = base.clone();
    unfinalised["status"] = serde_json::json!("verified");
    let receipt = parse_receipt(unfinalised);
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::TerminalNotFinalised { status }) if status == ReceiptStatus::Verified
    ));

    let mut early = base;
    early["finalized_at"] = serde_json::json!("2026-08-29T12:30:59Z");
    let receipt = parse_receipt(early);
    assert_eq!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::AppliedFinalised)
    );
}

#[test]
fn registry_records_are_pinned() {
    let text = std::fs::read_to_string(repo_root().join("formats/REGISTRY.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    let formats = parsed
        .get("format")
        .and_then(|v| v.as_table())
        .expect("formats/REGISTRY.toml has a [format.*] table");

    for id in ["deploy-intent", "deploy-receipt"] {
        let record = &formats[id];
        assert_eq!(record.get("epoch").unwrap().as_integer(), Some(1), "{id}");
        assert_eq!(
            record.get("recoverable").unwrap().as_bool(),
            Some(false),
            "{id}"
        );
        assert_eq!(
            record.get("foreign_parsers").unwrap().as_str(),
            Some("none"),
            "{id}"
        );
        assert_eq!(
            record.get("corpus").unwrap().as_str(),
            Some("formats/corpora/deploy/e1"),
            "{id} — the pair shares one home"
        );
        assert_eq!(record.get("sunset").unwrap().as_str(), Some("none"), "{id}");
    }
    assert_eq!(
        formats["deploy-intent"].get("schema").unwrap().as_str(),
        Some("schemas/deploy_intent.jtd.json")
    );
    assert_eq!(
        formats["deploy-receipt"].get("schema").unwrap().as_str(),
        Some("schemas/deploy_receipt.jtd.json")
    );

    // The pinned paths exist on disk.
    assert!(repo_root().join("schemas/deploy_intent.jtd.json").is_file());
    assert!(
        repo_root()
            .join("schemas/deploy_receipt.jtd.json")
            .is_file()
    );
    assert!(corpus().join("valid").is_dir());
    assert!(corpus().join("invalid").is_dir());

    // The generated FormatId agrees with both records.
    for id in ["deploy-intent", "deploy-receipt"] {
        let variant = FormatId::ALL
            .iter()
            .copied()
            .find(|format| format.id() == id)
            .unwrap_or_else(|| panic!("FormatId carries the {id} variant"));
        assert_eq!(variant.epoch(), 1);
        assert!(!variant.recoverable());
        assert_eq!(variant.foreign_parsers(), ForeignParsers::None);
    }
}
