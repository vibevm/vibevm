//! Generated D1 reader proof only; no operation/native semantic acceptance.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE"
);

use serde::de::DeserializeOwned;
use vibe_wire::generated::migration::e1::plan_request;
#[path = "wire_support/mod.rs"]
mod wire_support;

use vibe_wire::generated::migration::e1::{
    application_journal, operation_report, operation_request,
};

fn application_fixture(family: &str, class: &str, name: &str) -> String {
    format!("formats/corpora/migration-{family}/e1/{class}/{name}.json")
}
fn application_reject<T: DeserializeOwned>(family: &str, cases: &[(&str, &str)]) {
    for (name, expected) in cases {
        let path = application_fixture(family, "structural-invalid", name);
        let error = serde_json::from_value::<T>(wire_support::read_json(&path))
            .err()
            .unwrap_or_else(|| panic!("{path} unexpectedly decoded"));
        assert!(error.to_string().contains(expected), "{path}: {error}");
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn operation_request_valid_roundtrips_preserve_explicit_intent_and_authority() {
    let value: operation_request::OperationRequest =
        wire_support::round_trip(&application_fixture("operation-request", "valid", "apply"));
    let operation_request::OperationRequest::Apply(row) = value else {
        panic!("apply")
    };
    let embedded: plan_request::PlanRequest =
        serde_json::from_str(&row.approved_request_json).unwrap();
    assert!(
        embedded
            .source_text
            .ends_with("materialization=\"snapshot\"\n")
    );
    assert_eq!(
        row.before_content_id,
        "905cf0b4eb7376819f47c9f7da02fecb8920d91dd9b76b4cb3454da77fa81887"
    );
    assert_eq!(
        row.diff_id,
        "42faf5376728e4649c80657e6b0f93e9910a107261413a5d0c7f019bf5ab27e0"
    );
    assert_eq!(row.authority.target_relative, "vibe.toml");
    let inspect: operation_request::OperationRequest = wire_support::round_trip(
        &application_fixture("operation-request", "valid", "inspect"),
    );
    assert!(matches!(
        inspect,
        operation_request::OperationRequest::Inspect(_)
    ));
    let recover: operation_request::OperationRequest = wire_support::round_trip(
        &application_fixture("operation-request", "valid", "recover-complete"),
    );
    let operation_request::OperationRequest::Recover(row) = recover else {
        panic!("recover")
    };
    assert_eq!(
        row.action,
        operation_request::OperationRequestRecoverAction::Complete
    );
    let mut value = wire_support::read_json(&application_fixture(
        "operation-request",
        "valid",
        "recover-complete",
    ));
    value["action"] = serde_json::json!("restore");
    assert!(serde_json::from_value::<operation_request::OperationRequest>(value).is_ok());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn operation_report_valid_roundtrips_preserve_nullable_identity_evidence_effects_and_guarantees() {
    let completed: operation_report::OperationReport = wire_support::round_trip(
        &application_fixture("operation-report", "valid", "completed"),
    );
    assert_eq!(
        completed.operation,
        Some(operation_report::OperationReportOperation::Apply)
    );
    assert_eq!(
        completed.effects[0].kind,
        operation_report::EffectKind::TargetPublication
    );
    assert!(!completed.guarantees.uncooperating_writers_excluded);
    let refused: operation_report::OperationReport = wire_support::round_trip(
        &application_fixture("operation-report", "valid", "refused-unavailable"),
    );
    assert_eq!(refused.operation, None);
    assert_eq!(refused.operation_id, None);
    assert_eq!(refused.intent_id, None);
    assert_eq!(refused.before_content_id, None);
    assert_eq!(refused.after_content_id, None);
    assert_eq!(refused.diff_id, None);
    assert_eq!(refused.guarantees.barrier_binding, None);
    assert!(refused.effects.is_empty());
    assert_eq!(
        refused.error.unwrap().code,
        operation_report::ApplicationErrorCode::InvalidRequest
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn application_journal_valid_roundtrips_preserve_checkpoint_fields_and_mode() {
    for (name, mode) in [
        ("prepared", Some(420)),
        ("prepared-null-mode", None),
        ("restored", Some(420)),
    ] {
        let journal: application_journal::ApplicationJournal =
            wire_support::round_trip(&application_fixture("application-journal", "valid", name));
        assert_eq!(journal.unix_mode, mode);
        assert_eq!(journal.protocol_revision, 1);
        assert_ne!(journal.before_raw_sha256, journal.before_content_id);
        assert_eq!(journal.authority.target_relative, "vibe.toml");
        assert_eq!(journal.before_length, "97");
        assert_eq!(journal.after_length, "93");
    }
    for phase in [
        "prepared",
        "publishing",
        "published",
        "restoring",
        "restored",
    ] {
        let decoded: application_journal::ApplicationJournalPhase =
            serde_json::from_value(serde_json::json!(phase)).unwrap();
        assert_eq!(
            serde_json::to_value(decoded).unwrap(),
            serde_json::json!(phase)
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn operation_request_structural_invalids_refuse() {
    application_reject::<operation_request::OperationRequest>(
        "operation-request",
        &[
            ("unknown-root", "unknown field"),
            ("unknown-authority", "unknown field"),
            ("unknown-operation", "unknown variant"),
            ("schema-string", "invalid type"),
        ],
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn operation_report_structural_invalids_refuse_nested_guarantees_effects_codes() {
    application_reject::<operation_report::OperationReport>(
        "operation-report",
        &[
            ("missing-required-operation", "missing field"),
            ("unknown-guarantee", "unknown variant"),
            ("unknown-effect", "unknown variant"),
            ("unknown-error-code", "unknown variant"),
            ("unknown-warning", "unknown variant"),
            ("barrier-binding-number", "invalid type"),
        ],
    );
    let base = wire_support::read_json(&application_fixture(
        "operation-report",
        "valid",
        "completed",
    ));
    for field in [
        "operation",
        "operation_id",
        "intent_id",
        "before_content_id",
        "after_content_id",
        "diff_id",
        "error",
    ] {
        let mut value = base.clone();
        value.as_object_mut().unwrap().remove(field);
        let error = serde_json::from_value::<operation_report::OperationReport>(value).unwrap_err();
        assert!(
            error.to_string().contains("missing field"),
            "{field}: {error}"
        );
    }
    let mut value = base.clone();
    value["guarantees"]
        .as_object_mut()
        .unwrap()
        .remove("barrier_binding");
    assert!(
        serde_json::from_value::<operation_report::OperationReport>(value)
            .unwrap_err()
            .to_string()
            .contains("missing field")
    );
    for path in [
        "root",
        "guarantees",
        "effect",
        "error",
        "warning",
        "residue",
    ] {
        let mut value = base.clone();
        match path {
            "root" => value["extra"] = serde_json::json!(true),
            "guarantees" => value["guarantees"]["extra"] = serde_json::json!(true),
            "effect" => value["effects"][0]["extra"] = serde_json::json!(true),
            "error" => {
                value["error"] = serde_json::json!({"code":"invalid_request","field":"request","reason":"fixed","extra":true})
            }
            "warning" => {
                value["warnings"] =
                    serde_json::json!([{"code":"residue_retained","action":"inspect","extra":true}])
            }
            _ => {
                value["residue"] = serde_json::json!([{"location":"store","relative_path":"migrations/opaque","ownership":"unproven","extra":true}])
            }
        }
        assert!(
            serde_json::from_value::<operation_report::OperationReport>(value)
                .unwrap_err()
                .to_string()
                .contains("unknown field"),
            "{path}"
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn application_journal_structural_invalids_refuse() {
    application_reject::<application_journal::ApplicationJournal>(
        "application-journal",
        &[
            ("unknown-root", "unknown field"),
            ("unknown-authority", "unknown field"),
            ("unknown-phase", "unknown variant"),
            ("unix-mode-string", "invalid type"),
            ("before-length-number", "invalid type"),
            ("missing-unix-mode", "missing field"),
        ],
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn application_domain_invalids_remain_owner_validation_inputs() {
    // Structural acceptance only: each of these MUST later be refused by the
    // operation owner before effects. No domain/native acceptance is implied.
    for name in ["unsupported-schema", "wrong-before-content-id"] {
        let _: operation_request::OperationRequest = wire_support::round_trip(
            &application_fixture("operation-request", "semantic-invalid", name),
        );
    }
    for name in ["wrong-schema", "claims-external-writers-excluded"] {
        let _: operation_report::OperationReport = wire_support::round_trip(&application_fixture(
            "operation-report",
            "semantic-invalid",
            name,
        ));
    }
    for name in ["unsupported-protocol", "noncanonical-length"] {
        let _: application_journal::ApplicationJournal = wire_support::round_trip(
            &application_fixture("application-journal", "semantic-invalid", name),
        );
    }
}
