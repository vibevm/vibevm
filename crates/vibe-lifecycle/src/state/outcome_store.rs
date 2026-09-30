//! Pinned, per-run lifecycle outcome retention under the workspace lease.
//!
//! The report is published and read back before its owner marker. Neither
//! this store nor the mutable lifecycle freshness header can infer a terminal
//! outcome from EOF or successful execution rows.

use vibe_safefs::{Project, PublishStage};
use vibe_wire::generated::lifecycle_outcome_marker::LifecycleOutcomeMarker;

use super::outcome_marker::{LifecycleOutcomeMarkerError, validate_lifecycle_outcome_marker};
use super::run_journal::{
    LifecycleRunJournalError, LifecycleRunJournalObservation, inspect_reserved_lifecycle_run,
};
use crate::LifecycleLease;

const ROOT: [&str; 2] = [".vibe", "lifecycle-outcomes"];
const REPORT_CAP: usize = 8 * 1024 * 1024;
const MARKER_CAP: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleOutcomeRecoveryReason {
    MarkerMissing,
    CurrentRunHeaderWithoutMarker,
    ForeignCurrentRunHeader {
        observed: String,
    },
    UnattributedStateWithoutMarker,
    LifecycleStateUnreadable(String),
    MarkerMalformed(String),
    MarkerMismatch,
    ReportMissing,
    ReportMalformed(String),
    ReportMismatch,
    UnexpectedReport,
    OwnerRootUnusable(String),
    StorageUnreadable {
        member: &'static str,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleOutcomeInspection {
    Absent,
    Validated {
        marker: LifecycleOutcomeMarker,
        /// The exact persisted bytes, not a report reconstructed from state.
        report_bytes: Option<Vec<u8>>,
    },
    RecoveryRequired {
        reason: LifecycleOutcomeRecoveryReason,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum LifecycleOutcomeStoreError {
    #[error("invalid reserved lifecycle run ID or reservation ref")]
    InvalidIdentity,
    #[error(
        "cannot publish a lifecycle outcome before the reserved run has a durable lifecycle header"
    )]
    MissingRunHeader,
    #[error(
        "cannot publish lifecycle outcome for the reserved run: current lifecycle header belongs to `{observed}`"
    )]
    ForeignRunHeader { observed: String },
    #[error(
        "cannot publish lifecycle outcome for the reserved run: current lifecycle state has no run ID"
    )]
    UnattributedRunHeader,
    #[error("cannot prove the reserved lifecycle run header before outcome publication: {0}")]
    UnreadableRunHeader(String),
    #[error("lifecycle outcome marker/report failed owner validation: {0}")]
    Validation(#[from] LifecycleOutcomeMarkerError),
    #[error("lifecycle outcome {member} already has conflicting or corrupt bytes: {reason}")]
    ConflictingOccupant {
        member: &'static str,
        reason: String,
    },
    #[error(
        "lifecycle outcome storage failed at {phase}; prior domain effects remain unclassified: {reason}"
    )]
    Storage { phase: &'static str, reason: String },
    #[error(
        "lifecycle outcome {member} publication may have succeeded; reconcile the retained run before replying: {reason}"
    )]
    PublicationUncertain {
        member: &'static str,
        reason: String,
    },
    #[error(
        "lifecycle outcome {member} did not read back as published; prior domain effects remain unclassified"
    )]
    ReadbackMismatch { member: &'static str },
    #[error("lifecycle outcome {member} exceeds its bounded record size")]
    OverLimit { member: &'static str },
}

fn valid_identity(run_id: &str, reservation_ref: &str) -> bool {
    crate::process::is_valid_run_id(run_id)
        && reservation_ref
            .strip_prefix("sha256:")
            .is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            })
}

fn report_name(run_id: &str) -> String {
    format!("{run_id}.report.json")
}

fn marker_name(run_id: &str) -> String {
    format!("{run_id}.marker.json")
}

