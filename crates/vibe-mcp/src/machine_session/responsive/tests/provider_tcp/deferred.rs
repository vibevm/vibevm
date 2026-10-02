use super::*;
use vibe_wire::generated::lifecycle_outcome_marker::{
    LifecycleOutcomeMarkerEffectState as EffectState,
    LifecycleOutcomeMarkerOutcome as MarkerOutcome,
};

fn control_reply(value: frame::SessionFrame) -> stream::BodyControlReply {
    let stream::Body::ControlReply(reply) = inner(value).body else {
        panic!("typed control reply")
    };
    *reply
}

#[test]
fn real_provider_tcp_deferred_stop_preserves_partial_result_and_reattached_receipt() {
    let f = Fixture::new_deferred();
    let (mut s, mut c) = f.session(true, false);
    let ids = initialized(&mut s, &mut c, true);
    let run = s.reader.summary().run_id().to_owned();
    c.send(&request(&s, &ids, "deferred-stop", cancel("deferred-stop")));
    let requested = control_reply(c.response(&mut s, "deferred-stop"));
    assert_eq!(requested.request_id, "deferred-stop");
    assert_eq!(requested.status, stream::BodyControlReplyStatus::Requested);
    assert_eq!(
        requested.operation_effect_state,
        stream::BodyControlReplyOperationEffectState::Uncertain
    );
    assert!(requested.result_ref.is_none());

    // Cancellation acknowledges intent while the actual provider remains held.
    // Replay still crosses the socket and retains the incumbent typed events.
    let mut query = 0;
    until(|| {
        let id = format!("held-replay-{query}");
        query += 1;
        c.send(&request(&s, &ids, &id, replay()));
        let message = inner(c.response(&mut s, &id));
        assert_eq!(message.run_id, run);
        let stream::Body::ReplayReply(row) = message.body else {
            panic!("held replay")
        };
        assert!(row.terminal_ref.is_none());
        assert!(
            row.events
                .iter()
                .any(|e| matches!(e.payload, stream::Payload::Phase(_)))
        );
        assert!(
            row.events
                .iter()
                .any(|e| matches!(e.payload, stream::Payload::Progress(_)))
        );
        row.events
            .iter()
            .filter(|e| matches!(e.payload, stream::Payload::RawOutput(_)))
            .count()
            >= 2
    });
    assert!(matches!(
        s.inspect().unwrap(),
        FixedOperationStatus::Running { .. }
    ));
    assert!(!f.path("finished").exists());
    assert_eq!(std::fs::read(f.path("counter")).unwrap(), b"effect\n");

    let (mut readonly, mut peer) = f.session(false, false);
    let read_ids = initialized(&mut readonly, &mut peer, false);
    assert_eq!(readonly.reader.summary().run_id(), run);
    peer.send(&request(
        &readonly,
        &read_ids,
        "readonly-stop",
        cancel("readonly-stop"),
    ));
    assert_eq!(
        control_reply(peer.response(&mut readonly, "readonly-stop")).status,
        stream::BodyControlReplyStatus::Unavailable
    );
    peer.send(&request(
        &readonly,
        &read_ids,
        "readonly-detach",
        stream::Body::ControlRequest(Box::new(stream::BodyControlRequest {
            action: stream::BodyControlRequestAction::Detach,
            after_sequence: "0".into(),
            request_id: "readonly-detach".into(),
        })),
    ));
    assert_eq!(
        control_reply(peer.response(&mut readonly, "readonly-detach")).status,
        stream::BodyControlReplyStatus::Detached
    );
    drop(readonly);
    drop(peer);
    assert!(matches!(
        s.inspect().unwrap(),
        FixedOperationStatus::Running { .. }
    ));
    assert!(!f.path("finished").exists());

    f.finish();
    assert_eq!(std::fs::read(f.path("finished")).unwrap(), b"finished");
    let FixedOperationStatus::ResultAvailable {
        marker,
        report_bytes: Some(report),
        ..
    } = s.inspect().unwrap()
    else {
        panic!("fresh retained partial result")
    };
    assert_eq!(marker.run_id, run);
    assert_eq!(marker.outcome, MarkerOutcome::Uncertain);
    assert_eq!(marker.effect_state, EffectState::Uncertain);
    let report_value: serde_json::Value = serde_json::from_slice(&report).unwrap();
    assert_eq!(report_value["ok"], true);
    assert!(
        report_value["contributions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["phase"] == "validate" && r["status"] == "ok")
    );
    let result_ref = format!("lifecycle:{run}:{}", marker.reservation_ref);
    let outcome_dir = f.path(".vibe/lifecycle-outcomes");
    let marker_path = outcome_dir.join(format!("{run}.marker.json"));
    let receipt_path = outcome_dir.join(format!("{run}.cancel.json"));
    let marker_bytes = std::fs::read(&marker_path).unwrap();
    let receipt_bytes = std::fs::read(&receipt_path).unwrap();

    c.send(&request(&s, &ids, "late-stop", cancel("late-stop")));
    let late = control_reply(c.response(&mut s, "late-stop"));
    assert_eq!(late.status, stream::BodyControlReplyStatus::AlreadyTerminal);
    assert_eq!(
        late.operation_effect_state,
        stream::BodyControlReplyOperationEffectState::Uncertain
    );
    assert_eq!(late.result_ref.as_deref(), Some(result_ref.as_str()));
    c.send(&request(&s, &ids, "partial-replay", replay()));
    let terminal = inner(c.response(&mut s, "partial-replay"));
    assert_eq!(terminal.run_id, run);
    let stream::Body::ReplayReply(row) = terminal.body else {
        panic!("partial terminal replay")
    };
    assert_eq!(row.terminal_ref.as_deref(), Some(result_ref.as_str()));
    let mut raw = [Vec::new(), Vec::new()];
    for event in row.events {
        if let stream::Payload::RawOutput(row) = event.payload {
            let i = usize::from(row.origin == stream::PayloadRawOutputOrigin::Stderr);
            assert_eq!(row.byte_offset, raw[i].len().to_string());
            for pair in row.hex_bytes.as_bytes().chunks_exact(2) {
                raw[i].push(u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap());
            }
        }
    }
    assert_eq!(raw, [vec![255, 65, 0], vec![254, 66, 0]]);
    // Half-close establishes an orderly input EOF even with unread automatic
    // events. Dropping a socket with unread bytes can instead send a TCP reset.
    c.socket.shutdown(std::net::Shutdown::Write).unwrap();
    until(|| s.poll().unwrap() == PollStatus::Closed);
    drop(c);

    // The original inner ID must equal its outer correlation. A fresh
    // negotiated connection allows that same receipt ID without redispatching
    // a duplicate outer ID on the old connection or weakening correlation.
    for _ in 0..2 {
        let (mut attached, mut peer) = f.session(true, false);
        let fresh_ids = initialized(&mut attached, &mut peer, true);
        assert_ne!(fresh_ids.session, ids.session);
        assert_eq!(attached.reader.summary().run_id(), run);
        peer.send(&request(
            &attached,
            &fresh_ids,
            "deferred-stop",
            cancel("deferred-stop"),
        ));
        let reached = inner(peer.response(&mut attached, "deferred-stop"));
        assert_eq!(reached.run_id, run);
        assert_eq!(
            reached,
            serde_json::from_slice::<stream::OperationStreamMessage>(&receipt_bytes).unwrap()
        );
        let stream::Body::ControlReply(reply) = reached.body else {
            panic!("retained receipt")
        };
        assert_eq!(
            reply.status,
            stream::BodyControlReplyStatus::ReachedSafeStop
        );
        assert_eq!(
            reply.operation_effect_state,
            stream::BodyControlReplyOperationEffectState::Uncertain
        );
        assert_eq!(reply.result_ref.as_deref(), Some(result_ref.as_str()));
        let FixedOperationStatus::ResultAvailable {
            marker: fresh_marker,
            report_bytes: Some(fresh_report),
            ..
        } = attached.inspect().unwrap()
        else {
            panic!("fresh domain authority")
        };
        assert_eq!(fresh_marker, marker);
        assert_eq!(fresh_report, report);
        assert_eq!(std::fs::read(&marker_path).unwrap(), marker_bytes);
        assert_eq!(std::fs::read(&receipt_path).unwrap(), receipt_bytes);
        assert_eq!(std::fs::read(f.path("counter")).unwrap(), b"effect\n");
    }
}
