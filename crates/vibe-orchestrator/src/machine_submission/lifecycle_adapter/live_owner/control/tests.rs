use super::super::tests::{Setup, begin, finish, setup};
use super::*;
use std::sync::atomic::Ordering;
use std::sync::mpsc;

fn request(
    binding: &Binding,
    id: &str,
    action: BodyControlRequestAction,
) -> OperationStreamMessage {
    binding
        .history
        .lock()
        .unwrap()
        .message(Body::ControlRequest(Box::new(BodyControlRequest {
            action,
            after_sequence: "0".into(),
            request_id: id.into(),
        })))
}
fn status(message: OperationStreamMessage) -> BodyControlReply {
    let Body::ControlReply(row) = message.body else {
        panic!("expected control reply")
    };
    *row
}
fn cancel(
    owner: &LiveOwner,
    observer: &mut Observer,
    binding: &Binding,
    s: &Setup,
) -> BodyControlReply {
    status(
        owner
            .control(
                binding,
                &s.coordinates,
                observer,
                &request(binding, "cancel-one", BodyControlRequestAction::Cancel),
            )
            .unwrap(),
    )
}
fn before_dispatch(s: &Setup) -> (LiveOwner, Observer, Binding, mpsc::Sender<()>) {
    before_dispatch_fault(s, None)
}
fn before_dispatch_fault(
    s: &Setup,
    fault: Option<u8>,
) -> (LiveOwner, Observer, Binding, mpsc::Sender<()>) {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let released = Mutex::new(released);
    let hook = Arc::new(move |binding: &Binding| {
        let name = receipt_name(binding).unwrap();
        match fault {
            Some(0) => vibe_safefs::fail_before_publish(Some(&name)),
            Some(1) => vibe_safefs::fail_after_publish(Some(&name)),
            Some(2) => vibe_safefs::fail_before_stage_cleanup(Some(&name)),
            _ => {}
        }
        ready_tx.send(()).unwrap();
        released
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
    });
    let c = &s.coordinates;
    let (owner, observer) = launch(OwnedDispatch {
        path: c.root.clone(),
        namespace: c.namespace.clone(),
        scope: c.scope.clone(),
        key: c.key.clone(),
        request: DefaultLifecycleRequest {
            requested: Phase::Validate,
            force: false,
            agent_mode: RunAgentMode::Cli,
            assume_yes: true,
            trace_flag: false,
            install_inputs: Default::default(),
            policy: Default::default(),
        },
        target_os: TargetOs::current().unwrap(),
        observed_at: "2026-09-30T00:00:05Z".parse().unwrap(),
        fixture: Some(TestFixture {
            process: s.process.clone(),
            before_effect: Some(hook),
        }),
    })
    .unwrap();
    ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let binding = owner.wait_ready(Duration::from_secs(10)).unwrap();
    (owner, observer, binding, release)
}
fn joined(owner: &LiveOwner) {
    owner.worker.lock().unwrap().take().unwrap().join().unwrap();
}
#[test]
fn request_wins_retains_real_stop_and_idempotent_receipt_without_stage_or_provider() {
    let s = setup(false);
    let (owner, mut observer, binding, release) = before_dispatch(&s);
    assert_eq!(
        cancel(&owner, &mut observer, &binding, &s).status,
        Status::Requested
    );
    assert_eq!(
        cancel(&owner, &mut observer, &binding, &s).status,
        Status::Requested
    );
    assert!(matches!(
        owner.control(
            &binding,
            &s.coordinates,
            &mut observer,
            &request(&binding, "different", BodyControlRequestAction::Cancel)
        ),
        Err(ControlError::ConflictingRequest)
    ));
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
    assert!(!s.coordinates.root.join(".vibe/lifecycle-outcomes").exists());
    assert!(!s.coordinates.root.join(".vibe/lifecycle").exists());
    release.send(()).unwrap();
    joined(&owner);
    let lease = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    let result = domain(&lease, &binding).unwrap();
    assert!(stopped_marker(&result));
    read_receipt(&lease, &binding, "cancel-one").unwrap();
    drop(lease);
    let home = s.coordinates.root.join(".vibe/lifecycle-outcomes");
    let before: Vec<_> = std::fs::read_dir(&home)
        .unwrap()
        .map(|e| {
            let path = e.unwrap().path();
            (path.clone(), std::fs::read(path).unwrap())
        })
        .collect();
    assert_eq!(before.len(), 2, "only marker and cancel receipt");
    for _ in 0..3 {
        let response = cancel(&owner, &mut observer, &binding, &s);
        assert_eq!(response.status, Status::ReachedSafeStop);
        assert_eq!(response.request_id, "cancel-one");
        assert_eq!(response.operation_effect_state, Effect::NoEffects);
        assert_eq!(response.result_ref, Some(reference(&binding)));
    }
    for (path, bytes) in before {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
    assert!(!s.coordinates.root.join(".vibe/lifecycle").exists());
    assert!(
        matches!(observer.inspect().unwrap(), Observation::ResultAvailable { result, .. } if stopped_marker(&result))
    );
}
#[test]
fn missing_or_tampered_marker_and_receipt_never_replay_memory_reached() {
    for member in ["marker", "cancel"] {
        for tamper in [false, true] {
            let s = setup(false);
            let (owner, mut observer, binding, release) = before_dispatch(&s);
            assert_eq!(
                cancel(&owner, &mut observer, &binding, &s).status,
                Status::Requested
            );
            release.send(()).unwrap();
            joined(&owner);
            assert_eq!(
                cancel(&owner, &mut observer, &binding, &s).status,
                Status::ReachedSafeStop
            );
            let path = s.coordinates.root.join(format!(
                ".vibe/lifecycle-outcomes/{}.{member}.json",
                binding.run_id
            ));
            if tamper {
                std::fs::write(path, b"{}").unwrap();
            } else {
                std::fs::remove_file(path).unwrap();
            }
            assert_eq!(
                cancel(&owner, &mut observer, &binding, &s).status,
                Status::Unavailable
            );
            assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
        }
    }
}
#[test]
fn occupied_foreign_receipt_is_never_overwritten_or_repaired() {
    let s = setup(false);
    let (owner, mut observer, binding, release) = before_dispatch(&s);
    assert_eq!(
        cancel(&owner, &mut observer, &binding, &s).status,
        Status::Requested
    );
    let project = Project::open(&s.coordinates.root).unwrap();
    let dir = project.dir(&[".vibe", "lifecycle-outcomes"], true).unwrap();
    project
        .publish_new_in(&dir, &receipt_name(&binding).unwrap(), b"foreign")
        .unwrap();
    release.send(()).unwrap();
    joined(&owner);
    for _ in 0..2 {
        assert_eq!(
            cancel(&owner, &mut observer, &binding, &s).status,
            Status::Unavailable
        );
    }
    assert_eq!(
        project
            .read_file_bounded_in(&dir, &receipt_name(&binding).unwrap(), 64)
            .unwrap()
            .unwrap(),
        b"foreign"
    );
    let lease = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    assert!(stopped_marker(&domain(&lease, &binding).unwrap()));
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
}
#[test]
fn active_provider_is_unavailable_and_late_terminal_uses_actual_domain_effect() {
    let s = setup(false);
    let (owner, mut observer, binding) = begin(&s, None);
    assert_eq!(
        cancel(&owner, &mut observer, &binding, &s).status,
        Status::Unavailable
    );
    finish(&owner, &s);
    let lease = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    let actual = domain(&lease, &binding).unwrap();
    drop(lease);
    for _ in 0..2 {
        let row = cancel(&owner, &mut observer, &binding, &s);
        assert_eq!(row.status, Status::AlreadyTerminal);
        assert_eq!(row.operation_effect_state, effect(&actual));
        assert_eq!(row.result_ref, Some(reference(&binding)));
    }
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
    assert!(
        !s.coordinates
            .root
            .join(format!(
                ".vibe/lifecycle-outcomes/{}",
                receipt_name(&binding).unwrap()
            ))
            .exists()
    );
}
#[test]
fn host_detach_removes_only_passed_subscription_and_reuses_capacity() {
    let s = setup(false);
    let (owner, mut observer, binding) = begin(&s, None);
    let mut another = owner.attach(&binding, &s.coordinates).unwrap();
    let row = status(
        owner
            .control(
                &binding,
                &s.coordinates,
                &mut observer,
                &request(&binding, "detach-one", BodyControlRequestAction::Detach),
            )
            .unwrap(),
    );
    assert_eq!(row.status, Status::Detached);
    assert_eq!(row.operation_effect_state, Effect::Uncertain);
    assert_eq!(owner.shared.record.lock().unwrap().subscriptions.len(), 1);
    assert!(!observer.detach());
    assert!(another.inspect().is_ok());
    let replacement = owner.attach(&binding, &s.coordinates).unwrap();
    assert_eq!(owner.shared.record.lock().unwrap().subscriptions.len(), 2);
    assert!(another.detach());
    drop(replacement);
    assert!(owner.shared.record.lock().unwrap().subscriptions.is_empty());
    finish(&owner, &s);
}
#[test]
fn scope_foreign_observer_and_wrong_frame_cannot_latch_cancel() {
    let s = setup(false);
    let other = setup(false);
    let (owner, mut observer, binding, release) = before_dispatch(&s);
    let (foreign_owner, mut foreign_observer, foreign_binding, foreign_release) =
        before_dispatch(&other);
    assert!(matches!(
        owner.control(
            &binding,
            &s.coordinates,
            &mut foreign_observer,
            &request(&binding, "cancel-one", BodyControlRequestAction::Cancel)
        ),
        Err(ControlError::Binding)
    ));
    let mut wrong = request(&binding, "cancel-one", BodyControlRequestAction::Cancel);
    wrong.stream_id.push_str("-foreign");
    assert!(
        owner
            .control(&binding, &s.coordinates, &mut observer, &wrong)
            .is_err()
    );
    let mut trusted = s.coordinates.clone();
    trusted.key.push_str("-foreign");
    assert!(
        owner
            .control(
                &binding,
                &trusted,
                &mut observer,
                &request(&binding, "cancel-one", BodyControlRequestAction::Cancel)
            )
            .is_err()
    );
    assert!(matches!(
        owner.shared.record.lock().unwrap().gate(),
        Gate::Open
    ));
    cancel(&owner, &mut observer, &binding, &s);
    cancel(
        &foreign_owner,
        &mut foreign_observer,
        &foreign_binding,
        &other,
    );
    release.send(()).unwrap();
    foreign_release.send(()).unwrap();
    joined(&owner);
    joined(&foreign_owner);
}
#[test]
fn source_or_header_drift_blocks_stop_proof_and_dispatch() {
    for header in [false, true] {
        let s = setup(false);
        let (owner, mut observer, binding, release) = before_dispatch(&s);
        assert_eq!(
            cancel(&owner, &mut observer, &binding, &s).status,
            Status::Requested
        );
        let drift_path = if header {
            s.coordinates.root.join(".vibe/lifecycle.toml")
        } else {
            s.coordinates.root.join(&binding.source.script_relative)
        };
        std::fs::write(drift_path, b"changed\n").unwrap();
        release.send(()).unwrap();
        joined(&owner);
        assert_eq!(
            cancel(&owner, &mut observer, &binding, &s).status,
            Status::Unavailable
        );
        assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
        assert!(!s.coordinates.root.join(".vibe/lifecycle-outcomes").exists());
    }
}
#[test]
fn retained_stop_query_with_busy_writer_keeps_same_run_unavailable() {
    let s = setup(false);
    let (owner, mut observer, binding, release) = before_dispatch(&s);
    cancel(&owner, &mut observer, &binding, &s);
    release.send(()).unwrap();
    joined(&owner);
    let writer = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    assert_eq!(
        cancel(&owner, &mut observer, &binding, &s).status,
        Status::Unavailable
    );
    assert!(
        matches!(owner.inspect().unwrap(), Observation::RecoveryRequired { run_id: Some(id), .. } if id == binding.run_id)
    );
    drop(writer);
    assert_eq!(
        cancel(&owner, &mut observer, &binding, &s).status,
        Status::ReachedSafeStop
    );
}

#[test]
fn race_has_only_requested_without_provider_or_closed_gate_with_one_provider() {
    for _ in 0..12 {
        let s = setup(false);
        let (owner, mut observer, binding, release) = before_dispatch(&s);
        release.send(()).unwrap();
        let response = cancel(&owner, &mut observer, &binding, &s);
        match response.status {
            Status::Requested => {
                joined(&owner);
                assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
                assert_eq!(
                    cancel(&owner, &mut observer, &binding, &s).status,
                    Status::ReachedSafeStop
                );
            }
            Status::Unavailable => {
                finish(&owner, &s);
                assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
                assert_eq!(
                    cancel(&owner, &mut observer, &binding, &s).status,
                    Status::AlreadyTerminal
                );
            }
            other => panic!("illegal race response: {other:?}"),
        }
    }
}
#[test]
fn historical_reached_preserves_retained_proof_after_source_and_header_changes() {
    for header in [false, true] {
        let s = setup(false);
        let (owner, mut observer, binding, release) = before_dispatch(&s);
        cancel(&owner, &mut observer, &binding, &s);
        release.send(()).unwrap();
        joined(&owner);
        assert_eq!(
            cancel(&owner, &mut observer, &binding, &s).status,
            Status::ReachedSafeStop
        );
        let path = if header {
            s.coordinates.root.join(".vibe/lifecycle.toml")
        } else {
            s.coordinates.root.join(&binding.source.script_relative)
        };
        std::fs::write(path, b"changed\n").unwrap();
        assert_eq!(
            cancel(&owner, &mut observer, &binding, &s).status,
            Status::ReachedSafeStop
        );
        assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn receipt_publication_failure_or_response_loss_never_promotes_to_reached_or_repairs() {
    for fault in 0..3 {
        let s = setup(false);
        let (owner, mut observer, binding, release) = before_dispatch_fault(&s, Some(fault));
        assert_eq!(
            cancel(&owner, &mut observer, &binding, &s).status,
            Status::Requested
        );
        release.send(()).unwrap();
        joined(&owner);
        let home = s.coordinates.root.join(".vibe/lifecycle-outcomes");
        let before: Vec<_> = std::fs::read_dir(&home)
            .unwrap()
            .map(|e| {
                let path = e.unwrap().path();
                (path.clone(), std::fs::read(path).unwrap())
            })
            .collect();
        let lease = LifecycleLease::acquire(&s.coordinates.root).unwrap();
        assert!(stopped_marker(&domain(&lease, &binding).unwrap()));
        drop(lease);
        assert_eq!(owner.state(), LiveState::ResultAvailable);
        assert!(
            matches!(owner.inspect().unwrap(), Observation::ResultAvailable { result, .. } if stopped_marker(&result))
        );
        for _ in 0..3 {
            let row = cancel(&owner, &mut observer, &binding, &s);
            assert_eq!(row.status, Status::Unavailable);
            assert_eq!(row.operation_effect_state, Effect::Uncertain);
        }
        for (path, bytes) in before {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
        let path = home.join(receipt_name(&binding).unwrap());
        assert_eq!(path.exists(), fault != 0);
    }
}

#[test]
fn historical_reached_and_late_terminal_survive_unrelated_current_source_header_changes() {
    for stopped in [false, true] {
        let s = setup(false);
        let (owner, mut observer, binding) = if stopped {
            let (o, mut v, b, release) = before_dispatch(&s);
            cancel(&o, &mut v, &b, &s);
            release.send(()).unwrap();
            joined(&o);
            (o, v, b)
        } else {
            let (o, v, b) = begin(&s, None);
            finish(&o, &s);
            (o, v, b)
        };
        let expected = if stopped {
            Status::ReachedSafeStop
        } else {
            Status::AlreadyTerminal
        };
        assert_eq!(cancel(&owner, &mut observer, &binding, &s).status, expected);
        std::fs::write(
            s.coordinates.root.join(&binding.source.script_relative),
            b"next source\n",
        )
        .unwrap();
        std::fs::write(
            s.coordinates.root.join(".vibe/lifecycle.toml"),
            b"unrelated current header\n",
        )
        .unwrap();
        assert_eq!(cancel(&owner, &mut observer, &binding, &s).status, expected);
        assert_eq!(
            s.process.count.load(Ordering::SeqCst),
            usize::from(!stopped)
        );
    }
}
