//! Synthetic integration inputs and the independently fixed C1 golden vector.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");

use super::*;
use serde_json::json;
use vibe_core::manifest::{
    CURRENT_ABSENT, CURRENT_EPOCH1, IDENTITY, LEGACY_ABSENT, SNAPSHOT_TO_COPY,
};

#[path = "tests_apply.rs"]
mod tests_apply;

const BEFORE: &str = "[package]\nname=\"demo\"\ngroup=\"org.example\"\nkind=\"flow\"\nversion=\"1.1.0\"\nmaterialization=\"snapshot\"\n";
const AFTER: &str = "[package]\nname=\"demo\"\ngroup=\"org.example\"\nkind=\"flow\"\nversion=\"1.1.0\"\nmaterialization=\"copy\"\n";
fn sample_request() -> PlanRequest {
    PlanRequest {
        schema: 1,
        source_text: BEFORE.to_owned(),
        source_profile: Some(LEGACY_ABSENT.to_owned()),
        target_profile: Some(CURRENT_ABSENT.to_owned()),
        recipe: Some(SNAPSHOT_TO_COPY.to_owned()),
        source_decisions: Vec::new(),
    }
}

fn fixture(family: &str, category: &str, name: &str) -> Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora")
        .join(family)
        .join("e1")
        .join(category)
        .join(format!("{name}.json"));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn source_decision_wire_reaches_the_explicit_boot_migration_producer() {
    let old = sample_request();
    assert!(
        serde_json::to_value(&old)
            .unwrap()
            .get("source_decisions")
            .is_none()
    );
    let value = fixture("migration-plan-request", "valid", "source-decision");
    let request = decode_migration_request(value).unwrap();
    assert_eq!(request.source_decisions.len(), 1);
    assert_eq!(
        request.source_decisions[0].file,
        "vibevm/vibespecs/boot/10-flow.xml"
    );
    let planned = report(request.clone());
    let output = plan(&planned);
    assert!(output.after.starts_with(&request.source_text));
    assert_eq!(output.edits.len(), 1);
    validate_report_for_request(&request, &planned).unwrap();
    let mut undecided = request;
    undecided.source_decisions.clear();
    let pending = report(undecided.clone());
    let Outcome::NeedsInput(row) = &pending.outcome else {
        panic!("media decision required")
    };
    assert_eq!(row.questions.len(), 1);
    assert_eq!(row.questions[0].field, QuestionField::SourceMedia);
    assert_eq!(
        row.questions[0].resource_path.as_deref(),
        Some("vibevm/vibespecs/boot/10-flow.xml")
    );
    validate_report_for_request(&undecided, &pending).unwrap();
    for name in [
        "source-path-escape",
        "source-duplicate",
        "source-media-mismatch",
    ] {
        preflight_response(
            plan_migration_value(fixture("migration-plan-request", "semantic-invalid", name)),
            Code::InvalidRequest,
        );
    }
    preflight_response(
        plan_migration_value(fixture(
            "migration-plan-request",
            "structural-invalid",
            "source-unknown-interpretation",
        )),
        Code::InvalidRequest,
    );
    preflight_response(
        plan_migration_value(fixture(
            "migration-plan-request",
            "presence-invalid",
            "source-decisions-null",
        )),
        Code::InvalidRequest,
    );
}

