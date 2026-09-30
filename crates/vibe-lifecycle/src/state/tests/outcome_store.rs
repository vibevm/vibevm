//! Per-run outcome bytes survive changes to the mutable freshness header.

use std::sync::Arc;

use sha2::{Digest, Sha256};
use vibe_wire::generated::lifecycle_outcome_marker::{
    LifecycleOutcomeMarker, LifecycleOutcomeMarkerEffectState as EffectState,
    LifecycleOutcomeMarkerOutcome as Outcome, ReportEvidence, ReportEvidenceAvailable,
    ReportEvidenceUnavailable, ReportEvidenceUnavailableReason,
};
use vibe_wire::generated::lifecycle_report::{LifecycleReport, LifecycleStepReport};

use super::{OTHER_RUN, RUN_ID, lease};
use crate::LifecycleStateStore;
use crate::state::{
    LifecycleOutcomeInspection, LifecycleOutcomeRecoveryReason, LifecycleOutcomeStoreError,
    inspect_retained_lifecycle_outcome, retain_lifecycle_outcome,
};

fn reservation_ref() -> String {
    format!("sha256:{}", "a".repeat(64))
}

fn report() -> Vec<u8> {
    serde_json::to_vec(&LifecycleReport {
        command: "lifecycle".into(),
        requested: "build".into(),
        chain: vec!["build".into()],
        steps: vec![LifecycleStepReport {
            phase: "build".into(),
            status: "ok".into(),
        }],
        ok: true,
        contributions: Vec::new(),
        notices: Vec::new(),
        delegation: None,
        trace: None,
        verification: None,
    })
    .unwrap()
}

fn completed_marker(run_id: &str, bytes: &[u8]) -> LifecycleOutcomeMarker {
    LifecycleOutcomeMarker {
        schema: 1,
        run_id: run_id.into(),
        reservation_ref: reservation_ref(),
        outcome: Outcome::Completed,
        effect_state: EffectState::Landed,
        report: ReportEvidence::Available(Box::new(ReportEvidenceAvailable {
            bytes: u32::try_from(bytes.len()).unwrap(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        })),
    }
}

fn current_header(lease: &Arc<crate::LifecycleLease>, run_id: &str) -> LifecycleStateStore {
    LifecycleStateStore::begin(
        Arc::clone(lease),
        "build".into(),
        vec!["build".into()],
        "2026-09-30T00:00:00Z".into(),
        run_id.into(),
        ".".into(),
        false,
    )
    .unwrap()
}

#[test]
fn fresh_absence_without_outcome_or_lifecycle_header_stays_absent() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    assert!(matches!(
        inspect_retained_lifecycle_outcome(&lease, RUN_ID, &reservation_ref()).unwrap(),
        LifecycleOutcomeInspection::Absent
    ));
    assert!(!root.path().join(LifecycleStateStore::FILE).exists());
    assert!(!root.path().join(".vibe/lifecycle-outcomes").exists());
    let bytes = report();
    let marker = completed_marker(RUN_ID, &bytes);
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes)),
        Err(LifecycleOutcomeStoreError::MissingRunHeader)
    ));
    assert!(!root.path().join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn matching_current_header_without_marker_requires_recovery() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let _store = current_header(&lease, RUN_ID);
    assert!(matches!(
        inspect_retained_lifecycle_outcome(&lease, RUN_ID, &reservation_ref()).unwrap(),
        LifecycleOutcomeInspection::RecoveryRequired {
            reason: LifecycleOutcomeRecoveryReason::CurrentRunHeaderWithoutMarker
        }
    ));
    let bytes = report();
    let marker = completed_marker(RUN_ID, &bytes);
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes))
            .unwrap(),
        LifecycleOutcomeInspection::Validated { .. }
    ));
}

