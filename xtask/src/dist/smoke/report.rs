//! Pure joining of five local probe claims to one inspectable report and
//! subject summary. Persistence and native origin remain separate checks.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use vibe_publish::SUPPORTED_DISTRIBUTION_TARGETS;
use vibe_publish::distribution_smoke_report::validate_distribution_smoke_report_bytes;
use vibe_publish::publication_subject::{
    PublicationSubjectExpectations, validate_publication_subject,
};
use vibe_wire::generated::distribution_smoke_report::{
    DistributionSmokeReport, DistributionSmokeTarget, DistributionSmokeTargetStatus,
};
use vibe_wire::generated::publication_subject::{
    PublicationSmoke, PublicationSmokeProbe, PublicationSmokeProbeStatus, PublicationSubject,
};

use super::NativeProbeRecord;
use super::report_store::StoredSmokeReport;

#[allow(dead_code)]
pub(super) struct PreparedSmokeReport {
    pub(super) bytes: Vec<u8>,
    pub(super) digest: String,
}

fn report_status(status: &PublicationSmokeProbeStatus) -> DistributionSmokeTargetStatus {
    match status {
        PublicationSmokeProbeStatus::Passed => DistributionSmokeTargetStatus::Passed,
        PublicationSmokeProbeStatus::Failed => DistributionSmokeTargetStatus::Failed,
        PublicationSmokeProbeStatus::Skipped => DistributionSmokeTargetStatus::Skipped,
        PublicationSmokeProbeStatus::Unavailable => DistributionSmokeTargetStatus::Unavailable,
    }
}

fn subject_status(status: &DistributionSmokeTargetStatus) -> PublicationSmokeProbeStatus {
    match status {
        DistributionSmokeTargetStatus::Passed => PublicationSmokeProbeStatus::Passed,
        DistributionSmokeTargetStatus::Failed => PublicationSmokeProbeStatus::Failed,
        DistributionSmokeTargetStatus::Skipped => PublicationSmokeProbeStatus::Skipped,
        DistributionSmokeTargetStatus::Unavailable => PublicationSmokeProbeStatus::Unavailable,
    }
}

/// Canonicalize one record per supported target and validate the raw encoded
/// report against the provisional subject's exact committed/assets identity.
/// These records are claims: this pure operation does not run a process.
#[allow(dead_code)]
pub(super) fn prepare_report(
    subject: &PublicationSubject,
    records: Vec<NativeProbeRecord>,
) -> Result<PreparedSmokeReport> {
    if records.len() != SUPPORTED_DISTRIBUTION_TARGETS.len() {
        bail!("smoke report requires exactly five probe records");
    }
    let mut by_target = BTreeMap::new();
    for record in records {
        if !SUPPORTED_DISTRIBUTION_TARGETS.contains(&record.target.as_str()) {
            bail!("smoke probe names unsupported target `{}`", record.target);
        }
        if by_target.insert(record.target.clone(), record).is_some() {
            bail!("smoke probe duplicates a target");
        }
    }
    let mut targets = Vec::with_capacity(SUPPORTED_DISTRIBUTION_TARGETS.len());
    for (index, target) in SUPPORTED_DISTRIBUTION_TARGETS.iter().enumerate() {
        let record = by_target
            .remove(*target)
            .with_context(|| format!("smoke probe for `{target}` is missing"))?;
        let subject_target = subject
            .targets
            .get(index)
            .with_context(|| format!("provisional subject lacks `{target}`"))?;
        if subject_target.target != *target || record.bundle_digest != subject_target.bundle.sha256
        {
            bail!("smoke probe for `{target}` differs from exact subject bundle digest");
        }
        targets.push(DistributionSmokeTarget {
            target: record.target,
            bundle_name: subject_target.bundle.name.clone(),
            bundle_sha256: record.bundle_digest,
            bundle_size_bytes: subject_target.bundle.size_bytes.clone(),
            host: record.host,
            status: report_status(&record.status),
            reason: record.reason,
            stdout: record.stdout,
            stderr: record.stderr,
            elapsed_ms: record.elapsed_ms.to_string(),
            timed_out: record.timed_out,
            output_truncated: record.output_truncated,
            runner_ref: record.runner_ref,
            tested_sha256: record.tested_sha256,
            executable_sha256: record.executable_sha256,
            exit_code: record.exit_code,
        });
    }
    let report = DistributionSmokeReport {
        schema: 1,
        version: subject.version.clone(),
        source_commit: subject.source.commit.clone(),
        source_tree: subject.source.tree.clone(),
        smoke_policy_id: subject.smoke.policy_id.clone(),
        smoke_policy_epoch: subject.smoke.policy_epoch,
        targets,
    };
    let bytes = serde_json::to_vec(&report).context("serializing canonical smoke report")?;
    let digest = validate_distribution_smoke_report_bytes(&bytes, subject)
        .context("validating complete smoke report against publication subject")?;
    Ok(PreparedSmokeReport { bytes, digest })
}

/// After an actual content-addressed store returned its held receipt, derive
/// the subject summary. The report bytes remain the digest source; the subject
/// digest is never an input, avoiding a report↔subject identity cycle.
#[allow(dead_code)]
pub(super) fn summary_from_stored_report(
    subject: &PublicationSubject,
    prepared: &PreparedSmokeReport,
    stored: &StoredSmokeReport,
) -> Result<PublicationSmoke> {
    let digest = validate_distribution_smoke_report_bytes(&prepared.bytes, subject)?;
    if digest != prepared.digest || digest != stored.digest {
        bail!("stored smoke digest differs from validated raw report bytes");
    }
    let suffix = digest
        .strip_prefix("sha256:")
        .context("validated smoke digest lacks SHA-256 prefix")?;
    if stored.relative_path != format!("smoke/e1/{suffix}.json") {
        bail!("stored smoke location differs from its content address");
    }
    let report: DistributionSmokeReport = serde_json::from_slice(&prepared.bytes)?;
    let probes = report
        .targets
        .iter()
        .map(|row| {
            let status = subject_status(&row.status);
            let evidence_ref = match &status {
                PublicationSmokeProbeStatus::Passed | PublicationSmokeProbeStatus::Failed => {
                    format!("report={digest};target={}", row.target)
                }
                PublicationSmokeProbeStatus::Skipped => {
                    format!("reason=explicit-skip;host={};report={digest}", row.host)
                }
                PublicationSmokeProbeStatus::Unavailable => {
                    format!(
                        "reason=native-unavailable;host={};report={digest}",
                        row.host
                    )
                }
            };
            PublicationSmokeProbe {
                probe_id: "launch".into(),
                target: row.target.clone(),
                status,
                tested_sha256: row.tested_sha256.clone(),
                runner_ref: row.runner_ref.clone(),
                evidence_ref: Some(evidence_ref),
            }
        })
        .collect();
    let summary = PublicationSmoke {
        policy_id: report.smoke_policy_id,
        policy_epoch: report.smoke_policy_epoch,
        probes,
    };
    let mut with_summary = subject.clone();
    with_summary.smoke = summary.clone();
    validate_publication_subject(
        &with_summary,
        &PublicationSubjectExpectations {
            version: "1.1.0",
            repository: "vibevm/vibevm",
            smoke_policy_id: "distribution-smoke/e1",
            smoke_policy_epoch: 1,
        },
    )?;
    Ok(summary)
}

#[cfg(test)]
#[path = "report/tests.rs"]
mod tests;
