use super::super::tests::{begin, finish, setup};
use super::*;
use vibe_lifecycle::process::{CaptureEvidence, CaptureFault, CaptureFaultStage, ProcessOutput};

fn evidence() -> ObservedOutcome {
    let stream = CaptureEvidence {
        captured: true,
        eof: true,
        counters_complete: true,
        ..Default::default()
    };
    ObservedOutcome {
        output: ProcessOutput {
            code: Some(0),
            ..Default::default()
        },
        exit_observed: true,
        stdout: stream.clone(),
        stderr: stream,
        faults: Vec::new(),
    }
}
#[test]
fn deferred_stop_requires_actual_exit_complete_drains_and_fault_coherence() {
    let good = evidence();
    assert!(quiescent(&NativeEvidence::from_system(&good)));
    let mut lost = good.clone();
    lost.stdout.truncated = true;
    lost.stdout.dropped_delivery_bytes = 42;
    lost.stdout.discarded_prefix_bytes = 42;
    assert!(
        quiescent(&NativeEvidence::from_system(&lost)),
        "observation loss alone is not a drain failure"
    );
    let mut no_exit = good.clone();
    no_exit.exit_observed = false;
    assert!(
        !quiescent(&NativeEvidence::from_system(&no_exit)),
        "code Some(0) is insufficient"
    );
    let mut held_pipe = good.clone();
    held_pipe.stderr.eof = false;
    assert!(!quiescent(&NativeEvidence::from_system(&held_pipe)));
    let mut absent = good.clone();
    absent.stdout.captured = false;
    assert!(!quiescent(&NativeEvidence::from_system(&absent)));
    let fault = CaptureFault {
        stage: CaptureFaultStage::Read,
        origin: None,
        reason: "fault".into(),
    };
    let mut bad = good.clone();
    bad.stdout.faults.push(fault.clone());
    assert!(!quiescent(&NativeEvidence::from_system(&bad)));
    let mut bad = good;
    bad.faults.push(fault);
    assert!(!quiescent(&NativeEvidence::from_system(&bad)));
}
fn cancel(binding: &Binding, id: &str) -> OperationStreamMessage {
    binding
        .history
        .lock()
        .unwrap()
        .message(Body::ControlRequest(Box::new(BodyControlRequest {
            action: BodyControlRequestAction::Cancel,
            after_sequence: "0".into(),
            request_id: id.into(),
        })))
}
fn reply_body(message: OperationStreamMessage) -> BodyControlReply {
    let Body::ControlReply(row) = message.body else {
        panic!("control reply")
    };
    *row
}
#[test]
fn deferred_stop_fixture_code_never_mints_native_proof_and_conflicts_cannot_relatch() {
    let s = setup(false);
    let (owner, mut observer, binding) = begin(&s, None);
    // Fault fixture deliberately opens admission; it cannot create native exit
    // evidence even though its legacy runner returns code Some(0).
    *owner.shared.record.lock().unwrap().gate_mut() = Gate::DeferredOpen;
    let frame = cancel(&binding, "deferred-one");
    for _ in 0..2 {
        let row = reply_body(
            owner
                .control(&binding, &s.coordinates, &mut observer, &frame)
                .unwrap(),
        );
        assert_eq!(row.status, BodyControlReplyStatus::Requested);
        assert_eq!(
            row.operation_effect_state,
            BodyControlReplyOperationEffectState::Uncertain
        );
    }
    assert!(
        owner
            .control(
                &binding,
                &s.coordinates,
                &mut observer,
                &cancel(&binding, "foreign-id")
            )
            .is_err()
    );
    let mut malformed = frame.clone();
    malformed.stream_id.push_str("foreign");
    assert!(
        owner
            .control(&binding, &s.coordinates, &mut observer, &malformed)
            .is_err()
    );
    assert!(matches!(
        observer.inspect().unwrap(),
        Observation::Running { .. }
    ));
    finish(&owner, &s);
    assert!(matches!(
        owner.inspect().unwrap(),
        Observation::RecoveryRequired { .. }
    ));
    assert_eq!(
        reply_body(
            owner
                .control(&binding, &s.coordinates, &mut observer, &frame)
                .unwrap()
        )
        .status,
        BodyControlReplyStatus::Unavailable
    );
}
#[test]
fn deferred_stop_publication_winner_closes_admission_before_normal_marker() {
    let s = setup(false);
    let (owner, mut observer, binding) = begin(&s, None);
    *owner.shared.record.lock().unwrap().gate_mut() = Gate::DeferredOpen;
    let ready = ReadyObserver {
        shared: owner.shared.clone(),
        physical_root: Mutex::new(None),
        before_effect: None,
    };
    // This deterministic direct-owner boundary wins while the fake provider is
    // held. It only tests admission closure, never certifies quiescence.
    assert!(ready.close_deferred_gate().unwrap().is_none());
    assert_eq!(
        reply_body(
            owner
                .control(
                    &binding,
                    &s.coordinates,
                    &mut observer,
                    &cancel(&binding, "late")
                )
                .unwrap()
        )
        .status,
        BodyControlReplyStatus::Unavailable
    );
    finish(&owner, &s);
    assert_eq!(
        reply_body(
            owner
                .control(
                    &binding,
                    &s.coordinates,
                    &mut observer,
                    &cancel(&binding, "late")
                )
                .unwrap()
        )
        .status,
        BodyControlReplyStatus::AlreadyTerminal
    );
}
