use super::*;

#[test]
fn failed_health_report_keeps_actual_argv_and_stream_digest() {
    let stream = health::StreamEvidence {
        total_bytes: 3,
        sha256: "sha256:abc".into(),
        truncated: false,
        redacted: true,
        utf8: health::Utf8State::Valid,
        head: b"bad".to_vec(),
        tail: Vec::new(),
    };
    let failure = super::super::verifier::HealthFailureEvidence {
        phase: health::HealthPhase::After,
        check_id: "cargo".into(),
        terminal: "execution-failed".into(),
        prior_checks: vec![health::CheckResult {
            id: "lint".into(),
            state: health::CheckState::Completed(health::HealthVerdict::Pass),
            commands: vec![health::CommandExecution {
                step: health::CommandStep::Verify,
                actual_argv: vec!["lint.exe".into()],
                exit_code: 0,
                stdout: stream.clone(),
                stderr: stream.clone(),
                result: None,
            }],
        }],
        prior_executions: vec![health::CommandExecution {
            step: health::CommandStep::Build,
            actual_argv: vec!["cargo.exe".into(), "check".into()],
            exit_code: 0,
            stdout: stream.clone(),
            stderr: stream.clone(),
            result: None,
        }],
        execution: Some(health::CommandExecution {
            step: health::CommandStep::Test,
            actual_argv: vec!["cargo.exe".into(), "test".into()],
            exit_code: 1,
            stdout: stream.clone(),
            stderr: stream,
            result: None,
        }),
        message: "failed".into(),
    };
    let bytes = serde_json::to_vec(&failure).unwrap();
    let record = tx::VerificationRecord {
        phase: tx::VerificationPhase::AfterHealth,
        evidence_sha256: tx::Digest(format!("sha256:{}", "0".repeat(64))),
        evidence: tx::VerificationEvidence {
            accepted: false,
            assurance: tx::Assurance::Reduced,
            summary: "failed".into(),
            canonical_evidence: bytes,
        },
    };
    let health_plan = vec![
        serde_json::json!({"id":"lint","tests":null,"effects":{"network":"inherit"}}),
        serde_json::json!({"id":"cargo","tests":"skipped-by-contract","effects":{"network":"tool-offline"}}),
    ];
    let wire = health_failure_result(&record, w::Phase::After, &health_plan)
        .unwrap()
        .unwrap();
    assert_eq!(wire.len(), 3);
    assert_eq!(wire[0].id, "lint");
    assert_eq!(wire[0].argv, ["lint.exe"]);
    assert_eq!(wire[0].terminal, w::TerminalState::Pass);
    assert_eq!(wire[1].argv, ["cargo.exe", "check"]);
    assert_eq!(wire[1].step, w::HealthResultStep::Build);
    assert_eq!(wire[1].terminal, w::TerminalState::Pass);
    assert!(wire[1].tests_skipped);
    assert_eq!(wire[2].argv, ["cargo.exe", "test"]);
    assert_eq!(wire[2].step, w::HealthResultStep::Test);
    assert_eq!(wire[2].terminal, w::TerminalState::ExecutionFailed);
    assert_eq!(wire[2].stderr.sha256, "sha256:abc");
    assert_eq!(wire[2].findings[0].id, "health-execution-failure");
    assert!(wire[2].tests_skipped);
    assert!(residuals(false).is_empty());
}

#[test]
fn rollback_failed_never_claims_unchanged_file_witnesses() {
    let digest = tx::Digest(format!("sha256:{}", "a".repeat(64)));
    assert!(!may_emit_restored_witnesses(
        tx::Outcome::RollbackFailed,
        Some(&digest),
        Some(&digest),
    ));
    assert!(may_emit_restored_witnesses(
        tx::Outcome::RolledBack,
        Some(&digest),
        Some(&digest),
    ));
}
