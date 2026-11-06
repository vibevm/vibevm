//! Source-specific planning and exact original-request replay, without filesystem authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");
use super::*;
use serde_json::json;
use vibe_core::manifest::{BOOT_SOURCE_INSERT, SOURCE_BOOT_CURRENT, SOURCE_BOOT_LEGACY};

pub(super) fn source(path: &str) -> String {
    format!(
        "# café retained\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.0.0'\r\nformat='normal'\r\n\r\n[boot_snippet]\r\nsource='{path}' # kept exactly"
    )
}
pub(super) fn request(path: &str) -> Value {
    json!({"schema":1,"source_text":source(path),"source_profile":SOURCE_BOOT_LEGACY,
        "target_profile":SOURCE_BOOT_CURRENT,"recipe":BOOT_SOURCE_INSERT})
}
pub(super) fn decision(path: &str, media: &str) -> Value {
    json!({"file":path,"media":media,"interpretation":"vibe-spec","role":"spec","loading":"explicit"})
}
pub(super) fn answered(path: &str, media: &str) -> Value {
    let mut value = request(path);
    value["source_decisions"] = json!([decision(path, media)]);
    value
}
pub(super) fn report(value: Value) -> PlanReport {
    let request = decode_migration_request(value).unwrap();
    let MigrationPlanningResponse::Report(report) = plan_migration_request(request.clone()) else {
        panic!("planner report")
    };
    validate_report_for_request(&request, &report).unwrap();
    report
}
pub(super) fn plan(value: Value) -> Plan {
    let Outcome::Planned(row) = report(value).outcome else {
        panic!("source plan")
    };
    row.plan
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 2
)]
fn source_question_answer_preserves_exact_bytes_and_original_decision() {
    for (path, media) in [
        ("boot/choice.md", "application/xml"),
        ("boot/choice.xml", "text/markdown"),
        ("boot/choice.bin", "application/xml"),
    ] {
        let unanswered = request(path);
        let questions = serde_json::to_value(report(unanswered.clone())).unwrap();
        assert_eq!(
            questions["outcome"],
            json!({"status":"needs_input","questions":[{
            "field":"source_media","resource_path":path,"choices":["application/xml","text/markdown"]}]})
        );
        let input = answered(path, media);
        let decoded = decode_migration_request(input.clone()).unwrap();
        let planned = report(input);
        let Outcome::Planned(row) = &planned.outcome else {
            panic!("planned")
        };
        assert_eq!(row.plan.before, source(path));
        assert!(row.plan.after.starts_with(&row.plan.before));
        assert!(!row.plan.after.replace("\r\n", "").contains('\n'));
        assert_eq!(row.plan.edits.len(), 1);
        assert_eq!(
            row.plan.edits[0].before_start,
            source(path).len().to_string()
        );
        assert_eq!(row.plan.edits[0].before_end, row.plan.edits[0].before_start);
        assert_eq!(row.plan.edits[0].before_token, "");
        let manifest = vibe_core::manifest::Manifest::parse_str(&row.plan.after).unwrap();
        let package = manifest.require_package().unwrap();
        let descriptor = &package.sources.as_ref().unwrap()[0];
        assert_eq!(descriptor.selector().path(), path);
        assert_eq!(descriptor.media(), media);
        let current = report(json!({"schema":1,"source_text":row.plan.after,
            "source_profile":SOURCE_BOOT_CURRENT,"target_profile":SOURCE_BOOT_CURRENT,
            "recipe":vibe_core::manifest::IDENTITY}));
        let Outcome::NoOp(current) = current.outcome else {
            panic!("declared current identity")
        };
        assert_eq!(current.plan.before, row.plan.after);
        assert_eq!(current.plan.after, row.plan.after);
        assert!(current.plan.edits.is_empty());
        let mut changed = decoded.clone();
        changed.source_decisions.clear();
        assert!(
            validate_report_for_request(&changed, &planned).is_err(),
            "after text cannot invent an omitted answer"
        );
        let mut changed = decoded;
        changed.source_decisions[0].media = if media == "application/xml" {
            "text/markdown"
        } else {
            "application/xml"
        }
        .into();
        assert!(
            validate_report_for_request(&changed, &planned).is_err(),
            "original media choice must bind replay"
        );
    }
}

