//! Generated epoch-1 report projection from actual transaction evidence.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use vibe_wire::generated::scrape::e1::report as w;

use super::model as tx;
use crate::health;
use crate::model::PreparedScrape;

pub fn report_to_wire(
    report: &tx::TransactionReport,
    prepared: &PreparedScrape,
) -> Result<w::Report, tx::TransactionError> {
    let plan = prepared
        .plan
        .to_wire()
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
    report_to_wire_plan(report, &plan)
}

/// Recovery-safe projection using the generated plan snapshot stored before
/// mutation. No source contract or freshly prepared model is consulted.
pub fn report_to_wire_plan(
    report: &tx::TransactionReport,
    plan: &vibe_wire::generated::scrape::e1::plan::Plan,
) -> Result<w::Report, tx::TransactionError> {
    let committed = report.outcome == tx::Outcome::Verified;
    let value = serde_json::to_value(plan)
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
    let items = value["items"].as_array().cloned().unwrap_or_default();
    let mut deleted_artifacts = items
        .iter()
        .filter(|item| {
            item["entry_kind"] == "file"
                && item["disposition"]
                    .as_str()
                    .is_some_and(|kind| kind.starts_with("delete"))
        })
        .map(|item| {
            Ok(w::DeletedArtifact {
                bytes: json_string(item, "bytes")?.unwrap_or_else(|| "0".to_owned()),
                class: json_deleted_class(json_required(item, "class")?)?,
                modification: json_modification(json_required(item, "modification")?)?,
                path: json_required(item, "path")?.to_owned(),
                provenance: item["rule_ids"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
                sha256: json_required(item, "sha256")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, tx::TransactionError>>()?;
    let restored = may_emit_restored_witnesses(
        report.outcome,
        report.before_tree.as_ref(),
        report.after_tree.as_ref(),
    );
    let unchanged_files = items
        .iter()
        .filter(|item| {
            item["entry_kind"] == "file"
                && ((committed && item["disposition"] == "keep") || restored)
        })
        .map(|item| {
            Ok(w::FileWitness {
                bytes: json_required(item, "bytes")?.to_owned(),
                path: json_required(item, "path")?.to_owned(),
                sha256: json_required(item, "sha256")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, tx::TransactionError>>()?;
    let mut rewrites = value["rewrites"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|rewrite| {
            Ok(w::RewriteResult {
                after_sha256: json_required(rewrite, "after_sha256")?.to_owned(),
                before_sha256: json_required(rewrite, "before_sha256")?.to_owned(),
                erasure_equivalent: true,
                id: json_required(rewrite, "id")?.to_owned(),
                kind: json_required(rewrite, "kind")?.to_owned(),
                matches: u32::try_from(
                    rewrite["spans"]
                        .as_array()
                        .ok_or_else(|| {
                            tx::TransactionError::Verification(
                                "plan rewrite has no exact spans array".to_owned(),
                            )
                        })?
                        .len(),
                )
                .map_err(|_| {
                    tx::TransactionError::Verification(
                        "plan rewrite span count exceeds report uint32".to_owned(),
                    )
                })?,
                path: json_required(rewrite, "path")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, tx::TransactionError>>()?;
    let mut relocations = value["relocations"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            Ok(w::RelocationResult {
                bytes: json_required(row, "bytes")?.to_owned(),
                from: json_required(row, "from")?.to_owned(),
                id: json_required(row, "id")?.to_owned(),
                mode_preserved: true,
                sha256: json_required(row, "sha256")?.to_owned(),
                to: json_required(row, "to")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, tx::TransactionError>>()?;
    let mut dependency_graphs = value["native_lock_changes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            Ok(w::DependencyGraphResult {
                after: json_strings(row, "after_graph"),
                before: json_strings(row, "before_graph"),
                manager: match json_required(row, "manager")? {
                    "cargo" => w::DependencyGraphResultManager::Cargo,
                    "npm" => w::DependencyGraphResultManager::Npm,
                    "pnpm" => w::DependencyGraphResultManager::Pnpm,
                    "yarn" => w::DependencyGraphResultManager::Yarn,
                    "go" => w::DependencyGraphResultManager::Go,
                    other => {
                        return Err(tx::TransactionError::Verification(format!(
                            "unknown lock manager `{other}`"
                        )));
                    }
                },
                path: json_required(row, "path")?.to_owned(),
                removed: json_strings(row, "removed"),
            })
        })
        .collect::<Result<Vec<_>, tx::TransactionError>>()?;
    if !committed {
        deleted_artifacts.clear();
        rewrites.clear();
        relocations.clear();
        dependency_graphs.clear();
    }
    Ok(w::Report {
        after_tree_digest: report.after_tree.as_ref().map(|digest| digest.0.clone()),
        apply: actual_steps(report, tx::MutationDirection::Apply),
        assurance: match report.assurance {
            tx::Assurance::Full => w::ReportAssurance::Full,
            tx::Assurance::Reduced => w::ReportAssurance::Reduced,
        },
        before_tree_digest: report
            .before_tree
            .as_ref()
            .map(|digest| digest.0.clone())
            .ok_or_else(|| tx::TransactionError::Verification("missing before tree".into()))?,
        cleanup: match report.cleanup {
            tx::Cleanup::Complete => w::ReportCleanup::Complete,
            tx::Cleanup::Pending => w::ReportCleanup::Pending,
        },
        command: w::ReportCommand::Scrape,
        deleted_artifacts,
        dependency_graphs,
        events: report
            .events
            .iter()
            .map(|event| bounded_text(event))
            .collect(),
        health: health_results_from_plan(report, &value)?,
        mode: match report.mode {
            tx::TransactionMode::Export => w::ReportMode::Export,
            tx::TransactionMode::InPlace => w::ReportMode::InPlace,
        },
        outcome: match report.outcome {
            tx::Outcome::Verified => w::ReportOutcome::Verified,
            tx::Outcome::Refused => w::ReportOutcome::Refused,
            tx::Outcome::RolledBack => w::ReportOutcome::RolledBack,
            tx::Outcome::RollbackFailed => w::ReportOutcome::RollbackFailed,
        },
        plan_id: report.plan_id.0.clone(),
        project_display_root: json_required(&value["project"], "display_root")?.to_owned(),
        recovery: recovery_steps(report),
        relocations,
        residuals: residuals(report.verification.iter().any(|record| {
            record.phase == tx::VerificationPhase::FinalResidual && record.evidence.accepted
        })),
        rewrites,
        rollback: actual_steps(report, tx::MutationDirection::Rollback),
        schema: 1,
        transaction_id: report.transaction_id.0.clone(),
        unchanged_files,
    })
}

fn health_results_from_plan(
    report: &tx::TransactionReport,
    plan: &serde_json::Value,
) -> Result<Vec<w::HealthResult>, tx::TransactionError> {
    let health_plan = plan["healthchecks"].as_array().cloned().unwrap_or_default();
    let mut answer = Vec::new();
    for record in &report.verification {
        let phase = match record.phase {
            tx::VerificationPhase::Before => w::Phase::Before,
            tx::VerificationPhase::AfterHealth => w::Phase::After,
            _ => continue,
        };
        if let Some(failure) = health_failure_result(record, phase.clone(), &health_plan)? {
            answer.extend(failure);
            continue;
        }
        let result: health::PhaseHealthResult =
            serde_json::from_slice(&record.evidence.canonical_evidence)
                .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
        for check in result.checks {
            append_check_rows(&mut answer, check, phase.clone(), &health_plan);
        }
    }
    Ok(answer)
}

fn append_check_rows(
    answer: &mut Vec<w::HealthResult>,
    check: health::CheckResult,
    phase: w::Phase,
    health_plan: &[serde_json::Value],
) {
    let (tests_skipped, network_verified) = planned_health_flags(health_plan, &check.id);
    let (terminal, findings) = terminal_findings(&check.state);
    if check.commands.is_empty() {
        answer.push(w::HealthResult {
            argv: Vec::new(),
            findings,
            id: check.id,
            network_verified,
            phase,
            stderr: empty_stream(),
            stdout: empty_stream(),
            step: w::HealthResultStep::None,
            terminal,
            tests_skipped,
        });
        return;
    }
    let last = check.commands.len() - 1;
    for (index, execution) in check.commands.into_iter().enumerate() {
        answer.push(w::HealthResult {
            argv: execution.actual_argv,
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
            step: health_step(execution.step),
            terminal: if index == last {
                terminal.clone()
            } else {
                w::TerminalState::Pass
            },
            tests_skipped,
        });
    }
}

fn planned_health_flags(health_plan: &[serde_json::Value], id: &str) -> (bool, bool) {
    let prepared = health_plan.iter().find(|row| row["id"] == id);
    let tests_skipped = prepared
        .and_then(|row| row["tests"].as_str())
        .is_some_and(|tests| tests.starts_with("skipped"));
    let network_verified = prepared.is_some_and(|row| row["effects"]["network"] == "deny");
    (tests_skipped, network_verified)
}

fn terminal_findings(state: &health::CheckState) -> (w::TerminalState, Vec<w::Finding>) {
    match state {
        health::CheckState::Skipped { .. } => (w::TerminalState::Skipped, Vec::new()),
        health::CheckState::Completed(health::HealthVerdict::Pass) => {
            (w::TerminalState::Pass, Vec::new())
        }
        health::CheckState::Completed(health::HealthVerdict::Structured(value)) => (
            match value.status {
                health::HealthStatus::Pass => w::TerminalState::Pass,
                health::HealthStatus::Warn => w::TerminalState::Warn,
                health::HealthStatus::Fail => w::TerminalState::Fail,
            },
            value
                .findings
                .iter()
                .map(|finding| w::Finding {
                    id: finding.id.clone(),
                    message: finding.message.clone(),
                    severity: match finding.severity {
                        health::Severity::Info => w::Severity::Info,
                        health::Severity::Warning => w::Severity::Warning,
                        health::Severity::Error => w::Severity::Error,
                    },
                    evidence: finding.evidence.clone(),
                })
                .collect(),
        ),
    }
}
fn json_required<'a>(
    value: &'a serde_json::Value,
    key: &str,
) -> Result<&'a str, tx::TransactionError> {
    value[key]
        .as_str()
        .ok_or_else(|| tx::TransactionError::Verification(format!("plan field `{key}` is absent")))
}
fn json_string(
    value: &serde_json::Value,
    key: &str,
) -> Result<Option<String>, tx::TransactionError> {
    Ok(if value[key].is_null() {
        None
    } else {
        Some(json_required(value, key)?.to_owned())
    })
}
fn json_strings(value: &serde_json::Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect()
}
fn json_deleted_class(value: &str) -> Result<w::DeletedArtifactClass, tx::TransactionError> {
    Ok(match value {
        "generated-owned" => w::DeletedArtifactClass::GeneratedOwned,
        "managed-region" => w::DeletedArtifactClass::ManagedRegion,
        "authored-metadata" => w::DeletedArtifactClass::AuthoredMetadata,
        "authored-product" => w::DeletedArtifactClass::AuthoredProduct,
        "unknown" => w::DeletedArtifactClass::Unknown,
        _ => {
            return Err(tx::TransactionError::Verification(
                "unknown deleted class".into(),
            ));
        }
    })
}
fn json_modification(value: &str) -> Result<w::DeletedArtifactModification, tx::TransactionError> {
    Ok(match value {
        "unmodified" => w::DeletedArtifactModification::Unmodified,
        "modified" => w::DeletedArtifactModification::Modified,
        "unknown" => w::DeletedArtifactModification::Unknown,
        "not-applicable" => w::DeletedArtifactModification::NotApplicable,
        _ => {
            return Err(tx::TransactionError::Verification(
                "unknown modification".into(),
            ));
        }
    })
}

fn health_failure_result(
    record: &tx::VerificationRecord,
    phase: w::Phase,
    health_plan: &[serde_json::Value],
) -> Result<Option<Vec<w::HealthResult>>, tx::TransactionError> {
    let Ok(failure) = serde_json::from_slice::<super::verifier::HealthFailureEvidence>(
        &record.evidence.canonical_evidence,
    ) else {
        return Ok(None);
    };
    let terminal = match failure.terminal.as_str() {
        "execution-failed" => w::TerminalState::ExecutionFailed,
        "cancelled" => w::TerminalState::Cancelled,
        "timed-out" => w::TerminalState::TimedOut,
        other => {
            return Err(tx::TransactionError::Verification(format!(
                "unknown health failure terminal `{other}`"
            )));
        }
    };
    let mut rows = Vec::new();
    for check in failure.prior_checks {
        append_check_rows(&mut rows, check, phase.clone(), health_plan);
    }
    let (tests_skipped, network_verified) = planned_health_flags(health_plan, &failure.check_id);
    rows.extend(
        failure
            .prior_executions
            .into_iter()
            .map(|execution| w::HealthResult {
                argv: execution.actual_argv,
                findings: Vec::new(),
                id: failure.check_id.clone(),
                network_verified,
                phase: phase.clone(),
                stderr: stream(&execution.stderr),
                stdout: stream(&execution.stdout),
                step: health_step(execution.step),
                terminal: w::TerminalState::Pass,
                tests_skipped,
            })
            .collect::<Vec<_>>(),
    );
    rows.push(w::HealthResult {
        argv: failure
            .execution
            .as_ref()
            .map(|execution| execution.actual_argv.clone())
            .unwrap_or_default(),
        findings: vec![w::Finding {
            evidence: None,
            id: "health-execution-failure".to_owned(),
            message: bounded_text(&failure.message),
            severity: w::Severity::Error,
        }],
        id: failure.check_id,
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
            .map_or(w::HealthResultStep::None, |execution| {
                health_step(execution.step)
            }),
        terminal,
        tests_skipped,
    });
    Ok(Some(rows))
}

fn health_step(step: health::CommandStep) -> w::HealthResultStep {
    match step {
        health::CommandStep::Install => w::HealthResultStep::Install,
        health::CommandStep::Build => w::HealthResultStep::Build,
        health::CommandStep::Test => w::HealthResultStep::Test,
        health::CommandStep::Verify => w::HealthResultStep::Verify,
    }
}

fn bounded_text(value: &str) -> String {
    value.chars().take(4096).collect()
}

fn may_emit_restored_witnesses(
    outcome: tx::Outcome,
    before: Option<&tx::Digest>,
    after: Option<&tx::Digest>,
) -> bool {
    outcome == tx::Outcome::RolledBack && before.is_some() && before == after
}

fn stream(value: &health::StreamEvidence) -> w::StreamWitness {
    w::StreamWitness {
        bytes: value.total_bytes.to_string(),
        head: String::from_utf8_lossy(&value.head).into_owned(),
        sha256: value.sha256.clone(),
        tail: String::from_utf8_lossy(&value.tail).into_owned(),
        truncated: value.truncated,
        redacted: value.redacted,
        utf8: value.utf8 == health::Utf8State::Valid,
    }
}
fn empty_stream() -> w::StreamWitness {
    w::StreamWitness {
        bytes: "0".into(),
        head: String::new(),
        sha256: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        tail: String::new(),
        truncated: false,
        redacted: true,
        utf8: true,
    }
}
fn actual_steps(
    report: &tx::TransactionReport,
    direction: tx::MutationDirection,
) -> Vec<w::RecoveryStep> {
    report
        .actual_mutations
        .iter()
        .filter(|step| step.direction == direction)
        .enumerate()
        .map(|(index, step)| w::RecoveryStep {
            action: format!("{:?}", step.kind),
            operation_id: step.id.clone(),
            result: w::RecoveryStepResult::Complete,
            sequence: u32::try_from(index).unwrap_or(u32::MAX),
        })
        .collect()
}
fn recovery_steps(report: &tx::TransactionReport) -> Vec<w::RecoveryStep> {
    report
        .actual_mutations
        .iter()
        .filter(|step| step.origin == tx::MutationOrigin::Recovery)
        .enumerate()
        .map(|(index, step)| w::RecoveryStep {
            action: format!("{:?}/{:?}", step.direction, step.kind),
            operation_id: step.id.clone(),
            result: w::RecoveryStepResult::Complete,
            sequence: u32::try_from(index).unwrap_or(u32::MAX),
        })
        .collect()
}
fn residuals(accepted: bool) -> Vec<w::ResidualCount> {
    if !accepted {
        return Vec::new();
    }
    use w::ResidualCountClass as C;
    [
        C::SourceMetadata,
        C::DependencyIdentity,
        C::ManifestPath,
        C::ManagedMarker,
        C::GeneratedArtifact,
        C::ToolConfig,
        C::ToolScript,
        C::SelectedSpecUri,
        C::EnvironmentReference,
        C::LockEntry,
        C::LinkEscape,
    ]
    .into_iter()
    .map(|class| w::ResidualCount { class, count: 0 })
    .collect()
}
#[cfg(test)]
mod tests {
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
}
