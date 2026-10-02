use super::*;

#[test]
fn real_provider_tcp_fragmented_replay_control_eof_and_fresh_result() {
    let f = Fixture::new(false);
    let (mut s, mut c) = f.session(true, false);
    let ids = initialized(&mut s, &mut c, true);
    let bytes = encoded(&request(&s, &ids, "fragmented", replay()));
    for byte in &bytes[..bytes.len() - 1] {
        c.socket.write_all(&[*byte]).unwrap();
        s.poll().unwrap();
    }
    assert!(
        !s.pending
            .iter()
            .any(|(v, _)| v.request_id.as_deref() == Some("fragmented"))
    );
    c.socket.write_all(&bytes[bytes.len() - 1..]).unwrap();
    let stream::Body::ReplayReply(reply) = inner(c.response(&mut s, "fragmented")).body else {
        panic!("replay")
    };
    assert!(
        reply
            .events
            .iter()
            .any(|e| matches!(e.payload, stream::Payload::Phase(_)))
    );
    assert!(
        reply
            .events
            .iter()
            .any(|e| matches!(e.payload, stream::Payload::Progress(_)))
    );
    let mut query = 0;
    until(|| {
        let id = format!("blocked-raw-{query}");
        query += 1;
        c.send(&request(&s, &ids, &id, replay()));
        let stream::Body::ReplayReply(row) = inner(c.response(&mut s, &id)).body else {
            panic!("replay")
        };
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
    c.send(&request(&s, &ids, "active-cancel", cancel("active-cancel")));
    assert_eq!(
        control_status(c.response(&mut s, "active-cancel")),
        stream::BodyControlReplyStatus::Unavailable
    );
    c.send(&request(&s, &ids, "fragmented", replay()));
    assert!(matches!(
        c.response(&mut s, "fragmented").body,
        frame::Body::Refusal(_)
    ));
    drop(c);
    until(|| s.poll().unwrap() == PollStatus::Closed);
    assert!(matches!(
        f.host.inspect().unwrap(),
        FixedOperationStatus::Running { .. }
    ));
    f.finish();
    let (mut terminal, mut peer) = f.session(true, false);
    let ids = initialized(&mut terminal, &mut peer, true);
    peer.send(&request(&terminal, &ids, "final-replay", replay()));
    let stream::Body::ReplayReply(reply) = inner(peer.response(&mut terminal, "final-replay")).body
    else {
        panic!("terminal replay")
    };
    assert!(reply.terminal_ref.is_some());
    let mut raw = [Vec::new(), Vec::new()];
    for event in reply.events {
        if let stream::Payload::RawOutput(row) = event.payload {
            let i = usize::from(row.origin == stream::PayloadRawOutputOrigin::Stderr);
            assert_eq!(row.byte_offset, raw[i].len().to_string());
            for pair in row.hex_bytes.as_bytes().chunks_exact(2) {
                raw[i].push(u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap());
            }
        }
    }
    assert_eq!(raw, [vec![255, 65, 0], vec![254, 66, 0]]);
    peer.send(&request(&terminal, &ids, "late", cancel("late")));
    assert_eq!(
        control_status(peer.response(&mut terminal, "late")),
        stream::BodyControlReplyStatus::AlreadyTerminal
    );
    let FixedOperationStatus::ResultAvailable {
        marker,
        report_bytes: Some(report),
        ..
    } = terminal.inspect().unwrap()
    else {
        panic!("fresh result")
    };
    assert_eq!(
        serde_json::to_value(marker).unwrap()["outcome"],
        "completed"
    );
    let report: serde_json::Value = serde_json::from_slice(&report).unwrap();
    assert!(
        report["contributions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["phase"] == "validate" && r["status"] == "ok")
    );
    let path = f.path(".vibe/lifecycle-outcomes").join(format!(
        "{}.marker.json",
        terminal.reader.summary().run_id()
    ));
    let original = std::fs::read(&path).unwrap();
    std::fs::write(&path, b"corrupt").unwrap();
    assert!(matches!(
        terminal.inspect().unwrap(),
        FixedOperationStatus::RecoveryRequired { .. }
    ));
    std::fs::write(path, original).unwrap();
}

#[test]
fn real_provider_tcp_foreign_constructor_and_readonly_control() {
    let a = Fixture::new(false);
    let b = Fixture::new(false);
    let (port, mut rejected_peer) = pair(false);
    assert!(matches!(
        ResponsiveSession::new(a.host.attach().unwrap(), Some(b.host.grant_control()), port),
        Err(SessionBuildError::ObserverUnavailable)
    ));
    assert_eq!(
        rejected_peer.socket.read(&mut [0u8; 1]).unwrap(),
        0,
        "foreign pairing closes before any initialization frame"
    );
    for f in [&a, &b] {
        assert!(matches!(
            f.host.inspect().unwrap(),
            FixedOperationStatus::Running { .. }
        ));
    }
    let mut detached = a.host.attach().unwrap();
    detached.detach();
    assert!(a.host.grant_control().validate_observer(&detached).is_err());
    let (port, _peer) = pair(false);
    assert!(matches!(
        ResponsiveSession::new(detached, Some(a.host.grant_control()), port),
        Err(SessionBuildError::ObserverUnavailable)
    ));
    let (mut s, mut c) = a.session(false, false);
    let ids = initialized(&mut s, &mut c, false);
    c.send(&request(
        &s,
        &ids,
        "readonly-cancel",
        cancel("readonly-cancel"),
    ));
    assert_eq!(
        control_status(c.response(&mut s, "readonly-cancel")),
        stream::BodyControlReplyStatus::Unavailable
    );
    let detach = stream::Body::ControlRequest(Box::new(stream::BodyControlRequest {
        action: stream::BodyControlRequestAction::Detach,
        after_sequence: "0".into(),
        request_id: "detach".into(),
    }));
    c.send(&request(&s, &ids, "detach", detach));
    assert_eq!(
        control_status(c.response(&mut s, "detach")),
        stream::BodyControlReplyStatus::Detached
    );
    assert!(matches!(
        a.host.inspect().unwrap(),
        FixedOperationStatus::Running { .. }
    ));
    a.finish();
    b.finish();
}

#[test]
fn real_provider_tcp_outer_and_inner_scope_refusals() {
    let f = Fixture::new(false);
    let (mut s, mut c) = f.session(true, false);
    for change in 0..5 {
        let mut bad = offer();
        let frame::Body::InitializeOffer(row) = &mut bad.body else {
            unreachable!()
        };
        match change {
            0 => {
                row.schema_offers.pop();
            }
            1 => row.schema_offers[0].epoch = 2,
            2 => row.api_versions = vec!["future".into()],
            3 => row.position_encodings = vec!["utf16".into()],
            _ => bad.session_id = Some("client-selected".into()),
        }
        c.send(&bad);
        assert!(matches!(
            c.response(&mut s, "init").body,
            frame::Body::InitializationRefused(_)
        ));
    }
    let ids = initialized(&mut s, &mut c, true);
    for n in 0..9 {
        let id = format!("foreign-{n}");
        let mut value = request(&s, &ids, &id, cancel(&id));
        match n {
            0 => value.session_id = Some("old-session".into()),
            1 => {
                let frame::Body::Request(row) = &mut value.body else {
                    unreachable!()
                };
                row.context_id = "foreign".into();
            }
            2 => {
                let frame::Body::Request(row) = &mut value.body else {
                    unreachable!()
                };
                row.target_id = Some("foreign".into());
            }
            _ => {
                let frame::Body::Request(row) = &mut value.body else {
                    unreachable!()
                };
                let mut message = s.stream_request(cancel(&id));
                match n {
                    3 => message.context_id = "foreign".into(),
                    4 => message.target_id = Some("foreign".into()),
                    5 => message.run_id = "foreign".into(),
                    6 => message.stream_id = "foreign".into(),
                    7 => message.schema = 2,
                    _ => message.operation_id = "foreign".into(),
                }
                row.envelope_json = serde_json::to_string(&message).unwrap();
            }
        }
        c.send(&value);
        let frame::Body::Refusal(refusal) = c.response(&mut s, &id).body else {
            panic!("foreign request must be refused");
        };
        assert_eq!(
            refusal.outcome.effect_state,
            frame::OutcomeEffectState::NoEffects
        );
        assert!(refusal.envelope_json.is_none());
    }
    assert!(matches!(
        f.host.inspect().unwrap(),
        FixedOperationStatus::Running { .. }
    ));
    f.finish();
}

#[test]
fn real_provider_tcp_blocked_writer_queries_capacity_and_no_detach_mutation() {
    let f = Fixture::new(false);
    let (mut s, mut c) = f.session(true, true);
    c.send(&frame::SessionFrame {
        schema: 1,
        session_id: None,
        request_id: Some("preinit".into()),
        body: frame::Body::Request(Box::new(frame::BodyRequest {
            context_id: "unassigned".into(),
            target_id: None,
            envelope_json: "{}".into(),
        })),
    });
    until(|| {
        s.poll().unwrap();
        s.active_bytes > 0
    });
    c.send(&offer());
    until(|| {
        s.poll().unwrap();
        s.session.phase() == SessionPhase::Initialized
    });
    let (session, row) = s
        .pending
        .iter()
        .find_map(|(v, _)| match &v.body {
            frame::Body::Initialized(row) => {
                Some((v.session_id.clone().unwrap(), row.contexts[0].clone()))
            }
            _ => None,
        })
        .expect("actual initialized descriptor remains queued behind blocked active frame");
    let ids = Assigned {
        session,
        context: row.context_id,
        target: row.target_ids.first().cloned(),
    };
    // Fill only through real inbound replay/control, never queue injection.
    let mut n = 0;
    while s.can_reserve() {
        let id = format!("blocked-{n}");
        let body = if n % 2 == 0 { replay() } else { cancel(&id) };
        c.send(&request(&s, &ids, &id, body));
        until(|| {
            s.poll().unwrap();
            s.pending
                .iter()
                .any(|(v, _)| v.request_id.as_deref() == Some(&id))
        });
        assert!(s.queued_frames() <= MAX_DELIVERY_FRAMES && s.queued_bytes() <= MAX_DELIVERY_BYTES);
        if n % 2 == 1 {
            let reply = s
                .pending
                .iter()
                .find(|(v, _)| v.request_id.as_deref() == Some(&id))
                .unwrap()
                .0
                .clone();
            assert_eq!(
                control_status(reply),
                stream::BodyControlReplyStatus::Unavailable
            );
        }
        n += 1;
        assert!(n <= 16);
    }
    let cursor = s.cursor;
    s.auto_event().unwrap();
    assert_eq!(s.cursor, cursor);
    let body = stream::Body::ControlRequest(Box::new(stream::BodyControlRequest {
        action: stream::BodyControlRequestAction::Detach,
        after_sequence: "0".into(),
        request_id: "full-detach".into(),
    }));
    c.send(&request(&s, &ids, "full-detach", body));
    let mut overloaded = false;
    until(|| match s.poll() {
        Err(SessionError::Overloaded { binding }) => {
            assert_eq!(binding.run_id(), s.reader.summary().run_id());
            overloaded = true;
            true
        }
        Ok(_) => false,
        other => panic!("expected overload: {other:?}"),
    });
    assert!(overloaded);
    // Connection teardown detached, but control was never dispatched.
    assert!(!s.session.correlation_exhausted());
    assert!(matches!(
        f.host.inspect().unwrap(),
        FixedOperationStatus::Running { .. }
    ));
    f.finish();
}

#[test]
fn real_provider_tcp_malformed_and_session_drop_preserve_host_job() {
    let f = Fixture::new(false);
    let (mut s, mut c) = f.session(true, false);
    let _ids = initialized(&mut s, &mut c, true);
    c.socket
        .write_all(&((MAX_FRAME_BYTES + 1) as u32).to_be_bytes())
        .unwrap();
    until(|| match s.poll() {
        Err(SessionError::Transport { binding, .. }) => {
            assert_eq!(binding.run_id(), s.reader.summary().run_id());
            true
        }
        Ok(_) => false,
        other => panic!("malformed frame must detach: {other:?}"),
    });
    assert_eq!(s.poll().unwrap(), PollStatus::Closed);
    let (other, _peer) = f.session(true, false);
    drop(other);
    assert!(matches!(
        f.host.inspect().unwrap(),
        FixedOperationStatus::Running { .. }
    ));
    f.finish();
}

#[test]
fn real_provider_tcp_history_eviction_terminal_reference_and_fresh_authority() {
    let f = Fixture::new(true);
    let (mut s, mut c) = f.session(false, false);
    let ids = initialized(&mut s, &mut c, false);
    f.finish();
    c.send(&request(&s, &ids, "evicted", replay()));
    let stream::Body::ReplayReply(reply) = inner(c.response(&mut s, "evicted")).body else {
        panic!("replay")
    };
    assert!(
        reply.gap.is_some(),
        "actual provider flood must evict bounded history"
    );
    assert!(reply.terminal_ref.is_some());
    assert!(matches!(
        s.inspect().unwrap(),
        FixedOperationStatus::ResultAvailable {
            report_bytes: Some(_),
            ..
        }
    ));
}
