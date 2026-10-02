//! Closed generated action data does not construct admission authority.
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        lifecycle_action_error::LifecycleActionError, lifecycle_action_reply::LifecycleActionReply,
        lifecycle_action_request::LifecycleActionRequest,
    },
};
#[path = "wire_support/mod.rs"]
mod wire_support;
fn corpus<T: DeserializeOwned + Serialize>(format: &str, names: &[&str]) {
    for name in names {
        let path =
            format!("formats/corpora/machine-lifecycle-action-{format}/e1/valid/{name}.json");
        let _: T = wire_support::round_trip(&path);
        let row = wire_support::read_json(&path);
        for key in row.as_object().unwrap().keys() {
            let mut missing = row.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                serde_json::from_value::<T>(missing).is_err(),
                "{format}.{name}.{key}"
            );
        }
    }
    let extra = wire_support::read_json(&format!(
        "formats/corpora/machine-lifecycle-action-{format}/e1/structural-invalid/extra.json"
    ));
    assert!(serde_json::from_value::<T>(extra).is_err());
    assert!(
        FormatId::ALL
            .iter()
            .any(|f| f.id() == format!("machine-lifecycle-action-{format}") && f.epoch() == 1)
    );
}
#[test]
fn all_action_reply_error_roots_roundtrip_and_register() {
    corpus::<LifecycleActionRequest>(
        "request",
        &[
            "prepare", "answer", "submit", "inspect", "attach", "replay", "control", "detach",
        ],
    );
    corpus::<LifecycleActionReply>(
        "reply",
        &[
            "question",
            "validated",
            "submitted",
            "view",
            "absent",
            "attached",
            "stream_response",
            "detached",
        ],
    );
    corpus::<LifecycleActionError>("error", &["unavailable"]);
}
#[test]
fn all_action_arms_and_options_are_closed_and_required() {
    for name in [
        "prepare", "answer", "submit", "inspect", "attach", "replay", "control", "detach",
    ] {
        let row = wire_support::read_json(&format!(
            "formats/corpora/machine-lifecycle-action-request/e1/valid/{name}.json"
        ));
        let mut extra = row.clone();
        extra["action"]["grant"] = json!(true);
        assert!(serde_json::from_value::<LifecycleActionRequest>(extra).is_err());
        for key in row["action"].as_object().unwrap().keys() {
            let mut missing = row.clone();
            missing["action"].as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<LifecycleActionRequest>(missing).is_err());
        }
    }
    let base = wire_support::read_json(
        "formats/corpora/machine-lifecycle-action-request/e1/valid/prepare.json",
    );
    for key in ["force", "trace", "deploy_profile", "control"] {
        let mut row = base.clone();
        row["action"]["requested_options"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(serde_json::from_value::<LifecycleActionRequest>(row).is_err());
    }
    let mut row = base;
    row["action"]["requested_options"]["control"] = json!("kill");
    assert!(serde_json::from_value::<LifecycleActionRequest>(row).is_err());
}
#[test]
fn every_error_nullable_distinguishes_missing_from_null() {
    let base = wire_support::read_json(
        "formats/corpora/machine-lifecycle-action-error/e1/valid/unavailable.json",
    );
    for key in [
        "requested_action",
        "requested_phase",
        "requested_scope",
        "scope_unavailable_reason",
        "known_run_id",
        "view_json",
    ] {
        let mut row = base.clone();
        row[key] = Value::Null;
        assert!(serde_json::from_value::<LifecycleActionError>(row.clone()).is_ok());
        row.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<LifecycleActionError>(row).is_err());
    }
}

#[test]
fn observer_replies_are_closed_and_all_binding_fields_required() {
    for name in ["attached", "stream_response", "detached"] {
        let base = wire_support::read_json(&format!(
            "formats/corpora/machine-lifecycle-action-reply/e1/valid/{name}.json"
        ));
        let mut extra = base.clone();
        extra["reply"]["grant"] = json!(true);
        assert!(serde_json::from_value::<LifecycleActionReply>(extra).is_err());
        for key in base["reply"].as_object().unwrap().keys() {
            let mut missing = base.clone();
            missing["reply"].as_object_mut().unwrap().remove(key);
            assert!(
                serde_json::from_value::<LifecycleActionReply>(missing).is_err(),
                "{name}.{key}"
            );
        }
    }
    let mut wrong = wire_support::read_json(
        "formats/corpora/machine-lifecycle-action-reply/e1/valid/stream_response.json",
    );
    wrong["reply"]["format_id"] = json!("arbitrary-json");
    assert!(serde_json::from_value::<LifecycleActionReply>(wrong).is_err());
}
