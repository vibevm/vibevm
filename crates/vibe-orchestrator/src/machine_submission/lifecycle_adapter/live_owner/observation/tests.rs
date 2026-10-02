use super::*;

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
fn replay(owner: &LiveOwner, binding: &Binding, after: &str) -> BodyReplayReply {
    let message = binding
        .history
        .lock()
        .unwrap()
        .message(Body::ReplayRequest(Box::new(BodyReplayRequest {
            after_sequence: after.into(),
            max_events: 64,
            max_bytes: 65536,
        })));
    let reply = owner
        .replay(binding, &binding.coordinates, &message)
        .unwrap();
    crate::machine_operation::stream_message::validate(
        &reply,
        binding.history.lock().unwrap().expected(),
    )
    .unwrap();
    let Body::ReplayReply(reply) = reply.body else {
        panic!("replay body")
    };
    *reply
}
fn joined(owner: &LiveOwner) {
    let record = owner.shared.record.lock().unwrap();
    let (record, timeout) = owner
        .shared
        .changed
        .wait_timeout_while(record, Duration::from_secs(15), |r| {
            matches!(r.state, LiveState::Starting | LiveState::Running)
        })
        .unwrap();
    assert!(!timeout.timed_out());
    drop(record);
    owner.worker.lock().unwrap().take().unwrap().join().unwrap();
}

