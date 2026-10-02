//! Actual native fixed-manifest owner proofs; compile only in this packet.
use super::*;
use std::fs;
use std::time::Instant;
const WAIT: Duration = Duration::from_secs(15);
const MANIFEST: &str = "[project]\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'once'\npoint = 'phase:validate'\nhandler = { kind = 'script', base = 'run' }\n";
pub(super) struct Fixture {
    dir: tempfile::TempDir,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
}
impl Fixture {
    pub(super) fn new(failing: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), MANIFEST).unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        fs::write(dir.path().join("run.sh"), format!(
            "#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\ntouch entered\ni=0\nwhile [ ! -f release ]; do i=$((i+1)); if [ \"$i\" -gt 400 ]; then exit 70; fi; sleep 0.05; done\ntouch finished\nexit {}\n", if failing { 23 } else { 0 }
        )).unwrap();
        let scope = SubmissionScope {
            context: "admitted-native".into(),
            target: "root".into(),
            principal: "host".into(),
        };
        let lease = vibe_lifecycle::LifecycleLease::acquire(dir.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
        drop(lease);
        Self {
            dir,
            namespace,
            scope,
        }
    }
    pub(super) fn host(&self) -> Arc<FixedLifecycleDecisionHost> {
        Arc::new(
            FixedLifecycleDecisionHost::from_host(
                self.dir.path().into(),
                self.namespace.clone(),
                self.scope.clone(),
                TargetOs::Linux,
                facts(),
            )
            .unwrap(),
        )
    }
    fn count_one(&self) {
        assert_eq!(
            fs::read(self.dir.path().join("counter")).unwrap(),
            b"effect\n"
        );
    }
    fn release(&self) {
        fs::write(self.dir.path().join("release"), "release").unwrap();
    }
    fn no_effect(&self) {
        assert!(!self.dir.path().join("counter").exists());
        let lease = vibe_lifecycle::LifecycleLease::acquire(self.dir.path()).unwrap();
        assert!(SubmissionReservations::inspect_existing_key(
            &lease,
            &self.namespace,
            &self.scope,
            "once"
        )
        .unwrap()
        .is_none());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::write(self.dir.path().join("release"), "cleanup");
    }
}
pub(super) fn facts() -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: true,
        allow_detach: true,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: true,
    }
}
fn options(control: RequestedOptionsControl) -> RequestedOptions {
    RequestedOptions {
        control,
        force: false,
        trace: false,
        deploy_profile: None,
    }
}
pub(super) fn approved(
    host: &FixedLifecycleDecisionHost,
    control: RequestedOptionsControl,
) -> ValidatedFixedPlanEvidence {
    let mut q = host.prepare_requested_options(options(control)).unwrap();
    if matches!(q.body, Body::SelectionQuestion(_)) {
        let mut answer = q.clone();
        answer.body = Body::SelectionAnswer(Box::new(BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        q = match host.answer_typed(&answer).unwrap() {
            DecisionOutcome::Question(q) => q,
            _ => panic!("resulting approval"),
        };
    }
    q.body = Body::ApprovalAnswer(Box::new(BodyApprovalAnswer { approved: true }));
    match host.answer_typed(&q).unwrap() {
        DecisionOutcome::Validated(e) => e,
        _ => panic!("sealed evidence"),
    }
}
fn now() -> Timestamp {
    "2026-10-02T00:00:00Z".parse().unwrap()
}
fn until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !predicate() {
        assert!(Instant::now() < deadline, "finite native deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn ready(job: &ReadonlyFixedJob) -> String {
    let mut run = None;
    until(
        || match job.wait_ready(Duration::from_millis(10)).unwrap() {
            Readiness::Ready(summary) => {
                run = Some(summary.run_id().to_owned());
                true
            }
            Readiness::Starting => false,
            other => panic!("actual Ready required: {other:?}"),
        },
    );
    run.unwrap()
}
fn terminal(job: &ReadonlyFixedJob) -> FixedOperationStatus {
    let mut status = None;
    until(|| {
        let s = job.inspect().unwrap();
        if matches!(
            s,
            FixedOperationStatus::Starting | FixedOperationStatus::Running { .. }
        ) {
            false
        } else {
            status = Some(s);
            true
        }
    });
    status.unwrap()
}
#[test]
fn concurrent_shared_service_same_actual_job_ready_query_replay_fresh_question_and_conflict() {
    let f = Fixture::new(false);
    let host = f.host();
    let evidence = approved(&host, RequestedOptionsControl::None);
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut calls = Vec::new();
    for _ in 0..2 {
        let (h, e, b) = (host.clone(), evidence.clone(), barrier.clone());
        calls.push(std::thread::spawn(move || {
            b.wait();
            h.submit_no_control(&e, "once", now()).unwrap()
        }));
    }
    barrier.wait();
    let a = calls.remove(0).join().unwrap();
    let b = calls.remove(0).join().unwrap();
    let run = ready(&a);
    assert_eq!(ready(&b), run);
    until(|| f.dir.path().join("entered").exists());
    f.count_one();
    assert!(
        matches!(b.inspect().unwrap(), FixedOperationStatus::Running { run_id } if run_id == run)
    );
    let mut observer = a.attach_readonly().unwrap();
    assert!(observer.detach());
    let retry = host.submit_no_control(&evidence, "once", now()).unwrap();
    assert_eq!(ready(&retry), run);
    f.release();
    assert!(
        matches!(terminal(&a), FixedOperationStatus::ResultAvailable { run_id, report_bytes: Some(_), .. } if run_id == run)
    );
    let fresh = approved(&host, RequestedOptionsControl::None);
    assert_ne!(fresh.question_id, evidence.question_id);
    assert_eq!(fresh.identity(), evidence.identity());
    assert_eq!(
        ready(&host.submit_no_control(&fresh, "once", now()).unwrap()),
        run
    );
    let mut changed = facts();
    changed.allow_detach = false;
    host.set_host_policy(changed).unwrap();
    let different = approved(&host, RequestedOptionsControl::None);
    assert!(matches!(
        host.submit_no_control(&different, "once", now()),
        Err(FixedDecisionError::AnswerConflict)
    ));
    f.count_one();
    // Cold observation must use the retained capsule and C1, not current files.
    fs::remove_file(f.dir.path().join("run.sh")).unwrap();
    fs::write(f.dir.path().join("vibe.toml"), "broken current source").unwrap();
    let cold = f.host();
    assert!(
        matches!(cold.inspect_existing("once").unwrap(), Some(FixedOperationStatus::ResultAvailable { run_id, .. }) if run_id == run)
    );
    assert!(matches!(
        cold.submit_no_control(&evidence, "new", now()),
        Err(FixedDecisionError::ForeignScope)
    ));
    f.count_one();
}
#[test]
fn foreign_host_revocation_and_unsupported_controls_do_not_reserve() {
    let f = Fixture::new(false);
    let h = f.host();
    let foreign = f.host();
    let e = approved(&h, RequestedOptionsControl::None);
    assert!(matches!(
        foreign.submit_no_control(&e, "once", now()),
        Err(FixedDecisionError::ForeignScope)
    ));
    for control in [
        RequestedOptionsControl::Detach,
        RequestedOptionsControl::CancelPreDispatch,
        RequestedOptionsControl::CancelDeferredBoundary,
    ] {
        let e = approved(&h, control);
        assert!(matches!(
            h.submit_no_control(&e, "once", now()),
            Err(FixedDecisionError::ImplementationGap(_))
        ));
    }
    let mut revoked = facts();
    revoked.approve_captured_script = false;
    h.set_host_policy(revoked).unwrap();
    assert!(matches!(
        h.submit_no_control(&e, "once", now()),
        Err(FixedDecisionError::StalePolicy)
    ));
    f.no_effect();
}
#[test]
fn source_drift_before_async_admission_does_not_reserve() {
    let f = Fixture::new(false);
    let h = f.host();
    let e = approved(&h, RequestedOptionsControl::None);
    fs::write(f.dir.path().join("run.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    let job = h.submit_no_control(&e, "once", now()).unwrap();
    assert!(matches!(
        terminal(&job),
        FixedOperationStatus::RecoveryRequired { run_id: None, .. }
    ));
    f.no_effect();
}
#[test]
fn actual_dispatch_guard_rechecks_revocation_after_factory_and_before_c1() {
    let f = Fixture::new(false);
    let h = f.host();
    let e = approved(&h, RequestedOptionsControl::None);
    let mut profile = AdmittedProfile::from_owned(&h, &e).unwrap();
    let revoke = h.clone();
    profile.before_guard = Some(Arc::new(move || {
        let mut facts = facts();
        facts.approve_captured_script = false;
        revoke.set_host_policy(facts).unwrap();
    }));
    let owner = HostOwnedFixedOperation::start_admitted(
        f.dir.path().into(),
        f.namespace.clone(),
        f.scope.clone(),
        "once".into(),
        TargetOs::Linux,
        now(),
        profile,
    )
    .unwrap();
    let job = ReadonlyFixedJob { owner };
    assert!(matches!(
        terminal(&job),
        FixedOperationStatus::RecoveryRequired { run_id: None, .. }
    ));
    f.no_effect();
}
#[test]
fn root_replacement_at_actual_guard_does_not_dispatch() {
    let f = Fixture::new(false);
    let h = f.host();
    let e = approved(&h, RequestedOptionsControl::None);
    let mut profile = AdmittedProfile::from_owned(&h, &e).unwrap();
    let root = f.dir.path().to_path_buf();
    let moved = root.with_extension("retained-old-root");
    let moved_hook = moved.clone();
    profile.before_guard = Some(Arc::new(move || {
        fs::rename(&root, &moved_hook).unwrap();
        fs::create_dir(&root).unwrap();
    }));
    let owner = HostOwnedFixedOperation::start_admitted(
        f.dir.path().into(),
        f.namespace.clone(),
        f.scope.clone(),
        "once".into(),
        TargetOs::Linux,
        now(),
        profile,
    )
    .unwrap();
    let job = ReadonlyFixedJob { owner };
    // The physical scope check may itself refuse inspection on the replaced root.
    until(|| {
        !matches!(
            job.wait_ready(Duration::from_millis(10)),
            Ok(Readiness::Starting)
        )
    });
    assert!(!moved.join("counter").exists());
    fs::remove_dir_all(f.dir.path()).unwrap();
    fs::rename(&moved, f.dir.path()).unwrap();
    f.no_effect();
}
#[test]
fn post_effect_failure_and_damaged_marker_report_capsule_keep_identity_without_redispatch() {
    for damage in ["failure", "marker", "report", "capsule", "c1"] {
        let f = Fixture::new(damage == "failure");
        let h = f.host();
        let e = approved(&h, RequestedOptionsControl::None);
        let job = h.submit_no_control(&e, "once", now()).unwrap();
        let run = ready(&job);
        until(|| f.dir.path().join("entered").exists());
        f.release();
        let result = terminal(&job);
        assert!(
            matches!(&result, FixedOperationStatus::ResultAvailable { run_id, report_bytes: Some(_), .. } if run_id == &run),
            "before {damage} injection expected retained result for {run}; actual {result:?}"
        );
        if damage == "failure" {
            let FixedOperationStatus::ResultAvailable { marker, .. } = result else {
                unreachable!()
            };
            assert_eq!(marker.outcome, vibe_wire::generated::lifecycle_outcome_marker::LifecycleOutcomeMarkerOutcome::Failed);
        }
        f.count_one();
        if damage != "failure" {
            if damage == "c1" {
                let path = f.dir.path().join(".vibe/machine-submissions.json");
                let mut index: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                index["namespaces"][f.namespace.id()]["keys"]["once"]["request_sha256"] =
                    serde_json::Value::String("0".repeat(64));
                fs::write(path, serde_json::to_vec(&index).unwrap()).unwrap();
            } else {
                let suffix = match damage {
                    "marker" => "marker.json",
                    "report" => "report.json",
                    _ => "admitted-input.json",
                };
                fs::write(
                    f.dir
                        .path()
                        .join(".vibe/lifecycle-outcomes")
                        .join(format!("{run}.{suffix}")),
                    "damaged",
                )
                .unwrap();
            }
            let status = if damage == "capsule" {
                h.inspect_existing("once").unwrap().unwrap()
            } else {
                job.inspect().unwrap()
            };
            assert!(
                matches!(status, FixedOperationStatus::RecoveryRequired { run_id: Some(id), .. } if id == run)
            );
        }
        let retry = h.submit_no_control(&e, "once", now()).unwrap();
        assert_eq!(ready(&retry), run);
        f.count_one();
    }
}

#[test]
fn source_replacement_at_actual_guard_does_not_reserve() {
    let f = Fixture::new(false);
    let h = f.host();
    let e = approved(&h, RequestedOptionsControl::None);
    let mut profile = AdmittedProfile::from_owned(&h, &e).unwrap();
    let script = f.dir.path().join("run.sh");
    profile.before_guard = Some(Arc::new(move || {
        fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    }));
    let owner = HostOwnedFixedOperation::start_admitted(
        f.dir.path().into(),
        f.namespace.clone(),
        f.scope.clone(),
        "once".into(),
        TargetOs::Linux,
        now(),
        profile,
    )
    .unwrap();
    let job = ReadonlyFixedJob { owner };
    assert!(matches!(
        terminal(&job),
        FixedOperationStatus::RecoveryRequired { run_id: None, .. }
    ));
    f.no_effect();
}

#[test]
fn invalid_c1_keys_refuse_before_job_claim_or_reservation() {
    let f = Fixture::new(false);
    let host = f.host();
    let evidence = approved(&host, RequestedOptionsControl::None);
    for key in [
        "".to_owned(),
        " padded".to_owned(),
        "padded ".to_owned(),
        "line\nbreak".to_owned(),
        "x".repeat(257),
        "é".repeat(129),
    ] {
        assert!(matches!(
            host.submit_no_control(&evidence, &key, now()),
            Err(FixedDecisionError::InvalidData)
        ));
        assert!(host.jobs.lock().unwrap().is_empty());
    }
    assert!(!f.dir.path().join("counter").exists());
    assert!(!f.dir.path().join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn same_owner_terminal_inspections_serialize_fresh_native_evidence() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Barrier, Mutex,
    };
    let f = Fixture::new(false);
    let host = f.host();
    let evidence = approved(&host, RequestedOptionsControl::None);
    let job = host.submit_no_control(&evidence, "once", now()).unwrap();
    let run = ready(&job);
    until(|| f.dir.path().join("entered").exists());
    let (held_tx, held_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let first = AtomicBool::new(true);
    job.owner.after_cold_lease_for_test(Arc::new(move || {
        if first.swap(false, Ordering::SeqCst) {
            held_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv_timeout(WAIT).unwrap();
        }
    }));
    // The worker's actual terminal publication holds a real cold writer lease.
    f.release();
    held_rx.recv_timeout(WAIT).unwrap();
    let start = Arc::new(Barrier::new(5));
    let (done_tx, done_rx) = mpsc::channel();
    let mut readers = Vec::new();
    for _ in 0..4 {
        let (job, start, done) = (job.clone(), start.clone(), done_tx.clone());
        readers.push(std::thread::spawn(move || {
            start.wait();
            let first = job.inspect().unwrap();
            done.send(first.clone()).unwrap();
            for _ in 0..16 {
                assert_eq!(job.inspect().unwrap(), first);
            }
            first
        }));
    }
    start.wait();
    // Without owner coordination these return RecoveryRequired/Busy while the
    // publication lease is held. Always release before asserting or joining.
    let premature = done_rx.recv_timeout(Duration::from_millis(100));
    release_tx.send(()).unwrap();
    let results: Vec<_> = readers.into_iter().map(|r| r.join().unwrap()).collect();
    assert!(
        matches!(premature, Err(mpsc::RecvTimeoutError::Timeout)),
        "terminal inspection escaped the held owner evidence access: {premature:?}"
    );
    for result in &results {
        assert!(
            matches!(result,
            FixedOperationStatus::ResultAvailable { run_id, report_bytes: Some(_), .. }
            if run_id == &run),
            "native retained evidence required: {result:?}"
        );
        assert_eq!(result, &results[0]);
    }
    f.count_one();
    // External Busy remains a refusal; the lock is coordination, not reentry.
    let external = vibe_lifecycle::LifecycleLease::acquire(f.dir.path()).unwrap();
    assert!(matches!(job.inspect().unwrap(),
        FixedOperationStatus::RecoveryRequired { run_id: Some(id), .. } if id == run));
    drop(external);
    assert_eq!(job.inspect().unwrap(), results[0]);
    let retry = host.submit_no_control(&evidence, "once", now()).unwrap();
    assert_eq!(ready(&retry), run);
    f.count_one();
}
