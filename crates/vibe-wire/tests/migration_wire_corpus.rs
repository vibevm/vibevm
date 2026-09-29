//! Generated-reader proof only. Semantic result checks and the optional-selection
//! presence guard belong to the pure adapter; this target does not implement them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");

use serde::de::DeserializeOwned;
use vibe_wire::generated::migration::e1::{plan_error, plan_report, plan_request};

#[path = "wire_support/mod.rs"]
mod wire_support;

fn fixture(family: &str, class: &str, name: &str) -> String {
    format!("formats/corpora/migration-plan-{family}/e1/{class}/{name}.json")
}

fn reject<T: DeserializeOwned>(family: &str, cases: &[(&str, &str)]) {
    for (name, expected) in cases {
        let path = fixture(family, "structural-invalid", name);
        let value = wire_support::read_json(&path);
        let error = serde_json::from_value::<T>(value)
            .err()
            .unwrap_or_else(|| panic!("{path} unexpectedly decoded"));
        assert!(
            error.to_string().contains(expected),
            "{path}: expected {expected:?}, observed {error}"
        );
    }
}

#[test]
fn request_valid_roundtrips_preserve_text_and_optional_choices() {
    let legacy: plan_request::PlanRequest =
        wire_support::round_trip(&fixture("request", "valid", "legacy-selection"));
    assert_eq!(
        legacy.source_profile.as_deref(),
        Some("materialization-legacy-absent-r1")
    );
    assert_eq!(legacy.recipe.as_deref(), Some("snapshot-to-copy-r1"));
    assert!(
        legacy
            .source_text
            .ends_with("materialization=\"snapshot\"\n")
    );
    let missing: plan_request::PlanRequest =
        wire_support::round_trip(&fixture("request", "valid", "missing-selection"));
    assert!(missing.source_text.starts_with("# café\r\n"));
    assert!(missing.source_text.ends_with("\r\n"));
    assert_eq!(
        (
            missing.source_profile,
            missing.target_profile,
            missing.recipe
        ),
        (None, None, None)
    );
    let epoch: plan_request::PlanRequest =
        wire_support::round_trip(&fixture("request", "valid", "current-epoch-one"));
    assert_eq!(
        epoch.source_profile.as_deref(),
        Some("materialization-current-epoch1-r1")
    );
    assert!(epoch.source_text.contains("\nepoch=1\n"));

    // The wire selection is a string: an unknown domain selection must survive
    // decoding for the domain's typed refusal, rather than defaulting here.
    let mut unknown = serde_json::to_value(legacy).unwrap();
    unknown["source_profile"] = serde_json::json!("future-profile");
    let unknown: plan_request::PlanRequest = serde_json::from_value(unknown).unwrap();
    assert_eq!(unknown.source_profile.as_deref(), Some("future-profile"));
}

