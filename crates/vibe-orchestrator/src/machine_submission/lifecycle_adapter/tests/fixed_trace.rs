//! Admission and observer faults at the actual protected callback boundary.
use super::*;
use crate::machine_submission::{FixedLifecycleDecisionHost, HostPolicyFacts};
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::shared::TraceReportStatus;

fn admitted(root: &Path, ns: &SubmissionNamespace, scope: &SubmissionScope) -> AdmittedProfile {
    let host = FixedLifecycleDecisionHost::from_host(
        root.into(),
        ns.clone(),
        scope.clone(),
        TargetOs::current().unwrap(),
        HostPolicyFacts {
            approve_captured_script: true,
            approve_exact_local_install: false,
            allow_detach: true,
            allow_cancel_pre_dispatch: true,
            allow_cancel_deferred_boundary: true,
        },
    )
    .unwrap();
    host.trace_profile_for_test()
}
fn trace_request() -> DefaultLifecycleRequest {
    let mut asked = request();
    asked.trace_flag = true;
    asked
}
fn run_dir(root: &Path) -> PathBuf {
    std::fs::read_dir(root.join(".vibe/trace"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().len() == 32)
        .unwrap()
}
fn report(outcome: &RetainedLifecycleResult) -> LifecycleReport {
    serde_json::from_slice(outcome.report_bytes.as_ref().unwrap()).unwrap()
}
fn call_trace(
    root: &Path,
    ns: &SubmissionNamespace,
    scope: &SubmissionScope,
    profile: &AdmittedProfile,
    process: &dyn ProcessRunner,
    after_stage: Option<&dyn Fn()>,
    after_effect: Option<&dyn Fn() -> Result<()>>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_one_script_validate_internal(
        root,
        ns,
        scope,
        "trace",
        trace_request(),
        TargetOs::current().unwrap(),
        "2026-10-03T00:00:05Z".parse().unwrap(),
        process,
        &ScriptProbe,
        after_stage,
        after_effect,
        None,
        Some(profile),
    )
}

#[test]
fn open_refusal_is_unavailable_observation_and_still_executes_once() {
    let dir = script_project();
    let scope = scope();
    let ns = issue(dir.path(), &scope);
    let profile = admitted(dir.path(), &ns, &scope);
    std::fs::write(dir.path().join(".vibe/trace"), "blocked trace home").unwrap();
    let process = CountedScriptProcess::new(dir.path());
    let first = call_trace(dir.path(), &ns, &scope, &profile, &process, None, None).unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("executes despite observer refusal")
    };
    assert_eq!(process.count(), 1);
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Completed);
    let r = report(&outcome);
    assert!(r.ok);
    let t = r.trace.unwrap();
    assert_eq!(t.run_id, run_id);
    assert_eq!(t.status, TraceReportStatus::Unavailable);
    assert!(!t.finalised && !t.warnings.is_empty());
    assert_eq!(t.snapshots, "0");
    assert_eq!(
        call_trace(dir.path(), &ns, &scope, &profile, &process, None, None).unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(process.count(), 1);
}

#[test]
fn finalize_refusal_keeps_completed_domain_and_truthful_running_trace() {
    let dir = script_project();
    let scope = scope();
    let ns = issue(dir.path(), &scope);
    let profile = admitted(dir.path(), &ns, &scope);
    let process = CountedScriptProcess::new(dir.path());
    let refuse = || {
        let index = run_dir(dir.path()).join("index.json");
        std::fs::remove_file(&index).unwrap();
        std::fs::create_dir(&index).unwrap();
        Ok(())
    };
    let SubmissionOutcome::Invoked { outcome, .. } = call_trace(
        dir.path(),
        &ns,
        &scope,
        &profile,
        &process,
        None,
        Some(&refuse),
    )
    .unwrap() else {
        panic!("observer finalization cannot veto")
    };
    assert_eq!(process.count(), 1);
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Completed);
    let r = report(&outcome);
    assert!(r.ok);
    let t = r.trace.unwrap();
    assert_eq!(t.status, TraceReportStatus::Running);
    assert!(!t.finalised && !t.warnings.is_empty());
    assert_eq!(t.snapshots, "0");
}

