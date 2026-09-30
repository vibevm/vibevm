use super::*;
use vibe_wire::generated::machine::e1::operation_envelope::{BodyError, BodyResult};
use vibe_workspace::migration_planning::{
    MigrationPlanningResponse, decode_migration_request, plan_migration_request,
};

fn service() -> MachineOperationService {
    MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted).unwrap()
}
fn request() -> PlanRequest {
    PlanRequest {
        schema: 1,
        source_text: "not a manifest".to_owned(),
        source_profile: None,
        target_profile: None,
        recipe: None,
        source_decisions: Vec::new(),
    }
}
fn envelope(body: Body) -> OperationEnvelope {
    OperationEnvelope {
        schema: 1,
        operation_id: Some(PLANNER_ID.to_owned()),
        domain_owner: Some(PLANNER_ID.to_owned()),
        source_id: Some(PLANNER_SOURCE.to_owned()),
        evidence_id: None,
        body,
    }
}
fn needs_input_result(request: &PlanRequest) -> OperationEnvelope {
    let MigrationPlanningResponse::Report(report) = plan_migration_request(request.clone()) else {
        panic!("pure planner unexpectedly preflighted")
    };
    assert!(matches!(report.outcome, Outcome::NeedsInput(_)));
    envelope(Body::Result(Box::new(BodyResult {
        format_id: Some(REPORT_FORMAT.to_owned()),
        payload_json: Some(serde_json::to_string(&report).unwrap()),
        effect_state: BodyResultEffectState::NoEffects,
        completeness: BodyResultCompleteness::Partial,
        provenance_id: None,
    })))
}
fn refused_request() -> PlanRequest {
    PlanRequest {
        source_profile: Some("unknown-profile-r1".to_owned()),
        target_profile: Some("materialization-current-absent-r1".to_owned()),
        recipe: Some("identity-r1".to_owned()),
        ..request()
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn admitted_planner_result_replays_before_publication() {
    let request = request();
    let value = needs_input_result(&request);
    assert_eq!(
        service()
            .validate_pure_planner_publication(Some(&request), value.clone())
            .unwrap(),
        value
    );
    assert!(
        service()
            .validate_pure_planner_publication(None, value)
            .is_err()
    );

    let current = PlanRequest {
        schema: 1,
        source_text: "[package]\nname = 'demo'\ngroup = 'org.example'\nkind = 'tool'\nversion = '1.1.0'\nmaterialization = 'copy'\n".to_owned(),
        source_profile: Some("materialization-current-absent-r1".to_owned()),
        target_profile: Some("materialization-current-absent-r1".to_owned()),
        recipe: Some("identity-r1".to_owned()),
        source_decisions: Vec::new(),
    };
    let MigrationPlanningResponse::Report(report) = plan_migration_request(current.clone()) else {
        panic!("current identity request unexpectedly preflighted")
    };
    assert!(matches!(report.outcome, Outcome::NoOp(_)));
    let complete = envelope(Body::Result(Box::new(BodyResult {
        format_id: Some(REPORT_FORMAT.to_owned()),
        payload_json: Some(serde_json::to_string(&report).unwrap()),
        effect_state: BodyResultEffectState::NoEffects,
        completeness: BodyResultCompleteness::Complete,
        provenance_id: None,
    })));
    assert!(
        service()
            .validate_pure_planner_publication(Some(&current), complete.clone())
            .is_ok()
    );
    let unadmitted =
        MachineOperationService::pure_text_planner(PurePlannerAdmission::Unavailable).unwrap();
    assert_eq!(
        unadmitted
            .validate_pure_planner_publication(Some(&current), complete.clone())
            .unwrap_err()
            .original,
        complete
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn domain_refusal_and_preflight_error_keep_distinct_owner_roots() {
    let request = refused_request();
    let MigrationPlanningResponse::Report(report) = plan_migration_request(request.clone()) else {
        panic!("expected a domain refusal")
    };
    assert!(matches!(report.outcome, Outcome::Refused(_)));
    let refused = envelope(Body::Error(Box::new(BodyError {
        format_id: Some(REPORT_FORMAT.to_owned()),
        payload_json: Some(serde_json::to_string(&report).unwrap()),
        effect_state: BodyErrorEffectState::NoEffects,
        completeness: BodyErrorCompleteness::Complete,
        provenance_id: None,
    })));
    assert!(
        service()
            .validate_pure_planner_publication(Some(&request), refused.clone())
            .is_ok()
    );
    assert!(
        service()
            .validate_pure_planner_publication(None, refused)
            .is_err()
    );

    let error = decode_migration_request(serde_json::json!({
        "schema": 2,
        "source_text": "not a manifest"
    }))
    .unwrap_err();
    let preflight = envelope(Body::Error(Box::new(BodyError {
        format_id: Some(ERROR_FORMAT.to_owned()),
        payload_json: Some(serde_json::to_string(&error).unwrap()),
        effect_state: BodyErrorEffectState::NoEffects,
        completeness: BodyErrorCompleteness::Complete,
        provenance_id: None,
    })));
    assert!(
        service()
            .validate_pure_planner_publication(None, preflight.clone())
            .is_ok()
    );
    let unbound =
        MachineOperationService::new(Vec::new(), PurePlannerAdmission::Unavailable).unwrap();
    assert_eq!(
        unbound
            .validate_pure_planner_publication(None, preflight.clone())
            .unwrap_err()
            .original,
        preflight
    );
    assert!(
        service()
            .validate_pure_planner_publication(Some(&request), preflight)
            .is_err()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn forged_payload_effect_identity_and_nonresult_stage_retain_original() {
    let request = request();
    let valid = needs_input_result(&request);
    let mut candidates = Vec::new();
    let mut bad = valid.clone();
    bad.schema = 2;
    candidates.push(bad);
    let mut bad = valid.clone();
    bad.domain_owner = Some("rust:foreign:owner".to_owned());
    candidates.push(bad);
    let mut bad = valid.clone();
    if let Body::Result(row) = &mut bad.body {
        row.payload_json = Some("{invalid-json".to_owned());
    }
    candidates.push(bad);
    let mut bad = valid.clone();
    if let Body::Result(row) = &mut bad.body {
        row.effect_state = BodyResultEffectState::Uncertain;
    }
    candidates.push(bad);
    let mut bad = valid;
    if let Body::Result(row) = &mut bad.body {
        row.completeness = BodyResultCompleteness::Complete;
    }
    candidates.push(bad);
    for name in ["event", "prepared", "admission", "input"] {
        let fixture = format!(
            "{}/../../formats/corpora/machine-operation-envelope/e1/valid/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let other: OperationEnvelope =
            serde_json::from_str(&std::fs::read_to_string(fixture).unwrap()).unwrap();
        candidates.push(other);
    }
    for original in candidates {
        let failure = service()
            .validate_pure_planner_publication(Some(&request), original.clone())
            .unwrap_err();
        assert_eq!(failure.original, original);
    }
}