#[test]
fn path_specific_questions_require_resource_and_closed_choice_vocabulary() {
    let value = fixture("migration-plan-report", "valid", "needs-input-source");
    let report: PlanReport = serde_json::from_value(value).unwrap();
    validate_report(&report).unwrap();
    let Outcome::NeedsInput(row) = &report.outcome else {
        panic!("source question")
    };
    assert_eq!(row.questions[0].field, QuestionField::SourceMedia);
    assert_eq!(
        row.questions[0].resource_path.as_deref(),
        Some("vibevm/vibespecs/boot/10-flow.xml")
    );
    for name in [
        "source-question-missing-path",
        "global-question-with-path",
        "source-question-duplicate",
        "source-question-path-escape",
    ] {
        let report: PlanReport =
            serde_json::from_value(fixture("migration-plan-report", "semantic-invalid", name))
                .unwrap();
        assert!(validate_report(&report).is_err(), "{name}");
    }
    let mut unknown = report.clone();
    let Outcome::NeedsInput(row) = &mut unknown.outcome else {
        unreachable!()
    };
    row.questions[0].choices = vec!["text/plain;not-media".into()];
    assert!(validate_report(&unknown).is_err());
}
fn report(request: PlanRequest) -> PlanReport {
    match plan_migration_request(request) {
        MigrationPlanningResponse::Report(report) => report,
        other => panic!("expected report, got {other:?}"),
    }
}
fn plan(report: &PlanReport) -> &Plan {
    match &report.outcome {
        Outcome::Planned(row) => &row.plan,
        Outcome::NoOp(row) => &row.plan,
        other => panic!("expected plan, got {other:?}"),
    }
}
fn plan_mut(report: &mut PlanReport) -> &mut Plan {
    match &mut report.outcome {
        Outcome::Planned(row) => &mut row.plan,
        Outcome::NoOp(row) => &mut row.plan,
        other => panic!("expected plan, got {other:?}"),
    }
}
fn preflight_response(response: MigrationPlanningResponse, code: Code) {
    match response {
        MigrationPlanningResponse::Preflight(error) => {
            assert_eq!(error.schema, 1);
            assert_eq!(error.stage, PlanErrorStage::Preflight);
            assert_eq!(error.error.code, code);
            validate_preflight_error(&error).unwrap();
        }
        other => panic!("expected preflight, got {other:?}"),
    }
}

