use super::*;

#[test]
fn envelope_cap_counts_escaped_bytes_before_allocating_payload() {
    let message = stream::OperationStreamMessage {
        schema: 1,
        context_id: "context".into(),
        target_id: None,
        operation_id: "org.test:validate".into(),
        run_id: "run".into(),
        stream_id: "stream".into(),
        body: stream::Body::Event(Box::new(stream::BodyEvent {
            event: stream::EventRecord {
                sequence: "1".into(),
                payload: stream::Payload::Diagnostic(Box::new(stream::PayloadDiagnostic {
                    message: "\u{0000}".repeat(12_000),
                    severity: stream::PayloadDiagnosticSeverity::Error,
                })),
            },
        })),
    };
    assert!(envelope(&message).is_err());
    let mut smaller = message;
    let stream::Body::Event(event) = &mut smaller.body else {
        unreachable!()
    };
    let stream::Payload::Diagnostic(diagnostic) = &mut event.event.payload else {
        unreachable!()
    };
    diagnostic.message = "visible".into();
    let encoded = envelope(&smaller).unwrap();
    assert_eq!(
        serde_json::from_str::<stream::OperationStreamMessage>(&encoded).unwrap(),
        smaller
    );
}

#[test]
fn escaped_outer_frame_must_fit_its_own_codec_limit() {
    let payload = "\u{0000}".repeat(MAX_FRAME_BYTES / 2);
    let value = frame::SessionFrame {
        schema: 1,
        session_id: Some("session".into()),
        request_id: Some("request".into()),
        body: frame::Body::Complete(Box::new(frame::BodyComplete {
            envelope_json: Some(payload),
            outcome: outcome(frame::OutcomeCompleteness::Complete),
        })),
    };
    assert!(serialized_size(&value, MAX_FRAME_BYTES).is_err());
}

// Real public host factory + incumbent SystemProcessRunner. Runtime belongs to
// the accepting parent; no cfg(test) owner, backend, or authority is fabricated.
#[cfg(target_os = "linux")]
mod provider_tcp;