#[test]
fn report_valid_roundtrips_preserve_all_outcomes_and_bound_vectors() {
    let report: plan_report::PlanReport =
        wire_support::round_trip(&fixture("report", "valid", "planned-golden"));
    let plan_report::Outcome::Planned(planned) = report.outcome else {
        panic!("planned fixture lost its outcome");
    };
    let plan = planned.plan;
    // The exact independent C1 fixed vector; no production hashing helper is
    // called by this reader proof. Source/identity conformance remains C1 proof.
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
    assert_eq!(plan.edits.len(), 1);
    assert_eq!(plan.edits[0].before_start, "86");
    assert_eq!(plan.edits[0].before_end, "96");
    assert_eq!(&plan.before[86..96], plan.edits[0].before_token);
    assert_eq!(plan.edits[0].after_token, "\"copy\"");
    assert!(matches!(plan.source_epoch, plan_report::Epoch::Absent(_)));
    assert!(matches!(plan.target_epoch, plan_report::Epoch::Absent(_)));

    let report: plan_report::PlanReport =
        wire_support::round_trip(&fixture("report", "valid", "no-op-epoch-one"));
    let plan_report::Outcome::NoOp(noop) = report.outcome else {
        panic!("no-op fixture lost its outcome");
    };
    assert!(noop.plan.edits.is_empty());
    assert_eq!(noop.plan.before, noop.plan.after);
    assert_eq!(noop.plan.before_identity, noop.plan.after_identity);
    assert_eq!(
        noop.plan.before_identity,
        "ca8b7cc9cce5eb9f5e8b7f293d754e75473bc541ba5ae801b4183c6e575ecd01"
    );
    assert_eq!(
        noop.plan.diff_identity,
        "0a60a70e3d686316a828aff1410cae82dab0ac8d1431d000136513d4a190e00d"
    );
    assert!(matches!(
        noop.plan.source_epoch,
        plan_report::Epoch::Epoch(_)
    ));
    assert!(matches!(
        noop.plan.target_epoch,
        plan_report::Epoch::Epoch(_)
    ));

    let report: plan_report::PlanReport =
        wire_support::round_trip(&fixture("report", "valid", "needs-input"));
    let plan_report::Outcome::NeedsInput(needs) = report.outcome else {
        panic!("questions fixture lost its outcome");
    };
    assert_eq!(needs.questions.len(), 3);
    assert_eq!(
        needs.questions[0].field,
        plan_report::QuestionField::SourceProfile
    );
    assert_eq!(
        needs.questions[0].choices,
        [
            "materialization-legacy-absent-r1",
            "materialization-current-absent-r1",
            "materialization-current-epoch1-r1"
        ]
    );
    assert_eq!(needs.questions[2].field, plan_report::QuestionField::Recipe);
    assert_eq!(
        needs.questions[2].choices,
        ["snapshot-to-copy-r1", "identity-r1"]
    );
    let report: plan_report::PlanReport =
        wire_support::round_trip(&fixture("report", "valid", "refused"));
    let plan_report::Outcome::Refused(refused) = report.outcome else {
        panic!("refusal fixture lost its outcome");
    };
    assert_eq!(
        refused.error.code,
        plan_report::MigrationPlanningErrorCode::UnknownSelection
    );
}

#[test]
fn preflight_valid_roundtrips_share_one_error_type_and_nine_codes() {
    for name in ["invalid-request", "unsupported-schema", "input-read"] {
        let value: plan_error::PlanError =
            wire_support::round_trip(&fixture("error", "valid", name));
        assert_eq!(value.stage, plan_error::PlanErrorStage::Preflight);
        let shared: vibe_wire::generated::shared::MigrationPlanningError = value.error;
        let report_error: plan_report::MigrationPlanningError = shared.clone();
        let standalone_error: plan_error::MigrationPlanningError = report_error;
        assert_eq!(standalone_error, shared);
    }
    for code in [
        "invalid_request",
        "unsupported_schema",
        "input_read",
        "unknown_selection",
        "incompatible_edge",
        "malformed_syntax",
        "unrecognized_shape",
        "invalid_current_manifest",
        "unprovable_edit",
    ] {
        let value: plan_report::MigrationPlanningErrorCode =
            serde_json::from_value(serde_json::json!(code)).unwrap();
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            serde_json::json!(code)
        );
    }
}

#[test]
fn request_structural_invalids_refuse() {
    reject::<plan_request::PlanRequest>(
        "request",
        &[
            ("schema-string", "invalid type"),
            ("source-text-number", "invalid type"),
            ("profile-number", "invalid type"),
            ("unknown-root", "unknown field"),
        ],
    );
}

