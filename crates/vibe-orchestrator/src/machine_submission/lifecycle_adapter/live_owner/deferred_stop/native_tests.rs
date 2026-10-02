//! Compile-only worker delivery; parent executes these Linux native oracles.
use super::*;
use std::fs;
use std::time::Instant;
const WAIT: Duration = Duration::from_secs(15);
fn wait(label: &str, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !predicate() {
        assert!(Instant::now() < deadline, "deadline: {label}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new(tail: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), ONE_SCRIPT_VALIDATE_MANIFEST).unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        // Same root/C1/script profile as the named native process_tests owner.
        fs::write(dir.path().join("run.sh"), format!(
            "#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\nprintf 'kept\\n' > artifact\nprintf 'entered\\n' > entered\ni=0\nwhile [ ! -f release ]; do\n i=$((i + 1))\n if [ \"$i\" -ge 600 ]; then exit 70; fi\n sleep 0.05\ndone\n{tail}\n"
        )).unwrap();
        Self(dir)
    }
    fn root(&self) -> &Path {
        self.0.path()
    }
    fn release(&self) {
        fs::write(self.root().join("release"), b"go").unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::write(self.root().join("release"), b"cleanup");
    }
}
fn coordinates(root: &Path) -> HostCoordinates {
    let scope = SubmissionScope {
        context: "deferred-native".into(),
        target: "root".into(),
        principal: "host".into(),
    };
    let namespace = {
        let lease = LifecycleLease::acquire(root).unwrap();
        SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap()
    };
    HostCoordinates {
        root: root.into(),
        scope,
        namespace,
        key: "native-once".into(),
    }
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
fn start_host(c: &HostCoordinates, opted: bool) -> HostOwnedFixedOperation {
    let factory = if opted {
        HostOwnedFixedOperation::start_with_deferred_stop
    } else {
        HostOwnedFixedOperation::start
    };
    factory(
        c.root.clone(),
        c.namespace.clone(),
        c.scope.clone(),
        c.key.clone(),
        request(),
        TargetOs::Linux,
        "2026-09-30T00:00:05Z".parse().unwrap(),
    )
    .unwrap()
}
fn frame(summary: &BindingSummary, body: Body) -> OperationStreamMessage {
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
fn cancel(summary: &BindingSummary) -> OperationStreamMessage {
    frame(
        summary,
        Body::ControlRequest(Box::new(BodyControlRequest {
            action: BodyControlRequestAction::Cancel,
            after_sequence: "0".into(),
            request_id: "native-stop".into(),
        })),
    )
}
fn row(message: OperationStreamMessage) -> BodyControlReply {
    let Body::ControlReply(row) = message.body else {
        panic!("control reply")
    };
    *row
}
fn ready(host: &HostOwnedFixedOperation) -> BindingSummary {
    let Readiness::Ready(summary) = host.wait_ready(WAIT).unwrap() else {
        panic!("actual Ready")
    };
    summary
}
fn terminal(host: &HostOwnedFixedOperation) -> FixedOperationStatus {
    wait("callback and publication", || {
        !matches!(
            host.inspect().unwrap(),
            FixedOperationStatus::Starting | FixedOperationStatus::Running { .. }
        )
    });
    host.inspect().unwrap()
}
#[test]
fn deferred_stop_native_natural_exit_preserves_report_receipt_cold_replay_and_artifact() {
    let f = Fixture::new("exit 0");
    let c = coordinates(f.root());
    let host = start_host(&c, true);
    let summary = ready(&host);
    wait("script commits", || f.root().join("entered").exists());
    let mut observer = host.attach().unwrap();
    let capability = host.grant_control();
    let message = cancel(&summary);
    let requested = row(capability.control(&mut observer, &message).unwrap());
    assert_eq!(requested.status, BodyControlReplyStatus::Requested);
    assert_eq!(
        requested.operation_effect_state,
        BodyControlReplyOperationEffectState::Uncertain
    );
    std::thread::scope(|scope| {
        let host = host.clone();
        scope
            .spawn(move || {
                let reader = host.attach().unwrap();
                assert!(matches!(
                    reader.inspect().unwrap(),
                    FixedOperationStatus::Running { .. }
                ));
                let replay = frame(
                    reader.summary(),
                    Body::ReplayRequest(Box::new(BodyReplayRequest {
                        after_sequence: "0".into(),
                        max_events: 64,
                        max_bytes: 65536,
                    })),
                );
                assert!(matches!(
                    reader.replay(&replay).unwrap().body,
                    Body::ReplayReply(_)
                ));
            })
            .join()
            .unwrap();
    });
    assert_eq!(fs::read(f.root().join("artifact")).unwrap(), b"kept\n");
    assert_eq!(fs::read(f.root().join("counter")).unwrap(), b"effect\n");
    f.release();
    let FixedOperationStatus::ResultAvailable {
        marker,
        report_bytes,
        ..
    } = terminal(&host)
    else {
        panic!("partial marker")
    };
    assert_eq!(marker.outcome, MarkerOutcome::Uncertain);
    assert_eq!(marker.effect_state, EffectState::Uncertain);
    let report = report_bytes.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&report).unwrap();
    assert_eq!(value["ok"], true);
    assert!(
        value["contributions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["phase"] == "validate" && r["status"] == "ok")
    );
    let reached = capability.control(&mut observer, &message).unwrap();
    let reply = row(reached.clone());
    assert_eq!(reply.status, BodyControlReplyStatus::ReachedSafeStop);
    assert_eq!(
        reply.operation_effect_state,
        BodyControlReplyOperationEffectState::Uncertain
    );
    assert!(reply.result_ref.is_some());
    let home = f.root().join(".vibe/lifecycle-outcomes");
    let path = home.join(format!("{}.cancel.json", summary.run_id()));
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        serde_json::from_slice::<OperationStreamMessage>(&bytes).unwrap(),
        reached
    );
    for _ in 0..2 {
        assert_eq!(
            capability.control(&mut observer, &message).unwrap(),
            reached
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    let lease = LifecycleLease::acquire(f.root()).unwrap();
    assert!(
        matches!(inspect_retained_lifecycle_outcome(&lease, summary.run_id(), &marker.reservation_ref).unwrap(),
        LifecycleOutcomeInspection::Validated { report_bytes: Some(b), .. } if b == report)
    );
    drop(lease);
    let cold = start_host(&c, true);
    wait("cold owner finishes", || {
        matches!(
            cold.inspect().unwrap(),
            FixedOperationStatus::RecoveryRequired { .. }
        )
    });
    let lease = LifecycleLease::acquire(f.root()).unwrap();
    assert!(
        matches!(inspect_retained_lifecycle_outcome(&lease, summary.run_id(), &marker.reservation_ref).unwrap(),
        LifecycleOutcomeInspection::Validated { marker: m, report_bytes: Some(b) } if m == marker && b == report)
    );
    drop(lease);
    assert_eq!(fs::read(f.root().join("counter")).unwrap(), b"effect\n");
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        capability.control(&mut observer, &message).unwrap(),
        reached
    );
    let mut late = message.clone();
    let Body::ControlRequest(late_row) = &mut late.body else {
        unreachable!()
    };
    late_row.request_id = "new-cancel-after-stop".into();
    assert_eq!(
        row(capability.control(&mut observer, &late).unwrap()).status,
        BodyControlReplyStatus::AlreadyTerminal
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    for damaged in [b"tampered".as_slice(), b"".as_slice()] {
        fs::write(&path, damaged).unwrap();
        assert_eq!(
            row(capability.control(&mut observer, &message).unwrap()).status,
            BodyControlReplyStatus::Unavailable
        );
    }
    fs::remove_file(&path).unwrap();
    assert_eq!(
        row(capability.control(&mut observer, &message).unwrap()).status,
        BodyControlReplyStatus::Unavailable
    );
}
#[test]
fn deferred_stop_native_default_and_opted_without_cancel_keep_legacy_completed() {
    for opted in [false, true] {
        let f = Fixture::new("exit 0");
        let c = coordinates(f.root());
        let host = start_host(&c, opted);
        let summary = ready(&host);
        wait("entered", || f.root().join("entered").exists());
        if !opted {
            let mut reader = host.attach().unwrap();
            assert_eq!(
                row(host
                    .grant_control()
                    .control(&mut reader, &cancel(&summary))
                    .unwrap())
                .status,
                BodyControlReplyStatus::Unavailable
            );
        }
        f.release();
        let FixedOperationStatus::ResultAvailable { marker, .. } = terminal(&host) else {
            panic!("normal result")
        };
        assert_eq!(marker.outcome, MarkerOutcome::Completed);
        assert_eq!(fs::read(f.root().join("counter")).unwrap(), b"effect\n");
        assert!(
            !f.root()
                .join(format!(
                    ".vibe/lifecycle-outcomes/{}.cancel.json",
                    summary.run_id()
                ))
                .exists()
        );
    }
}
#[test]
fn deferred_stop_native_failed_provider_and_descendant_held_pipe_never_reached() {
    for tail in ["exit 9", "sleep 3 &\nexit 0"] {
        let f = Fixture::new(tail);
        let c = coordinates(f.root());
        let host = start_host(&c, true);
        let summary = ready(&host);
        wait("entered", || f.root().join("entered").exists());
        let mut reader = host.attach().unwrap();
        let capability = host.grant_control();
        let message = cancel(&summary);
        assert_eq!(
            row(capability.control(&mut reader, &message).unwrap()).status,
            BodyControlReplyStatus::Requested
        );
        f.release();
        let status = terminal(&host);
        if tail == "exit 9" {
            let FixedOperationStatus::ResultAvailable { marker, .. } = status else {
                panic!("actual failed result")
            };
            assert_eq!(marker.outcome, MarkerOutcome::Failed);
        }
        assert_eq!(
            row(capability.control(&mut reader, &message).unwrap()).status,
            BodyControlReplyStatus::Unavailable
        );
        assert_eq!(fs::read(f.root().join("artifact")).unwrap(), b"kept\n");
    }
}
#[test]
fn deferred_stop_native_foreign_receipt_preserves_partial_marker_without_repair() {
    let f = Fixture::new("exit 0");
    let c = coordinates(f.root());
    let host = start_host(&c, true);
    let summary = ready(&host);
    wait("entered", || f.root().join("entered").exists());
    let mut reader = host.attach().unwrap();
    let capability = host.grant_control();
    let message = cancel(&summary);
    assert_eq!(
        row(capability.control(&mut reader, &message).unwrap()).status,
        BodyControlReplyStatus::Requested
    );
    let path = f.root().join(format!(
        ".vibe/lifecycle-outcomes/{}.cancel.json",
        summary.run_id()
    ));
    fs::write(&path, b"foreign").unwrap();
    f.release();
    let FixedOperationStatus::ResultAvailable { marker, .. } = terminal(&host) else {
        panic!("partial marker")
    };
    assert_eq!(marker.outcome, MarkerOutcome::Uncertain);
    for _ in 0..2 {
        assert_eq!(
            row(capability.control(&mut reader, &message).unwrap()).status,
            BodyControlReplyStatus::Unavailable
        );
        let mut late = message.clone();
        let Body::ControlRequest(request) = &mut late.body else {
            unreachable!()
        };
        request.request_id = "new-after-receipt-failure".into();
        let reply = row(capability.control(&mut reader, &late).unwrap());
        assert_eq!(reply.status, BodyControlReplyStatus::AlreadyTerminal);
        assert_eq!(
            reply.operation_effect_state,
            BodyControlReplyOperationEffectState::Uncertain
        );
        assert_eq!(
            reply.result_ref,
            Some(format!(
                "lifecycle:{}:{}",
                summary.run_id(),
                marker.reservation_ref
            ))
        );
        assert_eq!(fs::read(&path).unwrap(), b"foreign");
    }
}

fn start_hook(
    c: &HostCoordinates,
    boundary: Option<Arc<dyn Fn(&Binding) + Send + Sync>>,
    after_effect: Option<Arc<dyn Fn() -> Result<()> + Send + Sync>>,
) -> HostOwnedFixedOperation {
    let (owner, initial) = launch(OwnedDispatch {
        path: c.root.clone(),
        namespace: c.namespace.clone(),
        scope: c.scope.clone(),
        key: c.key.clone(),
        request: request(),
        target_os: TargetOs::Linux,
        observed_at: "2026-09-30T00:00:05Z".parse().unwrap(),
        deferred_stop: true,
        profile: None,
        fixture: None,
        before_boundary: boundary,
        after_effect,
    })
    .unwrap();
    drop(initial);
    HostOwnedFixedOperation::from_test_owner(owner, c.clone())
}
#[test]
fn deferred_stop_native_boundary_request_and_publication_winners_are_deterministic() {
    for requested in [true, false] {
        let f = Fixture::new("exit 0");
        let c = coordinates(f.root());
        let (arrived_tx, arrived_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = Mutex::new(release_rx);
        let hook = Arc::new(move |_: &Binding| {
            arrived_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv_timeout(WAIT).unwrap();
        });
        let host = start_hook(&c, Some(hook), None);
        let summary = ready(&host);
        wait("entered", || f.root().join("entered").exists());
        let mut reader = host.attach().unwrap();
        let capability = host.grant_control();
        let message = cancel(&summary);
        f.release();
        arrived_rx.recv_timeout(WAIT).unwrap();
        if requested {
            assert_eq!(
                row(capability.control(&mut reader, &message).unwrap()).status,
                BodyControlReplyStatus::Requested
            );
        }
        release_tx.send(()).unwrap();
        let FixedOperationStatus::ResultAvailable { marker, .. } = terminal(&host) else {
            panic!("marker")
        };
        assert_eq!(
            marker.outcome,
            if requested {
                MarkerOutcome::Uncertain
            } else {
                MarkerOutcome::Completed
            }
        );
        assert_eq!(
            row(capability.control(&mut reader, &message).unwrap()).status,
            if requested {
                BodyControlReplyStatus::ReachedSafeStop
            } else {
                BodyControlReplyStatus::AlreadyTerminal
            }
        );
        assert_eq!(fs::read(f.root().join("counter")).unwrap(), b"effect\n");
    }
}
#[test]
fn deferred_stop_native_receipt_publication_faults_preserve_valid_partial_outcome() {
    for fault in 0..3 {
        let f = Fixture::new("exit 0");
        let c = coordinates(f.root());
        let hook = Arc::new(move |binding: &Binding| {
            let name = receipt_name(binding).unwrap();
            match fault {
                0 => vibe_safefs::fail_before_publish(Some(&name)),
                1 => vibe_safefs::fail_after_publish(Some(&name)),
                _ => vibe_safefs::fail_before_stage_cleanup(Some(&name)),
            }
        });
        let host = start_hook(&c, Some(hook), None);
        let summary = ready(&host);
        wait("entered", || f.root().join("entered").exists());
        let mut reader = host.attach().unwrap();
        let capability = host.grant_control();
        let message = cancel(&summary);
        assert_eq!(
            row(capability.control(&mut reader, &message).unwrap()).status,
            BodyControlReplyStatus::Requested
        );
        f.release();
        let FixedOperationStatus::ResultAvailable {
            marker,
            report_bytes,
            ..
        } = terminal(&host)
        else {
            panic!("partial marker")
        };
        assert_eq!(marker.outcome, MarkerOutcome::Uncertain);
        assert!(report_bytes.is_some());
        let path = f.root().join(format!(
            ".vibe/lifecycle-outcomes/{}.cancel.json",
            summary.run_id()
        ));
        let bytes = fs::read(&path).ok();
        for _ in 0..2 {
            assert_eq!(
                row(capability.control(&mut reader, &message).unwrap()).status,
                BodyControlReplyStatus::Unavailable
            );
            assert_eq!(fs::read(&path).ok(), bytes);
        }
    }
}
#[test]
fn deferred_stop_native_after_effect_error_or_panic_never_fabricates_receipt() {
    for panic in [false, true] {
        let f = Fixture::new("exit 0");
        let c = coordinates(f.root());
        let hook = Arc::new(move || -> Result<()> {
            assert!(!panic, "injected after-effect panic");
            bail!("injected after-effect error")
        });
        let host = start_hook(&c, None, Some(hook));
        let summary = ready(&host);
        wait("entered", || f.root().join("entered").exists());
        let mut reader = host.attach().unwrap();
        let capability = host.grant_control();
        let message = cancel(&summary);
        assert_eq!(
            row(capability.control(&mut reader, &message).unwrap()).status,
            BodyControlReplyStatus::Requested
        );
        f.release();
        assert!(matches!(
            terminal(&host),
            FixedOperationStatus::RecoveryRequired { .. }
        ));
        assert_eq!(
            row(capability.control(&mut reader, &message).unwrap()).status,
            BodyControlReplyStatus::Unavailable
        );
        assert!(
            !f.root()
                .join(format!(
                    ".vibe/lifecycle-outcomes/{}.cancel.json",
                    summary.run_id()
                ))
                .exists()
        );
        assert_eq!(fs::read(f.root().join("artifact")).unwrap(), b"kept\n");
    }
}
#[test]
fn deferred_stop_native_physical_root_drift_never_authorizes_stop_receipt() {
    let f = Fixture::new("exit 0");
    let c = coordinates(f.root());
    let root = c.root.clone();
    let moved = root.with_extension("moved");
    let moved_hook = moved.clone();
    let hook = Arc::new(move |_: &Binding| {
        fs::rename(&root, &moved_hook).unwrap();
        fs::create_dir(&root).unwrap();
    });
    let host = start_hook(&c, Some(hook), None);
    let summary = ready(&host);
    wait("entered", || f.root().join("entered").exists());
    let mut reader = host.attach().unwrap();
    let capability = host.grant_control();
    let message = cancel(&summary);
    assert_eq!(
        row(capability.control(&mut reader, &message).unwrap()).status,
        BodyControlReplyStatus::Requested
    );
    f.release();
    wait("root moved and lease released", || {
        moved.exists() && LifecycleLease::acquire(&moved).is_ok()
    });
    assert!(capability.control(&mut reader, &message).is_err());
    assert!(
        !moved
            .join(format!(
                ".vibe/lifecycle-outcomes/{}.cancel.json",
                summary.run_id()
            ))
            .exists()
    );
    fs::remove_dir_all(&c.root).unwrap();
    fs::rename(&moved, &c.root).unwrap();
}
