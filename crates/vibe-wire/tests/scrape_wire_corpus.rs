//! Epoch-1 scrape plan/report/verifier-result corpus and computed registry policy.

use std::path::PathBuf;

use vibe_wire::generated::format_id::{ForeignParsers, FormatId, UnknownFields};
use vibe_wire::generated::scrape::e1::health_result::HealthResult as ScrapeHealthResult;
use vibe_wire::generated::scrape::e1::plan::Plan as ScrapePlan;
use vibe_wire::generated::scrape::e1::report::Report as ScrapeReport;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus() -> PathBuf {
    repo_root().join("formats/corpora/scrape/e1")
}

fn document(path: &str) -> serde_json::Value {
    let path = corpus().join(path);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} readable: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is JSON: {error}", path.display()))
}

#[test]
fn valid_documents_round_trip_through_only_the_generated_types() {
    let plan_doc = document("valid/plan-minimal.json");
    let plan: ScrapePlan = serde_json::from_value(plan_doc.clone()).expect("plan parses");
    assert_eq!(serde_json::to_value(plan).unwrap(), plan_doc);

    let variants_doc = document("valid/plan-unions.json");
    let variants: ScrapePlan =
        serde_json::from_value(variants_doc.clone()).expect("plan unions parse");
    assert_eq!(serde_json::to_value(variants).unwrap(), variants_doc);

    let report_doc = document("valid/report-minimal.json");
    let report: ScrapeReport = serde_json::from_value(report_doc.clone()).expect("report parses");
    assert_eq!(serde_json::to_value(report).unwrap(), report_doc);

    let health_doc = document("valid/health-pass.json");
    let health: ScrapeHealthResult =
        serde_json::from_value(health_doc.clone()).expect("health result parses");
    assert_eq!(serde_json::to_value(health).unwrap(), health_doc);
}

#[test]
fn plan_item_assertion_and_contract_boundary_arms_are_closed_unions() {
    use vibe_wire::generated::scrape::e1::plan::{Assertion, ContractBoundary, Healthcheck, Item};

    let plan: ScrapePlan = serde_json::from_value(document("valid/plan-unions.json")).unwrap();
    assert!(matches!(plan.items[0], Item::Keep(_)));
    assert!(matches!(plan.items[1], Item::Rewrite(_)));
    assert!(matches!(plan.items[2], Item::Relocate(_)));
    assert!(matches!(plan.items[3], Item::DeleteUnmodified(_)));
    assert!(matches!(plan.items[4], Item::DeleteModified(_)));
    assert!(matches!(plan.items[5], Item::DeleteUnknown(_)));
    assert!(matches!(plan.items[6], Item::DeleteLast(_)));
    assert!(matches!(plan.assertions[0], Assertion::PathsAbsentV1(_)));
    assert!(matches!(
        plan.assertions[1],
        Assertion::TextLiteralAbsentV1(_)
    ));
    assert!(matches!(
        plan.assertions[2],
        Assertion::CargoPathPrefixAbsentV1(_)
    ));
    assert!(matches!(
        plan.assertions[3],
        Assertion::LanguageMetadataAbsentV1(_)
    ));
    assert!(matches!(
        plan.assertions[4],
        Assertion::DependencyIdentitiesAbsentV1(_)
    ));
    assert!(matches!(
        plan.contract_boundary,
        ContractBoundary::DeleteLast(_)
    ));
    let Healthcheck::Custom(custom) = &plan.healthchecks[0] else {
        panic!("custom health arm")
    };
    assert_eq!(custom.reads, ["**"]);
    assert!(custom.writes.is_empty());
    assert!(!custom.spawn);
    assert_eq!(custom.snapshot.len(), 1);

    let mut preserve = document("valid/plan-minimal.json");
    preserve["contract"]["contained"] = serde_json::json!(false);
    preserve["contract"]["display_path"] = serde_json::json!("C:/contracts/scrape.toml");
    preserve["contract"]["action"] = serde_json::json!("preserve");
    preserve["contract_boundary"] = serde_json::json!({ "kind": "preserve" });
    let preserve: ScrapePlan = serde_json::from_value(preserve).unwrap();
    assert!(matches!(
        preserve.contract_boundary,
        ContractBoundary::Preserve(_)
    ));
}