#[test]
fn report_structural_invalids_refuse_root_outcome_plan_edit_and_question() {
    reject::<plan_report::PlanReport>(
        "report",
        &[
            ("unknown-root", "unknown field"),
            ("schema-string", "invalid type"),
            ("unknown-stage", "unknown variant"),
            ("unknown-outcome", "unknown variant"),
            ("wrong-branch-payload", "unknown field"),
            ("unknown-plan-field", "unknown field"),
            ("unknown-edit-field", "unknown field"),
            ("wrong-epoch-tag", "unknown variant"),
            ("question-extra", "unknown field"),
            ("question-unknown-field", "unknown variant"),
            ("question-choices-number", "invalid type"),
        ],
    );
}

#[test]
fn empty_epoch_payload_markers_refuse_extra_members() {
    reject::<plan_report::PlanReport>(
        "report",
        &[
            ("absent-extra", "unknown field"),
            ("epoch-one-extra", "unknown field"),
        ],
    );
}

#[test]
fn shared_error_extra_members_and_unknown_codes_refuse_in_both_roots() {
    let cases = [
        ("shared-unknown-code", "unknown variant"),
        ("shared-extra-field", "unknown field"),
    ];
    reject::<plan_report::PlanReport>("report", &cases);
    reject::<plan_error::PlanError>("error", &cases);
}

#[test]
fn preflight_root_stage_and_primitive_invalids_refuse() {
    reject::<plan_error::PlanError>(
        "error",
        &[
            ("unknown-root", "unknown field"),
            ("schema-string", "invalid type"),
            ("unknown-stage", "unknown variant"),
        ],
    );
}

#[test]
fn numeric_offsets_refuse_instead_of_lossy_conversion() {
    reject::<plan_report::PlanReport>(
        "report",
        &[
            ("before-start-number", "invalid type"),
            ("before-end-number", "invalid type"),
        ],
    );
}

#[test]
fn optional_null_is_presence_invalid_but_direct_derived_reader_accepts() {
    // These are invalid under the source contract, NOT generated-reader
    // rejection fixtures. Option<String> currently erases explicit null.
    // The owning adapter's later presence check must refuse all three inputs.
    for field in ["source_profile", "target_profile", "recipe"] {
        let name = format!("{}-null", field.replace('_', "-"));
        let authored = wire_support::read_json(&fixture("request", "presence-invalid", &name));
        assert!(authored.get(field).unwrap().is_null());
        let decoded: plan_request::PlanRequest = serde_json::from_value(authored.clone()).unwrap();
        match field {
            "source_profile" => assert!(decoded.source_profile.is_none()),
            "target_profile" => assert!(decoded.target_profile.is_none()),
            "recipe" => assert!(decoded.recipe.is_none()),
            _ => unreachable!(),
        }
        let lowered = serde_json::to_value(decoded).unwrap();
        assert!(lowered.get(field).is_none());
        assert_ne!(lowered, authored, "direct Option lowering hides presence");
    }
}

#[test]
fn semantic_invalid_frames_require_later_owner_validation() {
    // Serde accepts these shapes; no semantic validation pass is claimed.
    let request: plan_request::PlanRequest = serde_json::from_value(wire_support::read_json(
        &fixture("request", "semantic-invalid", "unsupported-schema"),
    ))
    .unwrap();
    assert_eq!(request.schema, 2);
    for name in [
        "unsupported-schema",
        "preflight-code-in-planner",
        "noncanonical-offset",
        "offset-out-of-bounds",
        "profile-epoch-mismatch",
    ] {
        let authored = wire_support::read_json(&fixture("report", "semantic-invalid", name));
        let report: plan_report::PlanReport = serde_json::from_value(authored.clone()).unwrap();
        assert_eq!(serde_json::to_value(report).unwrap(), authored);
    }
    let error: plan_error::PlanError = serde_json::from_value(wire_support::read_json(&fixture(
        "error",
        "semantic-invalid",
        "planner-code-in-preflight",
    )))
    .unwrap();
    assert_eq!(error.stage, plan_error::PlanErrorStage::Preflight);
    assert_eq!(
        error.error.code,
        plan_error::MigrationPlanningErrorCode::UnknownSelection
    );
}
