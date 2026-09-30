//! Pure validation of one exact five-target machine smoke report.
//!
//! A valid report is coherent data, not proof of its host's origin and not
//! publication authority. Persistence and subject evidence refs belong to
//! the later owner adapter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PUBLICATION-SMOKE-POLICY");

use vibe_wire::generated::distribution_smoke_report::{
    DistributionSmokeReport, DistributionSmokeTarget, DistributionSmokeTargetStatus,
};
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_subject::{PublicationSubjectExpectations, validate_publication_subject};
use crate::release_manifest::SUPPORTED_DISTRIBUTION_TARGETS;
use crate::sha256_digest;

const REPORT_MAX_BYTES: usize = 256 * 1024;
const OUTPUT_MAX_BYTES: usize = 16 * 1024;
const REASON_MAX_BYTES: usize = 512;
const HOST_MAX_BYTES: usize = 128;
const RUNNER_MAX_BYTES: usize = 256;
const ELAPSED_MAX_MS: u64 = 30_000;
const PASSED_REASON: &str = "exact staged native executable returned vibe 1.1.0";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DistributionSmokeReportError {
    #[error("distribution smoke report `{field}` is invalid: {reason}")]
    Invalid { field: String, reason: String },
}

fn invalid(field: impl Into<String>, reason: impl Into<String>) -> DistributionSmokeReportError {
    DistributionSmokeReportError::Invalid {
        field: field.into(),
        reason: reason.into(),
    }
}

fn bounded_text(field: &str, value: &str, cap: usize) -> Result<(), DistributionSmokeReportError> {
    if value.is_empty()
        || value.len() > cap
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(invalid(
            field,
            "blank, overlong, padded or control-bearing text",
        ));
    }
    Ok(())
}

fn digest(field: &str, value: &str) -> Result<(), DistributionSmokeReportError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(invalid(field, "expected sha256:<64 lowercase hex>"));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(invalid(field, "expected sha256:<64 lowercase hex>"));
    }
    Ok(())
}

fn canonical_u64(field: &str, value: &str, max: u64) -> Result<u64, DistributionSmokeReportError> {
    let number = value
        .parse::<u64>()
        .map_err(|_| invalid(field, "expected canonical decimal u64"))?;
    if number > max || number.to_string() != value {
        return Err(invalid(field, "noncanonical or over-limit decimal u64"));
    }
    Ok(number)
}

fn compatible_host(target: &str, host: &str) -> bool {
    match target {
        "x86_64-pc-windows-msvc" => host == "windows/x86_64",
        "x86_64-unknown-linux-musl" | "x86_64-unknown-linux-gnu" => {
            matches!(host, "linux/x86_64/gnu" | "linux/x86_64/musl")
        }
        "x86_64-apple-darwin" => host == "macos/x86_64",
        "aarch64-apple-darwin" => host == "macos/aarch64",
        _ => false,
    }
}