fn read_home(lease: &LifecycleLease) -> Result<Option<Project>, LifecycleOutcomeRecoveryReason> {
    let observed = lease
        .project()
        .dir_if_present(&ROOT)
        .map_err(|error| LifecycleOutcomeRecoveryReason::OwnerRootUnusable(format!("{error:#}")))?;
    let Some(observed) = observed else {
        return Ok(None);
    };
    let held = lease
        .project()
        .subproject(&ROOT)
        .map_err(|error| LifecycleOutcomeRecoveryReason::OwnerRootUnusable(format!("{error:#}")))?;
    if observed
        .identity()
        .map_err(|error| LifecycleOutcomeRecoveryReason::OwnerRootUnusable(format!("{error:#}")))?
        != held.root_identity().map_err(|error| {
            LifecycleOutcomeRecoveryReason::OwnerRootUnusable(format!("{error:#}"))
        })?
    {
        return Err(LifecycleOutcomeRecoveryReason::OwnerRootUnusable(
            "outcome owner root changed while it was pinned".into(),
        ));
    }
    Ok(Some(held))
}

fn read_member(
    home: &Project,
    name: &str,
    cap: usize,
    member: &'static str,
) -> Result<Option<Vec<u8>>, LifecycleOutcomeRecoveryReason> {
    home.read_file_bounded(name, cap).map_err(|error| {
        LifecycleOutcomeRecoveryReason::StorageUnreadable {
            member,
            reason: format!("{error:#}"),
        }
    })
}

/// An empty outcome directory is not proof that the domain never started.
/// The mutable lifecycle state can prove only that it did start, never that
/// it completed. A foreign current header also refuses fresh dispatch.
fn absent_or_current_run(
    lease: &LifecycleLease,
    reserved_run_id: &str,
) -> LifecycleOutcomeInspection {
    match inspect_reserved_lifecycle_run(lease, reserved_run_id) {
        Ok(LifecycleRunJournalObservation::Absent) => LifecycleOutcomeInspection::Absent,
        Ok(LifecycleRunJournalObservation::RecoveryRequired { .. }) => {
            LifecycleOutcomeInspection::RecoveryRequired {
                reason: LifecycleOutcomeRecoveryReason::CurrentRunHeaderWithoutMarker,
            }
        }
        Ok(LifecycleRunJournalObservation::Uncertain { .. }) => {
            LifecycleOutcomeInspection::RecoveryRequired {
                reason: LifecycleOutcomeRecoveryReason::UnattributedStateWithoutMarker,
            }
        }
        Err(LifecycleRunJournalError::ForeignCurrent { observed, .. }) => {
            LifecycleOutcomeInspection::RecoveryRequired {
                reason: LifecycleOutcomeRecoveryReason::ForeignCurrentRunHeader { observed },
            }
        }
        Err(error) => LifecycleOutcomeInspection::RecoveryRequired {
            reason: LifecycleOutcomeRecoveryReason::LifecycleStateUnreadable(error.to_string()),
        },
    }
}

/// First publication needs evidence from the actual lifecycle owner, not
/// merely a caller-constructed report and marker. A previously validated
/// marker can replay after a later header replaces this one; this gate is
/// used only when no marker has been retained yet.
fn require_current_run_header(
    lease: &LifecycleLease,
    reserved_run_id: &str,
) -> Result<(), LifecycleOutcomeStoreError> {
    match inspect_reserved_lifecycle_run(lease, reserved_run_id) {
        Ok(LifecycleRunJournalObservation::RecoveryRequired { .. }) => Ok(()),
        Ok(LifecycleRunJournalObservation::Absent) => {
            Err(LifecycleOutcomeStoreError::MissingRunHeader)
        }
        Ok(LifecycleRunJournalObservation::Uncertain { .. }) => {
            Err(LifecycleOutcomeStoreError::UnattributedRunHeader)
        }
        Err(LifecycleRunJournalError::ForeignCurrent { observed, .. }) => {
            Err(LifecycleOutcomeStoreError::ForeignRunHeader { observed })
        }
        Err(error) => Err(LifecycleOutcomeStoreError::UnreadableRunHeader(
            error.to_string(),
        )),
    }
}

