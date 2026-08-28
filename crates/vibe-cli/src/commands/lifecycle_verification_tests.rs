//! The CLI failure projection carries the R7.5 evidence member WHOLE.
//!
//! This surface chooses a registered report family from a neutral measurement.
//! It has never been allowed to reformat what it was handed, and the member is
//! the case where reformatting would be silent: an `evidence_id` recomputed,
//! reordered or dropped here still looks like a lifecycle report, and the
//! external orchestrator that joins claims by that id would simply stop
//! finding them.

use vibe_orchestrator::failure::Measurement;
use vibe_wire::behaviour::verification_evidence::validate;
use vibe_wire::generated::shared::{
    DigestWitness, EvidenceRun, EvidenceStatus, InputMeasurement, VerificationEvidence,
};

use super::{compile_trace::RegisteredReportDraft, lifecycle_family, registered_family};

fn member(status: EvidenceStatus) -> VerificationEvidence {
    let stale = status == EvidenceStatus::Stale;
    let witness = |byte: char| DigestWitness {
        algorithm: "sha256:vibe-input-manifest-v1".into(),
        bytes: Some("3".into()),
        digest: format!("sha256:{}", byte.to_string().repeat(64)),
        files: Some(1),
    };
    VerificationEvidence {
        artifacts: Vec::new(),
        evidence: 1,
        evidence_id: format!("sha256:{}", "d".repeat(64)),
        inputs: vec![InputMeasurement {
            declaration_fingerprint: format!("sha256:{}", "e".repeat(64)),
            execution: "org.demo/tools#compile".into(),
            patterns: vec!["data/**".into()],
            phase: "build".into(),
            status: status.clone(),
            measured: Some(witness('1')),
            measured_run_id: Some("0".repeat(32)),
            observed: Some(witness(if stale { '2' } else { '1' })),
            reason_code: None,
        }],
        observed_at: "2026-08-28T12:00:05Z".parse().expect("a fixture instant"),
        run: EvidenceRun {
            chain: vec!["build".into(), "verify".into()],
            requested: "verify".into(),
            run_id: "0".repeat(32),
            selected: ".".into(),
            started: "2026-08-28T11:59:40Z".into(),
        },
        status,
    }
}

fn measurement(verification: Option<VerificationEvidence>) -> Measurement {
    let verification = verification.map(Box::new);
    Measurement::Lifecycle {
        rows: Vec::new(),
        stopped_phase: "verify".into(),
        requested: "verify".into(),
        chain: vec!["build".into(), "verify".into()],
        verification,
    }
}

fn attached(draft: RegisteredReportDraft) -> Option<VerificationEvidence> {
    match draft {
        RegisteredReportDraft::Lifecycle(values) => values.into_report(None).verification,
        _ => panic!("a lifecycle measurement reports in the lifecycle family"),
    }
}

/// A stale stop reaches `vibe verify --json` as the SAME member the engine
/// minted — `evidence_id` included.
#[test]
fn the_failure_projection_hands_on_the_exact_member() {
    let expected = member(EvidenceStatus::Stale);
    validate(&expected).expect("the fixture member is itself valid");

    let projected = attached(lifecycle_family(measurement(Some(expected.clone()))));
    assert_eq!(
        projected,
        Some(expected),
        "the projection chooses a family; it never rebuilds the comparison",
    );
}

/// The same law through the outer entry point a phase verb really calls.
#[test]
fn the_registered_family_entry_point_preserves_it_too() {
    let expected = member(EvidenceStatus::Matched);
    let projected = attached(registered_family(
        std::path::Path::new("/p"),
        measurement(Some(expected.clone())),
    ));
    assert_eq!(projected, Some(expected));
}

/// A prerequisite install's barrier stopped long before verify, so its
/// lifecycle-shaped projection carries no comparison — never an invented one.
#[test]
fn a_slot_shaped_failure_projects_no_member() {
    let projected = attached(lifecycle_family(Measurement::Slot {
        progress: Box::default(),
        reports: Vec::new(),
        packages_resolved: 1,
    }));
    assert!(projected.is_none());
}

/// A lifecycle failure that never reached the boundary omits the member.
#[test]
fn an_unreconciled_lifecycle_failure_projects_no_member() {
    assert!(attached(lifecycle_family(measurement(None))).is_none());
}
