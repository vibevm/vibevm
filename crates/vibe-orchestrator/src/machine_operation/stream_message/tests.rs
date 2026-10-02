use super::*;
use serde_json::{Value, json};

fn binding() -> ExpectedBinding<'static> {
    ExpectedBinding {
        context_id: "context:one",
        target_id: None,
        operation_id: "rust:domain:operation",
        run_id: "run:one",
        stream_id: "stream:one",
    }
}
fn value(body: Value) -> Value {
    json!({"schema":1,"context_id":"context:one","target_id":null,
        "operation_id":"rust:domain:operation","run_id":"run:one","stream_id":"stream:one","body":body})
}
fn check(v: Value) -> Check {
    let typed: OperationStreamMessage = serde_json::from_value(v).unwrap();
    validate(&typed, binding())
}
fn event_value(sequence: &str, payload: Value) -> Value {
    json!({"sequence":sequence,"payload":payload})
}
fn diagnostic(sequence: &str) -> Value {
    event_value(
        sequence,
        json!({"type":"diagnostic","severity":"info","message":"observed"}),
    )
}
fn event_body(payload: Value) -> Value {
    json!({"kind":"event","event":event_value("1",payload)})
}
fn replay(after: &str, newest: Value, events: Vec<Value>, gap: Value, expired: bool) -> Value {
    json!({"kind":"replay_reply","after_sequence":after,"newest_sequence":newest,
        "events":events,"gap":gap,"terminal_ref":"result:one","expired":expired})
}
fn gap_value(from: &str, to: &str, reason: &str) -> Value {
    json!({"from_sequence":from,"to_sequence":to,"reason":reason,"terminal_ref":"result:one"})
}
fn control(action: &str, status: &str, effect: &str, result: Value) -> Value {
    value(
        json!({"kind":"control_reply","request_id":"request:one","action":action,
        "status":status,"operation_effect_state":effect,"result_ref":result}),
    )
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM",
    r = 1
)]
fn corpus_values_pass_pure_validation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/machine-operation-stream-message/e1/valid");
    let mut count = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        parse(&bytes, binding()).unwrap();
        count += 1;
    }
    assert_eq!(count, 12);
}

#[test]
fn exact_binding_and_bounded_opaque_ids_are_not_authority() {
    let original = value(json!({"kind":"event","event":diagnostic("1")}));
    for key in ["context_id", "target_id", "operation_id", "stream_id"] {
        let mut foreign = original.clone();
        foreign[key] = json!(if key == "operation_id" {
            "rust:foreign:operation"
        } else {
            "foreign:one"
        });
        assert_eq!(check(foreign), Err(StreamValidationError::ScopeMismatch));
    }
    let mut unknown = original.clone();
    unknown["run_id"] = json!("run:unknown");
    assert_eq!(check(unknown), Err(StreamValidationError::UnknownRun));
    for id in ["", " padded", "control\n", &"x".repeat(257)] {
        let mut invalid = original.clone();
        invalid["context_id"] = json!(id);
        assert!(check(invalid).is_err());
    }
    let mut bad_epoch = original.clone();
    bad_epoch["schema"] = json!(2);
    assert_eq!(
        check(bad_epoch),
        Err(StreamValidationError::UnsupportedEpoch)
    );
    let mut unnamespaced = original;
    unnamespaced["operation_id"] = json!("operation");
    assert!(check(unnamespaced).is_err());
    let mut bounded: OperationStreamMessage =
        serde_json::from_value(value(json!({"kind":"event","event":diagnostic("1")}))).unwrap();
    bounded.context_id = "é".repeat(128);
    let selected = ExpectedBinding {
        context_id: &bounded.context_id,
        ..binding()
    };
    validate(&bounded, selected).unwrap();
    bounded.context_id.push('é');
    assert!(validate(&bounded, binding()).is_err());
}

#[test]
fn decimal_precision_and_overflow_are_preserved() {
    for seq in ["9007199254740993", "18446744073709551615"] {
        check(value(json!({"kind":"event","event":diagnostic(seq)}))).unwrap();
    }
    for seq in ["0", "01", "+1", "-1", "1.0", "18446744073709551616"] {
        assert!(check(value(json!({"kind":"event","event":diagnostic(seq)}))).is_err());
    }
    check(value(replay(
        "18446744073709551615",
        json!("18446744073709551615"),
        vec![],
        Value::Null,
        false,
    )))
    .unwrap();
    assert!(
        check(value(replay(
            "18446744073709551615",
            json!("18446744073709551615"),
            vec![diagnostic("1")],
            Value::Null,
            false
        )))
        .is_err()
    );
}