/// Inspect only files beneath the already-held root. A missing marker beside
/// report bytes is an interrupted publication, never an absent operation.
#[allow(
    dead_code,
    reason = "C.2 owner adapter follows this retained-domain seam"
)]
pub fn inspect_retained_lifecycle_outcome(
    lease: &LifecycleLease,
    reserved_run_id: &str,
    expected_reservation_ref: &str,
) -> Result<LifecycleOutcomeInspection, LifecycleOutcomeStoreError> {
    if !valid_identity(reserved_run_id, expected_reservation_ref) {
        return Err(LifecycleOutcomeStoreError::InvalidIdentity);
    }
    let home = match read_home(lease) {
        Ok(Some(home)) => home,
        Ok(None) => return Ok(absent_or_current_run(lease, reserved_run_id)),
        Err(reason) => return Ok(LifecycleOutcomeInspection::RecoveryRequired { reason }),
    };
    let marker_bytes = match read_member(&home, &marker_name(reserved_run_id), MARKER_CAP, "marker")
    {
        Ok(bytes) => bytes,
        Err(reason) => return Ok(LifecycleOutcomeInspection::RecoveryRequired { reason }),
    };
    let report_bytes = match read_member(&home, &report_name(reserved_run_id), REPORT_CAP, "report")
    {
        Ok(bytes) => bytes,
        Err(reason) => return Ok(LifecycleOutcomeInspection::RecoveryRequired { reason }),
    };
    let Some(marker_bytes) = marker_bytes else {
        return Ok(if report_bytes.is_some() {
            LifecycleOutcomeInspection::RecoveryRequired {
                reason: LifecycleOutcomeRecoveryReason::MarkerMissing,
            }
        } else {
            absent_or_current_run(lease, reserved_run_id)
        });
    };
    let marker: LifecycleOutcomeMarker = match serde_json::from_slice(&marker_bytes) {
        Ok(marker) => marker,
        Err(error) => {
            return Ok(LifecycleOutcomeInspection::RecoveryRequired {
                reason: LifecycleOutcomeRecoveryReason::MarkerMalformed(error.to_string()),
            });
        }
    };
    match validate_lifecycle_outcome_marker(
        &marker,
        reserved_run_id,
        expected_reservation_ref,
        report_bytes.as_deref(),
    ) {
        Ok(()) => Ok(LifecycleOutcomeInspection::Validated {
            marker,
            report_bytes,
        }),
        Err(error) => Ok(LifecycleOutcomeInspection::RecoveryRequired {
            reason: match error {
                LifecycleOutcomeMarkerError::ReservationMismatch => {
                    LifecycleOutcomeRecoveryReason::MarkerMismatch
                }
                LifecycleOutcomeMarkerError::MissingReport => {
                    LifecycleOutcomeRecoveryReason::ReportMissing
                }
                LifecycleOutcomeMarkerError::UnexpectedReport => {
                    LifecycleOutcomeRecoveryReason::UnexpectedReport
                }
                LifecycleOutcomeMarkerError::ReportMismatch => {
                    LifecycleOutcomeRecoveryReason::ReportMismatch
                }
                LifecycleOutcomeMarkerError::InvalidReport(reason) => {
                    LifecycleOutcomeRecoveryReason::ReportMalformed(reason)
                }
                other => LifecycleOutcomeRecoveryReason::MarkerMalformed(other.to_string()),
            },
        }),
    }
}

fn publish_member(
    home: &Project,
    name: &str,
    bytes: &[u8],
    member: &'static str,
) -> Result<(), LifecycleOutcomeStoreError> {
    home.write_atomic(name, bytes).map(|_| ()).map_err(|error| {
        let stage = error.stage;
        let reason = format!("{:#}", error.into_report());
        if stage == PublishStage::PossiblyPublished {
            LifecycleOutcomeStoreError::PublicationUncertain { member, reason }
        } else {
            LifecycleOutcomeStoreError::Storage {
                phase: member,
                reason,
            }
        }
    })
}

