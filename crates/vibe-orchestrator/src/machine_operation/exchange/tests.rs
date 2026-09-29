use super::*;
use crate::machine_operation::PurePlannerAdmission;
use serde_json::json;

fn service(admission: PurePlannerAdmission) -> MachineOperationService {
    MachineOperationService::pure_text_planner(admission).unwrap()
}
fn request_value() -> Value {
    json!({
        "schema": 1,
        "source_text": "# café\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.1.0'\r\nmaterialization='snapshot' # keep\r\n",
        "source_profile": "materialization-legacy-absent-r1",
        "target_profile": "materialization-current-absent-r1",
        "recipe": "snapshot-to-copy-r1"
    })
}
fn input(value: Value) -> OperationEnvelope {
    envelope(Body::Input(Box::new(
        vibe_wire::generated::machine::e1::operation_envelope::BodyInput {
            format_id: INPUT_FORMAT.to_owned(),
            payload_json: value.to_string(),
        },
    )))
}
fn typed_report(value: &OperationEnvelope) -> PlanReport {
    let payload = match &value.body {
        Body::Result(body) => {
            assert_eq!(body.effect_state, BodyResultEffectState::NoEffects);
            body.payload_json.as_deref().unwrap()
        }
        Body::Error(body) => {
            assert_eq!(body.effect_state, BodyErrorEffectState::NoEffects);
            body.payload_json.as_deref().unwrap()
        }
        _ => panic!("planner publication was neither Result nor Error"),
    };
    serde_json::from_str(payload).unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE",
    r = 1
)]
fn direct_exchange_matches_owner_planner_without_effects() {
    let service = service(PurePlannerAdmission::Admitted);
    let value = request_value();
    let original = input(value.clone());
    let PurePlannerExchange::Published { status, envelope } =
        service.exchange_pure_planner(original.clone())
    else {
        panic!("admitted pure planner failed to publish")
    };
    assert_eq!(status, PublishedPlannerStatus::Success);
    assert_eq!(envelope.operation_id.as_deref(), Some(PLANNER_ID));
    assert_eq!(envelope.evidence_id, None);
    assert_eq!(
        typed_report(&envelope),
        match vibe_workspace::migration_planning::plan_migration_value(value.clone()) {
            MigrationPlanningResponse::Report(report) => report,
            _ => panic!("owner did not return the expected report"),
        }
    );
    let request = decode_migration_request(value).unwrap();
    service
        .validate_pure_planner_publication(Some(&request), envelope)
        .unwrap();
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn embedded_bad_optional_unknown_and_malformed_json_stay_typed_preflight() {
    let service = service(PurePlannerAdmission::Admitted);
    let mut wrong_optional = request_value();
    wrong_optional["source_profile"] = Value::Null;
    let mut unknown = request_value();
    unknown["private_path"] = json!("PRIVATE_SENTINEL");
    for value in [wrong_optional, unknown] {
        let original = input(value);
        let PurePlannerExchange::Preflight {
            cause,
            envelope,
            original_input,
        } = service.exchange_pure_planner(original.clone())
        else {
            panic!("presence or unknown field bypassed preflight")
        };
        assert_eq!(cause, ExchangePreflightCause::EmbeddedRequest);
        assert_eq!(original_input, original);
        assert_eq!(envelope.operation_id.as_deref(), Some(PLANNER_ID));
        let Body::Error(body) = envelope.body else {
            panic!("preflight lost Error arm")
        };
        assert_eq!(body.effect_state, BodyErrorEffectState::NoEffects);
        let error: PlanError = serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
        validate_preflight_error(&error).unwrap();
    }
    let mut malformed = input(request_value());
    let Body::Input(body) = &mut malformed.body else {
        unreachable!()
    };
    body.payload_json = "{broken-json".to_owned();
    assert!(matches!(
        service.exchange_pure_planner(malformed),
        PurePlannerExchange::Preflight {
            cause: ExchangePreflightCause::EmbeddedRequest,
            ..
        }
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn outer_route_arm_and_generated_unknown_field_refuse_before_planning() {
    let service = service(PurePlannerAdmission::Admitted);
    let mut wrong_route = input(request_value());
    wrong_route.domain_owner = Some("rust:foreign:owner".to_owned());
    let mut wrong_arm = input(request_value());
    wrong_arm.body = Body::Result(Box::new(BodyResult {
        format_id: None,
        payload_json: None,
        effect_state: BodyResultEffectState::NoEffects,
        completeness: BodyResultCompleteness::Unavailable,
        provenance_id: None,
    }));
    for original in [wrong_route, wrong_arm] {
        let PurePlannerExchange::Preflight {
            cause,
            envelope,
            original_input,
        } = service.exchange_pure_planner(original.clone())
        else {
            panic!("foreign outer route or arm was admitted")
        };
        assert_eq!(cause, ExchangePreflightCause::OuterRoute);
        assert_eq!(original_input, original);
        assert!(envelope.operation_id.is_none());
        assert!(matches!(envelope.body, Body::Error(_)));
    }
    let mut raw = serde_json::to_value(input(request_value())).unwrap();
    raw["unexpected"] = json!(true);
    assert!(serde_json::from_value::<OperationEnvelope>(raw).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE",
    r = 1
)]
fn questions_refusal_unavailable_and_unvalidated_publication_keep_channels() {
    let admitted = service(PurePlannerAdmission::Admitted);
    let needs = input(json!({"schema":1,"source_text":request_value()["source_text"]}));
    let PurePlannerExchange::Published { status, envelope } = admitted.exchange_pure_planner(needs)
    else {
        panic!("questions became an error")
    };
    assert_eq!(status, PublishedPlannerStatus::NeedsInput);
    assert!(matches!(
        typed_report(&envelope).outcome,
        Outcome::NeedsInput(_)
    ));
    let mut unknown_profile = request_value();
    unknown_profile["source_profile"] = json!("unknown-r2");
    let refused = input(unknown_profile);
    let PurePlannerExchange::Published { status, envelope } =
        admitted.exchange_pure_planner(refused)
    else {
        panic!("domain refusal lost its published Error channel")
    };
    assert_eq!(status, PublishedPlannerStatus::Refused);
    assert!(matches!(envelope.body, Body::Error(_)));
    assert!(matches!(
        typed_report(&envelope).outcome,
        Outcome::Refused(_)
    ));
    let original = input(request_value());
    assert!(matches!(
        service(PurePlannerAdmission::Unavailable).exchange_pure_planner(original.clone()),
        PurePlannerExchange::Unavailable { original_input, .. } if original_input == original
    ));
    let request = decode_migration_request(request_value()).unwrap();
    let MigrationPlanningResponse::Report(mut forged) =
        vibe_workspace::migration_planning::plan_migration_request(request.clone())
    else {
        panic!("fixture lost report")
    };
    forged.schema = 2;
    let PurePlannerExchange::Diagnostic {
        original_input,
        evidence,
        ..
    } = publish_report(&admitted, original.clone(), &request, forged)
    else {
        panic!("invalid owner report was published normally")
    };
    assert_eq!(original_input, original);
    assert!(matches!(
        evidence,
        ExchangeDiagnosticEvidence::UnvalidatedPublication(_)
    ));
}
