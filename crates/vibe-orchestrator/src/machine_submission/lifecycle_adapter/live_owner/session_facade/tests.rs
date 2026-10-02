//! Compile-only worker fixtures. Parent owns execution and acceptance.
use super::super::tests::{Setup, begin, finish, setup};
use super::*;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use vibe_wire::generated::machine::e1::operation_stream_message::*;

fn wrap(owner: LiveOwner, coordinates: HostCoordinates) -> HostOwnedFixedOperation {
    HostOwnedFixedOperation {
        inner: Arc::new(Inner { owner, coordinates }),
    }
}
fn running(s: &Setup) -> HostOwnedFixedOperation {
    let (owner, observer, _) = begin(s, None);
    drop(observer);
    wrap(owner, s.coordinates.clone())
}
fn message(summary: &BindingSummary, body: Body) -> OperationStreamMessage {
    OperationStreamMessage {
        schema: 1,
        context_id: summary.context_id().into(),
        target_id: summary.target_id().map(str::to_owned),
        operation_id: summary.operation_id().into(),
        run_id: summary.run_id().into(),
        stream_id: summary.stream_id().into(),
        body,
    }
}
fn replay(summary: &BindingSummary) -> OperationStreamMessage {
    message(
        summary,
        Body::ReplayRequest(Box::new(BodyReplayRequest {
            after_sequence: "0".into(),
            max_events: 64,
            max_bytes: 65536,
        })),
    )
}
fn cancel(summary: &BindingSummary) -> OperationStreamMessage {
    message(
        summary,
        Body::ControlRequest(Box::new(BodyControlRequest {
            action: BodyControlRequestAction::Cancel,
            after_sequence: "0".into(),
            request_id: "host-stop".into(),
        })),
    )
}
fn status(reply: OperationStreamMessage) -> BodyControlReplyStatus {
    let Body::ControlReply(reply) = reply.body else {
        panic!("control reply")
    };
    reply.status
}

