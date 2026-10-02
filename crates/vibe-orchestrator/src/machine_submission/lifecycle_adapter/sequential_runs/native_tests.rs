//! Actual SystemProcessRunner/current sealed approval; parent owns runtime.
use super::*;
use crate::machine_submission::{FixedLifecycleDecisionHost, HostPolicyFacts, ReadonlyFixedJob};
use std::fs;
use std::time::{Duration, Instant};
const WAIT: Duration = Duration::from_secs(15);
struct Fixture {
    dir: tempfile::TempDir,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), ONE_SCRIPT_VALIDATE_MANIFEST).unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        fs::write(dir.path().join("run.sh"), format!(
            "#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\nif [ -f fail ]; then exit 23; fi\n")).unwrap();
        let scope = SubmissionScope {
            context: "sequential-native".into(),
            target: "root".into(),
            principal: "host".into(),
        };
        let lease = LifecycleLease::acquire(dir.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
        drop(lease);
        Self {
            dir,
            namespace,
            scope,
        }
    }
    fn host(&self) -> FixedLifecycleDecisionHost {
        FixedLifecycleDecisionHost::from_host(
            self.dir.path().into(),
            self.namespace.clone(),
            self.scope.clone(),
            TargetOs::Linux,
            HostPolicyFacts {
                approve_captured_script: true,
                allow_detach: true,
                allow_cancel_pre_dispatch: true,
                allow_cancel_deferred_boundary: true,
            },
        )
        .unwrap()
    }
    fn member(&self, run: &str, suffix: &str) -> std::path::PathBuf {
        self.dir
            .path()
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run}.{suffix}"))
    }
    fn count(&self) -> usize {
        fs::read_to_string(self.dir.path().join("counter"))
            .unwrap()
            .lines()
            .count()
    }
}
fn now() -> Timestamp {
    "2026-10-02T00:00:00Z".parse().unwrap()
}
fn terminal(job: &ReadonlyFixedJob) -> FixedOperationStatus {
    let deadline = Instant::now() + WAIT;
    loop {
        let status = job.inspect().unwrap();
        if !matches!(
            status,
            FixedOperationStatus::Starting | FixedOperationStatus::Running { .. }
        ) {
            return status;
        }
        assert!(Instant::now() < deadline, "bounded terminal wait");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn ready(job: &ReadonlyFixedJob) -> String {
    let deadline = Instant::now() + WAIT;
    loop {
        match job.wait_ready(Duration::from_millis(10)).unwrap() {
            Readiness::Ready(binding) => return binding.run_id().to_owned(),
            Readiness::Starting => assert!(Instant::now() < deadline),
            other => panic!("actual Ready required: {other:?}"),
        }
    }
}
#[test]
fn three_actual_completed_runs_preserve_original_key_cold_results_and_capsules() {
    let f = Fixture::new();
    let host = f.host();
    let evidence = host.sequential_approved_for_test();
    let mut retained = Vec::new();
    for key in ["first", "second", "third"] {
        let job = host.submit_no_control(&evidence, key, now()).unwrap();
        let run = ready(&job);
        let result = terminal(&job);
        assert!(
            matches!(&result, FixedOperationStatus::ResultAvailable { marker, report_bytes: Some(_), .. }
            if marker.outcome == MarkerOutcome::Completed)
        );
        let lease = LifecycleLease::acquire(f.dir.path()).unwrap();
        let row = SubmissionReservations::inspect_existing_key(&lease, &f.namespace, &f.scope, key)
            .unwrap()
            .unwrap();
        assert_eq!(row.domain_run_id, run);
        assert_eq!(row.journal_admission, JournalAdmission::Ready);
        assert_eq!(
            LifecycleStateStore::peek_with_lease(&lease)
                .unwrap()
                .unwrap()
                .run
                .run_id
                .as_deref(),
            Some(run.as_str())
        );
        drop(lease);
        let members: Vec<_> = ["marker.json", "report.json", "admitted-input.json"]
            .into_iter()
            .map(|suffix| (suffix, fs::read(f.member(&run, suffix)).unwrap()))
            .collect();
        retained.push((key, run, result, members));
    }
    assert!(
        retained
            .iter()
            .enumerate()
            .all(|(i, a)| retained[..i].iter().all(|b| a.1 != b.1))
    );
    // A new idempotency key reserves a new run; it is not a force request.
    // The incumbent engine reuses its exact successful fingerprint.
    for (index, (_, _, outcome, _)) in retained.iter().enumerate() {
        let FixedOperationStatus::ResultAvailable {
            report_bytes: Some(bytes),
            ..
        } = outcome
        else {
            panic!("actual retained report");
        };
        let report: vibe_wire::generated::lifecycle_report::LifecycleReport =
            serde_json::from_slice(bytes).unwrap();
        assert_eq!(report.contributions.len(), 1);
        assert_eq!(
            report.contributions[0].status,
            if index == 0 { "ok" } else { "fresh" }
        );
    }
    assert_eq!(f.count(), 1);
    let cold = f.host();
    for (key, run, result, members) in &retained {
        assert_eq!(cold.inspect_existing(key).unwrap().as_ref(), Some(result));
        let retry = host.submit_no_control(&evidence, key, now()).unwrap();
        assert_eq!(ready(&retry), *run);
        assert_eq!(terminal(&retry), *result);
        for (suffix, bytes) in members {
            assert_eq!(fs::read(f.member(run, suffix)).unwrap(), *bytes);
        }
    }
    assert_eq!(f.count(), 1);
}
#[test]
fn valid_failed_terminal_allows_next_invocation_and_retains_partial_truth() {
    let f = Fixture::new();
    fs::write(f.dir.path().join("fail"), "fail").unwrap();
    let host = f.host();
    let evidence = host.sequential_approved_for_test();
    let failed = host.submit_no_control(&evidence, "failed", now()).unwrap();
    let run = ready(&failed);
    let original = terminal(&failed);
    assert!(
        matches!(&original, FixedOperationStatus::ResultAvailable { marker, report_bytes: Some(_), .. }
        if marker.outcome == MarkerOutcome::Failed)
    );
    let report = fs::read(f.member(&run, "report.json")).unwrap();
    fs::remove_file(f.dir.path().join("fail")).unwrap();
    let next = host.submit_no_control(&evidence, "next", now()).unwrap();
    assert_ne!(ready(&next), run);
    assert!(
        matches!(terminal(&next), FixedOperationStatus::ResultAvailable { marker, .. } if marker.outcome == MarkerOutcome::Completed)
    );
    assert_eq!(host.inspect_existing("failed").unwrap(), Some(original));
    assert_eq!(fs::read(f.member(&run, "report.json")).unwrap(), report);
    assert_eq!(f.count(), 2);
}
#[test]
fn damaged_or_nonterminal_previous_proof_refuses_before_new_ready_and_provider() {
    for damage in [
        "missing-marker",
        "corrupt-marker",
        "missing-report",
        "corrupt-report",
        "unavailable-report",
        "uncertain",
        "parked",
        "unready",
        "unattributed",
        "delegated",
        "continuation",
    ] {
        let f = Fixture::new();
        let host = f.host();
        let evidence = host.sequential_approved_for_test();
        let first = host.submit_no_control(&evidence, "first", now()).unwrap();
        let run = ready(&first);
        assert!(matches!(
            terminal(&first),
            FixedOperationStatus::ResultAvailable { .. }
        ));
        let marker_path = f.member(&run, "marker.json");
        let report_path = f.member(&run, "report.json");
        match damage {
            "missing-marker" => fs::remove_file(&marker_path).unwrap(),
            "corrupt-marker" => fs::write(&marker_path, "broken").unwrap(),
            "missing-report" => fs::remove_file(&report_path).unwrap(),
            "corrupt-report" => fs::write(&report_path, "broken").unwrap(),
            "uncertain" | "parked" | "unavailable-report" => {
                let mut marker: LifecycleOutcomeMarker =
                    serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
                marker.outcome = if damage == "parked" {
                    MarkerOutcome::Parked
                } else {
                    MarkerOutcome::Uncertain
                };
                if damage == "unavailable-report" {
                    marker = create_lifecycle_outcome_marker(
                        &run,
                        &marker.reservation_ref,
                        MarkerOutcome::Failed,
                        EffectState::Uncertain,
                        LifecycleReportPayload::Unavailable(
                            ReportEvidenceUnavailableReason::NotProduced,
                        ),
                    )
                    .unwrap();
                    fs::remove_file(&report_path).unwrap();
                }
                fs::write(&marker_path, serde_json::to_vec(&marker).unwrap()).unwrap();
            }
            "unready" => {
                let path = f.dir.path().join(".vibe/machine-submissions.json");
                let mut index: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                index["namespaces"][f.namespace.id()]["keys"]["first"]["journal_ready"] =
                    false.into();
                fs::write(path, serde_json::to_vec(&index).unwrap()).unwrap();
            }
            _ => {
                let path = f.dir.path().join(LifecycleStateStore::FILE);
                let mut state: vibe_wire::generated::lifecycle_state::LifecycleState =
                    toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
                if damage == "unattributed" {
                    state.run.run_id = None;
                }
                if damage == "delegated" {
                    let row = state.execution.values_mut().next().unwrap();
                    row.status =
                        vibe_wire::generated::lifecycle_state::ExecutionRecordStatus::Delegated;
                    row.scope =
                        Some(vibe_wire::generated::lifecycle_state::ExecutionRecordScope::Phase);
                    row.tasks = vec![format!(".vibe/outbox/{run}/task.json")];
                }
                if damage == "continuation" {
                    state.run.slot_continuation =
                        Some(vibe_wire::generated::lifecycle_state::SlotContinuation {
                            targets: vec![],
                        });
                }
                fs::write(path, toml::to_string(&state).unwrap()).unwrap();
            }
        }
        let state_before = fs::read(f.dir.path().join(LifecycleStateStore::FILE)).unwrap();
        let members: Vec<_> = ["marker.json", "report.json", "admitted-input.json"]
            .into_iter()
            .map(|suffix| (suffix, fs::read(f.member(&run, suffix)).ok()))
            .collect();
        let next = host.submit_no_control(&evidence, "next", now()).unwrap();
        assert!(
            matches!(
                terminal(&next),
                FixedOperationStatus::RecoveryRequired { .. }
            ),
            "{damage}"
        );
        assert!(
            !matches!(
                next.wait_ready(Duration::from_millis(10)).unwrap(),
                Readiness::Ready(_)
            ),
            "{damage}"
        );
        let lease = LifecycleLease::acquire(f.dir.path()).unwrap();
        if let Some(row) =
            SubmissionReservations::inspect_existing_key(&lease, &f.namespace, &f.scope, "next")
                .unwrap()
        {
            assert_eq!(row.journal_admission, JournalAdmission::Pending);
            assert!(row.reservation_ref.is_none());
            assert!(!f.member(&row.domain_run_id, "admitted-input.json").exists());
        }
        assert_eq!(
            fs::read(f.dir.path().join(LifecycleStateStore::FILE)).unwrap(),
            state_before
        );
        for (suffix, bytes) in members {
            assert_eq!(fs::read(f.member(&run, suffix)).ok(), bytes);
        }
        assert_eq!(f.count(), 1);
    }
}
