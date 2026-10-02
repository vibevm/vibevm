//! Real SystemProcessRunner/C1 fixtures, compiled only by the worker.
use super::*;
use crate::{
    command::DefaultLifecycleRequest,
    machine_submission::{
        DecisionOutcome, FixedLifecycleDecisionHost, HostOwnedFixedOperation, HostPolicyFacts,
    },
};
use std::{
    fs,
    time::{Duration, Instant},
};
use vibe_core::manifest::TargetOs;
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::machine::e1::input_decision_message::{
    Body, BodyApprovalAnswer, BodySelectionAnswer, RequestedOptions, RequestedOptionsControl,
};
struct Fixture {
    dir: tempfile::TempDir,
    scope: SubmissionScope,
    namespace: SubmissionNamespace,
}
impl Fixture {
    fn new(fail: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"),"[project]\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'once'\npoint = 'phase:validate'\nhandler = { kind = 'script', base = 'run' }\n").unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        fs::write(dir.path().join("run.sh"),format!("#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\nprintf entered > entered\ni=0\nwhile [ ! -f release ]; do\ni=$((i+1))\nif [ \"$i\" -ge 200 ]; then exit 70; fi\nsleep 0.05\ndone\nexit {}\n",if fail {17}else{0})).unwrap();
        let scope = SubmissionScope {
            context: "view-fixture".into(),
            target: "root".into(),
            principal: "actual-host".into(),
        };
        let namespace = {
            let lease = LifecycleLease::acquire(dir.path()).unwrap();
            SubmissionReservations::issue_namespace(&lease, &scope, 60000).unwrap()
        };
        Self {
            dir,
            scope,
            namespace,
        }
    }
    fn start(&self, requested: vibe_lifecycle::Phase) -> HostOwnedFixedOperation {
        HostOwnedFixedOperation::start(
            self.dir.path().into(),
            self.namespace.clone(),
            self.scope.clone(),
            "once".into(),
            DefaultLifecycleRequest {
                requested,
                force: false,
                agent_mode: RunAgentMode::Cli,
                assume_yes: true,
                trace_flag: false,
                install_inputs: Default::default(),
                policy: Default::default(),
            },
            TargetOs::Linux,
            "2026-10-02T00:00:00Z".parse().unwrap(),
        )
        .unwrap()
    }
    fn release(&self) {
        fs::write(self.dir.path().join("release"), "release").unwrap();
    }
    fn start_admitted(&self) -> crate::machine_submission::ReadonlyFixedJob {
        let admitting = self.decision_host(true);
        let mut q = admitting
            .prepare_requested_options(RequestedOptions {
                control: RequestedOptionsControl::None,
                force: false,
                trace: false,
                deploy_profile: None,
            })
            .unwrap();
        q.body = Body::SelectionAnswer(Box::new(BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        let DecisionOutcome::Question(mut q) = admitting.answer_typed(&q).unwrap() else {
            panic!("approval question")
        };
        q.body = Body::ApprovalAnswer(Box::new(BodyApprovalAnswer { approved: true }));
        let DecisionOutcome::Validated(evidence) = admitting.answer_typed(&q).unwrap() else {
            panic!("actual admitted evidence")
        };
        admitting
            .submit_no_control(&evidence, "once", "2026-10-02T00:00:00Z".parse().unwrap())
            .unwrap()
    }
    fn count_one(&self) {
        assert_eq!(
            fs::read(self.dir.path().join("counter")).unwrap(),
            b"effect\n"
        );
    }
    fn cold(&self) -> FixedLifecycleDecisionHost {
        self.decision_host(false)
    }
    fn decision_host(&self, approve_captured_script: bool) -> FixedLifecycleDecisionHost {
        FixedLifecycleDecisionHost::from_host(
            self.dir.path().into(),
            self.namespace.clone(),
            self.scope.clone(),
            TargetOs::Linux,
            HostPolicyFacts {
                approve_captured_script,
                allow_detach: false,
                allow_cancel_pre_dispatch: false,
                allow_cancel_deferred_boundary: false,
            },
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.release();
    }
}
fn until(mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !f() {
        assert!(Instant::now() < deadline, "finite fixture deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn terminal(h: &HostOwnedFixedOperation) -> FixedOwnerEvidenceSnapshot {
    until(|| {
        !matches!(
            h.inspect(),
            Ok(FixedOperationStatus::Starting | FixedOperationStatus::Running { .. })
        )
    });
    h.evidence()
}
fn terminal_readonly(
    h: &crate::machine_submission::ReadonlyFixedJob,
) -> FixedOwnerEvidenceSnapshot {
    until(|| {
        !matches!(
            h.inspect(),
            Ok(FixedOperationStatus::Starting | FixedOperationStatus::Running { .. })
        )
    });
    h.evidence()
}
fn roundtrip(s: &FixedOwnerEvidenceSnapshot) -> LifecycleSubmissionView {
    let v = project_snapshot(s).unwrap();
    assert_eq!(parse(&serialize(&v, s).unwrap(), s).unwrap(), v);
    v
}
fn refuses(
    s: &FixedOwnerEvidenceSnapshot,
    v: &LifecycleSubmissionView,
    mutate: impl FnOnce(&mut serde_json::Value),
) {
    let mut a = serde_json::to_value(v).unwrap();
    mutate(&mut a);
    assert!(parse(&serde_json::to_vec(&a).unwrap(), s).is_err());
}
#[test]
fn actual_active_completed_cold_and_damaged_c1_identity_without_redispatch() {
    let f = Fixture::new(false);
    let h = f.start_admitted();
    until(|| f.dir.path().join("entered").exists());
    let live = h.evidence();
    let v = roundtrip(&live);
    assert!(matches!(v.status, Status::Running(_)));
    assert_eq!(v.last_phase.as_ref().unwrap().phase, "validate");
    assert_eq!(v.known_contribution_total.as_deref(), Some("1"));
    assert_eq!(v.observed_terminal_work_count.as_deref(), Some("0"));
    refuses(&live, &v, |v| v["partial"] = false.into());
    refuses(&live, &v, |v| {
        v["requested_scope"]["principal"] = "other".into()
    });
    refuses(&live, &v, |v| v["run_id"] = "f".repeat(32).into());
    f.release();
    let done = terminal_readonly(&h);
    let v = roundtrip(&done);
    assert!(
        matches!(&v.status,Status::ResultAvailable(r) if r.outcome==StatusResultAvailableOutcome::Completed)
    );
    refuses(&done, &v, |v| v["status"]["effect_state"] = "none".into());
    for (field, value) in [
        ("known_contribution_total", "01"),
        ("observed_terminal_work_count", "18446744073709551616"),
    ] {
        refuses(&done, &v, |v| v[field] = value.into());
    }
    // Every projected top-level field remains bound to the opaque real sample.
    let encoded = serde_json::to_value(&v).unwrap();
    for (key, value) in encoded.as_object().unwrap() {
        refuses(&done, &v, |altered| {
            altered[key] = match value {
                serde_json::Value::Null => true.into(),
                serde_json::Value::Bool(b) => (!b).into(),
                serde_json::Value::String(_) => "tampered".into(),
                serde_json::Value::Number(_) => 2.into(),
                _ => serde_json::Value::Null,
            };
        });
    }
    refuses(&done, &v, |v| {
        v["status"]["reservation_ref"] = ("sha256:".to_owned() + &"f".repeat(64)).into()
    });
    refuses(&done, &v, |v| {
        v["status"]["report"]["sha256"] = "f".repeat(64).into()
    });
    let cold = f.cold().inspect_existing_evidence("once").unwrap().unwrap();
    let cv = roundtrip(&cold);
    assert_eq!(cv.basis, LifecycleSubmissionViewBasis::RetainedReport);
    assert_eq!(cv.run_id, v.run_id);
    assert_eq!(cv.bound_scope, v.bound_scope);
    assert_eq!(cv.observed_terminal_work_count, None);
    f.count_one();
    let run = v.run_id.unwrap();
    fs::write(
        f.dir
            .path()
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run}.marker.json")),
        "damaged",
    )
    .unwrap();
    let cold = f.cold().inspect_existing_evidence("once").unwrap().unwrap();
    let cv = roundtrip(&cold);
    assert_eq!(cv.run_id.as_deref(), Some(run.as_str()));
    assert!(cv.bound_scope.is_some());
    assert!(cv.last_phase.is_none());
    assert!(cv.phase_unavailable_reason.is_some());
    assert!(matches!(cv.status, Status::RecoveryRequired(_)));
    f.count_one();
}
#[test]
fn actual_failed_write_and_damaged_final_keep_historical_phase() {
    for damaged in ["marker", "report"] {
        let f = Fixture::new(true);
        let h = f.start(vibe_lifecycle::Phase::Validate);
        until(|| f.dir.path().join("entered").exists());
        f.release();
        let done = terminal(&h);
        let v = roundtrip(&done);
        assert!(
            matches!(&v.status,Status::ResultAvailable(r) if r.outcome==StatusResultAvailableOutcome::Failed && r.effect_state!=StatusResultAvailableEffectState::None)
        );
        refuses(&done, &v, |v| v["status"]["outcome"] = "completed".into());
        assert!(v.work_outcomes.iter().any(|w| w.status == "fail"));
        fs::write(
            f.dir
                .path()
                .join(".vibe/lifecycle-outcomes")
                .join(format!("{}.{damaged}.json", v.run_id.as_ref().unwrap())),
            "damaged",
        )
        .unwrap();
        let after = h.evidence();
        let a = roundtrip(&after);
        assert!(matches!(
            a.status,
            Status::RecoveryRequired(_) | Status::InspectionUnavailable(_)
        ));
        assert_eq!(a.run_id, v.run_id);
        assert_eq!(a.last_phase, v.last_phase);
        assert_eq!(a.work_outcomes, v.work_outcomes);
        assert!(a.partial);
        f.count_one();
    }
}
#[test]
fn actual_early_owned_refusal_never_invents_observed_phase() {
    let f = Fixture::new(false);
    let h = f.start(vibe_lifecycle::Phase::Install);
    let s = terminal(&h);
    let v = roundtrip(&s);
    assert_eq!(v.requested_phase, "install");
    assert!(v.last_phase.is_none());
    assert!(v.run_id.is_none());
    assert!(v.partial);
    refuses(&s, &v, |v| {
        v["last_phase"] = serde_json::json!({"phase":"install","status":null})
    });
    for entry in fs::read_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-lifecycle-submission-view/e1/semantic-invalid"
    ))
    .unwrap()
    {
        let path = entry.unwrap().path();
        assert!(
            parse(&fs::read(&path).unwrap(), &s).is_err(),
            "{}",
            path.display()
        );
    }
    assert_eq!(
        parse(&vec![b' '; MAX_FRAME_BYTES + 1], &s),
        Err(ViewError::FrameLimit)
    );
    assert!(!f.dir.path().join("counter").exists());
}
#[test]
fn actual_inspection_failure_retains_cause_and_old_sample() {
    let f = Fixture::new(false);
    let h = f.start(vibe_lifecycle::Phase::Validate);
    until(|| f.dir.path().join("entered").exists());
    f.release();
    let done = terminal(&h);
    let moved = f.dir.path().with_extension("view-moved");
    fs::rename(f.dir.path(), &moved).unwrap();
    fs::create_dir(f.dir.path()).unwrap();
    let after = h.evidence();
    fs::remove_dir_all(f.dir.path()).unwrap();
    fs::rename(&moved, f.dir.path()).unwrap();
    let v = roundtrip(&after);
    assert!(
        matches!(&v.status,Status::InspectionUnavailable(r) if r.reason.contains("owner physical root changed") && !r.reason_truncated)
    );
    assert_eq!(v.last_phase, roundtrip(&done).last_phase);
    f.count_one();
}