#[test]
fn progress_raw_bytes_and_utf8_limits_are_semantic() {
    let progress = |current: &str, total: Value| {
        value(event_body(
            json!({"type":"progress","current":current,"total":total,"unit":"bytes"}),
        ))
    };
    check(progress(
        "18446744073709551615",
        json!("18446744073709551615"),
    ))
    .unwrap();
    check(progress("0", Value::Null)).unwrap();
    assert!(check(progress("11", json!("10"))).is_err());
    assert!(check(progress("0", json!("00"))).is_err());
    let raw = |hex: String, count: u32| {
        value(event_body(
            json!({"type":"raw_output","origin":"stderr","byte_offset":"0","hex_bytes":hex,"byte_count":count,"truncated":true}),
        ))
    };
    check(raw("ff".repeat(2048), 2048)).unwrap();
    check(raw(String::new(), 0)).unwrap();
    for (hex, count) in [
        ("FF".to_owned(), 1),
        ("f".to_owned(), 1),
        ("gg".to_owned(), 1),
        ("ff".to_owned(), 0),
        ("ff".repeat(2049), 2049),
    ] {
        assert!(check(raw(hex, count)).is_err());
    }
    let diag = |message: String| {
        value(event_body(
            json!({"type":"diagnostic","severity":"warning","message":message}),
        ))
    };
    check(diag("é".repeat(2048))).unwrap();
    assert!(check(diag("é".repeat(2049))).is_err());
}

#[test]
fn state_phase_and_terminal_references_require_coherent_values() {
    let state = |observed: &str, effect: &str, reference: Value| {
        value(event_body(
            json!({"type":"state","observed":observed,"effect_state":effect,"result_ref":reference}),
        ))
    };
    check(state(
        "result_available",
        "completed",
        json!("r".repeat(512)),
    ))
    .unwrap();
    for (observed, effect, result) in [
        ("result_available", "no_effects", Value::Null),
        ("running", "completed", Value::Null),
        ("running", "uncertain", json!(" ")),
        ("result_available", "completed", json!("r".repeat(513))),
    ] {
        assert!(check(state(observed, effect, result)).is_err());
    }
    let phase = |transition: &str, status: Value| {
        value(event_body(
            json!({"type":"phase","phase":"prepare","transition":transition,"status":status}),
        ))
    };
    check(phase("finished", json!("ok"))).unwrap();
    assert!(check(phase("started", json!("ok"))).is_err());
    assert!(check(phase("finished", Value::Null)).is_err());
    assert!(
        check(value(event_body(
            json!({"type":"terminal_reference","effect_state":"uncertain","result_ref":""})
        )))
        .is_err()
    );
}

#[test]
fn replay_gap_contiguity_and_expiry_never_invent_history() {
    check(value(replay(
        "0",
        json!("4"),
        vec![diagnostic("3"), diagnostic("4")],
        gap_value("1", "2", "evicted"),
        false,
    )))
    .unwrap();
    check(value(replay("0", Value::Null, vec![], Value::Null, true))).unwrap();
    check(value(replay(
        "0",
        Value::Null,
        vec![],
        gap_value("1", "2", "expired"),
        true,
    )))
    .unwrap();
    for b in [
        replay("0", Value::Null, vec![], Value::Null, false),
        replay("2", json!("1"), vec![], Value::Null, false),
        replay("0", json!("3"), vec![diagnostic("2")], Value::Null, false),
        replay(
            "0",
            json!("4"),
            vec![diagnostic("2")],
            gap_value("1", "2", "evicted"),
            false,
        ),
        replay(
            "0",
            json!("4"),
            vec![diagnostic("3"), diagnostic("3")],
            gap_value("1", "2", "evicted"),
            false,
        ),
        replay(
            "0",
            json!("2"),
            vec![diagnostic("3")],
            gap_value("1", "2", "evicted"),
            false,
        ),
        replay(
            "0",
            json!("4"),
            vec![],
            gap_value("2", "3", "evicted"),
            false,
        ),
        replay(
            "0",
            json!("4"),
            vec![],
            gap_value("1", "5", "evicted"),
            false,
        ),
        replay(
            "0",
            json!("4"),
            vec![],
            gap_value("2", "1", "evicted"),
            false,
        ),
        replay("0", json!("4"), vec![diagnostic("1")], Value::Null, true),
        replay(
            "0",
            json!("4"),
            vec![],
            gap_value("1", "4", "evicted"),
            true,
        ),
    ] {
        assert!(check(value(b)).is_err());
    }
}

