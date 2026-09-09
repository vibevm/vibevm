//! Domain-to-generated canonical health evidence projection.

use vibe_wire::generated::scrape::e2::verification_health_evidence as w;

use super::{health, tx};

pub(crate) struct FailureEvidence {
    pub(crate) phase: health::HealthPhase,
    pub(crate) check_id: String,
    pub(crate) terminal: w::ScrapeHealthTerminalState,
    pub(crate) execution: Option<health::CommandExecution>,
    pub(crate) prior_executions: Vec<health::CommandExecution>,
    pub(crate) prior_checks: Vec<health::CheckResult>,
    pub(crate) message: String,
}

pub(crate) fn phase_bytes(
    prepared: &health::PreparedHealth,
    result: &health::PhaseHealthResult,
) -> Result<Vec<u8>, tx::TransactionError> {
    let mut rows = Vec::new();
    for check in &result.checks {
        append_check_rows(&mut rows, prepared, check, phase(result.phase));
    }
    encode(rows)
}

pub(crate) fn failure_bytes(
    prepared: &health::PreparedHealth,
    failure: &FailureEvidence,
) -> Result<Vec<u8>, tx::TransactionError> {
    let phase = phase(failure.phase);
    let mut rows = Vec::new();
    for check in &failure.prior_checks {
        append_check_rows(&mut rows, prepared, check, phase.clone());
    }
    let (tests_skipped, network_verified) = planned_flags(prepared, &failure.check_id);
    rows.extend(
        failure
            .prior_executions
            .iter()
            .map(|execution| w::ScrapeHealthRow {
                argv: execution.actual_argv.clone(),
                findings: Vec::new(),
                id: failure.check_id.clone(),
                network_verified,
                phase: phase.clone(),
                stderr: stream(&execution.stderr),
                stdout: stream(&execution.stdout),
                step: step(execution.step),
                terminal: w::ScrapeHealthTerminalState::Pass,
                tests_skipped,
            }),
    );
    rows.push(w::ScrapeHealthRow {
        argv: failure
            .execution
            .as_ref()
            .map(|execution| execution.actual_argv.clone())
            .unwrap_or_default(),
        findings: vec![w::ScrapeHealthFinding {
            id: "health-execution-failure".to_owned(),
            message: bounded_text(&failure.message),
            severity: w::ScrapeHealthSeverity::Error,
            evidence: None,
        }],
        id: failure.check_id.clone(),
        network_verified,
        phase,
        stderr: failure
            .execution
            .as_ref()
            .map(|execution| stream(&execution.stderr))
            .unwrap_or_else(empty_stream),
        stdout: failure
            .execution
            .as_ref()
            .map(|execution| stream(&execution.stdout))
            .unwrap_or_else(empty_stream),
        step: failure
            .execution
            .as_ref()
            .map_or(w::ScrapeHealthStep::None, |execution| step(execution.step)),
        terminal: failure.terminal.clone(),
        tests_skipped,
    });
    encode(rows)
}

fn encode(rows: Vec<w::ScrapeHealthRow>) -> Result<Vec<u8>, tx::TransactionError> {
    serde_json::to_vec(&w::VerificationHealthEvidence { schema: 2, rows })
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))
}

fn append_check_rows(
    rows: &mut Vec<w::ScrapeHealthRow>,
    prepared: &health::PreparedHealth,
    check: &health::CheckResult,
    phase: w::ScrapeHealthPhase,
) {
    let (tests_skipped, network_verified) = planned_flags(prepared, &check.id);
    let (terminal, findings) = terminal_findings(&check.state);
    if check.commands.is_empty() {
        rows.push(w::ScrapeHealthRow {
            argv: Vec::new(),
            findings,
            id: check.id.clone(),
            network_verified,
            phase,
            stderr: empty_stream(),
            stdout: empty_stream(),
            step: w::ScrapeHealthStep::None,
            terminal,
            tests_skipped,
        });
        return;
    }
    let last = check.commands.len() - 1;
    rows.extend(
        check
            .commands
            .iter()
            .enumerate()
            .map(|(index, execution)| w::ScrapeHealthRow {
                argv: execution.actual_argv.clone(),
                findings: if index == last {
                    findings.clone()
                } else {
                    Vec::new()
                },
                id: check.id.clone(),
                network_verified,
                phase: phase.clone(),
                stderr: stream(&execution.stderr),
                stdout: stream(&execution.stdout),
                step: step(execution.step),
                terminal: if index == last {
                    terminal.clone()
                } else {
                    w::ScrapeHealthTerminalState::Pass
                },
                tests_skipped,
            }),
    );
}