/// Retain a domain-owned outcome after its actual execution. Only exact
/// same-byte report plus same marker may be retried; a report left before its
/// marker is the one repairable publication window. The C.1 dedup index is
/// never written here.
#[allow(
    dead_code,
    reason = "C.2 owner adapter follows this retained-domain seam"
)]
pub fn retain_lifecycle_outcome(
    lease: &LifecycleLease,
    reserved_run_id: &str,
    expected_reservation_ref: &str,
    marker: &LifecycleOutcomeMarker,
    report_bytes: Option<&[u8]>,
) -> Result<LifecycleOutcomeInspection, LifecycleOutcomeStoreError> {
    if !valid_identity(reserved_run_id, expected_reservation_ref) {
        return Err(LifecycleOutcomeStoreError::InvalidIdentity);
    }
    if report_bytes.is_some_and(|bytes| bytes.len() > REPORT_CAP) {
        return Err(LifecycleOutcomeStoreError::OverLimit { member: "report" });
    }
    validate_lifecycle_outcome_marker(
        marker,
        reserved_run_id,
        expected_reservation_ref,
        report_bytes,
    )?;
    let marker_bytes =
        serde_json::to_vec(marker).map_err(|error| LifecycleOutcomeStoreError::Storage {
            phase: "marker encoding",
            reason: error.to_string(),
        })?;
    if marker_bytes.len() > MARKER_CAP {
        return Err(LifecycleOutcomeStoreError::OverLimit { member: "marker" });
    }
    match inspect_retained_lifecycle_outcome(lease, reserved_run_id, expected_reservation_ref)? {
        LifecycleOutcomeInspection::Validated {
            marker: existing,
            report_bytes: existing_bytes,
        } if existing == *marker && existing_bytes.as_deref() == report_bytes => {
            return Ok(LifecycleOutcomeInspection::Validated {
                marker: existing,
                report_bytes: existing_bytes,
            });
        }
        LifecycleOutcomeInspection::Validated { .. } => {
            return Err(LifecycleOutcomeStoreError::ConflictingOccupant {
                member: "marker",
                reason: "a different valid outcome is already retained".into(),
            });
        }
        LifecycleOutcomeInspection::RecoveryRequired { reason }
            if !matches!(
                reason,
                LifecycleOutcomeRecoveryReason::MarkerMissing
                    | LifecycleOutcomeRecoveryReason::CurrentRunHeaderWithoutMarker
                    | LifecycleOutcomeRecoveryReason::ForeignCurrentRunHeader { .. }
                    | LifecycleOutcomeRecoveryReason::UnattributedStateWithoutMarker
                    | LifecycleOutcomeRecoveryReason::LifecycleStateUnreadable(_)
            ) =>
        {
            return Err(LifecycleOutcomeStoreError::ConflictingOccupant {
                member: "marker/report",
                reason: format!("{reason:?}"),
            });
        }
        LifecycleOutcomeInspection::Absent
        | LifecycleOutcomeInspection::RecoveryRequired { .. } => {}
    }
    require_current_run_header(lease, reserved_run_id)?;
    let home = lease.project().ensure_subproject(&ROOT).map_err(|error| {
        LifecycleOutcomeStoreError::Storage {
            phase: "owner root",
            reason: format!("{error:#}"),
        }
    })?;
    let name = report_name(reserved_run_id);
    let existing_report = home.read_file_bounded(&name, REPORT_CAP).map_err(|error| {
        LifecycleOutcomeStoreError::ConflictingOccupant {
            member: "report",
            reason: format!("{error:#}"),
        }
    })?;
    match (existing_report.as_deref(), report_bytes) {
        (Some(existing), Some(candidate)) if existing == candidate => {}
        (Some(_), _) => {
            return Err(LifecycleOutcomeStoreError::ConflictingOccupant {
                member: "report",
                reason: "existing bytes differ from this outcome".into(),
            });
        }
        (None, Some(candidate)) => publish_member(&home, &name, candidate, "report")?,
        (None, None) => {}
    }
    let reread_report = home.read_file_bounded(&name, REPORT_CAP).map_err(|error| {
        LifecycleOutcomeStoreError::Storage {
            phase: "report readback",
            reason: format!("{error:#}"),
        }
    })?;
    if reread_report.as_deref() != report_bytes {
        return Err(LifecycleOutcomeStoreError::ReadbackMismatch { member: "report" });
    }
    validate_lifecycle_outcome_marker(
        marker,
        reserved_run_id,
        expected_reservation_ref,
        reread_report.as_deref(),
    )?;
    let marker_name = marker_name(reserved_run_id);
    if home
        .read_file_bounded(&marker_name, MARKER_CAP)
        .map_err(|error| LifecycleOutcomeStoreError::ConflictingOccupant {
            member: "marker",
            reason: format!("{error:#}"),
        })?
        .is_some()
    {
        return Err(LifecycleOutcomeStoreError::ConflictingOccupant {
            member: "marker",
            reason: "marker appeared while report was prepared".into(),
        });
    }
    publish_member(&home, &marker_name, &marker_bytes, "marker")?;
    match inspect_retained_lifecycle_outcome(lease, reserved_run_id, expected_reservation_ref)? {
        LifecycleOutcomeInspection::Validated {
            marker: observed,
            report_bytes: observed_bytes,
        } if observed == *marker && observed_bytes.as_deref() == report_bytes => {
            Ok(LifecycleOutcomeInspection::Validated {
                marker: observed,
                report_bytes: observed_bytes,
            })
        }
        _ => Err(LifecycleOutcomeStoreError::ReadbackMismatch { member: "marker" }),
    }
}
