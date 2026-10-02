//! Actual fixed Validate effects and independent OS-process restart proofs.
use super::tests::{facts, service};
use super::*;
use crate::machine_operation::input_decision::*;
use crate::machine_submission::{FixedOperationStatus, ReadonlyFixedJob};
use std::fs;
use std::time::{Duration, Instant};

const MANIFEST: &str = "[project]\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'once'\npoint = 'phase:validate'\nhandler = { kind = 'script', base = 'run' }\n";
fn fixture(root: &std::path::Path) {
    fs::write(root.join("vibe.toml"), MANIFEST).unwrap();
    let quoted = root.to_str().unwrap().replace('\'', "'\\''");
    fs::write(
        root.join("run.sh"),
        format!("#!/bin/sh\nset -eu\ncd '{quoted}'\nprintf 'effect\\n' >> counter\n"),
    )
    .unwrap();
}
fn timestamp() -> vibe_wire::generated::shared::Timestamp {
    "2026-10-03T00:00:00Z".parse().unwrap()
}
fn terminal(job: &ReadonlyFixedJob) -> String {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match job.inspect().unwrap() {
            FixedOperationStatus::Starting | FixedOperationStatus::Running { .. } => {
                assert!(
                    Instant::now() < deadline,
                    "bounded actual provider deadline"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            FixedOperationStatus::ResultAvailable {
                run_id,
                report_bytes: Some(_),
                ..
            } => return run_id,
            other => panic!("retained real marker/report required: {other:?}"),
        }
    }
}
fn count_one(root: &std::path::Path) {
    assert_eq!(fs::read(root.join("counter")).unwrap(), b"effect\n");
}

#[test]
fn fresh_host_forgets_questions_and_approvals_replays_same_run_and_recovers_retired_key() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let first = service(root.path());
    let host = first.issue(60_000).unwrap();
    let mut old_question = host
        .prepare_requested_options(RequestedOptions {
            control: RequestedOptionsControl::None,
            force: false,
            trace: false,
            deploy_profile: None,
        })
        .unwrap();
    let evidence = host.sequential_approved_for_test();
    let original_identity = evidence.identity().to_owned();
    let run = terminal(
        &host
            .submit_no_control(&evidence, "once", timestamp())
            .unwrap(),
    );
    count_one(root.path());
    drop(host);
    drop(first);
    let second = service(root.path());
    let cold = second.reattach().unwrap();
    assert!(matches!(
        cold.submit_no_control(&evidence, "fresh", timestamp()),
        Err(FixedDecisionError::ForeignScope)
    ));
    old_question.body = Body::ApprovalAnswer(Box::new(BodyApprovalAnswer { approved: true }));
    assert!(matches!(
        cold.answer_typed(&old_question),
        Err(FixedDecisionError::ForeignScope | FixedDecisionError::UnknownQuestion)
    ));
    let fresh = cold.sequential_approved_for_test();
    assert_eq!(fresh.identity(), original_identity);
    assert_eq!(
        terminal(&cold.submit_no_control(&fresh, "once", timestamp()).unwrap()),
        run
    );
    count_one(root.path());
    let pointer: Pointer =
        serde_json::from_slice(&fs::read(root.path().join(&second.path)).unwrap()).unwrap();
    let GrantState::Issued { namespace, .. } = pointer.grant else {
        unreachable!()
    };
    let token = SubmissionNamespace::from_host_token(namespace).unwrap();
    let lease = second.lease().unwrap();
    SubmissionReservations::retire_namespace(&lease, &token, &second.scope).unwrap();
    let c1_after_retirement = fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap();
    drop(lease);
    drop(cold);
    let recovery = second.reattach().unwrap();
    assert!(
        matches!(recovery.inspect_existing("once").unwrap(), Some(FixedOperationStatus::ResultAvailable { run_id, report_bytes: Some(_), .. }) if run_id == run)
    );
    let fresh = recovery.sequential_approved_for_test();
    let unreserved = recovery
        .submit_no_control(&fresh, "new-after-retirement", timestamp())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match unreserved.inspect().unwrap() {
            FixedOperationStatus::Starting => {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(10));
            }
            FixedOperationStatus::RecoveryRequired { .. } => break,
            other => panic!("retired namespace must refuse fresh effect: {other:?}"),
        }
    }
    count_one(root.path());
    fs::remove_file(root.path().join(&second.path)).unwrap();
    assert!(second.issue(60_000).is_err());
    assert_eq!(
        fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
        c1_after_retirement
    );
    assert!(
        root.path()
            .join(format!(".vibe/lifecycle-outcomes/{run}.marker.json"))
            .exists()
    );
    assert!(
        root.path()
            .join(format!(".vibe/lifecycle-outcomes/{run}.report.json"))
            .exists()
    );
}