#[test]
fn stop_before_dispatch_has_no_observation_claims() {
    use std::sync::{atomic::Ordering, mpsc};
    let s = super::super::tests::setup(false);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let released = Mutex::new(released);
    let hook = Arc::new(move |_: &Binding| {
        ready_tx.send(()).unwrap();
        released
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
    });
    let c = &s.coordinates;
    let (owner, mut observer) = launch(OwnedDispatch {
        before_boundary: None,
        after_effect: None,
        deferred_stop: false,
        profile: None,
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
    ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let binding = owner.wait_ready(Duration::from_secs(10)).unwrap();
    let control = binding
        .history
        .lock()
        .unwrap()
        .message(Body::ControlRequest(Box::new(BodyControlRequest {
            action: BodyControlRequestAction::Cancel,
            after_sequence: "0".into(),
            request_id: "stop-before-observation".into(),
        })));
    let reply = owner.control(&binding, c, &mut observer, &control).unwrap();
    let Body::ControlReply(reply) = reply.body else {
        panic!("control reply")
    };
    assert_eq!(reply.status, BodyControlReplyStatus::Requested);
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
    release.send(()).unwrap();
    joined(&owner);
    let reply = owner.control(&binding, c, &mut observer, &control).unwrap();
    let Body::ControlReply(reply) = reply.body else {
        panic!("retained control reply")
    };
    assert_eq!(reply.status, BodyControlReplyStatus::ReachedSafeStop);
    assert_eq!(
        reply.operation_effect_state,
        BodyControlReplyOperationEffectState::NoEffects
    );
    let events = replay(&owner, &binding, "0");
    assert!(!events.expired && events.gap.is_none());
    assert!(
        events.events.iter().any(|event| matches!(
            &event.payload, Payload::State(row)
            if row.observed == stream::Observed::JournalReady
        )),
        "positive control: actual Ready was observed"
    );
    assert!(events.events.iter().all(|event| !matches!(
        &event.payload,
        Payload::Phase(_) | Payload::Progress(_) | Payload::RawOutput(_)
    )));
    assert_eq!(events.terminal_ref, reply.result_ref);
    assert!(events.terminal_ref.is_some());
    let Observation::ResultAvailable { result, .. } = observer.inspect().unwrap() else {
        panic!("retained pre-dispatch stop")
    };
    assert_eq!(result.marker.outcome, MarkerOutcome::Failed);
    assert_eq!(result.marker.effect_state, EffectState::None);
    assert!(result.report_bytes.is_none());
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
    assert!(!c.root.join(".vibe/lifecycle").exists());
}

#[cfg(target_os = "linux")]
struct NativeFixture(tempfile::TempDir);
#[cfg(target_os = "linux")]
impl NativeFixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("vibe.toml"), ONE_SCRIPT_VALIDATE_MANIFEST).unwrap();
        std::fs::write(dir.path().join("run.sh"), concat!(
            "#!/bin/sh\nset -eu\n",
            "printf 'effect\\n' >> counter\n",
            "printf '\\377A\\000'\nprintf '\\376B\\000' >&2\n",
            "printf entered > entered\ni=0\n",
            "while [ ! -f release ]; do i=$((i + 1)); ",
            "if [ \"$i\" -ge 200 ]; then printf timeout > finished; exit 70; fi; sleep 0.05; done\n",
            "printf finished > finished\n"
        )).unwrap();
        Self(dir)
    }
    fn coordinates(&self) -> HostCoordinates {
        let scope = SubmissionScope {
            context: "raw-context".into(),
            target: "root".into(),
            principal: "owner".into(),
        };
        let namespace = {
            let lease = LifecycleLease::acquire(self.0.path()).unwrap();
            SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap()
        };
        HostCoordinates {
            root: self.0.path().into(),
            namespace,
            scope,
            key: "raw-once".into(),
        }
    }
    fn release(&self) {
        std::fs::write(self.0.path().join("release"), b"release").unwrap();
    }
}
#[cfg(target_os = "linux")]
impl Drop for NativeFixture {
    fn drop(&mut self) {
        let _ = std::fs::write(self.0.path().join("release"), b"cleanup");
        let deadline = std::time::Instant::now() + Duration::from_secs(12);
        while self.0.path().join("entered").exists()
            && !self.0.path().join("finished").exists()
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn production_phase_raw_replay_stays_responsive() {
    let f = NativeFixture::new();
    let c = f.coordinates();
    let (owner, mut observer) = start(
        c.root.clone(),
        c.namespace.clone(),
        c.scope.clone(),
        c.key.clone(),
        request(),
        TargetOs::Linux,
        "2026-09-30T00:00:05Z".parse().unwrap(),
    )
    .unwrap();
    let binding = owner.wait_ready(Duration::from_secs(10)).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !c.root.join("entered").is_file() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let query_start = std::time::Instant::now();
    assert!(matches!(
        observer.inspect().unwrap(),
        Observation::Running { .. }
    ));
    assert_eq!(observer.cancel(), Cancellation::Unavailable);
    let active = replay(&owner, &binding, "0");
    assert!(
        active
            .events
            .iter()
            .any(|row| matches!(&row.payload, Payload::Phase(p)
        if p.phase == "validate" && p.transition == PayloadPhaseTransition::Started))
    );
    assert!(observer.detach());
    assert!(
        query_start.elapsed() < Duration::from_secs(1),
        "queries must finish while provider is blocked"
    );
    assert!(!c.root.join("finished").exists());
    f.release();
    joined(&owner);
    let final_reply = replay(&owner, &binding, "0");
    assert!(!final_reply.expired && final_reply.gap.is_none());
    assert!(final_reply.terminal_ref.is_some());
    let mut bytes = [Vec::new(), Vec::new()];
    let mut finished = false;
    let mut progress = false;
    let mut progress_counts = Vec::new();
    for event in &final_reply.events {
        match &event.payload {
            Payload::RawOutput(row) => {
                let index = if row.origin == PayloadRawOutputOrigin::Stdout {
                    0
                } else {
                    1
                };
                assert!(!row.truncated);
                assert_eq!(row.byte_offset, bytes[index].len().to_string());
                for pair in row.hex_bytes.as_bytes().chunks_exact(2) {
                    bytes[index]
                        .push(u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap());
                }
            }
            Payload::Phase(row)
                if row.phase == "validate"
                    && row.transition == PayloadPhaseTransition::Finished =>
            {
                assert_eq!(row.status.as_deref(), Some("ok"));
                finished = true;
            }
            Payload::Progress(row) => {
                assert_eq!(row.total.as_deref(), Some("1"));
                assert_eq!(row.unit, PayloadProgressUnit::WorkUnits);
                progress_counts.push(row.current.clone());
                progress |= row.current == "1";
            }
            Payload::State(row) if row.observed == stream::Observed::ResultAvailable => {
                assert_eq!(row.result_ref, final_reply.terminal_ref);
            }
            _ => {}
        }
    }
    assert_eq!(bytes, [vec![255, 65, 0], vec![254, 66, 0]]);
    assert!(finished && progress);
    assert_eq!(progress_counts.first().map(String::as_str), Some("0"));
    assert_eq!(progress_counts.last().map(String::as_str), Some("1"));
    assert!(
        progress_counts
            .iter()
            .all(|count| count == "0" || count == "1")
    );
    let Observation::ResultAvailable { result, .. } = owner.inspect().unwrap() else {
        panic!("retained report")
    };
    let report: serde_json::Value =
        serde_json::from_slice(result.report_bytes.as_ref().unwrap()).unwrap();
    let contributions = report["contributions"].as_array().unwrap();
    let row = contributions
        .iter()
        .find(|row| row["phase"] == "validate")
        .unwrap();
    assert_eq!(row["stdout"], String::from_utf8_lossy(&bytes[0]).as_ref());
    assert_eq!(row["stderr"], String::from_utf8_lossy(&bytes[1]).as_ref());
    assert_eq!(row["status"], "ok");
    assert_eq!(std::fs::read(c.root.join("counter")).unwrap(), b"effect\n");
}

#[cfg(target_os = "linux")]
#[test]
fn delivery_queue_loss_and_history_eviction_are_visible() {
    let s = super::super::tests::setup(false);
    let (owner, _observer, binding) = super::super::tests::begin(&s, None);
    // Consumer cannot run while history is held. Producer uses try_send and
    // finishes without acquiring history, even with a full bounded queue.
    let history = binding.history.lock().unwrap();
    let (sender, records) = std::sync::mpsc::sync_channel(1);
    let shared = owner.shared.clone();
    let receiver = std::thread::spawn(move || receive(shared, records));
    let spec = ProcessSpec {
        program: "/bin/sh".into(),
        args: vec![
            "-c".into(),
            "i=0; while [ \"$i\" -lt 10000 ]; do printf '\\377ABC'; i=$((i + 1)); done".into(),
        ],
        cwd: s.coordinates.root.clone(),
        env: Default::default(),
        stdin: None,
        stdout: StreamMode::Capture,
        stderr: StreamMode::Capture,
        scratch: s.coordinates.root.clone(),
    };
    let (done, result) = std::sync::mpsc::sync_channel(1);
    let producer = std::thread::spawn(move || {
        let outcome = SystemProcessRunner
            .run_observed(
                &spec,
                ObservedCapture {
                    records: Some(sender),
                    ..Default::default()
                },
            )
            .unwrap();
        done.send(outcome).unwrap();
    });
    let outcome = result
        .recv_timeout(Duration::from_secs(10))
        .expect("actual capture completes while history/consumer is blocked");
    producer.join().unwrap();
    assert_eq!(outcome.output.code, Some(0));
    assert_eq!(outcome.stdout.drained_bytes, 40000);
    assert!(outcome.stdout.dropped_delivery_bytes > 0);
    assert_eq!(outcome.output.stdout, [255, 65, 66, 67].repeat(10000));
    drop(history);
    let next = receiver.join().unwrap();
    assert!(next[0] < outcome.stdout.drained_bytes);
    summarize(
        &owner.shared,
        CaptureOrigin::Stdout,
        next[0],
        &outcome.stdout,
    );
    let reply = replay(&owner, &binding, "0");
    assert!(reply.events.iter().any(
        |row| matches!(&row.payload, Payload::RawOutput(p) if p.truncated && p.hex_bytes.is_empty())
    ));
    assert!(reply.events.iter().any(
        |row| matches!(&row.payload, Payload::Diagnostic(p) if p.message.contains("delivery_lost="))
    ));
    for _ in 0..100 {
        diagnostic(
            &owner.shared,
            PayloadDiagnosticSeverity::Info,
            &"observed".repeat(256),
        );
    }
    let evicted = replay(&owner, &binding, "0");
    assert!(evicted.gap.is_some());
    assert_eq!(evicted.gap.unwrap().reason, GapReason::Evicted);
    let tip = evicted.newest_sequence.unwrap();
    diagnostic(
        &owner.shared,
        PayloadDiagnosticSeverity::Error,
        &"x".repeat(5000),
    );
    let oversized = replay(&owner, &binding, &tip);
    assert!(oversized.events.is_empty());
    assert_eq!(oversized.gap.unwrap().reason, GapReason::Oversize);
    super::super::tests::finish(&owner, &s);
}

#[cfg(target_os = "linux")]
#[test]
fn production_failure_diagnostic_agrees_with_retained_report() {
    let f = NativeFixture::new();
    std::fs::write(
        f.0.path().join("run.sh"),
        "#!/bin/sh\nprintf 'effect\\n' >> counter\nprintf 'actual failure' >&2\nexit 7\n",
    )
    .unwrap();
    let c = f.coordinates();
    let (owner, _observer) = start(
        c.root.clone(),
        c.namespace.clone(),
        c.scope.clone(),
        c.key.clone(),
        request(),
        TargetOs::Linux,
        "2026-09-30T00:00:05Z".parse().unwrap(),
    )
    .unwrap();
    let binding = owner.wait_ready(Duration::from_secs(10)).unwrap();
    joined(&owner);
    let reply = replay(&owner, &binding, "0");
    assert!(reply.terminal_ref.is_some());
    assert!(
        reply
            .events
            .iter()
            .any(|row| matches!(&row.payload, Payload::Diagnostic(p)
        if p.severity == PayloadDiagnosticSeverity::Error))
    );
    assert!(
        reply
            .events
            .iter()
            .any(|row| matches!(&row.payload, Payload::Phase(p)
        if p.phase == "validate" && p.status.as_deref() == Some("fail")))
    );
    let Observation::ResultAvailable { result, .. } = owner.inspect().unwrap() else {
        panic!("failed report retained")
    };
    let report: serde_json::Value =
        serde_json::from_slice(result.report_bytes.as_ref().unwrap()).unwrap();
    let row = report["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["phase"] == "validate")
        .unwrap();
    assert_eq!(row["status"], "fail");
    assert_eq!(row["stderr"], "actual failure");
    assert_eq!(std::fs::read(c.root.join("counter")).unwrap(), b"effect\n");
}