#[test]
fn foreign_current_header_without_marker_refuses_fresh_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let _store = current_header(&lease, OTHER_RUN);
    assert!(matches!(
        inspect_retained_lifecycle_outcome(&lease, RUN_ID, &reservation_ref()).unwrap(),
        LifecycleOutcomeInspection::RecoveryRequired {
            reason: LifecycleOutcomeRecoveryReason::ForeignCurrentRunHeader { observed }
        } if observed == OTHER_RUN
    ));
    let bytes = report();
    let marker = completed_marker(RUN_ID, &bytes);
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes)),
        Err(LifecycleOutcomeStoreError::ForeignRunHeader { observed }) if observed == OTHER_RUN
    ));
    assert!(!root.path().join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn unattributed_lifecycle_state_without_marker_requires_recovery() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let _store = current_header(&lease, "");
    assert!(matches!(
        inspect_retained_lifecycle_outcome(&lease, RUN_ID, &reservation_ref()).unwrap(),
        LifecycleOutcomeInspection::RecoveryRequired {
            reason: LifecycleOutcomeRecoveryReason::UnattributedStateWithoutMarker
        }
    ));
    let bytes = report();
    let marker = completed_marker(RUN_ID, &bytes);
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes)),
        Err(LifecycleOutcomeStoreError::UnattributedRunHeader)
    ));
    assert!(!root.path().join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn complete_outcome_survives_replacement_of_current_freshness_header() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let _original = current_header(&lease, RUN_ID);
    let bytes = report();
    let marker = completed_marker(RUN_ID, &bytes);
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes))
            .unwrap(),
        LifecycleOutcomeInspection::Validated { .. }
    ));
    let _store = current_header(&lease, OTHER_RUN);
    assert!(matches!(
        inspect_retained_lifecycle_outcome(&lease, RUN_ID, &reservation_ref()).unwrap(),
        LifecycleOutcomeInspection::Validated { marker: observed, report_bytes: Some(observed_bytes) }
            if observed == marker && observed_bytes == bytes
    ));
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes))
            .unwrap(),
        LifecycleOutcomeInspection::Validated { .. }
    ));
}

#[test]
fn report_without_marker_requires_recovery_and_exact_byte_retry_commits_marker() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let _store = current_header(&lease, RUN_ID);
    let bytes = report();
    let marker = completed_marker(RUN_ID, &bytes);
    let home = lease
        .project()
        .ensure_subproject(&[".vibe", "lifecycle-outcomes"])
        .unwrap();
    home.write_atomic(&format!("{RUN_ID}.report.json"), &bytes)
        .unwrap();
    assert!(matches!(
        inspect_retained_lifecycle_outcome(&lease, RUN_ID, &reservation_ref()).unwrap(),
        LifecycleOutcomeInspection::RecoveryRequired {
            reason: LifecycleOutcomeRecoveryReason::MarkerMissing
        }
    ));
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes))
            .unwrap(),
        LifecycleOutcomeInspection::Validated { .. }
    ));
}

#[test]
fn wrong_reservation_and_corrupt_report_refuse_replay() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let _store = current_header(&lease, RUN_ID);
    let bytes = report();
    let marker = completed_marker(RUN_ID, &bytes);
    retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes)).unwrap();
    let foreign = format!("sha256:{}", "b".repeat(64));
    assert!(matches!(
        inspect_retained_lifecycle_outcome(&lease, RUN_ID, &foreign).unwrap(),
        LifecycleOutcomeInspection::RecoveryRequired {
            reason: LifecycleOutcomeRecoveryReason::MarkerMismatch
        }
    ));
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &foreign, &marker, Some(&bytes)),
        Err(LifecycleOutcomeStoreError::Validation(_))
    ));
    let home = lease
        .project()
        .subproject(&[".vibe", "lifecycle-outcomes"])
        .unwrap();
    home.write_atomic(&format!("{RUN_ID}.report.json"), b"corrupt report")
        .unwrap();
    assert!(matches!(
        inspect_retained_lifecycle_outcome(&lease, RUN_ID, &reservation_ref()).unwrap(),
        LifecycleOutcomeInspection::RecoveryRequired {
            reason: LifecycleOutcomeRecoveryReason::ReportMismatch
        }
    ));
    assert!(matches!(
        retain_lifecycle_outcome(&lease, RUN_ID, &reservation_ref(), &marker, Some(&bytes)),
        Err(LifecycleOutcomeStoreError::ConflictingOccupant { .. })
    ));
}

#[test]
fn unavailable_failed_or_uncertain_marker_never_becomes_success() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    for (run_id, outcome) in [(RUN_ID, Outcome::Failed), (OTHER_RUN, Outcome::Uncertain)] {
        let _store = current_header(&lease, run_id);
        let marker = LifecycleOutcomeMarker {
            schema: 1,
            run_id: run_id.into(),
            reservation_ref: reservation_ref(),
            outcome: outcome.clone(),
            effect_state: EffectState::Uncertain,
            report: ReportEvidence::Unavailable(Box::new(ReportEvidenceUnavailable {
                reason: ReportEvidenceUnavailableReason::NotProduced,
            })),
        };
        assert!(matches!(
            retain_lifecycle_outcome(&lease, run_id, &reservation_ref(), &marker, None).unwrap(),
            LifecycleOutcomeInspection::Validated { marker: observed, report_bytes: None }
                if observed.outcome == outcome && observed.effect_state == EffectState::Uncertain
        ));
    }
}