#[test]
fn every_closed_enum_refuses_an_unknown_spelling() {
    let mut plan = document("valid/plan-minimal.json");
    plan["mode"] = serde_json::json!("detach");
    assert!(serde_json::from_value::<ScrapePlan>(plan).is_err());

    let mut report = document("valid/report-minimal.json");
    report["outcome"] = serde_json::json!("success");
    assert!(serde_json::from_value::<ScrapeReport>(report).is_err());

    let mut health = document("valid/health-pass.json");
    health["status"] = serde_json::json!("healthy");
    assert!(serde_json::from_value::<ScrapeHealthResult>(health).is_err());
}

#[test]
fn every_scrape_reader_rejects_top_level_and_union_variant_leakage() {
    let mut plan = document("valid/plan-minimal.json");
    plan["future"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ScrapePlan>(plan).is_err());

    let mut plan = document("valid/plan-unions.json");
    plan["items"][0]["future"] = serde_json::json!(true);
    assert!(
        serde_json::from_value::<ScrapePlan>(plan).is_err(),
        "a tagged item arm rejects variant-inapplicable leakage"
    );

    let mut plan = document("valid/plan-unions.json");
    plan["contract_boundary"] = serde_json::json!({"kind": "preserve", "path": "leaked"});
    assert!(
        serde_json::from_value::<ScrapePlan>(plan).is_err(),
        "the preserve arm rejects delete-last members"
    );

    let mut plan = document("valid/plan-unions.json");
    plan["healthchecks"][0]["snapshot"][0]["future"] = serde_json::json!(true);
    assert!(
        serde_json::from_value::<ScrapePlan>(plan).is_err(),
        "custom snapshot entries are strict per-path identity objects"
    );

    let mut plan = document("valid/plan-unions.json");
    plan["healthchecks"][0]
        .as_object_mut()
        .unwrap()
        .remove("writes");
    assert!(
        serde_json::from_value::<ScrapePlan>(plan).is_err(),
        "custom reads/writes/spawn are required rather than inferred defaults"
    );

    let mut report = document("valid/report-minimal.json");
    report["future"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ScrapeReport>(report).is_err());

    let health = document("invalid/health-unknown-field.json");
    assert!(
        serde_json::from_value::<ScrapeHealthResult>(health).is_err(),
        "the engine-owned health reply rejects an unknown member"
    );
}

#[test]
fn registry_records_are_complete_and_computed_honestly() {
    let cases = [
        ("scrape-plan", true, ForeignParsers::Many),
        ("scrape-report", false, ForeignParsers::Many),
        ("scrape-health-result", true, ForeignParsers::None),
    ];
    for (name, recoverable, foreign) in cases {
        let format = FormatId::ALL
            .iter()
            .copied()
            .find(|format| format.id() == name)
            .unwrap_or_else(|| panic!("FormatId carries {name}"));
        assert_eq!(format.epoch(), 1, "{name} epoch");
        assert_eq!(format.recoverable(), recoverable, "{name} recoverability");
        assert_eq!(format.foreign_parsers(), foreign, "{name} parser role");
        assert_eq!(
            format.unknown_fields(),
            UnknownFields::Deny,
            "{name} strict reader"
        );
    }

    for relative in [
        "schemas/scrape/e1/plan.jtd.json",
        "schemas/scrape/e1/report.jtd.json",
        "schemas/scrape/e1/health_result.jtd.json",
    ] {
        assert!(repo_root().join(relative).is_file(), "{relative} exists");
    }
}
