//! The queue assembled: eight numbers, eight sections, and the rule that
//! an unasked question never prints as a zero.

use std::path::Path;

use vibe_wire::generated::doc_todo::SectionName;

use super::*;
use crate::citations::SpecSources;

const COORDINATE: &str = "org.acme/manual";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 12).expect("a date")
}

fn inputs() -> Inputs {
    Inputs {
        coordinate: COORDINATE.into(),
        package_version: "0.1.0".into(),
        product_version: "1.0.0".into(),
        sources: SpecSources::new(),
        corpus_root: None,
        obligations: Vec::new(),
        min: crate::coverage::FULL_COVERAGE,
        today: today(),
        backlog: None,
        journal: None,
        examples: None,
        surface: None,
    }
}

/// A package with two pages, one of which was read aloud a long time ago.
fn package(dir: &Path) {
    std::fs::write(
        dir.join("vibe.toml"),
        "[package]\ngroup = \"org.acme\"\nname = \"manual\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\n\n[i18n]\ncanonical = \"en\"\n",
    )
    .expect("the manifest");
    let pages = dir.join("vibevm/vibespecs/model");
    std::fs::create_dir_all(&pages).expect("the spec root");
    for name in ["boot-lane.xml", "packages.xml"] {
        std::fs::write(
            pages.join(name),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">Page</title>\n  \
               <p>One short sentence about the boot lane.</p>\n\
             </spec>\n",
        )
        .expect("a page");
    }
}

fn section(queue: &DocTodo, name: SectionName) -> &TodoSection {
    queue
        .sections
        .iter()
        .find(|s| s.name == name)
        .expect("every section is emitted")
}

#[test]
fn every_section_is_emitted_even_when_it_was_not_measured() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let queue = build(tmp.path(), &inputs()).expect("a queue");
    assert_eq!(queue.sections.len(), 8);
    assert!(!section(&queue, SectionName::Coverage).measured);
    assert!(!section(&queue, SectionName::Examples).measured);
    assert!(!section(&queue, SectionName::Debt).measured);
    assert!(!section(&queue, SectionName::VersionChange).measured);
    assert!(
        section(&queue, SectionName::Citations).measured,
        "a citation check needs nothing but the pages, so it always runs"
    );
}

#[test]
fn a_number_nobody_measured_is_null_and_not_zero() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let queue = build(tmp.path(), &inputs()).expect("a queue");
    assert_eq!(queue.metrics.coverage_percent, None);
    assert_eq!(queue.metrics.page_age_median_days, None);
    assert_eq!(queue.metrics.adaptation_divergences, None);
    assert_eq!(queue.metrics.findings_without_decision, None);
    assert_eq!(queue.metrics.days_since_reconcile, None);
    assert_eq!(
        queue.metrics.debt_p1, 0,
        "a debt count with no file to read is zero debt filed, which is true"
    );
    assert!(!queue.examples_measured);
}

#[test]
fn a_package_with_no_readings_owes_one_for_every_page() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let queue = build(tmp.path(), &inputs()).expect("a queue");
    let age = section(&queue, SectionName::PageAge);
    assert!(age.measured);
    assert_eq!(age.items.len(), 2);
    assert!(age.items[0].reason.contains("never read aloud"));
    assert_eq!(
        queue.metrics.page_age_median_days, None,
        "a manual nobody has read aloud has no median — it has a backlog"
    );
}

#[test]
fn the_debt_file_feeds_the_queue_and_the_p1_number() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let backlog = tmp.path().join("BACKLOG.md");
    std::fs::write(
        &backlog,
        "# Backlog\n\n- docs: P1 the install page is silent about the store\n\
         - docs: P2 the deploy page does not mention --dry-run\n",
    )
    .expect("the debt file");
    let queue = build(
        tmp.path(),
        &Inputs {
            backlog: Some(backlog),
            ..inputs()
        },
    )
    .expect("a queue");
    let debt = section(&queue, SectionName::Debt);
    assert!(debt.measured);
    assert_eq!(debt.items.len(), 2);
    assert_eq!(queue.metrics.debt_p1, 1);
}

#[test]
fn a_journal_entry_owing_a_decision_is_the_seventh_number() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let journal = tmp.path().join("JOURNAL.md");
    std::fs::write(
        &journal,
        "| Id | What happened | → regulation |\n|---|---|---|\n\
         | J-001 | the runner tripped | the loop opens with `git status` |\n\
         | J-002 | a false positive |  |\n",
    )
    .expect("the journal");
    let queue = build(
        tmp.path(),
        &Inputs {
            journal: Some(journal),
            ..inputs()
        },
    )
    .expect("a queue");
    assert_eq!(queue.metrics.findings_without_decision, Some(1));
}

#[test]
fn the_recorded_reading_gives_the_median_and_the_reconciliation_date() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    std::fs::create_dir_all(tmp.path().join("maintenance")).expect("the directory");
    std::fs::write(
        tmp.path().join(reviews::REVIEWS),
        "schema = 1\nreconciled = \"2026-06-14\"\n\n\
         [[page]]\npath = \"model/boot-lane.xml\"\nread = \"2026-01-01\"\nby = \"owner\"\n",
    )
    .expect("the record");
    let queue = build(tmp.path(), &inputs()).expect("a queue");
    assert_eq!(queue.metrics.page_age_median_days, Some(254));
    assert_eq!(queue.metrics.days_since_reconcile, Some(90));
    let age = section(&queue, SectionName::PageAge);
    assert!(
        age.items.iter().any(|i| i.reason.contains("254 days")),
        "a page unread for more than ninety days is in the queue with its age"
    );
    assert!(
        age.items
            .iter()
            .any(|i| i.reason.contains("never read aloud")),
        "and a page with no row at all is in it too"
    );
}

/// The queue is a measurer. Whatever the numbers say, building it is an
/// answer and never a refusal — a lock between a release of the product
/// and its documentation is exactly what the norm refuses.
#[test]
fn a_queue_full_of_rows_is_still_a_successful_reading() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let queue = build(tmp.path(), &inputs()).expect("a queue");
    assert!(queue.metrics.gaps < u32::MAX);
    assert!(to_json(&queue).contains("\"gaps\""));
    assert!(report::render_md(&queue).contains("Maintenance queue"));
}

#[test]
fn the_json_carries_the_eight_numbers_by_the_names_the_table_uses() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let queue = build(tmp.path(), &inputs()).expect("a queue");
    let text = to_json(&queue);
    let document: serde_json::Value = serde_json::from_str(&text).expect("a JSON document");
    let metrics = document
        .get("metrics")
        .and_then(|m| m.as_object())
        .expect("the metrics");
    let mut names: Vec<&String> = metrics.keys().collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "adaptation_divergences",
            "coverage_percent",
            "days_since_reconcile",
            "debt_p1",
            "findings_without_decision",
            "gaps",
            "page_age_median_days",
            "tics_per_100k_words",
        ]
    );
}