struct FailingScript(AtomicUsize);
impl ProcessRunner for FailingScript {
    fn run(&self, _: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(ProcessOutput {
            code: Some(23),
            stderr: b"script failure detail".to_vec(),
            ..Default::default()
        })
    }
}
#[test]
fn script_failure_retains_failed_trace_and_measured_rows() {
    let dir = script_project();
    let scope = scope();
    let ns = issue(dir.path(), &scope);
    let profile = admitted(dir.path(), &ns, &scope);
    let process = FailingScript(AtomicUsize::new(0));
    let SubmissionOutcome::Invoked { outcome, .. } =
        call_trace(dir.path(), &ns, &scope, &profile, &process, None, None).unwrap()
    else {
        panic!("failed script retains result")
    };
    assert_eq!(process.0.load(Ordering::SeqCst), 1);
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Failed);
    let r = report(&outcome);
    assert!(!r.ok);
    assert_eq!(r.contributions.len(), 1);
    assert_eq!(r.contributions[0].status, "fail");
    let t = r.trace.unwrap();
    assert_eq!(t.status, TraceReportStatus::Failed);
    assert!(t.finalised);
    let index = std::fs::read_to_string(run_dir(dir.path()).join("index.json")).unwrap();
    assert!(index.contains("command failed"));
    assert!(!index.contains("script failure detail"));
}

#[test]
fn post_effect_err_finalizes_failed_trace_preserves_original_and_never_redispatches() {
    for retention_fault in [false, true] {
        let dir = script_project();
        let scope = scope();
        let ns = issue(dir.path(), &scope);
        let profile = admitted(dir.path(), &ns, &scope);
        let process = CountedScriptProcess::new(dir.path());
        let fail = || {
            if retention_fault {
                let run = run_dir(dir.path())
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                std::fs::create_dir(
                    dir.path()
                        .join(".vibe/lifecycle-outcomes")
                        .join(format!("{run}.report.json")),
                )
                .unwrap();
            }
            anyhow::bail!("original post-effect fault")
        };
        let error = call_trace(
            dir.path(),
            &ns,
            &scope,
            &profile,
            &process,
            None,
            Some(&fail),
        )
        .unwrap_err();
        assert!(
            format!("{error:#}").contains("original post-effect fault"),
            "{error:#}"
        );
        let Some(SubmissionFailure::UncertainJournal { run_id, .. }) = error.downcast_ref() else {
            panic!("original uncertain callback: {error:#}")
        };
        assert_eq!(process.count(), 1);
        let index = std::fs::read_to_string(run_dir(dir.path()).join("index.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&index).unwrap();
        assert_eq!(v["status"], "failed");
        assert!(!index.contains("original post-effect fault"));
        let retry = call_trace(dir.path(), &ns, &scope, &profile, &process, None, None);
        if retention_fault {
            let error = retry.unwrap_err();
            assert!(
                matches!(error.downcast_ref::<SubmissionFailure>(), Some(SubmissionFailure::UncertainJournal { run_id: observed, .. }) if observed == run_id)
            );
        } else {
            let SubmissionOutcome::Replayed { outcome, .. } = retry.unwrap() else {
                panic!("retained uncertain report")
            };
            assert_eq!(outcome.marker.outcome, MarkerOutcome::Uncertain);
            assert_eq!(outcome.marker.effect_state, EffectState::Uncertain);
            let r = report(&outcome);
            assert_eq!(r.contributions.len(), 1);
            assert_eq!(r.contributions[0].status, "ok");
            assert_eq!(r.trace.as_ref().unwrap().run_id, *run_id);
            assert_eq!(r.trace.unwrap().status, TraceReportStatus::Failed);
        }
        assert_eq!(process.count(), 1);
        assert_eq!(
            std::fs::read_to_string(run_dir(dir.path()).join("index.json")).unwrap(),
            index
        );
    }
}

#[test]
fn unapproved_generic_trace_and_header_drift_refuse_before_provider_or_trace_open() {
    let mut asked = request();
    asked.trace_flag = true;
    let script = script_project();
    assert!(
        admit_one_script_validate_fixture(script.path(), &asked, TargetOs::current().unwrap())
            .is_err()
    );
    let agent = agent_project();
    let mut asked_agent = agent_request();
    asked_agent.trace_flag = true;
    assert!(
        admit_one_agent_validate_fixture(agent.path(), &asked_agent, TargetOs::current().unwrap())
            .is_err()
    );
    let scope = scope();
    let ns = issue(script.path(), &scope);
    let profile = admitted(script.path(), &ns, &scope);
    let process = CountedScriptProcess::new(script.path());
    let drift = || {
        let path = script.path().join(".vibe/lifecycle.toml");
        let h = std::fs::read_to_string(&path).unwrap();
        assert!(h.contains("compile_trace = true"));
        std::fs::write(
            path,
            h.replace("compile_trace = true", "compile_trace = false"),
        )
        .unwrap();
    };
    let result = call_trace(
        script.path(),
        &ns,
        &scope,
        &profile,
        &process,
        Some(&drift),
        None,
    );
    assert!(!matches!(result, Ok(SubmissionOutcome::Invoked { .. })));
    assert_eq!(process.count(), 0);
    assert!(!script.path().join(".vibe/trace").exists());
}
