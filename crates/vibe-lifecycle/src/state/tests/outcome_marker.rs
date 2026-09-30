use std::path::PathBuf;

use sha2::{Digest, Sha256};
use vibe_wire::generated::lifecycle_outcome_marker::{
    LifecycleOutcomeMarker, LifecycleOutcomeMarkerOutcome as Outcome, ReportEvidence,
};

use super::super::outcome_marker::{
    LifecycleOutcomeMarkerError, validate_lifecycle_outcome_marker,
};

fn corpus(category: &str, name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/lifecycle_outcome_marker/e1")
        .join(category)
        .join(format!("{name}.json"));
    std::fs::read(path).unwrap()
}

fn marker(category: &str, name: &str) -> LifecycleOutcomeMarker {
    serde_json::from_slice(&corpus(category, name)).unwrap()
}

fn report(run_id: &str, parked: bool) -> Vec<u8> {
    let mut value = serde_json::json!({
        "ok": true,
        "command": "lifecycle",
        "requested": "build",
        "chain": ["build"],
        "steps": [{"phase":"build", "status": if parked { "delegated" } else { "ok" }}],
        "contributions": [],
        "notices": []
    });
    if parked {
        value["delegation"] = serde_json::json!({
            "run_id":run_id,
            "tasks":[".vibe/tasks/one.json"],
            "resume":"vibe build"
        });
    }
    serde_json::to_vec(&value).unwrap()
}

fn assert_semantic_report_refusal(case: &mut LifecycleOutcomeMarker, report: serde_json::Value) {
    let bytes = serde_json::to_vec(&report).unwrap();
    let ReportEvidence::Available(evidence) = &mut case.report else {
        panic!("semantic report test requires available evidence");
    };
    evidence.bytes = u32::try_from(bytes.len()).unwrap();
    evidence.sha256 = format!("{:x}", Sha256::digest(&bytes));
    assert!(matches!(
        validate_lifecycle_outcome_marker(case, &case.run_id, &case.reservation_ref, Some(&bytes)),
        Err(LifecycleOutcomeMarkerError::OutcomeMismatch(_))
    ));
}

fn assert_semantic_report_acceptance(case: &mut LifecycleOutcomeMarker, report: serde_json::Value) {
    let bytes = serde_json::to_vec(&report).unwrap();
    let ReportEvidence::Available(evidence) = &mut case.report else {
        panic!("semantic report test requires available evidence");
    };
    evidence.bytes = u32::try_from(bytes.len()).unwrap();
    evidence.sha256 = format!("{:x}", Sha256::digest(&bytes));
    validate_lifecycle_outcome_marker(case, &case.run_id, &case.reservation_ref, Some(&bytes))
        .unwrap();
}

#[test]
fn strict_shape_corpus_rejects_unknown_missing_or_wrong_typed_members() {
    for name in [
        "unknown-root",
        "missing-report-digest",
        "wrong-effect-state",
        "unknown-report-arm",
    ] {
        assert!(
            serde_json::from_slice::<LifecycleOutcomeMarker>(&corpus("structural-invalid", name))
                .is_err(),
            "{name} must be structurally invalid"
        );
    }
}

#[test]
fn semantic_corpus_refuses_wrong_epoch_or_identity_grammar() {
    for name in [
        "unsupported-schema",
        "malformed-run-id",
        "malformed-reservation-ref",
    ] {
        let case = marker("semantic-invalid", name);
        assert!(
            validate_lifecycle_outcome_marker(
                &case,
                &"a".repeat(32),
                &format!("sha256:{}", "c".repeat(64)),
                None,
            )
            .is_err(),
            "{name} must fail semantic validation"
        );
    }
}

#[test]
fn unavailable_corpus_preserves_failed_or_uncertain_truth_without_a_report() {
    for name in ["failed-unavailable", "uncertain-unavailable"] {
        let case = marker("valid", name);
        validate_lifecycle_outcome_marker(&case, &case.run_id, &case.reservation_ref, None)
            .unwrap();
        assert!(matches!(case.outcome, Outcome::Failed | Outcome::Uncertain));
    }
}

#[test]
fn available_corpus_requires_exact_bytes_and_outcome_consistency() {
    for (name, parked) in [("completed-available", false), ("parked-available", true)] {
        let case = marker("valid", name);
        let bytes = report(&case.run_id, parked);
        validate_lifecycle_outcome_marker(&case, &case.run_id, &case.reservation_ref, Some(&bytes))
            .unwrap();
        assert_eq!(
            validate_lifecycle_outcome_marker(
                &case,
                &"f".repeat(32),
                &case.reservation_ref,
                Some(&bytes),
            ),
            Err(LifecycleOutcomeMarkerError::ReservationMismatch)
        );
        let mut wrong = bytes.clone();
        wrong.push(b' ');
        assert_eq!(
            validate_lifecycle_outcome_marker(
                &case,
                &case.run_id,
                &case.reservation_ref,
                Some(&wrong),
            ),
            Err(LifecycleOutcomeMarkerError::ReportMismatch)
        );
    }
}

