//! Content-addressed local storage for an already coherent smoke report.
//!
//! This is local preparation evidence. It cannot turn a claimed native run
//! into an attested run or authorize any release-host effect.

use std::fmt;

use vibe_publish::distribution_smoke_report::validate_distribution_smoke_report_bytes;
use vibe_safefs::{Project, PublishStage};
use vibe_wire::generated::publication_subject::PublicationSubject;

const REPORT_MAX_BYTES: usize = 256 * 1024;
const EVIDENCE_DIR: [&str; 2] = ["smoke", "e1"];

#[derive(Debug, PartialEq, Eq)]
pub(super) struct StoredSmokeReport {
    pub(super) digest: String,
    /// Relative to the supplied, held output project.
    pub(super) relative_path: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ReportStoreError {
    InvalidReport(String),
    RecoveryRequired { location: String, reason: String },
}

impl fmt::Display for ReportStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReport(reason) => write!(f, "invalid smoke report: {reason}"),
            Self::RecoveryRequired { location, reason } => {
                write!(f, "smoke report `{location}` needs recovery: {reason}")
            }
        }
    }
}

impl std::error::Error for ReportStoreError {}

fn recovery(location: &str, reason: impl Into<String>) -> ReportStoreError {
    ReportStoreError::RecoveryRequired {
        location: location.to_string(),
        reason: reason.into(),
    }
}

/// Validate before creating any directory, then publish under an exact digest
/// name with safefs' pinned create-new/no-replace operation. An identical
/// existing regular single-link file is idempotent; any other occupant or
/// uncertain publication is retained for explicit recovery, never replaced.
#[allow(dead_code)]
pub(super) fn store_report(
    output: &Project,
    bytes: &[u8],
    subject: &PublicationSubject,
) -> Result<StoredSmokeReport, ReportStoreError> {
    let digest = validate_distribution_smoke_report_bytes(bytes, subject)
        .map_err(|error| ReportStoreError::InvalidReport(error.to_string()))?;
    let suffix = digest
        .strip_prefix("sha256:")
        .expect("validated report digest has sha256 prefix");
    let name = format!("{suffix}.json");
    let relative_path = format!("{}/{}/{}", EVIDENCE_DIR[0], EVIDENCE_DIR[1], name);
    let evidence = output.ensure_subproject(&EVIDENCE_DIR).map_err(|error| {
        recovery(
            &relative_path,
            format!("pinning evidence subproject: {error:#}"),
        )
    })?;
    let directory = evidence.root_dir().map_err(|error| {
        recovery(
            &relative_path,
            format!("pinning evidence directory: {error:#}"),
        )
    })?;

    match evidence.read_file_bounded_in(&directory, &name, REPORT_MAX_BYTES) {
        Ok(Some(existing)) if existing == bytes => {
            return Ok(StoredSmokeReport {
                digest,
                relative_path,
            });
        }
        Ok(Some(_)) => return Err(recovery(&relative_path, "digest name has different bytes")),
        Ok(None) => {}
        Err(error) => {
            return Err(recovery(
                &relative_path,
                format!("existing entry cannot be safely read: {error:#}"),
            ));
        }
    }

    if let Err(error) = evidence.publish_new_in(&directory, &name, bytes) {
        if error.stage == PublishStage::BeforePublication {
            // Another writer may have won the exact name after our first read.
            // Count it as idempotent only after the same pinned bounded proof.
            if let Ok(Some(existing)) =
                evidence.read_file_bounded_in(&directory, &name, REPORT_MAX_BYTES)
                && existing == bytes
            {
                return Ok(StoredSmokeReport {
                    digest,
                    relative_path,
                });
            }
        }
        return Err(recovery(
            &relative_path,
            format!("create-new publication {:?}: {error}", error.stage),
        ));
    }
    let visible = evidence
        .read_file_bounded_in(&directory, &name, REPORT_MAX_BYTES)
        .map_err(|error| recovery(&relative_path, format!("post-publication read: {error:#}")))?;
    if visible.as_deref() != Some(bytes) {
        return Err(recovery(
            &relative_path,
            "published report disappeared or differs",
        ));
    }
    Ok(StoredSmokeReport {
        digest,
        relative_path,
    })
}

#[cfg(test)]
#[path = "report_store/tests.rs"]
mod tests;
