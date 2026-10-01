//! Counted/blocking seams prove host lifetime behavior, not real process death.
use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use vibe_lifecycle::process::{ProcessError, ProcessOutput, ProcessSpec};

struct BlockingProcess {
    root: PathBuf,
    count: AtomicUsize,
    entered: Sender<()>,
    release: Mutex<Receiver<()>>,
    panic_after_ready: bool,
}
impl ProcessRunner for BlockingProcess {
    fn run(&self, _: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        assert!(
            std::fs::read_dir(self.root.join(".vibe/lifecycle-outcomes"))
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".script-source.json")),
            "counted provider must follow durable source stage"
        );
        self.count.fetch_add(1, Ordering::SeqCst);
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        assert!(
            !self.panic_after_ready,
            "injected owner-thread panic after Ready"
        );
        Ok(ProcessOutput {
            code: Some(0),
            ..Default::default()
        })
    }
}
struct Setup {
    _dir: tempfile::TempDir,
    coordinates: HostCoordinates,
    process: Arc<BlockingProcess>,
    entered: Receiver<()>,
    release: Sender<()>,
}
fn setup(panic_after_ready: bool) -> Setup {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vibe.toml"), ONE_SCRIPT_VALIDATE_MANIFEST).unwrap();
    let script = if TargetOs::current() == Some(TargetOs::Windows) {
        "run.ps1"
    } else {
        "run.sh"
    };
    std::fs::write(dir.path().join(script), b"echo one\n").unwrap();
    let scope = SubmissionScope {
        context: "host-context".into(),
        target: "root".into(),
        principal: "owner".into(),
    };
    let namespace = {
        let lease = LifecycleLease::acquire(dir.path()).unwrap();
        SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap()
    };
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let root = dir.path().to_owned();
    Setup {
        coordinates: HostCoordinates {
            root: dir.path().to_owned(),
            namespace,
            scope,
            key: "one".into(),
        },
        _dir: dir,
        process: Arc::new(BlockingProcess {
            root,
            count: AtomicUsize::new(0),
            entered: entered_tx,
            release: Mutex::new(release_rx),
            panic_after_ready,
        }),
        entered,
        release,
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
fn begin(
    s: &Setup,
    hook: Option<Arc<dyn Fn(&Binding) + Send + Sync>>,
) -> (LiveOwner, Observer, Binding) {
    let c = &s.coordinates;
    let (owner, observer) = launch(OwnedDispatch {
        path: c.root.clone(),
        namespace: c.namespace.clone(),
        scope: c.scope.clone(),
        key: c.key.clone(),
        request: request(),
        target_os: TargetOs::current().unwrap(),
        observed_at: "2026-09-30T00:00:05Z".parse().unwrap(),
        fixture: Some(TestFixture {
            process: s.process.clone(),
            before_effect: hook,
        }),
    })
    .unwrap();
    let binding = owner
        .wait_ready(Duration::from_secs(10))
        .expect("actual Ready binding");
    s.entered.recv_timeout(Duration::from_secs(10)).unwrap();
    (owner, observer, binding)
}
fn finish(owner: &LiveOwner, s: &Setup) {
    s.release.send(()).unwrap();
    let record = owner.shared.record.lock().unwrap();
    let (record, timeout) = owner
        .shared
        .changed
        .wait_timeout_while(record, Duration::from_secs(10), |r| {
            matches!(r.state, LiveState::Starting | LiveState::Running)
        })
        .unwrap();
    assert!(!timeout.timed_out(), "owner must leave Running");
    drop(record);
    owner.worker.lock().unwrap().take().unwrap().join().unwrap();
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
}
#[test]
fn ready_binds_real_c1_before_stage_scratch_or_counted_provider() {
    let s = setup(false);
    let seen = Arc::new(AtomicBool::new(false));
    let seen_hook = seen.clone();
    let process = s.process.clone();
    let hook = Arc::new(move |binding: &Binding| {
        let record = binding.owner.upgrade().unwrap();
        let lease = record.record.lock().unwrap().lease.clone().unwrap();
        // The Ready seam itself establishes ordering; no receipt/path race.
        let c = &binding.coordinates;
        let row =
            SubmissionReservations::inspect_existing_key(&lease, &c.namespace, &c.scope, &c.key)
                .unwrap()
                .unwrap();
        assert_eq!(row.domain_run_id, binding.run_id);
        assert_eq!(row.reservation_ref.as_ref(), Some(&binding.reservation_ref));
        assert_eq!(row.request_sha256, binding.request_sha256);
        assert_eq!(row.plan_sha256, binding.plan_sha256);
        assert_eq!(row.journal_admission, JournalAdmission::Ready);
        assert_eq!(
            LifecycleStateStore::peek_with_lease(&lease)
                .unwrap()
                .unwrap()
                .run
                .run_id
                .as_deref(),
            Some(binding.run_id.as_str())
        );
        assert!(!c.root.join(".vibe/lifecycle-outcomes").exists());
        assert!(!c.root.join(".vibe/lifecycle").exists());
        assert_eq!(process.count.load(Ordering::SeqCst), 0);
        seen_hook.store(true, Ordering::SeqCst);
    });
    let (owner, observer, binding) = begin(&s, Some(hook));
    assert!(seen.load(Ordering::SeqCst));
    assert_eq!(
        owner.policy(),
        ConnectionLossPolicy::ContinueCurrentDispatch
    );
    assert_eq!(
        observer.inspect().unwrap(),
        Observation::Running {
            run_id: binding.run_id.clone()
        }
    );
    let second = owner.attach(&binding, &s.coordinates).unwrap();
    assert_eq!(second.inspect().unwrap(), observer.inspect().unwrap());
    assert!(matches!(
        LifecycleLease::acquire(&s.coordinates.root),
        Err(vibe_lifecycle::LifecycleLeaseError::Busy { .. })
    ));
    finish(&owner, &s);
}
#[test]
fn foreign_coordinates_and_handles_refuse_without_second_execution() {
    let s = setup(false);
    let (owner, _observer, binding) = begin(&s, None);
    let foreign = tempfile::tempdir().unwrap();
    let mut root = s.coordinates.clone();
    root.root = foreign.path().to_owned();
    let mut scope = s.coordinates.clone();
    scope.scope.context.push_str("foreign");
    let mut key = s.coordinates.clone();
    key.key.push_str("foreign");
    let mut namespace = s.coordinates.clone();
    namespace.namespace = {
        let lease = LifecycleLease::acquire(foreign.path()).unwrap();
        SubmissionReservations::issue_namespace(&lease, &namespace.scope, 60_000).unwrap()
    };
    for c in [root, scope, key, namespace] {
        assert!(owner.attach(&binding, &c).is_err());
    }
    let mut run = binding.clone();
    run.run_id.push_str("foreign");
    assert!(owner.attach(&run, &s.coordinates).is_err());
    let mut correlation = binding.clone();
    correlation.reservation_ref.push_str("foreign");
    assert!(owner.attach(&correlation, &s.coordinates).is_err());
    let mut hashes = binding.clone();
    hashes.plan_sha256.push_str("foreign");
    assert!(owner.attach(&hashes, &s.coordinates).is_err());
    // A separately allocated owner cannot accept an incumbent's opaque handle.
    let foreign_owner = LiveOwner {
        worker: Mutex::new(None),
        shared: Arc::new(Shared {
            record: Mutex::new(Record {
                state: LiveState::Starting,
                binding: None,
                lease: None,
                canonical_request: vec![],
                reason: String::new(),
                subscriptions: BTreeSet::new(),
                next_subscription: 1,
            }),
            changed: Condvar::new(),
        }),
    };
    assert!(foreign_owner.attach(&binding, &s.coordinates).is_err());
    finish(&owner, &s);
}
#[test]
fn dropping_and_explicit_detaching_observers_preserve_one_dispatch_and_lease() {
    let s = setup(false);
    let (owner, observer, binding) = begin(&s, None);
    let mut second = owner.attach(&binding, &s.coordinates).unwrap();
    assert_eq!(second.cancel(), Cancellation::Unavailable);
    assert_eq!(owner.state(), LiveState::Running);
    drop(observer);
    assert!(second.detach());
    assert!(!second.detach());
    assert!(second.inspect().is_err());
    assert!(owner.shared.record.lock().unwrap().subscriptions.is_empty());
    assert_eq!(owner.state(), LiveState::Running);
    assert!(matches!(
        LifecycleLease::acquire(&s.coordinates.root),
        Err(vibe_lifecycle::LifecycleLeaseError::Busy { .. })
    ));
    finish(&owner, &s);
    let attached = owner.attach(&binding, &s.coordinates).unwrap();
    let Observation::ResultAvailable { run_id, result } = attached.inspect().unwrap() else {
        panic!("domain result must be retained")
    };
    assert_eq!(run_id, binding.run_id);
    assert_eq!(result.marker.outcome, MarkerOutcome::Completed);
    assert!(result.report_bytes.is_some());
    let canonical_request = owner
        .shared
        .record
        .lock()
        .unwrap()
        .canonical_request
        .clone();
    // Cold inspection also works after all observation handles are dropped.
    drop(attached);
    drop(second);
    drop(binding);
    drop(owner);
    let cold_lease = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    assert_eq!(
        attach_existing_lifecycle(
            &cold_lease,
            &s.coordinates.namespace,
            &s.coordinates.scope,
            &s.coordinates.key,
            &canonical_request
        )
        .unwrap(),
        Some(SubmissionOutcome::Replayed {
            run_id,
            outcome: result
        })
    );
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
}
#[test]
fn terminal_attach_revalidates_c1_and_marker_report_instead_of_cached_completion() {
    let s = setup(false);
    let (owner, observer, binding) = begin(&s, None);
    finish(&owner, &s);
    let original = owner.inspect().unwrap();
    assert!(matches!(original, Observation::ResultAvailable { .. }));
    let canonical_request = owner
        .shared
        .record
        .lock()
        .unwrap()
        .canonical_request
        .clone();
    let lease = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    let retained = {
        attach_existing_lifecycle(
            &lease,
            &s.coordinates.namespace,
            &s.coordinates.scope,
            &s.coordinates.key,
            &canonical_request,
        )
        .unwrap()
        .unwrap()
    };
    assert!(matches!(retained, SubmissionOutcome::Replayed { .. }));
    drop(lease);
    let marker = s
        .coordinates
        .root
        .join(".vibe/lifecycle-outcomes")
        .join(format!("{}.marker.json", binding.run_id));
    let bytes = std::fs::read(&marker).unwrap();
    std::fs::remove_file(&marker).unwrap();
    assert!(
        matches!(observer.inspect().unwrap(), Observation::RecoveryRequired { run_id: Some(id), .. } if id == binding.run_id)
    );
    assert!(matches!(
        owner
            .attach(&binding, &s.coordinates)
            .unwrap()
            .inspect()
            .unwrap(),
        Observation::RecoveryRequired { .. }
    ));
    std::fs::write(&marker, b"tampered").unwrap();
    assert!(matches!(
        owner.inspect().unwrap(),
        Observation::RecoveryRequired { .. }
    ));
    std::fs::write(&marker, bytes).unwrap();
    assert_eq!(owner.inspect().unwrap(), original);
    let Observation::ResultAvailable { result, .. } = &original else {
        unreachable!()
    };
    let report_bytes = result.report_bytes.as_ref().unwrap();
    let report = std::fs::read_dir(s.coordinates.root.join(".vibe/lifecycle-outcomes"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| std::fs::read(path).ok().as_ref() == Some(report_bytes))
        .expect("domain retained exact report bytes");
    std::fs::write(&report, b"tampered report").unwrap();
    assert!(matches!(
        owner.inspect().unwrap(),
        Observation::RecoveryRequired { .. }
    ));
    std::fs::write(&report, report_bytes).unwrap();
    assert_eq!(owner.inspect().unwrap(), original);
    let index = s.coordinates.root.join(".vibe/machine-submissions.json");
    std::fs::remove_file(index).unwrap();
    assert!(matches!(
        owner.inspect().unwrap(),
        Observation::RecoveryRequired { .. }
    ));
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
}
#[test]
fn panic_after_ready_reports_same_run_recovery_and_never_retries() {
    let s = setup(true);
    let (owner, observer, binding) = begin(&s, None);
    finish(&owner, &s);
    assert_eq!(owner.state(), LiveState::OwnerStopped);
    assert!(
        matches!(observer.inspect().unwrap(), Observation::RecoveryRequired { run_id: Some(id), reason }
        if id == binding.run_id && reason.contains("owner thread panicked"))
    );
    assert!(
        matches!(owner.attach(&binding, &s.coordinates).unwrap().inspect().unwrap(),
        Observation::RecoveryRequired { run_id: Some(id), .. } if id == binding.run_id)
    );
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
}
#[test]
fn notification_source_or_header_drift_cannot_gain_effect() {
    for header in [false, true] {
        let s = setup(false);
        let c = &s.coordinates;
        let hook = Arc::new(move |binding: &Binding| {
            let path = if header {
                ".vibe/lifecycle.toml"
            } else {
                "run.sh"
            };
            std::fs::write(binding.coordinates.root.join(path), b"changed\n").unwrap();
            if !header {
                std::fs::write(binding.coordinates.root.join("run.ps1"), b"changed\n").unwrap();
            }
        });
        let (owner, _observer) = launch(OwnedDispatch {
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
        let binding = owner.wait_ready(Duration::from_secs(10)).unwrap();
        let record = owner.shared.record.lock().unwrap();
        let (record, timeout) = owner
            .shared
            .changed
            .wait_timeout_while(record, Duration::from_secs(10), |r| {
                r.state == LiveState::Running
            })
            .unwrap();
        assert!(!timeout.timed_out());
        drop(record);
        assert!(
            matches!(owner.inspect().unwrap(), Observation::RecoveryRequired { run_id: Some(id), .. } if id == binding.run_id)
        );
        assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
        assert!(!s.coordinates.root.join(".vibe/lifecycle-outcomes").exists());
    }
}

// Windows may refuse renaming a directory with retained open handles.
#[cfg(unix)]
#[test]
fn same_path_replaced_physical_root_refuses_live_attach() {
    let s = setup(false);
    let (owner, _observer, binding) = begin(&s, None);
    let original = &s.coordinates.root;
    let moved = original.with_extension("held-owner-root");
    std::fs::rename(original, &moved).unwrap();
    std::fs::create_dir(original).unwrap();
    assert!(owner.attach(&binding, &s.coordinates).is_err());
    assert!(owner.inspect().is_err());
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
    std::fs::remove_dir(original).unwrap();
    std::fs::rename(&moved, original).unwrap();
    finish(&owner, &s);
}

#[test]
fn initial_observer_detach_during_ready_notification_does_not_cancel_dispatch() {
    let s = setup(false);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (continue_tx, continue_rx) = mpsc::channel();
    let continue_rx = Mutex::new(continue_rx);
    let hook = Arc::new(move |_: &Binding| {
        ready_tx.send(()).unwrap();
        continue_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
    });
    let c = &s.coordinates;
    let (owner, mut observer) = launch(OwnedDispatch {
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
    assert!(observer.detach());
    drop(observer);
    let binding = owner.wait_ready(Duration::from_secs(10)).unwrap();
    assert!(owner.shared.record.lock().unwrap().subscriptions.is_empty());
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
    continue_tx.send(()).unwrap();
    s.entered.recv_timeout(Duration::from_secs(10)).unwrap();
    finish(&owner, &s);
    assert!(matches!(
        owner
            .attach(&binding, &s.coordinates)
            .unwrap()
            .inspect()
            .unwrap(),
        Observation::ResultAvailable { .. }
    ));
}

#[test]
fn failure_before_ready_stops_without_guessing_run_or_invoking_provider() {
    let s = setup(false);
    std::fs::write(s.coordinates.root.join("vibe.toml"), b"invalid manifest").unwrap();
    let c = &s.coordinates;
    let (owner, observer) = launch(OwnedDispatch {
        path: c.root.clone(),
        namespace: c.namespace.clone(),
        scope: c.scope.clone(),
        key: c.key.clone(),
        request: request(),
        target_os: TargetOs::current().unwrap(),
        observed_at: "2026-09-30T00:00:05Z".parse().unwrap(),
        fixture: Some(TestFixture {
            process: s.process.clone(),
            before_effect: None,
        }),
    })
    .unwrap();
    assert!(owner.wait_ready(Duration::from_secs(10)).is_none());
    owner.worker.lock().unwrap().take().unwrap().join().unwrap();
    assert_eq!(owner.state(), LiveState::OwnerStopped);
    assert!(matches!(
        observer.inspect().unwrap(),
        Observation::RecoveryRequired { run_id: None, .. }
    ));
    assert_eq!(s.process.count.load(Ordering::SeqCst), 0);
    assert!(!s.coordinates.root.join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn stopped_owner_releases_writer_lease_with_observers_still_alive() {
    for panicked in [false, true] {
        let s = setup(panicked);
        let (owner, observer, binding) = begin(&s, None);
        finish(&owner, &s);
        let lease = LifecycleLease::acquire(&s.coordinates.root)
            .expect("finished owner must release writer lease without dropping observer handles");
        drop(lease);
        assert!(owner.attach(&binding, &s.coordinates).is_ok());
        assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
        drop(observer);
    }
}

#[test]
fn terminal_inspection_with_another_writer_keeps_same_run_without_dispatch() {
    let s = setup(false);
    let (owner, observer, binding) = begin(&s, None);
    finish(&owner, &s);
    let writer = LifecycleLease::acquire(&s.coordinates.root).unwrap();
    for observed in [
        owner.inspect().unwrap(),
        observer.inspect().unwrap(),
        owner
            .attach(&binding, &s.coordinates)
            .unwrap()
            .inspect()
            .unwrap(),
    ] {
        assert!(matches!(observed,
            Observation::RecoveryRequired { run_id: Some(id), .. } if id == binding.run_id));
    }
    assert_eq!(s.process.count.load(Ordering::SeqCst), 1);
    drop(writer);
    assert!(matches!(
        owner.inspect().unwrap(),
        Observation::ResultAvailable { .. }
    ));
}
