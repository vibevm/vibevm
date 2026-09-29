//! Generated machine-session frame structure; negotiation and dispatch are owner work.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

use serde::de::DeserializeOwned;
use vibe_wire::generated::machine::e1::session_frame::{Body, SessionFrame};

#[path = "wire_support/mod.rs"]
mod wire_support;

fn fixture(class: &str, name: &str) -> String {
    format!("formats/corpora/machine-session-frame/e1/{class}/{name}.json")
}

fn reader_valid(name: &str) -> SessionFrame {
    let path = fixture("valid", name);
    let value: SessionFrame = wire_support::round_trip(&path);
    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        wire_support::read_json(&path)
    );
    value
}

fn reader_refuses<T: DeserializeOwned>(name: &str) {
    let path = fixture("structural-invalid", name);
    assert!(
        serde_json::from_value::<T>(wire_support::read_json(&path)).is_err(),
        "{path}"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn seven_closed_arms_roundtrip_as_generated_values() {
    for name in [
        "initialize-offer",
        "initialized",
        "initialization-refused",
        "request",
        "event",
        "complete",
        "refusal",
    ] {
        let _ = reader_valid(name);
    }
    let initialized = reader_valid("initialized");
    let Body::Initialized(value) = initialized.body else {
        panic!("initialized fixture changed arm")
    };
    assert!(!value.actual_capabilities.streaming);
    assert!(!value.actual_capabilities.replay);
    assert!(!value.actual_capabilities.control);
    assert_eq!(value.legacy_mcp_observed_version, "2024-11-05");
    let request = reader_valid("request");
    let Body::Request(value) = request.body else {
        panic!("request fixture changed arm")
    };
    assert_eq!(value.context_id, "context-a");
    assert!(value.target_id.is_none());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn structural_unknown_missing_and_wrong_type_refuse() {
    for name in [
        "unknown-root",
        "missing-request-id",
        "missing-target-id",
        "unknown-arm",
        "wrong-capability-type",
        "unknown-context-field",
        "wrong-envelope-type",
    ] {
        reader_refuses::<SessionFrame>(name);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION",
    r = 1
)]
fn unsupported_api_version_is_reader_valid_for_later_negotiation_refusal() {
    let path = fixture("semantic-invalid", "unsupported-api-version");
    let value: SessionFrame = serde_json::from_value(wire_support::read_json(&path)).unwrap();
    let Body::InitializeOffer(value) = value.body else {
        panic!("unsupported offer changed arm")
    };
    assert_eq!(value.api_versions, ["future-unknown"]);
}
