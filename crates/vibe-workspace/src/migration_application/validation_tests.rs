specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE"
);

use super::identity::*;
use super::*;
use serde_json::{Value, json};
use vibe_wire::generated::migration::e1::{application_journal::*, operation_report::*};

const APPLY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/migration-operation-request/e1/valid/apply.json"
));
const JOURNAL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/migration-application-journal/e1/valid/prepared.json"
));
fn fixture() -> (tempfile::TempDir, OwnedManifestMigrationFile) {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("sentinel"), "unchanged").unwrap();
    let grant =
        OwnedManifestMigrationFile::new(&project, "vibe.toml", &tmp.path().join("store")).unwrap();
    (tmp, grant)
}
fn bound(value: &mut Value, grant: &OwnedManifestMigrationFile) {
    value["authority"]["project_root"] = json!(grant.project_spelling);
    value["authority"]["recovery_store_root"] = json!(grant.store_spelling);
}
fn request(grant: &OwnedManifestMigrationFile) -> Value {
    let mut v: Value = serde_json::from_str(APPLY).unwrap();
    bound(&mut v, grant);
    v
}
fn journal(grant: &OwnedManifestMigrationFile) -> ApplicationJournal {
    let mut v: Value = serde_json::from_str(JOURNAL).unwrap();
    bound(&mut v, grant);
    v["project_binding"] = json!(format!("sha256:{}", "1".repeat(64)));
    v["store_binding"] = json!(format!("sha256:{}", "2".repeat(64)));
    let mut j: ApplicationJournal = serde_json::from_value(v).unwrap();
    j.intent_id = intent_identity(&j);
    j
}
fn check(
    v: Value,
    grant: &OwnedManifestMigrationFile,
) -> Result<ValidatedOperation, ApplicationValidationError> {
    decode_operation_json(&v.to_string(), grant)
}
fn replan(v: &mut Value) {
    use crate::migration_planning::{MigrationPlanningResponse, plan_migration_json};
    use vibe_wire::generated::migration::e1::plan_report::Outcome;
    let MigrationPlanningResponse::Report(r) =
        plan_migration_json(v["approved_request_json"].as_str().unwrap())
    else {
        panic!()
    };
    let p = match r.outcome {
        Outcome::Planned(p) => p.plan,
        Outcome::NoOp(p) => p.plan,
        _ => panic!(),
    };
    v["before_content_id"] = json!(p.before_identity);
    v["after_content_id"] = json!(p.after_identity);
    v["diff_id"] = json!(p.diff_identity);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn malformed_and_null_preflight_create_nothing() {
    let (tmp, grant) = fixture();
    for text in ["{", "null", "{}"] {
        let e = decode_operation_json(text, &grant).unwrap_err();
        assert!(e.operation.is_none());
        assert!(e.operation_id.is_none());
        let report = ReportBuilder::preflight(&e);
        assert!(report.effects.is_empty());
        assert!(report.intent_id.is_none());
        validate_operation_report(&report).unwrap();
    }
    let mut v = request(&grant);
    v["before_content_id"] = Value::Null;
    let e = check(v, &grant).unwrap_err();
    assert_eq!(e.operation, Some(OperationReportOperation::Apply));
    assert_eq!(e.operation_id.as_deref(), Some("synthetic-operation"));
    assert!(!tmp.path().join("store").exists());
    assert_eq!(
        std::fs::read_dir(grant.project().root_path())
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        std::fs::read(grant.project().root_path().join("sentinel")).unwrap(),
        b"unchanged"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn planner_selections_and_refusals_are_not_apply_plans() {
    let (_tmp, grant) = fixture();
    let original = request(&grant);
    assert!(matches!(
        check(original.clone(), &grant).unwrap(),
        ValidatedOperation::Apply(_)
    ));
    for planning in [
        json!({"schema":1,"source_text":""}),
        json!({"schema":1,"source_text":"", "recipe":null}),
        json!({"schema":1,"source_text":"", "source_profile":"unknown","target_profile":"unknown","recipe":"unknown"}),
    ] {
        let mut v = original.clone();
        v["approved_request_json"] = json!(planning.to_string());
        assert!(matches!(
            check(v, &grant).unwrap_err().code,
            ApplicationErrorCode::PlanRefused | ApplicationErrorCode::InvalidRequest
        ));
    }
    let mut v = original;
    let mut p: Value = serde_json::from_str(v["approved_request_json"].as_str().unwrap()).unwrap();
    p["source_text"] = json!(
        p["source_text"]
            .as_str()
            .unwrap()
            .replace("snapshot", "copy")
    );
    p["source_profile"] = json!("materialization-current-absent-r1");
    p["recipe"] = json!("identity-r1");
    v["approved_request_json"] = json!(p.to_string());
    replan(&mut v);
    let ValidatedOperation::Apply(a) = check(v, &grant).unwrap() else {
        panic!()
    };
    assert!(a.is_no_op);
    assert_eq!(a.plan.before, a.plan.after);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn grant_mismatch_source_identity_and_frozen_effect_refuse() {
    let (_tmp, grant) = fixture();
    let original = request(&grant);
    for field in ["project_root", "recovery_store_root", "target_relative"] {
        let mut v = original.clone();
        v["authority"][field] = json!(if field == "target_relative" {
            "other.toml"
        } else {
            "/other"
        });
        assert_eq!(
            check(v, &grant).unwrap_err().code,
            ApplicationErrorCode::AuthorityRefused
        );
    }
    let mut v = original.clone();
    v["before_content_id"] = json!("0".repeat(64));
    assert_eq!(
        check(v, &grant).unwrap_err().code,
        ApplicationErrorCode::IntentMismatch
    );
    let mut v = original;
    let mut p: Value = serde_json::from_str(v["approved_request_json"].as_str().unwrap()).unwrap();
    p["source_text"] = json!(format!(
        "{}frozen=true\n",
        p["source_text"].as_str().unwrap()
    ));
    v["approved_request_json"] = json!(p.to_string());
    replan(&mut v);
    assert_eq!(
        check(v, &grant).unwrap_err().field,
        "plan.after.package.frozen"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn six_semantic_corpus_inputs_refuse_at_their_owners() {
    let (_tmp, grant) = fixture();
    for text in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/migration-operation-request/e1/semantic-invalid/unsupported-schema.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/migration-operation-request/e1/semantic-invalid/wrong-before-content-id.json"
        )),
    ] {
        let mut v: Value = serde_json::from_str(text).unwrap();
        bound(&mut v, &grant);
        assert!(check(v, &grant).is_err());
    }
    for text in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/migration-application-journal/e1/semantic-invalid/unsupported-protocol.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/migration-application-journal/e1/semantic-invalid/noncanonical-length.json"
        )),
    ] {
        let mut v: Value = serde_json::from_str(text).unwrap();
        bound(&mut v, &grant);
        v["project_binding"] = json!(format!("sha256:{}", "1".repeat(64)));
        v["store_binding"] = json!(format!("sha256:{}", "2".repeat(64)));
        let j: ApplicationJournal = serde_json::from_value(v).unwrap();
        let e = validate_journal(&j, &grant).unwrap_err();
        assert!(matches!(e.field, "schema/protocol_revision" | "length"));
    }
    for text in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/migration-operation-report/e1/semantic-invalid/wrong-schema.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/migration-operation-report/e1/semantic-invalid/claims-external-writers-excluded.json"
        )),
    ] {
        let r: OperationReport = serde_json::from_str(text).unwrap();
        assert!(validate_operation_report(&r).is_err());
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn corrupted_journal_fields_refuse_before_intent_admission() {
    let (_tmp, grant) = fixture();
    let j = journal(&grant);
    validate_journal(&j, &grant).unwrap();
    for field in [
        "intent_id",
        "before_raw_sha256",
        "after_raw_sha256",
        "before_content_id",
        "diff_id",
    ] {
        let mut v = serde_json::to_value(&j).unwrap();
        v[field] = json!("0".repeat(64));
        assert!(
            decode_journal_json(&v.to_string(), &grant).is_err(),
            "{field}"
        );
    }
    for length in ["01", "-1", "18446744073709551616", "98"] {
        let mut bad = j.clone();
        bad.before_length = length.into();
        assert_eq!(validate_journal(&bad, &grant).unwrap_err().field, "length");
    }
    let mut bad = j.clone();
    bad.protocol_revision = 2;
    assert!(validate_journal(&bad, &grant).is_err());
    let mut bad = j.clone();
    bad.unix_mode = Some(0o10000);
    assert_eq!(
        validate_journal(&bad, &grant).unwrap_err().field,
        "unix_mode"
    );
    let mut bad = j;
    bad.store_binding = "opaque-but-not-safefs".into();
    assert_eq!(
        validate_journal(&bad, &grant).unwrap_err().field,
        "bindings"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn required_nullable_and_unknown_nested_fields_remain_strict() {
    let (_tmp, grant) = fixture();
    let j = journal(&grant);
    let mut v = serde_json::to_value(&j).unwrap();
    v.as_object_mut().unwrap().remove("unix_mode");
    assert!(decode_journal_json(&v.to_string(), &grant).is_err());
    let mut j = j;
    j.unix_mode = None;
    j.intent_id = intent_identity(&j);
    validate_journal(&j, &grant).unwrap();
    let mut v = serde_json::to_value(&j).unwrap();
    v["authority"]["unknown"] = json!(true);
    assert!(decode_journal_json(&v.to_string(), &grant).is_err());
    let mut v = request(&grant);
    v["unknown"] = json!(true);
    assert!(check(v, &grant).is_err());
    let r = ReportBuilder::preflight(&decode_operation_json("null", &grant).unwrap_err());
    let mut v = serde_json::to_value(r).unwrap();
    v.as_object_mut().unwrap().remove("intent_id");
    assert!(serde_json::from_value::<OperationReport>(v).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn independent_framed_golden_and_phase_exclusion() {
    // Expected values independently computed with Python hashlib+struct.pack
    // >Q byte lengths. UTF-8 operation ID exposes character-length framing bugs.
    let mut j: ApplicationJournal = serde_json::from_str(JOURNAL).unwrap();
    j.operation_id = "op-é".into();
    j.project_binding = format!("sha256:{}", "1".repeat(64));
    j.store_binding = format!("sha256:{}", "2".repeat(64));
    j.authority.project_root = "/project".into();
    j.authority.recovery_store_root = "/store".into();
    j.approved_request_json = "{}".into();
    j.before_content_id = "a".repeat(64);
    j.after_content_id = "b".repeat(64);
    j.diff_id = "c".repeat(64);
    j.before_raw_sha256 = "d".repeat(64);
    j.after_raw_sha256 = "e".repeat(64);
    j.before_length = "12".into();
    j.after_length = "13".into();
    j.unix_mode = Some(420);
    let intent = intent_identity(&j);
    assert_eq!(
        intent,
        "f85f7c23defa42f590cd23a6ae87c0ab9cc283152f46854f34a38557a828c633"
    );
    assert_eq!(
        operation_storage_key(&j.project_binding, &j.operation_id),
        "e8c5452a8f8be12306b7a17b2b1289aa105b59481361f24bcd96bfc664c5f42b"
    );
    assert_eq!(
        transaction_stage_name(&intent, StageAction::Apply),
        ".vibe-stage-tx-f75e2dff55dd022de72fdb512ce82b53"
    );
    assert_eq!(
        transaction_stage_name(&intent, StageAction::Restore),
        ".vibe-stage-tx-0ea9d8a7f38e114fc286e34c0fcd6fc0"
    );
    for phase in [
        ApplicationJournalPhase::Publishing,
        ApplicationJournalPhase::Published,
        ApplicationJournalPhase::Restoring,
        ApplicationJournalPhase::Restored,
    ] {
        j.phase = phase;
        assert_eq!(intent_identity(&j), intent);
    }
    j.unix_mode = None;
    assert_ne!(intent_identity(&j), intent);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn report_errors_retain_observed_effects_without_fabricated_guarantees() {
    let mut b = ReportBuilder::new(Some(OperationReportOperation::Apply), Some("id".into()));
    assert_eq!(
        b.evidence_mut().guarantees.concurrency,
        GuaranteesConcurrency::Unestablished
    );
    b.evidence_mut().effects.push(Effect {
        kind: EffectKind::BeforeImage,
        state: EffectState::Completed,
    });
    b.evidence_mut().warnings.push(Warning {
        code: WarningCode::ResidueRetained,
        action: WarningAction::Inspect,
    });
    b.error(
        ApplicationError {
            code: ApplicationErrorCode::JournalIo,
            field: "journal".into(),
            reason: "retained evidence".into(),
        },
        OperationReportOutcome::Refused,
    );
    let r = b.into_report().unwrap();
    assert_eq!(r.effects.len(), 1);
    assert_eq!(r.warnings.len(), 1);
    let mut bad = r.clone();
    bad.guarantees.barrier_binding = Some("invented".into());
    assert!(validate_operation_report(&bad).is_err());
    let mut bad = r;
    bad.effects.push(Effect {
        kind: EffectKind::Journal,
        state: EffectState::Uncertain,
    });
    assert!(validate_operation_report(&bad).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn expected_stage_residue_is_reportable_without_namespace_authority() {
    let mut r = ReportBuilder::new(Some(OperationReportOperation::Apply), Some("id".into())).report;
    r.intent_id = Some("a".repeat(64));
    let stage = transaction_stage_name(r.intent_id.as_ref().unwrap(), StageAction::Apply);
    r.residue.push(Residue {
        location: ResidueLocation::Project,
        ownership: ResidueOwnership::Unproven,
        relative_path: format!("docs/{stage}"),
    });
    validate_operation_report(&r).unwrap();
    r.residue[0].relative_path =
        transaction_stage_name(r.intent_id.as_ref().unwrap(), StageAction::Restore);
    validate_operation_report(&r).unwrap();
    for path in [
        format!(
            "docs/{}",
            transaction_stage_name(&"b".repeat(64), StageAction::Apply)
        ),
        format!("../{stage}"),
        format!("/{stage}"),
        format!(".vibe-stage-foreign/{stage}"),
        format!("docs/{stage}/other"),
        ".vibe-stage-arbitrary".into(),
    ] {
        r.residue[0].relative_path = path;
        assert!(validate_operation_report(&r).is_err());
    }
    r.residue[0].relative_path = stage;
    r.residue[0].location = ResidueLocation::Store;
    assert!(validate_operation_report(&r).is_err());
    r.residue[0].location = ResidueLocation::Project;
    r.intent_id = None;
    assert!(validate_operation_report(&r).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn no_op_replay_retains_only_coherent_completed_target_effects() {
    let mut r: OperationReport = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/migration-operation-report/e1/valid/completed.json"
    )))
    .unwrap();
    r.outcome = OperationReportOutcome::NoOp;
    validate_operation_report(&r).unwrap();
    for state in [EffectState::Pending, EffectState::Uncertain] {
        let mut bad = r.clone();
        bad.effects[0].state = state;
        assert!(validate_operation_report(&bad).is_err());
    }
    let mut bad = r.clone();
    bad.target_state = OperationReportTargetState::ApprovedBefore;
    assert!(validate_operation_report(&bad).is_err());
    let mut bad = r.clone();
    bad.intent_id = None;
    assert!(validate_operation_report(&bad).is_err());
    let mut bad = r.clone();
    bad.guarantees.concurrency = GuaranteesConcurrency::Unestablished;
    bad.guarantees.barrier_binding = None;
    assert!(validate_operation_report(&bad).is_err());
    r.effects[0].kind = EffectKind::TargetRestore;
    r.target_state = OperationReportTargetState::ApprovedBefore;
    validate_operation_report(&r).unwrap();
    r.target_state = OperationReportTargetState::ValidatedAfter;
    assert!(validate_operation_report(&r).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn recognized_target_bytes_require_all_planner_identities() {
    let r = ReportBuilder::new(None, None).report;
    validate_operation_report(&r).unwrap();
    for state in [
        OperationReportTargetState::ApprovedBefore,
        OperationReportTargetState::ValidatedAfter,
    ] {
        let mut bad = r.clone();
        bad.target_state = state;
        assert_eq!(
            validate_operation_report(&bad).unwrap_err().field,
            "identities"
        );
        bad.before_content_id = Some("a".repeat(64));
        bad.after_content_id = Some("b".repeat(64));
        bad.diff_id = Some("c".repeat(64));
        validate_operation_report(&bad).unwrap();
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn report_validation_failure_retains_all_observed_evidence() {
    let mut b = ReportBuilder::new(Some(OperationReportOperation::Apply), Some("id".into()));
    let r = b.evidence_mut();
    r.effects = vec![
        Effect {
            kind: EffectKind::BeforeImage,
            state: EffectState::Completed,
        },
        Effect {
            kind: EffectKind::Journal,
            state: EffectState::Uncertain,
        },
    ];
    r.warnings.push(Warning {
        code: WarningCode::PublicationUncertain,
        action: WarningAction::Inspect,
    });
    r.residue.push(Residue {
        location: ResidueLocation::Store,
        ownership: ResidueOwnership::Proved,
        relative_path: "migrations/retained/before.bin".into(),
    });
    b.error(
        ApplicationError {
            code: ApplicationErrorCode::JournalIo,
            field: "journal".into(),
            reason: "publication uncertain".into(),
        },
        OperationReportOutcome::Refused,
    );
    let expected = b.evidence_mut().clone();
    let failure = b.into_report().unwrap_err();
    assert_eq!(failure.fault.field, "outcome");
    assert_eq!(failure.report, expected);
    assert_eq!(failure.report.effects.len(), 2);
    assert_eq!(failure.report.warnings.len(), 1);
    assert_eq!(failure.report.residue.len(), 1);
}