#[test]
fn readonly_blocked_query_replay_drop_and_detach_continue_exactly_once() {
    let s = setup(false);
    let host = running(&s);
    let Readiness::Ready(summary) = host.wait_ready(Duration::ZERO).unwrap() else {
        panic!("Ready")
    };
    let mut reader = host.attach().unwrap();
    let another = host.attach().unwrap();
    assert_eq!(reader.summary(), &summary);
    assert!(
        matches!(reader.inspect().unwrap(), FixedOperationStatus::Running { run_id } if run_id == summary.run_id())
    );
    let Body::ReplayReply(reply) = reader.replay(&replay(&summary)).unwrap().body else {
        panic!("replay")
    };
    assert!(!reply.events.is_empty());
    drop(another);
    assert!(reader.detach());
    assert!(!reader.detach());
    assert!(reader.inspect().is_err());
    assert!(reader.replay(&replay(&summary)).is_err());
    drop(reader);
    assert!(
        host.inner
            .owner
            .shared
            .record
            .lock()
            .unwrap()
            .subscriptions
            .is_empty()
    );
    assert_eq!(host.inner.owner.state(), LiveState::Running);
    finish(&host.inner.owner, &s);
    let FixedOperationStatus::ResultAvailable {
        run_id,
        marker,
        report_bytes,
    } = host.attach().unwrap().inspect().unwrap()
    else {
        panic!("retained result")
    };
    assert_eq!(run_id, summary.run_id());
    assert_eq!(marker.outcome, MarkerOutcome::Completed);
    let report = report_bytes.unwrap();
    let report_value: serde_json::Value = serde_json::from_slice(&report).unwrap();
    assert!(
        report_value["contributions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["phase"] == "validate" && row["status"] == "ok")
    );
    let canonical = host
        .inner
        .owner
        .shared
        .record
        .lock()
        .unwrap()
        .canonical_request
        .clone();
    let lease = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    let Some(SubmissionOutcome::Replayed { outcome, .. }) = attach_existing_lifecycle(
        &lease,
        &s.coordinates.namespace,
        &s.coordinates.scope,
        &s.coordinates.key,
        &canonical,
    )
    .unwrap() else {
        panic!("C1 fresh domain replay")
    };
    assert_eq!(outcome.marker, marker);
    assert_eq!(outcome.report_bytes, Some(report));
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
}

#[test]
fn corrupted_terminal_evidence_does_not_become_cached_completion() {
    let s = setup(false);
    let host = running(&s);
    let reader = host.attach().unwrap();
    finish(&host.inner.owner, &s);
    let FixedOperationStatus::ResultAvailable {
        report_bytes: Some(report),
        ..
    } = reader.inspect().unwrap()
    else {
        panic!("result")
    };
    let marker = s
        .coordinates
        .root
        .join(".vibe/lifecycle-outcomes")
        .join(format!("{}.marker.json", reader.summary().run_id()));
    let original = std::fs::read(&marker).unwrap();
    std::fs::write(&marker, b"corrupt").unwrap();
    assert!(matches!(
        reader.inspect().unwrap(),
        FixedOperationStatus::RecoveryRequired { .. }
    ));
    // Retained replay references do not override the fresh inspection.
    assert!(reader.replay(&replay(reader.summary())).is_ok());
    std::fs::write(&marker, original).unwrap();
    let report_path = std::fs::read_dir(s.coordinates.root.join(".vibe/lifecycle-outcomes"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| std::fs::read(path).ok().as_ref() == Some(&report))
        .unwrap();
    std::fs::write(&report_path, b"corrupt report").unwrap();
    assert!(matches!(
        reader.inspect().unwrap(),
        FixedOperationStatus::RecoveryRequired { .. }
    ));
    std::fs::write(report_path, report).unwrap();
    std::fs::remove_file(s.coordinates.root.join(".vibe/machine-submissions.json")).unwrap();
    assert!(matches!(
        host.attach().unwrap().inspect().unwrap(),
        FixedOperationStatus::RecoveryRequired { .. }
    ));
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
}

#[test]
fn foreign_capability_and_detached_observer_refuse_before_latch() {
    let s = setup(false);
    let t = setup(false);
    let host = running(&s);
    let foreign = running(&t);
    let mut reader = host.attach().unwrap();
    let request = cancel(reader.summary());
    assert!(
        foreign
            .grant_control()
            .control(&mut reader, &request)
            .is_err()
    );
    assert!(matches!(
        host.inner.owner.shared.record.lock().unwrap().gate(),
        control::Gate::Closed
    ));
    assert_eq!(
        status(host.grant_control().control(&mut reader, &request).unwrap()),
        BodyControlReplyStatus::Unavailable
    );
    assert!(reader.detach());
    assert!(host.grant_control().control(&mut reader, &request).is_err());
    finish(&host.inner.owner, &s);
    finish(&foreign.inner.owner, &t);
}

#[test]
fn attach_subscription_cap_uses_the_existing_owner_without_admitting_again() {
    let s = setup(false);
    let host = running(&s);
    let readers: Vec<_> = (0..64).map(|_| host.attach().unwrap()).collect();
    assert!(host.attach().is_err());
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
    drop(readers);
    assert!(host.attach().is_ok());
    finish(&host.inner.owner, &s);
}

fn request() -> DefaultLifecycleRequest {
    DefaultLifecycleRequest {
        requested: Phase::Validate,
        force: false,
        agent_mode: RunAgentMode::Cli,
        assume_yes: true,
        trace_flag: false,
        install_inputs: Default::default(),
        policy: Default::default(),
    }
}
#[test]
fn host_capability_before_boundary_requests_then_proves_stop_without_provider() {
    let s = setup(false);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let hook = Arc::new(move |_: &Binding| {
        ready_tx.send(()).unwrap();
        release_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
    });
    let c = &s.coordinates;
    let (owner, initial) = launch(OwnedDispatch {
        path: c.root.clone(),
        namespace: c.namespace.clone(),
        scope: c.scope.clone(),
        key: c.key.clone(),
        request: request(),
        target_os: TargetOs::current().unwrap(),
        observed_at: "2026-09-30T00:00:05Z".parse().unwrap(),
        fixture: Some(TestFixture {
            process: s.process.clone(),
            before_effect: Some(hook),
        }),
    })
    .unwrap();
    drop(initial);
    let host = wrap(owner, c.clone());
    ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let mut reader = host.attach().unwrap();
    let stop = cancel(reader.summary());
    // A foreign actual owner cannot mutate the open latch.
    let other_setup = setup(false);
    let other = running(&other_setup);
    assert!(other.grant_control().control(&mut reader, &stop).is_err());
    assert!(matches!(
        host.inner.owner.shared.record.lock().unwrap().gate(),
        control::Gate::Open
    ));
    let capability = host.grant_control();
    assert_eq!(
        status(capability.control(&mut reader, &stop).unwrap()),
        BodyControlReplyStatus::Requested
    );
    release_tx.send(()).unwrap();
    host.inner
        .owner
        .worker
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(
        status(capability.control(&mut reader, &stop).unwrap()),
        BodyControlReplyStatus::ReachedSafeStop
    );
    let FixedOperationStatus::ResultAvailable {
        marker,
        report_bytes,
        ..
    } = reader.inspect().unwrap()
    else {
        panic!("stop marker")
    };
    assert_eq!(marker.outcome, MarkerOutcome::Failed);
    assert_eq!(marker.effect_state, EffectState::None);
    assert!(report_bytes.is_none());
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
    assert!(
        s.coordinates
            .root
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{}.cancel.json", reader.summary().run_id()))
            .is_file()
    );
    finish(&other.inner.owner, &other_setup);
}

#[test]
fn effectful_factory_early_failure_never_mints_ready_or_observer() {
    let s = setup(false);
    std::fs::write(s.coordinates.root.join("vibe.toml"), b"invalid manifest").unwrap();
    let c = &s.coordinates;
    let host = HostOwnedFixedOperation::start(
        c.root.clone(),
        c.namespace.clone(),
        c.scope.clone(),
        c.key.clone(),
        request(),
        TargetOs::current().unwrap(),
        "2026-09-30T00:00:05Z".parse().unwrap(),
    )
    .unwrap();
    host.inner
        .owner
        .worker
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .join()
        .unwrap();
    assert!(matches!(
        host.wait_ready(Duration::ZERO).unwrap(),
        Readiness::Stopped { run_id: None, .. }
    ));
    assert!(matches!(
        host.inspect().unwrap(),
        FixedOperationStatus::RecoveryRequired { run_id: None, .. }
    ));
    assert!(host.attach().is_err());
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
}

#[cfg(unix)]
#[test]
fn fresh_reader_root_check_refuses_replaced_root() {
    let s = setup(false);
    let host = running(&s);
    let reader = host.attach().unwrap();
    finish(&host.inner.owner, &s);
    let original = s.coordinates.root.clone();
    let moved = original.with_extension("sealedfacade-moved");
    std::fs::rename(&original, &moved).unwrap();
    std::fs::create_dir(&original).unwrap();
    assert!(reader.inspect().is_err());
    assert!(reader.replay(&replay(reader.summary())).is_err());
    assert!(host.attach().is_err());
    std::fs::remove_dir(&original).unwrap();
    std::fs::rename(moved, original).unwrap();
}
