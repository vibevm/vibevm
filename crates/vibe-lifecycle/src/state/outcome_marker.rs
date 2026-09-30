//! Pure semantics for the lifecycle owner's per-run outcome marker.
//!
//! This validates evidence a caller already read through a bounded pinned
//! capability. It neither reads a path nor infers completion from the mutable
//! lifecycle freshness header.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SUBMISSION-RESERVATION");

use sha2::{Digest, Sha256};
use vibe_wire::generated::lifecycle_outcome_marker::{
    LifecycleOutcomeMarker, LifecycleOutcomeMarkerOutcome as Outcome, ReportEvidence,
};
use vibe_wire::generated::lifecycle_report::LifecycleReport;

use crate::process::is_valid_run_id;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LifecycleOutcomeMarkerError {
    #[error("lifecycle outcome marker schema is not epoch one")]
    UnsupportedSchema,
    #[error("lifecycle outcome marker run ID is not 32 lowercase hex")]
    InvalidRunId,
    #[error("lifecycle outcome marker reservation ref is not sha256:<64 lowercase hex>")]
    InvalidReservationRef,
    #[error("lifecycle outcome marker differs from the expected reservation identity")]
    ReservationMismatch,
    #[error("lifecycle outcome marker has no durable report payload for its available arm")]
    MissingReport,
    #[error("lifecycle outcome marker report payload was supplied for its unavailable arm")]
    UnexpectedReport,
    #[error("lifecycle outcome marker report byte count or SHA-256 differs")]
    ReportMismatch,
    #[error("lifecycle outcome marker report is not a generated lifecycle report: {0}")]
    InvalidReport(String),
    #[error("lifecycle outcome marker report and recorded outcome disagree: {0}")]
    OutcomeMismatch(&'static str),
}

/// Validate a marker against the owner's reservation and exact persisted
/// report bytes. A successful return validates its recorded *outcome*; it
/// never upgrades `uncertain` to `completed` because a report says `ok`.
pub fn validate_lifecycle_outcome_marker(
    marker: &LifecycleOutcomeMarker,
    expected_run_id: &str,
    expected_reservation_ref: &str,
    report_bytes: Option<&[u8]>,
) -> Result<(), LifecycleOutcomeMarkerError> {
    if marker.schema != 1 {
        return Err(LifecycleOutcomeMarkerError::UnsupportedSchema);
    }
    if !is_valid_run_id(&marker.run_id) || !is_valid_run_id(expected_run_id) {
        return Err(LifecycleOutcomeMarkerError::InvalidRunId);
    }
    if !valid_reservation_ref(&marker.reservation_ref)
        || !valid_reservation_ref(expected_reservation_ref)
    {
        return Err(LifecycleOutcomeMarkerError::InvalidReservationRef);
    }
    if marker.run_id != expected_run_id || marker.reservation_ref != expected_reservation_ref {
        return Err(LifecycleOutcomeMarkerError::ReservationMismatch);
    }
    match &marker.report {
        ReportEvidence::Available(evidence) => {
            let bytes = report_bytes.ok_or(LifecycleOutcomeMarkerError::MissingReport)?;
            if u32::try_from(bytes.len()).ok() != Some(evidence.bytes)
                || !lower_hex_64(&evidence.sha256)
                || format!("{:x}", Sha256::digest(bytes)) != evidence.sha256
            {
                return Err(LifecycleOutcomeMarkerError::ReportMismatch);
            }
            let report: LifecycleReport = serde_json::from_slice(bytes)
                .map_err(|error| LifecycleOutcomeMarkerError::InvalidReport(error.to_string()))?;
            validate_report(marker, &report)
        }
        ReportEvidence::Unavailable(_) => {
            if report_bytes.is_some() {
                return Err(LifecycleOutcomeMarkerError::UnexpectedReport);
            }
            match marker.outcome {
                Outcome::Completed | Outcome::Parked => {
                    Err(LifecycleOutcomeMarkerError::OutcomeMismatch(
                        "completed or parked outcome requires a validated report",
                    ))
                }
                Outcome::Failed | Outcome::Uncertain => Ok(()),
            }
        }
    }
}

fn valid_reservation_ref(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(lower_hex_64)
}

fn lower_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn validate_report(
    marker: &LifecycleOutcomeMarker,
    report: &LifecycleReport,
) -> Result<(), LifecycleOutcomeMarkerError> {
    if report.command != "lifecycle"
        || report.requested.is_empty()
        || report.chain.last() != Some(&report.requested)
    {
        return Err(LifecycleOutcomeMarkerError::OutcomeMismatch(
            "report command, requested phase or chain is invalid",
        ));
    }
    match marker.outcome {
        Outcome::Completed => {
            if !report.ok
                || report.delegation.is_some()
                || report.steps.len() != report.chain.len()
                || !steps_match_prefix(report)
                || !report
                    .steps
                    .iter()
                    .all(|step| successful_status(&step.status))
            {
                return Err(LifecycleOutcomeMarkerError::OutcomeMismatch(
                    "completed requires a fully executed nondelegated successful report",
                ));
            }
        }
        Outcome::Parked => {
            if !report.ok
                || report
                    .delegation
                    .as_ref()
                    .map(|value| value.run_id.as_str())
                    != Some(marker.run_id.as_str())
                || report.steps.is_empty()
                || !steps_match_prefix(report)
                || report
                    .steps
                    .last()
                    .is_none_or(|step| step.status != "delegated")
                || report.steps[..report.steps.len() - 1]
                    .iter()
                    .any(|step| !successful_status(&step.status))
            {
                return Err(LifecycleOutcomeMarkerError::OutcomeMismatch(
                    "parked requires a successful report ending in this run's delegation",
                ));
            }
        }
        Outcome::Failed => {
            if report.ok || report.delegation.is_some() || !failed_steps_match(report) {
                return Err(LifecycleOutcomeMarkerError::OutcomeMismatch(
                    "failed requires a failed nondelegated report",
                ));
            }
        }
        Outcome::Uncertain => {
            // The caller sees the marker's uncertain outcome, even when a
            // valid available report happens to have `ok = true`.
        }
    }
    Ok(())
}

fn failed_steps_match(report: &LifecycleReport) -> bool {
    // The existing failure projection retains exactly the stopped phase, not
    // invented success rows for earlier phases. A longer measured prefix is
    // valid only when it follows the declared chain in order.
    if report.steps.last().is_none_or(|step| step.status != "fail")
        || report
            .chain
            .iter()
            .enumerate()
            .any(|(index, phase)| report.chain[..index].iter().any(|earlier| earlier == phase))
    {
        return false;
    }
    if report.steps.len() == 1 {
        return report
            .chain
            .iter()
            .any(|phase| phase == &report.steps[0].phase);
    }
    steps_match_prefix(report)
        && report.steps[..report.steps.len() - 1]
            .iter()
            .all(|step| successful_status(&step.status))
}

fn steps_match_prefix(report: &LifecycleReport) -> bool {
    report.steps.len() <= report.chain.len()
        && report
            .steps
            .iter()
            .zip(&report.chain)
            .all(|(step, phase)| step.phase == *phase)
}

fn successful_status(status: &str) -> bool {
    matches!(status, "ok" | "fresh" | "no-op")
}
