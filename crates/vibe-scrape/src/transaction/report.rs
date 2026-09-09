//! Generated epoch-1 report projection from actual transaction evidence.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use vibe_wire::generated::scrape::e1::report as w;

use super::model as tx;
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
        health: health_results(report)?,
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

fn health_results(
    report: &tx::TransactionReport,
) -> Result<w::ScrapeHealthRows, tx::TransactionError> {
    let mut answer = Vec::new();
    for record in &report.verification {
        if !matches!(
            record.phase,
            tx::VerificationPhase::Before | tx::VerificationPhase::AfterHealth
        ) {
            continue;
        }
        let evidence = serde_json::from_slice::<
            vibe_wire::generated::scrape::e2::verification_health_evidence::VerificationHealthEvidence,
        >(&record.evidence.canonical_evidence)
        .map_err(|error| tx::TransactionError::Verification(format!(
            "decoding schema-2 verification health evidence: {error}"
        )))?;
        if evidence.schema != 2 {
            return Err(tx::TransactionError::Verification(format!(
                "unsupported verification health evidence schema {}",
                evidence.schema
            )));
        }
        let expected_phase = match record.phase {
            tx::VerificationPhase::Before => w::ScrapeHealthPhase::Before,
            tx::VerificationPhase::AfterHealth => w::ScrapeHealthPhase::After,
            _ => unreachable!("non-health phases continued above"),
        };
        if evidence.rows.iter().any(|row| row.phase != expected_phase) {
            return Err(tx::TransactionError::Verification(
                "verification health evidence phase differs from its journal record".to_owned(),
            ));
        }
        answer.extend(evidence.rows);
    }
    Ok(answer)
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
mod tests;
