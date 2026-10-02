//! Generated readers carry decision data, never effect authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION");
use vibe_wire::generated::{format_id::FormatId, machine::e1::input_decision_message::*};
#[path = "wire_support/mod.rs"]
mod wire_support;
const ROOT: &str = "formats/corpora/machine-input-decision-message/e1";
const ARMS: [&str; 7] = [
    "selection-question",
    "selection-answer",
    "text-question",
    "text-answer",
    "approval-question",
    "approval-answer",
    "unavailable",
];
fn value(name: &str) -> serde_json::Value {
    wire_support::read_json(&format!("{ROOT}/valid/{name}.json"))
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION",
    r = 1
)]
fn seven_closed_arms_roundtrip_and_format_is_registered() {
    for name in ARMS {
        let row: InputDecisionMessage =
            wire_support::round_trip(&format!("{ROOT}/valid/{name}.json"));
        assert_eq!(row.schema, 1);
        assert_eq!(row.target_id, None);
    }
    assert!(
        FormatId::ALL
            .iter()
            .any(|f| f.id() == "machine-input-decision-message" && f.epoch() == 1)
    );
}
#[test]
fn actual_structural_corpus_refuses() {
    for name in [
        "unknown-root",
        "unknown-body",
        "unknown-choice",
        "unknown-options",
        "wrong-type",
        "missing-nullable",
        "missing-profile",
        "null-options",
        "unknown-enum",
        "unknown-arm",
        "wrong-approval-type",
    ] {
        let row = wire_support::read_json(&format!("{ROOT}/structural-invalid/{name}.json"));
        assert!(
            serde_json::from_value::<InputDecisionMessage>(row).is_err(),
            "{name}"
        );
    }
}
#[test]
fn every_required_nullable_binding_distinguishes_absence_from_null() {
    for key in [
        "question_id",
        "operation_id",
        "domain_owner",
        "context_id",
        "target_id",
        "principal_id",
        "plan_id",
        "source_id",
        "policy_generation",
    ] {
        let mut row = value("unavailable");
        row[key] = serde_json::Value::Null;
        assert!(serde_json::from_value::<InputDecisionMessage>(row.clone()).is_ok());
        row.as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<InputDecisionMessage>(row).is_err(),
            "{key}"
        );
    }
}
#[test]
fn all_arms_deny_extra_members_and_missing_required_members() {
    for name in ARMS {
        let row = value(name);
        let mut extra = row.clone();
        extra["body"]["extra"] = true.into();
        assert!(
            serde_json::from_value::<InputDecisionMessage>(extra).is_err(),
            "{name}"
        );
        for key in row["body"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| k.as_str() != "kind")
        {
            let mut missing = row.clone();
            missing["body"].as_object_mut().unwrap().remove(key);
            assert!(
                serde_json::from_value::<InputDecisionMessage>(missing).is_err(),
                "{name}.{key}"
            );
        }
    }
}
#[test]
fn options_have_exact_closed_vocabulary_and_types() {
    for control in [
        "none",
        "detach",
        "cancel_pre_dispatch",
        "cancel_deferred_boundary",
    ] {
        let mut row = value("text-answer");
        row["requested_options"]["control"] = control.into();
        assert!(serde_json::from_value::<InputDecisionMessage>(row).is_ok());
    }
    for key in ["force", "trace", "control", "deploy_profile"] {
        let mut row = value("text-answer");
        row["requested_options"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(
            serde_json::from_value::<InputDecisionMessage>(row).is_err(),
            "{key}"
        );
    }
}