#[test]
fn neutral_serialization_is_unchanged_and_empty_source_answers_still_ask() {
    let neutral = json!({"schema":1,"source_text":""});
    assert_eq!(
        serde_json::to_value(decode_migration_request(neutral.clone()).unwrap()).unwrap(),
        neutral
    );
    let mut empty = neutral.clone();
    empty["source_decisions"] = json!([]);
    assert_eq!(
        serde_json::to_value(decode_migration_request(empty).unwrap()).unwrap(),
        neutral
    );
    let value = serde_json::to_value(plan_migration_value(neutral)).unwrap();
    for question in value["outcome"]["questions"].as_array().unwrap() {
        assert!(question.get("resource_path").is_none());
        assert_ne!(question["field"], "source_media");
    }
    let answered_without_profiles =
        report(json!({"schema":1,"source_text":source("boot/no-guess.xml"),
        "source_decisions":[decision("boot/no-guess.xml", "application/xml")]}));
    let Outcome::NeedsInput(questions) = answered_without_profiles.outcome else {
        panic!("source lane selections")
    };
    assert_eq!(
        questions.questions[0].choices,
        [SOURCE_BOOT_LEGACY, SOURCE_BOOT_CURRENT]
    );
    assert_eq!(
        questions.questions[2].choices,
        [BOOT_SOURCE_INSERT, vibe_core::manifest::IDENTITY]
    );
    assert!(
        questions
            .questions
            .iter()
            .all(|question| question.resource_path.is_none())
    );
    let mut empty = request("boot/no-guess.xml");
    empty["source_decisions"] = json!([]);
    assert_eq!(report(empty), report(request("boot/no-guess.xml")));
}

#[test]
fn malformed_source_answers_are_preflight_and_valid_but_inapplicable_answers_refuse() {
    let path = "boot/choice.xml";
    let valid = decision(path, "application/xml");
    let mut malformed = vec![
        Value::Null,
        json!({}),
        json!([null]),
        json!([valid.clone(), valid.clone()]),
    ];
    for field in ["file", "media", "interpretation", "role", "loading"] {
        let mut row = valid.clone();
        row.as_object_mut().unwrap().remove(field);
        malformed.push(json!([row]));
        let mut row = valid.clone();
        row[field] = Value::Null;
        malformed.push(json!([row]));
    }
    for (key, value) in [
        ("trust", json!("verified")),
        ("directory", json!("boot")),
        ("file", json!("../foreign.xml")),
        ("interpretation", json!("future-parser")),
        ("role", json!("future-role")),
        ("loading", json!("future-loading")),
    ] {
        let mut row = valid.clone();
        row[key] = value;
        malformed.push(json!([row]));
    }
    for bad in malformed {
        let mut input = request(path);
        input["source_decisions"] = bad.clone();
        assert!(
            matches!(
                plan_migration_value(input),
                MigrationPlanningResponse::Preflight(_)
            ),
            "accepted malformed decisions {bad}"
        );
    }
    for row in [
        decision("boot/other.xml", "application/xml"),
        json!({"file":path,"media":"application/octet-stream","interpretation":"opaque","role":"resource","loading":"explicit"}),
    ] {
        let mut input = request(path);
        input["source_decisions"] = json!([row]);
        assert!(matches!(report(input).outcome, Outcome::Refused(_)));
    }
}

#[test]
fn forged_source_questions_and_refusals_do_not_validate_as_original_results() {
    let input = decode_migration_request(request("boot/choice.xml")).unwrap();
    let original = report(serde_json::to_value(&input).unwrap());
    for variant in 0..4 {
        let mut changed = original.clone();
        let Outcome::NeedsInput(row) = &mut changed.outcome else {
            panic!()
        };
        match variant {
            0 => row.questions[0].resource_path = None,
            1 => row.questions[0].resource_path = Some("boot/other.xml".into()),
            2 => row.questions[0].choices = vec!["application/xml".into()],
            _ => row.questions[0].field = QuestionField::Recipe,
        }
        assert!(validate_report_for_request(&input, &changed).is_err());
    }
    let mut unsupported = request("boot/choice.xml");
    unsupported["source_text"] =
        json!(source("boot/choice.xml").replace("format='normal'", "format='simple'"));
    let input = decode_migration_request(unsupported.clone()).unwrap();
    let mut refusal = report(unsupported);
    let Outcome::Refused(row) = &mut refusal.outcome else {
        panic!("unsupported shape must refuse, not ask")
    };
    row.error.reason = "invented refusal".into();
    assert!(validate_report_for_request(&input, &refusal).is_err());
}
