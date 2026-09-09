use super::*;
use crate::health;

fn prepared_check(
    id: &str,
    tests: Option<health::TestDisposition>,
    network: health::NetworkMode,
) -> health::PreparedHealthcheck {
    health::PreparedHealthcheck {
        id: id.to_owned(),
        kind: health::HealthcheckKind::Cargo,
        root: ".".to_owned(),
        applicability: health::Applicability::Applicable,
        tests,
        network,
        assets: Vec::new(),
        commands: Vec::new(),
        effects: health::EffectPlan {
            reads: Vec::new(),
            writes: Vec::new(),
            spawn: false,
        },
        sandbox: health::SandboxRequirement::for_check(network, false, false),
        protocol: health::ResultProtocol::BuiltIn,
        custom_bundle: None,
        assurance_reductions: Vec::new(),
        timeout_seconds: 1,
    }
}

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
    let prior_checks = vec![health::CheckResult {
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
    }];
    let prior_executions = vec![health::CommandExecution {
        step: health::CommandStep::Build,
        actual_argv: vec!["cargo.exe".into(), "check".into()],
        exit_code: 0,
        stdout: stream.clone(),
        stderr: stream.clone(),
        result: None,
    }];
    let execution = health::CommandExecution {
        step: health::CommandStep::Test,
        actual_argv: vec!["cargo.exe".into(), "test".into()],
        exit_code: 1,
        stdout: stream.clone(),
        stderr: stream,
        result: None,
    };
    let prepared = health::PreparedHealth {
        plan_id: "plan".to_owned(),
        baseline: health::BaselinePolicy::Strict,
        max_stdout_bytes: 16,
        max_stderr_bytes: 16,
        max_result_bytes: 16,
        termination_grace_seconds: 1,
        checks: vec![
            prepared_check("lint", None, health::NetworkMode::Inherit),
            prepared_check(
                "cargo",
                Some(health::TestDisposition::SkippedByContract),
                health::NetworkMode::ToolOffline,
            ),
        ],
        blockers: Vec::new(),
    };
    let message = "x".repeat(5000);
    let bytes = super::super::verifier::evidence::failure_bytes(
        &prepared,
        &super::super::verifier::evidence::FailureEvidence {
            phase: health::HealthPhase::After,
            check_id: "cargo".to_owned(),
            terminal: w::ScrapeHealthTerminalState::ExecutionFailed,
            execution: Some(execution),
            prior_executions,
            prior_checks,
            message,
        },
    )
    .unwrap();
    let wire = serde_json::from_slice::<
        vibe_wire::generated::scrape::e2::verification_health_evidence::VerificationHealthEvidence,
    >(&bytes)
    .unwrap()
    .rows;
    assert_eq!(wire.len(), 3);
    assert_eq!(wire[0].id, "lint");
    assert_eq!(wire[0].argv, ["lint.exe"]);
    assert_eq!(wire[0].terminal, w::ScrapeHealthTerminalState::Pass);
    assert_eq!(wire[1].argv, ["cargo.exe", "check"]);
    assert_eq!(wire[1].step, w::ScrapeHealthStep::Build);
    assert_eq!(wire[1].terminal, w::ScrapeHealthTerminalState::Pass);
    assert!(wire[1].tests_skipped);
    assert_eq!(wire[2].argv, ["cargo.exe", "test"]);
    assert_eq!(wire[2].step, w::ScrapeHealthStep::Test);
    assert_eq!(
        wire[2].terminal,
        w::ScrapeHealthTerminalState::ExecutionFailed
    );
    assert_eq!(wire[2].stderr.sha256, "sha256:abc");
    assert_eq!(wire[2].findings[0].id, "health-execution-failure");
    assert_eq!(wire[2].findings[0].message.chars().count(), 4096);
    assert!(wire[2].tests_skipped);
    assert!(residuals(false).is_empty());

    let report = tx::TransactionReport {
        project_key: tx::ProjectKey("project".to_owned()),
        transaction_id: tx::TransactionId("transaction".to_owned()),
        plan_id: tx::Digest("plan".to_owned()),
        mode: tx::TransactionMode::InPlace,
        outcome: tx::Outcome::Refused,
        assurance: tx::Assurance::Reduced,
        cleanup: tx::Cleanup::Complete,
        before_tree: None,
        after_tree: None,
        snapshots: Vec::new(),
        verification: vec![tx::VerificationRecord {
            phase: tx::VerificationPhase::Before,
            evidence_sha256: tx::Digest("digest".to_owned()),
            evidence: tx::VerificationEvidence {
                accepted: false,
                assurance: tx::Assurance::Reduced,
                summary: "failed".to_owned(),
                canonical_evidence: bytes,
            },
        }],
        planned_mutations: Vec::new(),
        actual_mutations: Vec::new(),
        events: Vec::new(),
    };
    let error = health_results(&report).unwrap_err().to_string();
    assert!(error.contains("phase differs from its journal record"));
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
