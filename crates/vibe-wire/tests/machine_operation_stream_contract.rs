//! Generated shape checks only; stream values grant no operation authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM");

use vibe_wire::generated::machine::e1::operation_stream_message::OperationStreamMessage;
#[path = "wire_support/mod.rs"]
mod wire_support;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM",
    r = 1
)]
fn all_body_and_payload_arms_roundtrip() {
    for name in [
        "event-state",
        "event-phase",
        "event-progress",
        "event-diagnostic",
        "event-raw_output",
        "event-terminal_reference",
        "event-no-effect-terminal",
        "gap",
        "replay-request",
        "replay-reply",
        "control-request",
        "control-reply",
    ] {
        let path = format!("formats/corpora/machine-operation-stream-message/e1/valid/{name}.json");
        let value: OperationStreamMessage = wire_support::round_trip(&path);
        assert_eq!(value.schema, 1);
        assert!(value.target_id.is_none());
    }
}

#[test]
fn strict_generated_reader_refuses_structural_errors() {
    for name in [
        "unknown-root",
        "unknown-arm",
        "wrong-type",
        "missing-nullable",
        "unknown-enum",
    ] {
        let path = format!(
            "formats/corpora/machine-operation-stream-message/e1/structural-invalid/{name}.json"
        );
        assert!(
            serde_json::from_value::<OperationStreamMessage>(wire_support::read_json(&path))
                .is_err(),
            "{path}"
        );
    }
    // Required nullable members are required at every nesting level.
    for (name, container, key) in [
        ("event-state", "payload", "result_ref"),
        ("event-phase", "payload", "status"),
        ("event-progress", "payload", "total"),
        ("replay-reply", "body", "newest_sequence"),
        ("replay-reply", "body", "gap"),
        ("replay-reply", "body", "terminal_ref"),
        ("control-reply", "body", "result_ref"),
        ("gap", "body", "terminal_ref"),
    ] {
        let mut value = wire_support::read_json(&format!(
            "formats/corpora/machine-operation-stream-message/e1/valid/{name}.json"
        ));
        let object = if container == "payload" {
            &mut value["body"]["event"]["payload"]
        } else {
            &mut value["body"]
        };
        object.as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<OperationStreamMessage>(value).is_err(),
            "missing {name}.{key}"
        );
    }
}