fn planned_flags(prepared: &health::PreparedHealth, id: &str) -> (bool, bool) {
    let check = prepared.checks.iter().find(|check| check.id == id);
    let tests_skipped = check.and_then(|check| check.tests).is_some_and(|tests| {
        matches!(
            tests,
            health::TestDisposition::SkippedByContract | health::TestDisposition::SkippedNotPresent
        )
    });
    let network_verified = check.is_some_and(|check| check.network == health::NetworkMode::Deny);
    (tests_skipped, network_verified)
}

fn terminal_findings(
    state: &health::CheckState,
) -> (w::ScrapeHealthTerminalState, Vec<w::ScrapeHealthFinding>) {
    match state {
        health::CheckState::Skipped { .. } => (w::ScrapeHealthTerminalState::Skipped, Vec::new()),
        health::CheckState::Completed(health::HealthVerdict::Pass) => {
            (w::ScrapeHealthTerminalState::Pass, Vec::new())
        }
        health::CheckState::Completed(health::HealthVerdict::Structured(value)) => (
            match value.status {
                health::HealthStatus::Pass => w::ScrapeHealthTerminalState::Pass,
                health::HealthStatus::Warn => w::ScrapeHealthTerminalState::Warn,
                health::HealthStatus::Fail => w::ScrapeHealthTerminalState::Fail,
            },
            value
                .findings
                .iter()
                .map(|finding| w::ScrapeHealthFinding {
                    id: finding.id.clone(),
                    message: finding.message.clone(),
                    severity: match finding.severity {
                        health::Severity::Info => w::ScrapeHealthSeverity::Info,
                        health::Severity::Warning => w::ScrapeHealthSeverity::Warning,
                        health::Severity::Error => w::ScrapeHealthSeverity::Error,
                    },
                    evidence: finding.evidence.clone(),
                })
                .collect(),
        ),
    }
}

fn phase(value: health::HealthPhase) -> w::ScrapeHealthPhase {
    match value {
        health::HealthPhase::Before => w::ScrapeHealthPhase::Before,
        health::HealthPhase::After => w::ScrapeHealthPhase::After,
    }
}

fn step(value: health::CommandStep) -> w::ScrapeHealthStep {
    match value {
        health::CommandStep::Install => w::ScrapeHealthStep::Install,
        health::CommandStep::Build => w::ScrapeHealthStep::Build,
        health::CommandStep::Test => w::ScrapeHealthStep::Test,
        health::CommandStep::Verify => w::ScrapeHealthStep::Verify,
    }
}

fn stream(value: &health::StreamEvidence) -> w::ScrapeHealthStreamWitness {
    w::ScrapeHealthStreamWitness {
        bytes: value.total_bytes.to_string(),
        head: String::from_utf8_lossy(&value.head).into_owned(),
        redacted: value.redacted,
        sha256: value.sha256.clone(),
        tail: String::from_utf8_lossy(&value.tail).into_owned(),
        truncated: value.truncated,
        utf8: value.utf8 == health::Utf8State::Valid,
    }
}

fn empty_stream() -> w::ScrapeHealthStreamWitness {
    w::ScrapeHealthStreamWitness {
        bytes: "0".to_owned(),
        head: String::new(),
        redacted: true,
        sha256: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            .to_owned(),
        tail: String::new(),
        truncated: false,
        utf8: true,
    }
}

fn bounded_text(value: &str) -> String {
    value.chars().take(4096).collect()
}