#[test]
fn completed_parked_failed_and_uncertain_are_distinct_from_report_ok() {
    let mut case = marker("valid", "completed-available");
    let bytes = report(&case.run_id, false);
    case.outcome = Outcome::Parked;
    assert!(matches!(
        validate_lifecycle_outcome_marker(&case, &case.run_id, &case.reservation_ref, Some(&bytes)),
        Err(LifecycleOutcomeMarkerError::OutcomeMismatch(_))
    ));
    case.outcome = Outcome::Failed;
    assert!(matches!(
        validate_lifecycle_outcome_marker(&case, &case.run_id, &case.reservation_ref, Some(&bytes)),
        Err(LifecycleOutcomeMarkerError::OutcomeMismatch(_))
    ));
    case.outcome = Outcome::Uncertain;
    validate_lifecycle_outcome_marker(&case, &case.run_id, &case.reservation_ref, Some(&bytes))
        .unwrap();
    assert_eq!(case.outcome, Outcome::Uncertain);
}

#[test]
fn report_phase_status_and_requested_chain_must_match_the_marker_outcome() {
    let mut completed = marker("valid", "completed-available");
    let mut wrong =
        serde_json::from_slice::<serde_json::Value>(&report(&completed.run_id, false)).unwrap();
    wrong["steps"][0]["status"] = "fail".into();
    assert_semantic_report_refusal(&mut completed, wrong);

    let mut completed = marker("valid", "completed-available");
    let mut wrong =
        serde_json::from_slice::<serde_json::Value>(&report(&completed.run_id, false)).unwrap();
    wrong["requested"] = "verify".into();
    assert_semantic_report_refusal(&mut completed, wrong);

    let mut parked = marker("valid", "parked-available");
    let mut wrong =
        serde_json::from_slice::<serde_json::Value>(&report(&parked.run_id, true)).unwrap();
    wrong["chain"] = serde_json::json!(["validate", "build"]);
    wrong["steps"] = serde_json::json!([
        {"phase":"validate", "status":"fail"},
        {"phase":"build", "status":"delegated"}
    ]);
    assert_semantic_report_refusal(&mut parked, wrong);

    let mut failed = marker("valid", "completed-available");
    failed.outcome = Outcome::Failed;
    let mut wrong =
        serde_json::from_slice::<serde_json::Value>(&report(&failed.run_id, false)).unwrap();
    wrong["ok"] = false.into();
    wrong["steps"] = serde_json::json!([
        {"phase":"build", "status":"fail"},
        {"phase":"build", "status":"ok"}
    ]);
    assert_semantic_report_refusal(&mut failed, wrong);

    let mut failed = marker("valid", "completed-available");
    failed.outcome = Outcome::Failed;
    let mut wrong =
        serde_json::from_slice::<serde_json::Value>(&report(&failed.run_id, false)).unwrap();
    wrong["ok"] = false.into();
    wrong["steps"] = serde_json::json!([{"phase":"verify", "status":"fail"}]);
    assert_semantic_report_refusal(&mut failed, wrong);
}

#[test]
fn failed_marker_accepts_only_a_truthful_stopped_phase_or_ordered_measured_prefix() {
    let mut failed = marker("valid", "completed-available");
    failed.outcome = Outcome::Failed;
    let mut report =
        serde_json::from_slice::<serde_json::Value>(&report(&failed.run_id, false)).unwrap();
    report["ok"] = false.into();
    report["chain"] = serde_json::json!(["validate", "install", "generate", "build"]);
    report["steps"] = serde_json::json!([{"phase":"build", "status":"fail"}]);
    assert_semantic_report_acceptance(&mut failed, report.clone());

    report["steps"] = serde_json::json!([{"phase":"install", "status":"fail"}]);
    assert_semantic_report_acceptance(&mut failed, report.clone());

    report["steps"] = serde_json::json!([
        {"phase":"validate", "status":"ok"},
        {"phase":"install", "status":"fail"}
    ]);
    assert_semantic_report_acceptance(&mut failed, report.clone());

    for wrong_steps in [
        serde_json::json!([{"phase":"verify", "status":"fail"}]),
        serde_json::json!([
            {"phase":"install", "status":"ok"},
            {"phase":"validate", "status":"fail"}
        ]),
        serde_json::json!([
            {"phase":"validate", "status":"ok"},
            {"phase":"validate", "status":"fail"}
        ]),
    ] {
        report["steps"] = wrong_steps;
        assert_semantic_report_refusal(&mut failed, report.clone());
    }
    report["steps"] = serde_json::json!([{"phase":"build", "status":"fail"}]);
    report["chain"] = serde_json::json!(["validate", "install", "build", "build"]);
    assert_semantic_report_refusal(&mut failed, report);
}
