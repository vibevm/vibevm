use super::super::tests::{begin, finish, setup};
use super::*;
use std::sync::atomic::Ordering;
fn history() -> History {
    History::new(
        &SubmissionScope {
            context: "context".into(),
            target: "target".into(),
            principal: "host".into(),
        },
        "run",
        "reservation",
    )
}
fn request(h: &History, after: u64, max_events: u16, max_bytes: u32) -> OperationStreamMessage {
    h.message(Body::ReplayRequest(Box::new(BodyReplayRequest {
        after_sequence: after.to_string(),
        max_events,
        max_bytes,
    })))
}
fn diagnostic(n: usize) -> Payload {
    Payload::Diagnostic(Box::new(PayloadDiagnostic {
        message: "x".repeat(n),
        severity: PayloadDiagnosticSeverity::Info,
    }))
}
fn reply(message: OperationStreamMessage) -> BodyReplayReply {
    let Body::ReplayReply(reply) = message.body else {
        panic!("expected replay")
    };
    *reply
}
#[test]
fn hard_count_and_bytes_evict_with_explicit_gap() {
    for size in [1, 4096] {
        let mut h = history();
        for _ in 0..100 {
            h.publish(diagnostic(size));
        }
        assert!(h.count <= 64 && h.bytes <= 65536 && h.slots.len() <= 128);
        let replay = reply(h.replay(&request(&h, 0, 64, 65536)).unwrap());
        let gap = replay.gap.unwrap();
        assert_eq!(gap.reason, GapReason::Evicted);
        assert_eq!(gap.from_sequence, "1");
        assert_eq!(
            replay.events[0].sequence.parse::<u64>().unwrap(),
            gap.to_sequence.parse::<u64>().unwrap() + 1
        );
        assert_eq!(replay.newest_sequence.as_deref(), Some("100"));
    }
}
#[test]
fn oversized_producer_has_visible_loss_and_bounded_metadata() {
    let mut h = history();
    h.publish(diagnostic(65537));
    h.publish(diagnostic(1));
    let replay = reply(h.replay(&request(&h, 0, 64, 65536)).unwrap());
    assert_eq!(replay.gap.unwrap().reason, GapReason::Oversize);
    assert_eq!(replay.events[0].sequence, "2");
    for _ in 0..200 {
        h.publish(diagnostic(65537));
        h.publish(diagnostic(1));
    }
    assert!(h.slots.len() <= 128);
    assert!(h.expired);
}
#[test]
fn reply_budget_future_foreign_and_body_are_refused() {
    let mut h = history();
    h.publish(diagnostic(4096));
    assert_eq!(
        h.replay(&request(&h, 0, 64, 1)),
        Err(ReplayError::ReplyBudget)
    );
    assert_eq!(
        h.replay(&request(&h, 2, 64, 65536)),
        Err(ReplayError::FutureCursor)
    );
    for field in 0..5 {
        let mut req = request(&h, 0, 64, 65536);
        match field {
            0 => req.context_id.push('x'),
            1 => req.target_id = None,
            2 => req.operation_id.push('x'),
            3 => req.run_id.push('x'),
            _ => req.stream_id.push('x'),
        }
        assert!(matches!(
            h.replay(&req),
            Err(ReplayError::InvalidMessage(_))
        ));
    }
    let event = h.message(Body::Event(Box::new(BodyEvent {
        event: EventRecord {
            sequence: "1".into(),
            payload: diagnostic(1),
        },
    })));
    assert_eq!(h.replay(&event), Err(ReplayError::WrongBody));
    let before = (h.count, h.bytes, h.tip);
    for _ in 0..100 {
        assert!(h.replay(&request(&h, 0, 1, 1)).is_err());
    }
    assert_eq!(before, (h.count, h.bytes, h.tip));
    let bounded = h.replay(&request(&h, 0, 1, 65536)).unwrap();
    assert!(serde_json::to_vec(&bounded).unwrap().len() <= 65536);
}
#[test]
fn independent_sequences_and_expiry_preserve_identity() {
    let mut a = history();
    let mut b = History::new(
        &SubmissionScope {
            context: "context".into(),
            target: "target".into(),
            principal: "host".into(),
        },
        "other-run",
        "other-reservation",
    );
    a.publish(diagnostic(1));
    a.publish(diagnostic(1));
    b.publish(diagnostic(1));
    assert_eq!(a.tip, 2);
    assert_eq!(b.tip, 1);
    a.terminal_ref = Some("lifecycle:opaque-domain-ref".into());
    a.expire();
    let r = reply(a.replay(&request(&a, 0, 64, 65536)).unwrap());
    assert!(r.expired && r.events.is_empty() && r.gap.is_none());
    assert_eq!(r.newest_sequence.as_deref(), Some("2"));
    assert_eq!(
        r.terminal_ref.as_deref(),
        Some("lifecycle:opaque-domain-ref")
    );
}
#[test]
fn counted_blocked_owner_replays_then_domain_terminal_and_corruption_stays_recovery() {
    let s = setup(false);
    let (owner, observer, binding) = begin(&s, None);
    let req = {
        let h = binding.history.lock().unwrap();
        request(&h, 0, 64, 65536)
    };
    let r = reply(owner.replay(&binding, &s.coordinates, &req).unwrap());
    let states: Vec<_> = r
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            Payload::State(state) => Some(state),
            _ => None,
        })
        .collect();
    assert_eq!(states.len(), 2);
    assert_eq!(states[0].observed, Observed::JournalReady);
    assert_eq!(states[0].effect_state, Effect::NoEffects);
    assert_eq!(states[1].observed, Observed::Running);
    assert_eq!(states[1].effect_state, Effect::Uncertain);
    for (index, event) in r.events.iter().enumerate() {
        assert_eq!(event.sequence, (index + 1).to_string());
    }
    assert!(
        r.events
            .iter()
            .any(|event| matches!(&event.payload, Payload::Phase(row)
        if row.phase == "validate" && row.transition == PayloadPhaseTransition::Started))
    );
    assert!(matches!(
        owner.inspect().unwrap(),
        Observation::Running { .. }
    ));
    let mut foreign = s.coordinates.clone();
    foreign.scope.context.push('x');
    assert_eq!(
        owner.replay(&binding, &foreign, &req),
        Err(ReplayError::Binding)
    );
    let mut subscriptions = vec![];
    for _ in 1..64 {
        subscriptions.push(owner.attach(&binding, &s.coordinates).unwrap());
    }
    let err = owner.attach(&binding, &s.coordinates).err().unwrap();
    assert!(err.downcast_ref::<SubscriptionLimit>().is_some());
    subscriptions.pop();
    subscriptions.push(owner.attach(&binding, &s.coordinates).unwrap());
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
    finish(&owner, &s);
    let Observation::ResultAvailable { run_id, result } = owner.inspect().unwrap() else {
        panic!("validated domain result")
    };
    let r = reply(owner.replay(&binding, &s.coordinates, &req).unwrap());
    let expected_effect = match result.marker.effect_state {
        LifecycleOutcomeMarkerEffectState::Landed => Effect::Completed,
        LifecycleOutcomeMarkerEffectState::None => Effect::NoEffects,
        LifecycleOutcomeMarkerEffectState::Uncertain => Effect::Uncertain,
    };
    let reference = r.terminal_ref.unwrap();
    assert!(reference.contains(&run_id) && reference.contains(&result.marker.reservation_ref));
    assert!(
        matches!(&r.events.last().unwrap().payload, Payload::State(p) if p.observed == Observed::ResultAvailable && p.effect_state == expected_effect && p.result_ref.as_ref() == Some(&reference))
    );
    let lease = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    let canonical = owner
        .shared
        .record
        .lock()
        .unwrap()
        .canonical_request
        .clone();
    assert_eq!(
        attach_existing_lifecycle(
            &lease,
            &s.coordinates.namespace,
            &s.coordinates.scope,
            &s.coordinates.key,
            &canonical
        )
        .unwrap(),
        Some(SubmissionOutcome::Replayed {
            run_id,
            outcome: result
        })
    );
    drop(lease);
    let marker = s
        .coordinates
        .root
        .join(".vibe/lifecycle-outcomes")
        .join(format!("{}.marker.json", binding.run_id));
    std::fs::write(marker, b"corrupt").unwrap();
    assert!(
        reply(owner.replay(&binding, &s.coordinates, &req).unwrap())
            .terminal_ref
            .is_some()
    );
    assert!(matches!(
        observer.inspect().unwrap(),
        Observation::RecoveryRequired { .. }
    ));
    {
        let mut h = binding.history.lock().unwrap();
        for _ in 0..100 {
            h.publish(diagnostic(1));
        }
    }
    assert!(
        reply(owner.replay(&binding, &s.coordinates, &req).unwrap())
            .gap
            .is_some()
    );
    assert!(matches!(
        owner.inspect().unwrap(),
        Observation::RecoveryRequired { .. }
    ));
    owner.expire_history(&binding, &s.coordinates).unwrap();
    let r = reply(owner.replay(&binding, &s.coordinates, &req).unwrap());
    assert!(r.expired && r.events.is_empty());
    assert_eq!(r.terminal_ref, Some(reference));
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
    drop(LifecycleLease::acquire(&s.coordinates.root).unwrap());
}

#[test]
fn foreign_history_cannot_borrow_an_incumbent_owner_binding() {
    let s = setup(false);
    let (owner, _observer, binding) = begin(&s, None);
    finish(&owner, &s);
    let mut foreign = binding.clone();
    foreign.history = Arc::new(Mutex::new(History::new(
        &s.coordinates.scope,
        &binding.run_id,
        "foreign-reservation",
    )));
    let req = {
        let h = foreign.history.lock().unwrap();
        request(&h, 0, 64, 65536)
    };
    assert_eq!(
        owner.replay(&foreign, &s.coordinates, &req),
        Err(ReplayError::Binding)
    );
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
}