// Normally inert helper; the parent proof explicitly starts this exact test
// in a fresh OS process with a fixture root authenticated by the test driver.
#[test]
fn child_process_worker() {
    let Some(root) = std::env::var_os("C4_CLI_GRANT_FIXTURE_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let service = service(&root);
    let first = std::env::var("C4_CLI_GRANT_CHILD_MODE").unwrap() == "issue";
    let host = if first {
        service.issue(60_000).unwrap()
    } else {
        service.reattach().unwrap()
    };
    let evidence = host.sequential_approved_for_test();
    let run = terminal(
        &host
            .submit_no_control(&evidence, "same-key", timestamp())
            .unwrap(),
    );
    fs::write(
        root.join(if first { "first-run" } else { "replayed-run" }),
        run,
    )
    .unwrap();
    count_one(&root);
}
fn child(root: &std::path::Path, mode: &str) {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "machine_submission::cli_grant::native_tests::child_process_worker",
            "--nocapture",
        ])
        .env("C4_CLI_GRANT_FIXTURE_ROOT", root)
        .env("C4_CLI_GRANT_CHILD_MODE", mode)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}

#[test]
fn two_independent_processes_keep_one_c1_namespace_domain_run_and_effect_after_process_exit() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    child(root.path(), "issue");
    let service = service(root.path());
    let before = fs::read(root.path().join(&service.path)).unwrap();
    child(root.path(), "reattach");
    assert_eq!(
        fs::read(root.path().join("first-run")).unwrap(),
        fs::read(root.path().join("replayed-run")).unwrap()
    );
    assert_eq!(fs::read(root.path().join(&service.path)).unwrap(), before);
    count_one(root.path());
    let index: serde_json::Value = serde_json::from_slice(
        &fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    assert_eq!(
        namespaces.values().next().unwrap()["keys"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    let run = fs::read_to_string(root.path().join("first-run")).unwrap();
    let host = service.reattach().unwrap();
    fs::remove_file(root.path().join("run.sh")).unwrap();
    fs::write(root.path().join("vibe.toml"), "broken current source").unwrap();
    assert!(
        matches!(host.inspect_existing("same-key").unwrap(), Some(FixedOperationStatus::ResultAvailable { run_id, report_bytes: Some(_), .. }) if run_id == run)
    );
}

#[test]
fn current_policy_revocation_on_reattached_host_refuses_new_approval_before_c1() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let service = service(root.path());
    service.issue(60_000).unwrap();
    let host = service.reattach().unwrap();
    let evidence = host.sequential_approved_for_test();
    let lease = service.lease().unwrap();
    let owner = service.current_owner(&lease).unwrap();
    let snapshot = owner.current_under_lease(&lease).unwrap();
    let mut revoked = facts();
    revoked.approve_captured_script = false;
    owner
        .update_under_lease(
            &lease,
            snapshot.generation(),
            service.source.clone(),
            revoked,
        )
        .unwrap();
    drop(lease);
    assert!(matches!(
        host.submit_no_control(&evidence, "once", timestamp()),
        Err(FixedDecisionError::StalePolicy)
    ));
    assert!(!root.path().join("counter").exists());
    let index: serde_json::Value = serde_json::from_slice(
        &fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert!(
        index["namespaces"]
            .as_object()
            .unwrap()
            .values()
            .all(|row| row["keys"].as_object().unwrap().is_empty())
    );
}
