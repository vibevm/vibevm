//! Isolated real MCP dispatch for the one pure machine-envelope route.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION");
use super::*;
use crate::dispatch_one;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_wire::generated::migration::e1::plan_report::PlanReport;
use vibe_workspace::migration_planning::validate_report_for_request;

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let key = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                out.insert(key, None);
                walk(root, &path, out);
            } else {
                out.insert(key, Some(std::fs::read(path).unwrap()));
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}
fn context(root: &Path) -> ServerContext {
    ServerContext::with_store_root(root.join("absent-project"), root.join("absent-store"))
}
fn admitted_service() -> MachineOperationService {
    MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted).unwrap()
}
fn input(inner: Value) -> Value {
    json!({"schema":1,"operation_id":OPERATION,"domain_owner":OPERATION,
        "source_id":SOURCE,"evidence_id":null,"body":{"stage":"input",
            "format_id":INPUT,"payload_json":inner.to_string()}})
}
fn request() -> Value {
    json!({"schema":1,"source_text":"# café\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.1.0'\r\nmaterialization='snapshot' # keep\r\n",
        "source_profile":"materialization-legacy-absent-r1",
        "target_profile":"materialization-current-absent-r1",
        "recipe":"snapshot-to-copy-r1"})
}
fn call(root: &Path, args: Value) -> Value {
    let before = snapshot(root);
    let wire = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"plan_manifest_migration_envelope","arguments":args}});
    let response: Value =
        serde_json::from_str(&dispatch_one(context(root), &wire.to_string()).unwrap()).unwrap();
    assert!(
        response["error"].is_null(),
        "generic dispatcher refused: {response}"
    );
    assert_eq!(snapshot(root), before, "pure route altered fixture state");
    for name in [
        "absent-project",
        "absent-store",
        ".vibe",
        "vibe.toml",
        "vibe.lock",
    ] {
        assert!(!root.join(name).exists(), "pure route created {name}");
    }
    response["result"].clone()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE",
    r = 1
)]
fn pure_planner_envelope_matches_owner_over_real_dispatch_without_state() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("keep.txt"), b"unchanged sentinel").unwrap();
    let args = request();
    let result = call(root.path(), input(args.clone()));
    assert_eq!(result["isError"], false);
    let envelope: OperationEnvelope =
        serde_json::from_value(result["structuredContent"].clone()).unwrap();
    assert_eq!(envelope.operation_id.as_deref(), Some(OPERATION));
    let Body::Result(body) = envelope.body else {
        panic!("pure result arm absent")
    };
    assert_eq!(body.effect_state, BodyResultEffectState::NoEffects);
    assert_eq!(body.completeness, BodyResultCompleteness::Complete);
    let actual: PlanReport = serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
    let MigrationPlanningResponse::Report(expected) =
        vibe_workspace::migration_planning::plan_migration_value(args)
    else {
        panic!("fixture did not reach planner")
    };
    assert_eq!(actual, expected);
    assert!(matches!(actual.outcome, Outcome::Planned(_)));
    assert_eq!(
        std::fs::read(root.path().join("keep.txt")).unwrap(),
        b"unchanged sentinel"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn malformed_outer_and_embedded_optional_null_refuse_before_service() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("keep.txt"), b"unchanged sentinel").unwrap();
    let mut variants = Vec::new();
    let mut extra = input(request());
    extra["extra"] = json!(true);
    variants.push(extra);
    let mut wrong_epoch = input(request());
    wrong_epoch["schema"] = json!(2);
    variants.push(wrong_epoch);
    let mut wrong_owner = input(request());
    wrong_owner["domain_owner"] = json!("rust:other:owner");
    variants.push(wrong_owner);
    let mut wrong_format = input(request());
    wrong_format["body"]["format_id"] = json!("migration-plan-report");
    variants.push(wrong_format);
    let mut wrong_arm = input(request());
    wrong_arm["body"]["stage"] = json!("prepared");
    variants.push(wrong_arm);
    for args in variants {
        assert!(matches!(
            PlanManifestMigrationEnvelopeMcpTool.run(&args, &context(root.path())),
            Err(ToolError::StructuredPreflight { .. })
        ));
        let response = call(root.path(), args);
        assert_eq!(response["isError"], true);
        let envelope: OperationEnvelope =
            serde_json::from_value(response["structuredContent"].clone()).unwrap();
        assert!(matches!(envelope.body, Body::Error(_)));
        assert!(envelope.operation_id.is_none());
        assert!(envelope.domain_owner.is_none());
        assert!(envelope.source_id.is_none());
    }
    let mut missing = request();
    missing["source_profile"] = Value::Null;
    let mut wrong = request();
    wrong["recipe"] = json!(7);
    let mut unknown = request();
    unknown["project"] = json!("PRIVATE_SENTINEL");
    for invalid in [missing, wrong, unknown] {
        let response = call(root.path(), input(invalid));
        assert_eq!(response["isError"], true);
        let envelope: OperationEnvelope =
            serde_json::from_value(response["structuredContent"].clone()).unwrap();
        let Body::Error(error) = envelope.body else {
            panic!("typed preflight error missing")
        };
        assert_eq!(envelope.operation_id.as_deref(), Some(OPERATION));
        assert_eq!(error.format_id.as_deref(), Some(ERROR));
        let parsed: PlanError =
            serde_json::from_str(error.payload_json.as_deref().unwrap()).unwrap();
        validate_preflight_error(&parsed).unwrap();
        assert_eq!(
            parsed.error.code,
            vibe_wire::generated::migration::e1::plan_error::MigrationPlanningErrorCode::InvalidRequest
        );
        assert!(
            !response["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("PRIVATE_SENTINEL")
        );
    }
    let mut malformed = input(request());
    malformed["body"]["payload_json"] = json!("{invalid-json");
    let response = call(root.path(), malformed);
    let envelope: OperationEnvelope =
        serde_json::from_value(response["structuredContent"].clone()).unwrap();
    let Body::Error(error) = envelope.body else {
        panic!("malformed embedded JSON lost preflight")
    };
    let parsed: PlanError = serde_json::from_str(error.payload_json.as_deref().unwrap()).unwrap();
    validate_preflight_error(&parsed).unwrap();
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn forged_owner_and_effect_are_diagnostic_not_published_as_reports() {
    let service = admitted_service();
    let request = decode_migration_request(request()).unwrap();
    let MigrationPlanningResponse::Report(report) =
        vibe_workspace::migration_planning::plan_migration_request(request.clone())
    else {
        panic!("expected report")
    };
    let valid = envelope(Body::Result(Box::new(BodyResult {
        format_id: Some(REPORT.to_owned()),
        payload_json: Some(serde_json::to_string(&report).unwrap()),
        effect_state: BodyResultEffectState::NoEffects,
        completeness: BodyResultCompleteness::Complete,
        provenance_id: None,
    })));
    assert!(
        service
            .validate_pure_planner_publication(Some(&request), valid.clone())
            .is_ok()
    );
    let mut foreign = valid.clone();
    foreign.domain_owner = Some("rust:foreign:owner".to_owned());
    let mut effect = valid;
    let Body::Result(row) = &mut effect.body else {
        unreachable!()
    };
    row.effect_state = BodyResultEffectState::Completed;
    for forged in [foreign, effect] {
        assert!(
            service
                .validate_pure_planner_publication(Some(&request), forged.clone())
                .is_err()
        );
        let output = publish(
            &service,
            forged,
            &request,
            false,
            "forged data must not publish",
        )
        .unwrap();
        assert!(output.is_error());
        assert!(
            !output.has_structured(),
            "invalid generated report escaped publication"
        );
        assert!(output.text().contains("UNVALIDATED"));
        assert!(
            output.text().contains("rust:foreign:owner")
                || output.text().contains("\"effect_state\":\"completed\"")
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn invalid_owning_preflight_retains_original_as_diagnostic_only() {
    let MigrationPlanningResponse::Preflight(mut original) = plan_migration_json("{bad-json")
    else {
        panic!("expected owning preflight error")
    };
    original.schema = 2;
    let ToolError::Internal(message) = invalid("invalid generated preflight", Some(&original))
    else {
        panic!("invalid owning error became a normal structured preflight")
    };
    assert!(message.contains("UNVALIDATED original error"));
    assert!(
        message.contains("\"schema\":2"),
        "original bytes were discarded"
    );
    assert!(!message.contains("normal report was produced"));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn real_questions_and_refusal_keep_owner_result_and_channel() {
    let root = tempfile::tempdir().unwrap();
    for (inner, status, is_error, stage) in [
        (
            json!({"schema":1,"source_text":request()["source_text"]}),
            "needs_input",
            false,
            "result",
        ),
        (
            {
                let mut bad = request();
                bad["source_profile"] = json!("unknown-r2");
                bad
            },
            "refused",
            true,
            "error",
        ),
    ] {
        let response = call(root.path(), input(inner.clone()));
        assert_eq!(response["isError"], is_error);
        let envelope: OperationEnvelope =
            serde_json::from_value(response["structuredContent"].clone()).unwrap();
        assert_eq!(response["structuredContent"]["body"]["stage"], stage);
        let payload = match envelope.body {
            Body::Result(body) => {
                assert_eq!(body.effect_state, BodyResultEffectState::NoEffects);
                assert_eq!(body.completeness, BodyResultCompleteness::Partial);
                body.payload_json.unwrap()
            }
            Body::Error(body) => {
                assert_eq!(body.effect_state, BodyErrorEffectState::NoEffects);
                assert_eq!(body.completeness, BodyErrorCompleteness::Complete);
                assert_eq!(body.format_id.as_deref(), Some(REPORT));
                body.payload_json.unwrap()
            }
            _ => panic!("planner outcome was not a result or refusal"),
        };
        let report: PlanReport = serde_json::from_str(&payload).unwrap();
        assert_eq!(
            serde_json::to_value(&report).unwrap()["outcome"]["status"],
            status
        );
        let request = decode_migration_request(inner).unwrap();
        validate_report_for_request(&request, &report).unwrap();
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn input_arm_unknown_field_and_missing_nullable_refuse_over_dispatch() {
    let root = tempfile::tempdir().unwrap();
    for source in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/machine-operation-envelope/e1/structural-invalid/unknown-input-arm-field.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/machine-operation-envelope/e1/structural-invalid/missing-input-evidence-id.json"
        )),
    ] {
        let args: Value = serde_json::from_str(source).unwrap();
        assert!(matches!(
            PlanManifestMigrationEnvelopeMcpTool.run(&args, &context(root.path())),
            Err(ToolError::StructuredPreflight { .. })
        ));
        let result = call(root.path(), args);
        assert_eq!(result["isError"], true);
        let error: OperationEnvelope =
            serde_json::from_value(result["structuredContent"].clone()).unwrap();
        assert!(matches!(error.body, Body::Error(_)));
        assert!(error.operation_id.is_none());
        assert!(error.domain_owner.is_none());
        assert!(error.source_id.is_none());
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn malformed_result_and_unowned_event_remain_diagnostic_only() {
    let service = admitted_service();
    let request = decode_migration_request(request()).unwrap();
    let bad_result = envelope(Body::Result(Box::new(BodyResult {
        format_id: Some(REPORT.to_owned()),
        payload_json: Some("{invalid-domain-json".to_owned()),
        effect_state: BodyResultEffectState::NoEffects,
        completeness: BodyResultCompleteness::Complete,
        provenance_id: None,
    })));
    let event: OperationEnvelope = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-operation-envelope/e1/valid/event.json"
    )))
    .unwrap();
    for (untrusted, evidence) in [
        (bad_result, "{invalid-domain-json"),
        (event, "\"stage\":\"event\""),
    ] {
        let output = publish(&service, untrusted, &request, false, "must not publish").unwrap();
        assert!(output.is_error());
        assert!(!output.has_structured());
        assert!(output.text().contains("UNVALIDATED"));
        assert!(output.text().contains(evidence));
    }
}