#[test]
fn replay_validator_accepts_bounded_insertions_but_not_duplicate_or_empty_edits() {
    let mut result = report(sample_request());
    let plan = plan_mut(&mut result);
    let original = plan.before.clone();
    let end = original.len().to_string();
    plan.edits = vec![
        TokenEdit {
            field: "package.sources".into(),
            before_start: "0".into(),
            before_end: "0".into(),
            before_token: String::new(),
            after_token: "# declared\r\n".into(),
        },
        TokenEdit {
            field: "package.sources".into(),
            before_start: end.clone(),
            before_end: end,
            before_token: String::new(),
            after_token: "# retained\r\n".into(),
        },
    ];
    plan.after = format!("# declared\r\n{original}# retained\r\n");
    assert!(validate_plan(plan, false).is_ok());

    let mut duplicate = plan.clone();
    duplicate.edits[1] = duplicate.edits[0].clone();
    assert!(validate_plan(&duplicate, false).is_err());

    let mut empty = plan.clone();
    empty.edits[0].after_token.clear();
    assert!(validate_plan(&empty, false).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn golden_projection_preserves_c1_text_spans_and_identities() {
    let request = sample_request();
    let report = report(request.clone());
    let plan = plan(&report);
    assert_eq!(report.schema, 1);
    assert_eq!(report.stage, PlanReportStage::Planner);
    assert_eq!(plan.before, BEFORE);
    assert_eq!(plan.after, AFTER);
    assert_eq!(plan.edits[0].before_start, "86");
    assert_eq!(plan.edits[0].before_end, "96");
    assert_eq!(plan.edits[0].before_token, "\"snapshot\"");
    assert_eq!(plan.edits[0].after_token, "\"copy\"");
    assert_eq!(
        plan.before_identity,
        "905cf0b4eb7376819f47c9f7da02fecb8920d91dd9b76b4cb3454da77fa81887"
    );
    assert_eq!(
        plan.after_identity,
        "6798d562a0ff899a5afd793054821fc6a114f8cd55fc53118d7772e9167a44df"
    );
    assert_eq!(
        plan.diff_identity,
        "42faf5376728e4649c80657e6b0f93e9910a107261413a5d0c7f019bf5ab27e0"
    );
    assert_eq!(plan.source_epoch, Epoch::Absent(Box::new(EpochAbsent {})));
    validate_report_for_request(&request, &report).unwrap();
    let value = serde_json::to_value(MigrationPlanningResponse::Report(report.clone())).unwrap();
    assert_eq!(value["stage"], "planner");
    assert!(value.get("Report").is_none());
    assert_eq!(value["outcome"]["status"], "planned");
    assert!(value["outcome"]["plan"].get("checked_manifest").is_none());
}

#[test]
fn no_op_and_epoch_one_preserve_separate_profile_provenance() {
    let mut request = sample_request();
    request.source_text = AFTER.replace("[package]\n", "[package]\nepoch=1\n");
    request.source_profile = Some(CURRENT_EPOCH1.to_owned());
    request.target_profile = request.source_profile.clone();
    request.recipe = Some(IDENTITY.to_owned());
    let report = report(request.clone());
    assert!(matches!(report.outcome, Outcome::NoOp(_)));
    let plan = plan(&report);
    assert_eq!(plan.before, request.source_text);
    assert_eq!(plan.before, plan.after);
    assert!(plan.edits.is_empty());
    assert_eq!(plan.before_identity, plan.after_identity);
    assert_eq!(plan.source_epoch, Epoch::Epoch(Box::new(EpochEpoch {})));
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(
        value["outcome"]["plan"]["source_epoch"],
        json!({"kind":"epoch_1"})
    );
    validate_report_for_request(&request, &report).unwrap();
}

#[test]
fn omitted_choices_preserve_ordered_questions_without_defaults() {
    let value = json!({"schema":1,"source_text":BEFORE});
    let request = decode_migration_request(value.clone()).unwrap();
    assert_eq!(request.source_profile, None);
    let result = plan_migration_value(value);
    let expected = report(request.clone());
    assert_eq!(result, MigrationPlanningResponse::Report(expected.clone()));
    let Outcome::NeedsInput(row) = &expected.outcome else {
        panic!("questions required")
    };
    assert_eq!(
        row.questions
            .iter()
            .map(|q| q.field.clone())
            .collect::<Vec<_>>(),
        vec![
            QuestionField::SourceProfile,
            QuestionField::TargetProfile,
            QuestionField::Recipe
        ]
    );
    assert_eq!(
        row.questions[0].choices,
        vec![LEGACY_ABSENT, CURRENT_ABSENT, CURRENT_EPOCH1]
    );
    assert_eq!(row.questions[2].choices, vec![SNAPSHOT_TO_COPY, IDENTITY]);
    validate_report_for_request(&request, &expected).unwrap();
    let mut singleton = sample_request();
    singleton.recipe = None;
    let result = report(singleton);
    let Outcome::NeedsInput(row) = result.outcome else {
        panic!("recipe question")
    };
    assert_eq!(row.questions.len(), 1);
    assert_eq!(row.questions[0].field, QuestionField::Recipe);
    assert_eq!(row.questions[0].choices, vec![SNAPSHOT_TO_COPY]);
}

#[test]
fn malformed_unknown_wrong_types_and_null_are_preflight() {
    preflight_response(plan_migration_json("{"), Code::InvalidRequest);
    for value in [
        json!(null),
        json!([]),
        json!({"schema":1}),
        json!({"schema":"1","source_text":BEFORE}),
        json!({"schema":1,"source_text":false}),
        json!({"schema":1,"source_text":BEFORE,"path":"secret"}),
    ] {
        preflight_response(plan_migration_value(value), Code::InvalidRequest);
    }
    for field in ["source_profile", "target_profile", "recipe"] {
        for bad in [Value::Null, json!(3), json!({}), json!([])] {
            let mut value = json!({"schema":1,"source_text":BEFORE});
            value[field] = bad;
            preflight_response(plan_migration_value(value), Code::InvalidRequest);
        }
    }
    preflight_response(
        plan_migration_value(json!({"schema":2,"source_text":BEFORE})),
        Code::UnsupportedSchema,
    );
    let mut typed = sample_request();
    typed.schema = 2;
    preflight_response(plan_migration_request(typed), Code::UnsupportedSchema);
    // Raw serde accepts null as None; the owning presence guard must not.
    let raw: PlanRequest =
        serde_json::from_value(json!({"schema":1,"source_text":BEFORE,"recipe":null})).unwrap();
    assert_eq!(raw.recipe, None);
}

#[test]
fn domain_refusal_remains_planner_stage_and_fixed_diagnosis() {
    for (change, code) in [
        ("unknown", Code::UnknownSelection),
        ("malformed", Code::MalformedSyntax),
    ] {
        let mut request = sample_request();
        if change == "unknown" {
            request.source_profile = Some("unknown-r9".to_owned());
        } else {
            request.source_text = "[package private-parser-detail".to_owned();
        }
        let report = report(request.clone());
        let Outcome::Refused(row) = &report.outcome else {
            panic!("domain refusal")
        };
        assert_eq!(row.error.code, code);
        assert!(!row.error.reason.contains("private-parser-detail"));
        assert_eq!(report.stage, PlanReportStage::Planner);
        validate_report_for_request(&request, &report).unwrap();
    }
}

#[test]
fn utf8_crlf_and_source_immutability_survive_projection() {
    let mut request = sample_request();
    request.source_text = ("# café 東京\n".to_owned() + BEFORE).replace('\n', "\r\n");
    let original = request.clone();
    let report = report(request.clone());
    let plan = plan(&report);
    assert_eq!(request, original);
    assert_eq!(plan.before, original.source_text);
    assert_eq!(
        plan.after,
        original.source_text.replacen("\"snapshot\"", "\"copy\"", 1)
    );
    let offset = plan.edits[0].before_start.parse::<usize>().unwrap();
    assert_eq!(&plan.before[offset..offset + 10], "\"snapshot\"");
    validate_report_for_request(&request, &report).unwrap();
}

#[test]
fn valid_json_value_typed_and_external_replay_agree() {
    let request = sample_request();
    let value = serde_json::to_value(&request).unwrap();
    let text = serde_json::to_string(&value).unwrap();
    let expected = plan_migration_request(request.clone());
    assert_eq!(plan_migration_value(value), expected);
    assert_eq!(plan_migration_json(&text), expected);
    let MigrationPlanningResponse::Report(report) = expected else {
        panic!("report")
    };
    let restored: PlanReport =
        serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    validate_report_for_request(&request, &restored).unwrap();
}

#[test]
fn externally_tampered_reports_are_refused_without_second_hash_recipe() {
    let request = sample_request();
    let original = report(request.clone());
    for variant in 0..10 {
        let mut report = original.clone();
        match variant {
            0 => plan_mut(&mut report).diff_identity = "0".repeat(64),
            1 => plan_mut(&mut report).after.push('!'),
            2 => plan_mut(&mut report).source_epoch = Epoch::Epoch(Box::new(EpochEpoch {})),
            3 => plan_mut(&mut report).edits[0].before_start = "086".to_owned(),
            4 => {
                plan_mut(&mut report).edits[0].before_end =
                    "999999999999999999999999999999".to_owned()
            }
            5 => plan_mut(&mut report).edits[0].before_token = "wrong".to_owned(),
            6 => {
                let second = plan(&report).edits[0].clone();
                plan_mut(&mut report).edits.push(second);
            }
            7 => plan_mut(&mut report).edits[0].field = "package.version".to_owned(),
            8 => report.schema = 2,
            _ => {
                let plan = plan(&report).clone();
                report.outcome = Outcome::NoOp(Box::new(OutcomeNoOp { plan }));
            }
        }
        assert!(
            validate_report_for_request(&request, &report).is_err(),
            "variant {variant}"
        );
    }
    let mut unicode = request.clone();
    unicode.source_text = "# café\n".to_owned() + BEFORE;
    let mut boundary = report(unicode.clone());
    plan_mut(&mut boundary).edits[0].before_start = "6".to_owned();
    assert!(validate_report_for_request(&unicode, &boundary).is_err());
    let mut missing = request.clone();
    missing.recipe = None;
    let mut questions = report(missing.clone());
    let Outcome::NeedsInput(row) = &mut questions.outcome else {
        panic!("questions")
    };
    row.questions[0].choices[0] = "invented-choice".to_owned();
    assert!(validate_report_for_request(&missing, &questions).is_err());
    let mut unknown = request.clone();
    unknown.source_profile = Some("unknown".to_owned());
    let mut refusal = report(unknown.clone());
    let Outcome::Refused(row) = &mut refusal.outcome else {
        panic!("refusal")
    };
    row.error.reason = "invented reason".to_owned();
    assert!(validate_report_for_request(&unknown, &refusal).is_err());
}

#[test]
fn preflight_roots_keep_three_codes_and_serialize_only_inner_document() {
    let error = input_read_error();
    validate_preflight_error(&error).unwrap();
    assert_eq!(error.error.code, Code::InputRead);
    let value = serde_json::to_value(MigrationPlanningResponse::Preflight(error.clone())).unwrap();
    assert_eq!(value["stage"], "preflight");
    assert_eq!(value["error"]["code"], "input_read");
    assert!(value.get("Preflight").is_none());
    for code in [
        Code::InvalidRequest,
        Code::UnsupportedSchema,
        Code::InputRead,
    ] {
        validate_preflight_error(&preflight(code, "request", "fixed")).unwrap();
    }
    let mut wrong = error.clone();
    wrong.schema = 2;
    assert!(validate_preflight_error(&wrong).is_err());
    for code in [
        Code::UnknownSelection,
        Code::IncompatibleEdge,
        Code::MalformedSyntax,
        Code::UnrecognizedShape,
        Code::InvalidCurrentManifest,
        Code::UnprovableEdit,
    ] {
        let mut wrong = error.clone();
        wrong.error.code = code;
        assert!(validate_preflight_error(&wrong).is_err());
    }
    let report = PlanReport {
        schema: 1,
        stage: PlanReportStage::Planner,
        outcome: Outcome::Refused(Box::new(OutcomeRefused { error: error.error })),
    };
    assert!(validate_report(&report).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn retained_presence_and_semantic_invalid_corpora_refuse() {
    // These retained wire fixtures are synthetic/golden inputs, not historical
    // authored manifests. Filesystem access belongs only to this test.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formats/corpora");
    let read = |relative: &str| {
        std::fs::read_to_string(root.join(relative)).expect("retained corpus fixture")
    };
    for name in ["recipe-null", "source-profile-null", "target-profile-null"] {
        let text = read(&format!(
            "migration-plan-request/e1/presence-invalid/{name}.json"
        ));
        let _: PlanRequest = serde_json::from_str(&text).expect("raw Option accepts null");
        preflight_response(plan_migration_json(&text), Code::InvalidRequest);
    }
    let text = read("migration-plan-request/e1/semantic-invalid/unsupported-schema.json");
    let _: PlanRequest = serde_json::from_str(&text).expect("uint32 accepts schema2 shape");
    preflight_response(plan_migration_json(&text), Code::UnsupportedSchema);
    let request = decode_migration_request(
        serde_json::from_str(&read(
            "migration-plan-request/e1/valid/legacy-selection.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(request.source_text, BEFORE);
    let expected = report(request.clone());
    validate_report_for_request(&request, &expected).unwrap();
    for name in [
        "unsupported-schema",
        "preflight-code-in-planner",
        "noncanonical-offset",
        "offset-out-of-bounds",
        "profile-epoch-mismatch",
    ] {
        let text = read(&format!(
            "migration-plan-report/e1/semantic-invalid/{name}.json"
        ));
        let report: PlanReport =
            serde_json::from_str(&text).expect("semantic-invalid report retains generated shape");
        assert!(
            validate_report_for_request(&request, &report).is_err(),
            "{name}"
        );
    }
    let error: PlanError = serde_json::from_str(&read(
        "migration-plan-error/e1/semantic-invalid/planner-code-in-preflight.json",
    ))
    .expect("shared code enum retains generated shape");
    assert!(validate_preflight_error(&error).is_err());
}
