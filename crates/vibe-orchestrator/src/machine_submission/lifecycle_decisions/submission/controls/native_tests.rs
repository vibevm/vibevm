//! Real SystemProcessRunner, C1 Ready, native exit/drain and retained receipts.
//! This packet compiles these oracles; runtime acceptance belongs to the parent.
use super::*;
use std::{fs, time::Instant};
use vibe_wire::generated::lifecycle_outcome_marker::{
    LifecycleOutcomeMarkerEffectState as MarkerEffect, LifecycleOutcomeMarkerOutcome as Outcome,
};
use vibe_wire::generated::machine::e1::operation_stream_message::{
    self as wire, BodyControlReplyOperationEffectState as Effect, BodyControlReplyStatus as Status,
};
const WAIT: Duration = Duration::from_secs(15);
struct Fixture {
    dir: tempfile::TempDir,
    host: FixedLifecycleDecisionHost,
}
impl Fixture {
    fn new(pre: bool, deferred: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), "[project]\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'once'\npoint = 'phase:validate'\nhandler = { kind = 'script', base = 'run' }\n").unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        fs::write(dir.path().join("run.sh"), format!("#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\nprintf 'kept\\n' > artifact\ntouch entered\ni=0\nwhile [ ! -f release ]; do i=$((i+1)); if [ \"$i\" -gt 400 ]; then exit 70; fi; sleep 0.05; done\ntouch finished\nexit 0\n")).unwrap();
        let scope = SubmissionScope {
            context: "scoped-native".into(),
            target: "root".into(),
            principal: "host".into(),
        };
        let lease = vibe_lifecycle::LifecycleLease::acquire(dir.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
        drop(lease);
        let host = FixedLifecycleDecisionHost::from_host(
            dir.path().into(),
            namespace,
            scope,
            TargetOs::Linux,
            HostPolicyFacts {
                approve_captured_script: true,
                allow_detach: true,
                allow_cancel_pre_dispatch: pre,
                allow_cancel_deferred_boundary: deferred,
            },
        )
        .unwrap();
        Self { dir, host }
    }
    fn evidence(&self, mode: RequestedOptionsControl) -> ValidatedFixedPlanEvidence {
        let mut q = self
            .host
            .prepare_requested_options(RequestedOptions {
                control: mode,
                force: false,
                trace: false,
                deploy_profile: None,
            })
            .unwrap();
        if matches!(q.body, Body::SelectionQuestion(_)) {
            let mut answer = q.clone();
            answer.body = Body::SelectionAnswer(Box::new(BodySelectionAnswer {
                choice_ids: vec!["none".into()],
            }));
            q = match self.host.answer_typed(&answer).unwrap() {
                DecisionOutcome::Question(q) => q,
                _ => panic!("approval required"),
            };
        }
        q.body = Body::ApprovalAnswer(Box::new(BodyApprovalAnswer { approved: true }));
        match self.host.answer_typed(&q).unwrap() {
            DecisionOutcome::Validated(e) => e,
            _ => panic!("sealed evidence required"),
        }
    }
    fn held(
        &self,
        e: &ValidatedFixedPlanEvidence,
    ) -> (ScopedFixedJob, std::sync::mpsc::Sender<()>) {
        let (release, receive) = std::sync::mpsc::channel();
        let receive = Mutex::new(receive);
        let mut profile = AdmittedProfile::from_owned(&self.host, e).unwrap();
        profile.after_ready = Some(Arc::new(move || {
            receive.lock().unwrap().recv_timeout(WAIT).unwrap()
        }));
        let owner = self.host.claim_owned(e, "once", now(), profile).unwrap();
        (
            ScopedFixedJob {
                owner,
                mode: e.options.control.clone(),
            },
            release,
        )
    }
    fn release(&self) {
        fs::write(self.dir.path().join("release"), "go").unwrap();
    }
    fn count(&self) {
        assert_eq!(
            fs::read(self.dir.path().join("counter")).unwrap(),
            b"effect\n"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::write(self.dir.path().join("release"), "cleanup");
    }
}
fn now() -> Timestamp {
    "2026-10-02T00:00:00Z".parse().unwrap()
}
fn until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !predicate() {
        assert!(Instant::now() < deadline, "finite native barrier");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn ready(job: &ScopedFixedJob) -> crate::machine_submission::BindingSummary {
    let mut result = None;
    until(
        || match job.wait_ready(Duration::from_millis(10)).unwrap() {
            Readiness::Ready(s) => {
                result = Some(s);
                true
            }
            Readiness::Starting => false,
            other => panic!("actual C1 Ready: {other:?}"),
        },
    );
    result.unwrap()
}
fn terminal(job: &ScopedFixedJob) -> FixedOperationStatus {
    until(|| {
        !matches!(
            job.inspect().unwrap(),
            FixedOperationStatus::Starting | FixedOperationStatus::Running { .. }
        )
    });
    job.inspect().unwrap()
}
fn request(
    reader: &ObservationHandle,
    id: &str,
    action: wire::BodyControlRequestAction,
) -> OperationStreamMessage {
    let s = reader.summary();
    OperationStreamMessage {
        schema: 1,
        context_id: s.context_id().into(),
        target_id: s.target_id().map(str::to_owned),
        operation_id: s.operation_id().into(),
        run_id: s.run_id().into(),
        stream_id: s.stream_id().into(),
        body: wire::Body::ControlRequest(Box::new(wire::BodyControlRequest {
            action,
            request_id: id.into(),
            after_sequence: "0".into(),
        })),
    }
}
fn cancel(reader: &ObservationHandle, id: &str) -> OperationStreamMessage {
    request(reader, id, wire::BodyControlRequestAction::Cancel)
}
fn row(
    job: &ScopedFixedJob,
    reader: &mut ObservationHandle,
    m: &OperationStreamMessage,
) -> wire::BodyControlReply {
    let wire::Body::ControlReply(r) = job.control(reader, m).unwrap().body else {
        panic!("typed reply")
    };
    *r
}
#[test]
fn pre_only_actual_ready_request_wins_no_provider_and_fresh_receipt_requery() {
    let f = Fixture::new(true, false);
    let e = f.evidence(RequestedOptionsControl::CancelPreDispatch);
    let (job, release) = f.held(&e);
    let summary = ready(&job);
    let retry = f.host.submit_scoped(&e, "once", now()).unwrap();
    assert_eq!(ready(&retry), summary);
    let mut reader = job.attach().unwrap();
    let m = cancel(&reader, "pre-stop");
    let r = row(&job, &mut reader, &m);
    assert_eq!(
        (r.status, r.operation_effect_state),
        (Status::Requested, Effect::NoEffects)
    );
    release.send(()).unwrap();
    let FixedOperationStatus::ResultAvailable {
        marker,
        report_bytes,
        ..
    } = terminal(&job)
    else {
        panic!("validated stop")
    };
    assert_eq!(
        (marker.outcome, marker.effect_state),
        (Outcome::Failed, MarkerEffect::None)
    );
    assert!(report_bytes.is_none());
    assert!(!f.dir.path().join("counter").exists());
    assert_eq!(row(&job, &mut reader, &m).status, Status::ReachedSafeStop);
    let late = cancel(&reader, "late-pre");
    assert_eq!(
        row(&job, &mut reader, &late).status,
        Status::AlreadyTerminal
    );
    assert_eq!(row(&job, &mut reader, &m).status, Status::ReachedSafeStop);
}
#[test]
fn defer_only_refuses_pre_then_native_active_stop_retains_partial_report_and_receipt() {
    let f = Fixture::new(false, true);
    let e = f.evidence(RequestedOptionsControl::CancelDeferredBoundary);
    let (job, release) = f.held(&e);
    let summary = ready(&job);
    let mut reader = job.attach().unwrap();
    let m = cancel(&reader, "defer-stop");
    assert_eq!(row(&job, &mut reader, &m).status, Status::Unavailable);
    release.send(()).unwrap();
    until(|| f.dir.path().join("entered").exists());
    f.count();
    let r = row(&job, &mut reader, &m);
    assert_eq!(
        (r.status, r.operation_effect_state),
        (Status::Requested, Effect::Uncertain)
    );
    f.release();
    let FixedOperationStatus::ResultAvailable {
        marker,
        report_bytes,
        ..
    } = terminal(&job)
    else {
        panic!("partial outcome")
    };
    assert_eq!(
        (marker.outcome, marker.effect_state),
        (Outcome::Uncertain, MarkerEffect::Uncertain)
    );
    assert!(report_bytes.is_some());
    assert_eq!(fs::read(f.dir.path().join("artifact")).unwrap(), b"kept\n");
    let reached = row(&job, &mut reader, &m);
    assert_eq!(
        (reached.status, reached.operation_effect_state),
        (Status::ReachedSafeStop, Effect::Uncertain)
    );
    let late = cancel(&reader, "late-defer");
    assert_eq!(
        row(&job, &mut reader, &late).status,
        Status::AlreadyTerminal
    );
    assert_eq!(row(&job, &mut reader, &m).status, Status::ReachedSafeStop);
    assert_eq!(
        ready(&f.host.submit_scoped(&e, "once", now()).unwrap()),
        summary
    );
    f.count();
}
#[test]
fn none_and_detach_refuse_cancel_pre_and_active_detach_keeps_one_job_alive() {
    for mode in [
        RequestedOptionsControl::None,
        RequestedOptionsControl::Detach,
    ] {
        let f = Fixture::new(false, false);
        let e = f.evidence(mode);
        let different = f.evidence(if e.options.control == RequestedOptionsControl::None {
            RequestedOptionsControl::Detach
        } else {
            RequestedOptionsControl::None
        });
        let (job, release) = f.held(&e);
        let summary = ready(&job);
        let mut reader = job.attach().unwrap();
        let m = cancel(&reader, "denied");
        assert_eq!(row(&job, &mut reader, &m).status, Status::Unavailable);
        assert!(matches!(
            f.host.submit_scoped(&different, "once", now()),
            Err(FixedDecisionError::AnswerConflict)
        ));
        release.send(()).unwrap();
        until(|| f.dir.path().join("entered").exists());
        assert_eq!(row(&job, &mut reader, &m).status, Status::Unavailable);
        let detach = request(&reader, "detach", wire::BodyControlRequestAction::Detach);
        assert_eq!(row(&job, &mut reader, &detach).status, Status::Detached);
        assert!(job.control(&mut reader, &m).is_err());
        assert_eq!(
            ready(&f.host.submit_scoped(&e, "once", now()).unwrap()),
            summary
        );
        f.release();
        assert!(matches!(
            terminal(&job),
            FixedOperationStatus::ResultAvailable { .. }
        ));
        f.count();
    }
}
#[test]
fn pre_only_dispatch_wins_never_grants_deferred_stop() {
    let f = Fixture::new(true, false);
    let e = f.evidence(RequestedOptionsControl::CancelPreDispatch);
    let job = f.host.submit_scoped(&e, "once", now()).unwrap();
    ready(&job);
    until(|| f.dir.path().join("entered").exists());
    let mut reader = job.attach().unwrap();
    let m = cancel(&reader, "too-late");
    assert_eq!(row(&job, &mut reader, &m).status, Status::Unavailable);
    f.release();
    let FixedOperationStatus::ResultAvailable { marker, .. } = terminal(&job) else {
        panic!("terminal")
    };
    assert_ne!(marker.outcome, Outcome::Uncertain);
    let r = row(&job, &mut reader, &m);
    assert_eq!(r.status, Status::AlreadyTerminal);
    f.count();
}
#[test]
fn foreign_binding_schema_body_and_detached_observer_refuse_before_gate() {
    for mode in [
        RequestedOptionsControl::None,
        RequestedOptionsControl::CancelPreDispatch,
        RequestedOptionsControl::CancelDeferredBoundary,
    ] {
        let f = Fixture::new(true, true);
        let e = f.evidence(mode);
        let (job, release) = f.held(&e);
        ready(&job);
        let foreign = Fixture::new(true, false);
        let foreign_e = foreign.evidence(RequestedOptionsControl::CancelPreDispatch);
        let (foreign_job, foreign_release) = foreign.held(&foreign_e);
        ready(&foreign_job);
        let mut other = foreign_job.attach().unwrap();
        let other_m = cancel(&other, "foreign");
        assert!(job.control(&mut other, &other_m).is_err());
        let mut reader = job.attach().unwrap();
        let m = cancel(&reader, "valid");
        for field in [
            "context",
            "target",
            "operation",
            "run",
            "stream",
            "schema",
            "inner",
            "body",
        ] {
            let mut bad = m.clone();
            match field {
                "context" => bad.context_id = "foreign".into(),
                "target" => bad.target_id = None,
                "operation" => bad.operation_id = "foreign".into(),
                "run" => bad.run_id = "foreign".into(),
                "stream" => bad.stream_id = "foreign".into(),
                "schema" => bad.schema = 2,
                "inner" => {
                    let wire::Body::ControlRequest(r) = &mut bad.body else {
                        unreachable!()
                    };
                    r.after_sequence = "01".into();
                }
                _ => {
                    bad.body = wire::Body::ReplayRequest(Box::new(wire::BodyReplayRequest {
                        after_sequence: "0".into(),
                        max_events: 1,
                        max_bytes: 1024,
                    }))
                }
            }
            assert!(job.control(&mut reader, &bad).is_err(), "{field}");
        }
        reader.detach();
        assert!(job.control(&mut reader, &m).is_err());
        release.send(()).unwrap();
        foreign_release.send(()).unwrap();
        f.release();
        foreign.release();
        terminal(&job);
        terminal(&foreign_job);
        f.count();
        foreign.count();
    }
}
#[test]
fn changed_policy_denies_new_submission_and_same_key_before_any_job_lookup() {
    let f = Fixture::new(true, false);
    let e = f.evidence(RequestedOptionsControl::CancelPreDispatch);
    let mut facts = HostPolicyFacts {
        approve_captured_script: true,
        allow_detach: true,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: false,
    };
    facts.allow_cancel_pre_dispatch = false;
    f.host.set_host_policy(facts).unwrap();
    assert!(matches!(
        f.host.submit_scoped(&e, "new", now()),
        Err(FixedDecisionError::StalePolicy)
    ));
    assert!(f.host.jobs.lock().unwrap().is_empty());
    assert!(!f.dir.path().join("counter").exists());
}

#[test]
fn selected_control_and_changed_policy_conflict_after_terminal_without_redispatch() {
    let f = Fixture::new(true, true);
    let e = f.evidence(RequestedOptionsControl::None);
    let different = f.evidence(RequestedOptionsControl::Detach);
    let job = f.host.submit_scoped(&e, "once", now()).unwrap();
    ready(&job);
    f.release();
    terminal(&job);
    assert!(matches!(
        f.host.submit_scoped(&different, "once", now()),
        Err(FixedDecisionError::AnswerConflict)
    ));
    f.host
        .set_host_policy(HostPolicyFacts {
            approve_captured_script: true,
            allow_detach: false,
            allow_cancel_pre_dispatch: false,
            allow_cancel_deferred_boundary: false,
        })
        .unwrap();
    let changed = f.evidence(RequestedOptionsControl::None);
    assert!(matches!(
        f.host.submit_scoped(&changed, "once", now()),
        Err(FixedDecisionError::AnswerConflict)
    ));
    f.count();
}
#[test]
fn missing_or_tampered_receipt_never_proves_reached_for_either_stop_mode() {
    for mode in [
        RequestedOptionsControl::CancelPreDispatch,
        RequestedOptionsControl::CancelDeferredBoundary,
    ] {
        for damage in ["missing", "tampered"] {
            let f = Fixture::new(true, true);
            let e = f.evidence(mode.clone());
            let (job, release) = f.held(&e);
            let summary = ready(&job);
            let mut reader = job.attach().unwrap();
            let m = cancel(&reader, "stop");
            if mode == RequestedOptionsControl::CancelPreDispatch {
                assert_eq!(row(&job, &mut reader, &m).status, Status::Requested);
                release.send(()).unwrap();
            } else {
                release.send(()).unwrap();
                until(|| f.dir.path().join("entered").exists());
                assert_eq!(row(&job, &mut reader, &m).status, Status::Requested);
                f.release();
            }
            terminal(&job);
            let receipt = f
                .dir
                .path()
                .join(".vibe/lifecycle-outcomes")
                .join(format!("{}.cancel.json", summary.run_id()));
            if damage == "missing" {
                fs::remove_file(receipt).unwrap();
            } else {
                fs::write(receipt, "tampered").unwrap();
            }
            assert_eq!(row(&job, &mut reader, &m).status, Status::Unavailable);
            let late = cancel(&reader, "late");
            assert_eq!(
                row(&job, &mut reader, &late).status,
                Status::AlreadyTerminal
            );
            if mode == RequestedOptionsControl::CancelDeferredBoundary {
                f.count();
            } else {
                assert!(!f.dir.path().join("counter").exists());
            }
        }
    }
}
#[test]
fn foreign_evidence_and_tampered_option_are_not_string_identity_grants() {
    let f = Fixture::new(true, true);
    let other = Fixture::new(true, true);
    let e = f.evidence(RequestedOptionsControl::None);
    assert!(matches!(
        other.host.submit_scoped(&e, "once", now()),
        Err(FixedDecisionError::ForeignScope)
    ));
    let mut tampered = e.clone();
    tampered.options.control = RequestedOptionsControl::CancelPreDispatch;
    assert!(matches!(
        f.host.submit_scoped(&tampered, "once", now()),
        Err(FixedDecisionError::InvalidData)
    ));
    assert!(f.host.jobs.lock().unwrap().is_empty());
    assert!(other.host.jobs.lock().unwrap().is_empty());
}