fn validate_row(
    index: usize,
    row: &DistributionSmokeTarget,
    subject: &PublicationSubject,
) -> Result<(), DistributionSmokeReportError> {
    let field = format!("targets[{index}]");
    let expected_target = SUPPORTED_DISTRIBUTION_TARGETS[index];
    let subject_target = &subject.targets[index];
    if row.target != expected_target
        || row.target != subject_target.target
        || row.bundle_name != subject_target.bundle.name
        || row.bundle_sha256 != subject_target.bundle.sha256
        || row.bundle_size_bytes != subject_target.bundle.size_bytes
    {
        return Err(invalid(
            &field,
            "target or exact bundle differs from publication subject",
        ));
    }
    digest(&format!("{field}.bundle_sha256"), &row.bundle_sha256)?;
    canonical_u64(
        &format!("{field}.bundle_size_bytes"),
        &row.bundle_size_bytes,
        u64::MAX,
    )?;
    bounded_text(&format!("{field}.host"), &row.host, HOST_MAX_BYTES)?;
    if !row
        .host
        .bytes()
        .all(|byte| byte.is_ascii_graphic() && byte != b';')
    {
        return Err(invalid(
            format!("{field}.host"),
            "host must be a portable bounded token",
        ));
    }
    bounded_text(&format!("{field}.reason"), &row.reason, REASON_MAX_BYTES)?;
    if row.stdout.len() > OUTPUT_MAX_BYTES || row.stderr.len() > OUTPUT_MAX_BYTES {
        return Err(invalid(&field, "captured stdout or stderr exceeds 16 KiB"));
    }
    let elapsed = canonical_u64(
        &format!("{field}.elapsed_ms"),
        &row.elapsed_ms,
        ELAPSED_MAX_MS,
    )?;
    if let Some(value) = row.executable_sha256.as_deref() {
        digest(&format!("{field}.executable_sha256"), value)?;
    }
    match row.status {
        DistributionSmokeTargetStatus::Passed | DistributionSmokeTargetStatus::Failed => {
            if !compatible_host(&row.target, &row.host) {
                return Err(invalid(&field, "executed probe names a foreign host"));
            }
            if row.tested_sha256.as_deref() != Some(row.bundle_sha256.as_str()) {
                return Err(invalid(
                    &field,
                    "executed probe lacks exact tested bundle digest",
                ));
            }
            let runner = row
                .runner_ref
                .as_deref()
                .ok_or_else(|| invalid(&field, "executed probe lacks an actual native runner"))?;
            bounded_text(&format!("{field}.runner_ref"), runner, RUNNER_MAX_BYTES)?;
            if runner != format!("system-native/{}", row.host) {
                return Err(invalid(
                    &field,
                    "runner differs from the reported native host",
                ));
            }
            let expected_executable = &subject_target.components[0].sha256;
            if row.executable_sha256.as_deref() != Some(expected_executable.as_str()) {
                return Err(invalid(
                    &field,
                    "measured executable differs from subject component",
                ));
            }
            if row.status == DistributionSmokeTargetStatus::Passed
                && (row.exit_code != Some(0)
                    || row.stdout.trim() != "vibe 1.1.0"
                    || row.timed_out
                    || row.output_truncated
                    || row.reason != PASSED_REASON)
            {
                return Err(invalid(
                    &field,
                    "passed probe lacks successful exact native output",
                ));
            }
            if row.status == DistributionSmokeTargetStatus::Failed && row.reason == PASSED_REASON {
                return Err(invalid(&field, "failed probe carries a success reason"));
            }
            if elapsed == 0 && row.timed_out {
                return Err(invalid(
                    &field,
                    "timed-out probe cannot have zero elapsed time",
                ));
            }
        }
        DistributionSmokeTargetStatus::Skipped | DistributionSmokeTargetStatus::Unavailable => {
            if row.tested_sha256.is_some()
                || row.runner_ref.is_some()
                || row.exit_code.is_some()
                || !row.stdout.is_empty()
                || !row.stderr.is_empty()
                || row.timed_out
                || row.output_truncated
            {
                return Err(invalid(
                    &field,
                    "non-executed probe claims process evidence",
                ));
            }
            if elapsed != 0 {
                return Err(invalid(
                    &field,
                    "non-executed probe has elapsed process time",
                ));
            }
        }
    }
    Ok(())
}

/// Validate the strict generated shape and complete semantic join, then
/// return the SHA-256 of the original bytes. This function reads no files,
/// launches no process, and cannot attest that a claimed host truly ran.
pub fn validate_distribution_smoke_report_bytes(
    bytes: &[u8],
    subject: &PublicationSubject,
) -> Result<String, DistributionSmokeReportError> {
    if bytes.is_empty() || bytes.len() > REPORT_MAX_BYTES {
        return Err(invalid("bytes", "empty or over 256 KiB"));
    }
    validate_publication_subject(
        subject,
        &PublicationSubjectExpectations {
            version: "1.1.0",
            repository: "vibevm/vibevm",
            smoke_policy_id: "distribution-smoke/e1",
            smoke_policy_epoch: 1,
        },
    )
    .map_err(|error| invalid("subject", error.to_string()))?;
    let report: DistributionSmokeReport =
        serde_json::from_slice(bytes).map_err(|error| invalid("shape", error.to_string()))?;
    if report.schema != 1
        || report.version != "1.1.0"
        || report.version != subject.version
        || report.source_commit != subject.source.commit
        || report.source_tree != subject.source.tree
        || report.smoke_policy_id != "distribution-smoke/e1"
        || report.smoke_policy_id != subject.smoke.policy_id
        || report.smoke_policy_epoch != 1
        || report.smoke_policy_epoch != subject.smoke.policy_epoch
        || report.targets.len() != SUPPORTED_DISTRIBUTION_TARGETS.len()
    {
        return Err(invalid(
            "identity",
            "source, policy, version or five-target shape differs",
        ));
    }
    for (index, row) in report.targets.iter().enumerate() {
        validate_row(index, row, subject)?;
    }
    Ok(sha256_digest(bytes))
}

#[cfg(test)]
#[path = "distribution_smoke_report/tests.rs"]
mod tests;
