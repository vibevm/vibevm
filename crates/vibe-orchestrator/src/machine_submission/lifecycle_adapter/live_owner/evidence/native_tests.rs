//! Compile-only here; actual finite Linux SystemProcessRunner proofs run by parent.
use super::*;
use crate::machine_submission::lifecycle_decisions::{
    DecisionOutcome, FixedLifecycleDecisionHost, HostPolicyFacts,
};
use std::fs;
use std::time::Instant;
use vibe_wire::generated::machine::e1::input_decision_message::{
    Body, BodyApprovalAnswer, BodySelectionAnswer, RequestedOptions,
};
fn decision_host(c: &HostCoordinates, approved: bool) -> FixedLifecycleDecisionHost {
    FixedLifecycleDecisionHost::from_host(
        c.root.clone(),
        c.namespace.clone(),
        c.scope.clone(),
        TargetOs::Linux,
        HostPolicyFacts {
            approve_captured_script: approved,
            approve_exact_local_install: false,
            allow_detach: false,
            allow_cancel_pre_dispatch: false,
            allow_cancel_deferred_boundary: false,
        },
    )
    .unwrap()
}

struct Fixture {
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new(failing: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), ONE_SCRIPT_VALIDATE_MANIFEST).unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        fs::write(dir.path().join("run.sh"), format!(
            "#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\nprintf entered > entered\ni=0\nwhile [ ! -f release ]; do\ni=$((i+1))\nif [ \"$i\" -ge 200 ]; then exit 70; fi\nsleep 0.05\ndone\nexit {}\n", if failing { 17 } else { 0 })).unwrap();
        Self { dir }
    }
    fn coordinates(&self) -> HostCoordinates {
        let scope = SubmissionScope {
            context: "actual-evidence".into(),
            target: "root".into(),
            principal: "actual-principal".into(),
        };
        let namespace = {
            let lease = LifecycleLease::acquire(self.dir.path()).unwrap();
            SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap()
        };
        HostCoordinates {
            root: self.dir.path().into(),
            namespace,
            scope,
            key: "actual-once".into(),
        }
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
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.release();
    }
}
fn until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !predicate() {
        assert!(Instant::now() < deadline, "finite native fixture deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn begin(c: &HostCoordinates, phase: Phase) -> HostOwnedFixedOperation {
    HostOwnedFixedOperation::start(
        c.root.clone(),
        c.namespace.clone(),
        c.scope.clone(),
        c.key.clone(),
        DefaultLifecycleRequest {
            requested: phase,
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
fn terminal(host: &HostOwnedFixedOperation) -> FixedOwnerEvidenceSnapshot {
    until(|| {
        !matches!(
            host.inspect(),
            Ok(FixedOperationStatus::Starting | FixedOperationStatus::Running { .. })
        )
    });
    host.evidence()
}
#[test]
fn actual_blocked_script_scope_phase_terminal_and_independent_cold_replay() {
    let f = Fixture::new(false);
    let c = f.coordinates();
    let admitting = decision_host(&c, true);
    let q = admitting
        .prepare_requested_options(RequestedOptions { control: vibe_wire::generated::machine::e1::input_decision_message::RequestedOptionsControl::None, force: false, trace: false, deploy_profile: None })
        .unwrap();
    let mut a = q.clone();
    a.body = Body::SelectionAnswer(Box::new(BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let DecisionOutcome::Question(mut q) = admitting.answer_typed(&a).unwrap() else {
        panic!("approval question")
    };
    q.body = Body::ApprovalAnswer(Box::new(BodyApprovalAnswer { approved: true }));
    let DecisionOutcome::Validated(e) = admitting.answer_typed(&q).unwrap() else {
        panic!("actual evidence")
    };
    let host = admitting
        .submit_no_control(&e, &c.key, "2026-10-02T00:00:00Z".parse().unwrap())
        .unwrap();
    let Readiness::Ready(summary) = host.wait_ready(Duration::from_secs(15)).unwrap() else {
        panic!("actual Ready")
    };
    until(|| f.dir.path().join("entered").exists());
    let live = host.evidence();
    assert_eq!(live.operation_id(), summary.operation_id());
    assert_eq!(live.operation_id(), "rust:vibe-lifecycle:validate");
    assert_eq!(live.requested_scope(), &c.scope);
    assert_eq!(live.requested_scope().principal, "actual-principal");
    assert_eq!(live.requested_phase(), Phase::Validate);
    assert_eq!(live.basis(), EvidenceBasis::LiveCallbacks);
    assert_eq!(live.last_observed_phase().unwrap().phase(), "validate");
    assert_eq!(live.known_contribution_total(), Some(1));
    assert_eq!(live.observed_terminal_work_count(), Some(0));
    let run = live.bound_run_id().unwrap().to_owned();
    f.count_one();
    f.release();
    until(|| {
        !matches!(
            host.inspect(),
            Ok(FixedOperationStatus::Starting | FixedOperationStatus::Running { .. })
        )
    });
    let done = host.evidence();
    assert_eq!(done.bound_run_id(), Some(run.as_str()));
    assert_eq!(done.observed_terminal_work_count(), Some(1));
    assert!(done.work_outcomes().iter().any(|r| r.status() == "ok"));
    assert!(
        matches!(done.status(), Some(FixedOperationStatus::ResultAvailable { marker, .. }) if marker.outcome == MarkerOutcome::Completed)
    );
    // Independent trusted cold actor: no approval, controls or callback history.
    let cold = decision_host(&c, false);
    assert!(cold.inspect_existing_evidence("missing").unwrap().is_none());
    let replay = cold.inspect_existing_evidence(&c.key).unwrap().unwrap();
    assert_eq!(replay.bound_run_id(), Some(run.as_str()));
    assert_eq!(replay.bound_scope(), Some(&c.scope));
    assert_eq!(replay.basis(), EvidenceBasis::RetainedReport);
    assert_eq!(replay.last_observed_phase().unwrap().phase(), "validate");
    assert_eq!(replay.observed_terminal_work_count(), None);
    assert!(
        matches!(replay.status(), Some(FixedOperationStatus::ResultAvailable { run_id, .. }) if run_id == &run)
    );
    f.count_one();
    fs::write(
        f.dir
            .path()
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run}.marker.json")),
        "damaged",
    )
    .unwrap();
    let damaged = cold.inspect_existing_evidence(&c.key).unwrap().unwrap();
    assert!(
        matches!(damaged.status(), Some(FixedOperationStatus::RecoveryRequired { run_id: Some(id), .. }) if id == &run)
    );
    assert_eq!(
        damaged.bound_run_id(),
        Some(run.as_str()),
        "known C1 run identity survives missing/invalid final evidence"
    );
    assert_eq!(damaged.bound_scope(), Some(&c.scope));
    assert!(damaged.last_observed_phase().is_none());
    f.count_one();
}
#[test]
fn actual_failure_and_damaged_final_evidence_preserve_work_without_redispatch() {
    for damaged in ["marker", "report"] {
        let f = Fixture::new(true);
        let c = f.coordinates();
        let host = begin(&c, Phase::Validate);
        until(|| f.dir.path().join("entered").exists());
        f.release();
        let done = terminal(&host);
        let run = done.bound_run_id().unwrap();
        assert!(
            matches!(done.status(), Some(FixedOperationStatus::ResultAvailable { marker, .. })
            if marker.outcome == MarkerOutcome::Failed && marker.effect_state != EffectState::None)
        );
        assert_eq!(done.last_observed_phase().unwrap().phase(), "validate");
        assert!(
            done.work_outcomes()
                .iter()
                .any(|row| row.status() == "fail")
        );
        let before = done.work_outcomes().to_vec();
        fs::write(
            f.dir
                .path()
                .join(".vibe/lifecycle-outcomes")
                .join(format!("{run}.{damaged}.json")),
            "damaged",
        )
        .unwrap();
        let after = host.evidence();
        assert!(
            matches!(
                after.status(),
                Some(FixedOperationStatus::RecoveryRequired { .. })
            ) || after.inspection_unavailable().is_some()
        );
        assert_eq!(after.work_outcomes(), before);
        assert_eq!(after.observed_terminal_work_count(), Some(1));
        assert_eq!(after.last_observed_phase().unwrap().phase(), "validate");
        f.count_one();
    }
}
#[test]
fn actual_early_refusal_never_copies_requested_phase_to_observed_phase() {
    let f = Fixture::new(false);
    let c = f.coordinates();
    // Genuine owner profile refusal, before Ready or any provider callback.
    let host = begin(&c, Phase::Install);
    let done = terminal(&host);
    assert_eq!(done.requested_phase(), Phase::Install);
    assert!(done.bound_run_id().is_none());
    assert!(done.last_observed_phase().is_none());
    assert!(done.phase_unavailable().is_some());
    assert!(done.work_outcomes().is_empty());
    assert!(!f.dir.path().join("counter").exists());
}

#[test]
fn actual_completed_root_replacement_preserves_observations_and_inspection_cause() {
    let f = Fixture::new(false);
    let c = f.coordinates();
    let host = begin(&c, Phase::Validate);
    until(|| f.dir.path().join("entered").exists());
    f.release();
    let done = terminal(&host);
    assert!(
        matches!(done.status(), Some(FixedOperationStatus::ResultAvailable { marker, .. }) if marker.outcome == MarkerOutcome::Completed)
    );
    let moved = f.dir.path().with_extension("evidence-moved");
    fs::rename(f.dir.path(), &moved).unwrap();
    fs::create_dir(f.dir.path()).unwrap();
    let after = host.evidence();
    fs::remove_dir_all(f.dir.path()).unwrap();
    fs::rename(&moved, f.dir.path()).unwrap();
    assert!(after.status().is_none());
    assert!(
        after
            .inspection_unavailable()
            .unwrap()
            .contains("owner physical root changed")
    );
    assert!(!after.inspection_diagnostic_truncated());
    assert_eq!(after.last_observed_phase().unwrap().phase(), "validate");
    assert_eq!(after.work_outcomes(), done.work_outcomes());
    assert_eq!(after.observed_terminal_work_count(), Some(1));
    f.count_one();
}