#[test]
fn frame_and_batch_limits_include_serialized_bytes() {
    let original = value(json!({"kind":"event","event":diagnostic("1")}));
    let mut bytes = serde_json::to_vec(&original).unwrap();
    bytes.resize(MAX_FRAME_BYTES, b' ');
    parse(&bytes, binding()).unwrap();
    bytes.push(b' ');
    assert_eq!(
        parse(&bytes, binding()),
        Err(StreamValidationError::FrameLimit)
    );
    let events: Vec<_> = (1..=64).map(|n| diagnostic(&n.to_string())).collect();
    check(value(replay(
        "0",
        json!("64"),
        events.clone(),
        Value::Null,
        false,
    )))
    .unwrap();
    let mut overflow = events;
    overflow.push(diagnostic("65"));
    assert!(
        check(value(replay(
            "0",
            json!("65"),
            overflow,
            Value::Null,
            false
        )))
        .is_err()
    );
    let large: Vec<_> = (1..=64)
        .map(|n| {
            event_value(
                &n.to_string(),
                json!({"type":"diagnostic","severity":"info","message":"x".repeat(4096)}),
            )
        })
        .collect();
    assert_eq!(
        check(value(replay("0", json!("64"), large, Value::Null, false))),
        Err(StreamValidationError::FrameLimit)
    );
    for (max_events, max_bytes) in [(0, 1), (65, 1), (1, 0), (1, 65537)] {
        assert!(check(value(json!({"kind":"replay_request","after_sequence":"0","max_events":max_events,"max_bytes":max_bytes}))).is_err());
    }
}

#[test]
fn cancellation_acknowledgment_does_not_claim_commit_or_rollback() {
    for (action, status, effect, result) in [
        ("cancel", "requested", "no_effects", Value::Null),
        ("cancel", "reached_safe_stop", "no_effects", Value::Null),
        (
            "cancel",
            "reached_safe_stop",
            "uncertain",
            json!("result:partial"),
        ),
        (
            "cancel",
            "already_terminal",
            "completed",
            json!("result:late"),
        ),
        ("detach", "detached", "completed", json!("result:late")),
    ] {
        check(control(action, status, effect, result)).unwrap();
    }
    for (action, status, effect, result) in [
        ("cancel", "requested", "completed", json!("result:one")),
        (
            "cancel",
            "reached_safe_stop",
            "completed",
            json!("result:one"),
        ),
        ("cancel", "reached_safe_stop", "uncertain", Value::Null),
        ("cancel", "already_terminal", "uncertain", Value::Null),
        ("cancel", "detached", "no_effects", Value::Null),
        ("detach", "reached_safe_stop", "no_effects", Value::Null),
        (
            "detach",
            "already_terminal",
            "completed",
            json!("result:one"),
        ),
        ("detach", "detached", "completed", Value::Null),
    ] {
        assert!(check(control(action, status, effect, result)).is_err());
    }
}

#[test]
fn known_unreplayed_events_require_payload_or_explicit_gap() {
    assert!(check(value(replay("0", json!("5"), vec![], Value::Null, false))).is_err());
    assert!(check(value(replay("5", json!("5"), vec![], Value::Null, false))).is_ok());
    assert!(
        check(value(replay(
            "0",
            json!("5"),
            vec![],
            gap_value("1", "5", "evicted"),
            false
        )))
        .is_ok()
    );
}

#[test]
fn active_and_pre_effect_states_cannot_claim_completed() {
    for observed in ["journal_ready", "running"] {
        assert!(
            check(value(event_body(
                json!({"type":"state", "observed":observed,
            "effect_state":"completed", "result_ref":"result:one"})
            )))
            .is_err()
        );
    }
}

#[test]
fn no_effect_terminal_result_can_be_referenced_without_inventing_effects() {
    let v = value(event_body(
        json!({"type":"terminal_reference", "effect_state":"no_effects",
        "result_ref":"result:no-effect-completed"}),
    ));
    let message: OperationStreamMessage =
        serde_json::from_value(v).expect("no-effect terminal is a legitimate domain result");
    assert!(validate(&message, binding()).is_ok());
}

#[test]
fn raw_offsets_require_canonical_u64_and_checked_end() {
    let raw = |offset: &str, hex: &str, count: u32| {
        value(event_body(
            json!({"type":"raw_output","origin":"stdout","byte_offset":offset,
            "hex_bytes":hex,"byte_count":count,"truncated":true}),
        ))
    };
    check(raw("18446744073709551615", "", 0)).unwrap();
    check(raw("18446744073709551614", "ff", 1)).unwrap();
    assert!(check(raw("18446744073709551615", "ff", 1)).is_err());
    for offset in ["", "00", "+0", "-1", "1.0", "18446744073709551616"] {
        assert!(check(raw(offset, "", 0)).is_err());
    }
}
