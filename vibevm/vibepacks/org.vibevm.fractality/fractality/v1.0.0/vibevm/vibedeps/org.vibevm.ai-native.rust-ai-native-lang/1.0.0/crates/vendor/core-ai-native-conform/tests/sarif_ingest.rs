//! B-026 SARIF ingest — the read half, exercised end to end through the
//! crate's public surface. A fixture report is read, facts appear, a rule
//! sees them; a broken report yields no facts and never fails the loader.
//! (Kept out of `src/sarif.rs` so that module stays under the 600-line
//! budget — the byte-stable renderer's two unit tests stay inline there.)

use core_ai_native_conform::rules::LintSuppressionNeedsReason;
use core_ai_native_conform::{Fact, FindingStatus, Rule, sarif};

fn clippy_report() -> &'static str {
    r#"{
      "version": "2.1.0",
      "runs": [{
        "tool": { "driver": { "name": "clippy" } },
        "results": [
          { "ruleId": "clippy::unwrap_used",
            "message": { "text": "used .unwrap()" },
            "locations": [{ "physicalLocation": {
              "artifactLocation": { "uri": "src/a.rs" },
              "region": { "startLine": 4 } }}] },
          { "ruleId": "clippy::unwrap_used",
            "message": { "text": "used .unwrap()" },
            "locations": [{ "physicalLocation": {
              "artifactLocation": { "uri": "src/a.rs" },
              "region": { "startLine": 9 } }}],
            "suppressions": [{ "kind": "inSource", "justification": "FFI boundary" }] },
          { "ruleId": "clippy::unwrap_used",
            "message": { "text": "no reason given" },
            "locations": [{ "physicalLocation": {
              "artifactLocation": { "uri": "src/b.rs" },
              "region": { "startLine": 2 } }}],
            "suppressions": [{ "kind": "inSource" }] }
        ]
      }]
    }"#
}

#[test]
fn ingest_reads_tool_rule_location_and_suppressions() {
    let facts = sarif::ingest(clippy_report());
    assert_eq!(facts.len(), 3, "three results, all located");
    // Tool + rule + status come through; the citation form is live.
    assert!(
        facts
            .iter()
            .any(|f| f.cites_lint("clippy", "clippy::unwrap_used", Some(false)))
    );
    let ack = facts
        .iter()
        .find(|f| f.cites_lint("clippy", "clippy::unwrap_used", Some(true)))
        .expect("a suppressed clippy::unwrap_used diagnosis is parsed");
    match ack {
        Fact::LintDiagnosis {
            reason: Some(r), ..
        } => assert_eq!(r, "FFI boundary"),
        other => panic!("expected a suppressed diagnosis with a reason, got {other:?}"),
    }
}

#[test]
fn broken_or_unfamiliar_report_yields_no_facts_not_a_panic() {
    // Not JSON at all.
    assert!(sarif::ingest("this is not { a report").is_empty());
    // JSON but not SARIF (no runs).
    assert!(sarif::ingest(r#"{"version":"2.1.0"}"#).is_empty());
    // A result missing its ruleId is skipped (cannot be cited).
    assert!(
        sarif::ingest(
            r#"{"runs":[{"tool":{"driver":{"name":"x"}},"results":[
           {"message":{"text":"m"},"locations":[{"physicalLocation":{
             "artifactLocation":{"uri":"a"},"region":{"startLine":1}}}]}]}]}"#
        )
        .is_empty()
    );
    // A result with no location is skipped (no site to cite).
    assert!(
        sarif::ingest(
            r#"{"runs":[{"tool":{"driver":{"name":"x"}},"results":[
           {"ruleId":"r","message":{"text":"m"}}]}]}"#
        )
        .is_empty()
    );
}

#[test]
fn load_reports_buckets_by_file_and_counts() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("lint");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("clippy.sarif"), clippy_report()).unwrap();
    // A second, clean report (zero results) ingests as zero facts and
    // still counts as a report read.
    std::fs::write(
        dir.join("clean.json"),
        r#"{"runs":[{"tool":{"driver":{"name":"eslint"}},"results":[]}]}"#,
    )
    .unwrap();

    let (facts, reports, diagnoses) = sarif::load_reports(tmp.path(), &["lint".into()]);
    assert_eq!(reports, 2, "both reports parsed as SARIF");
    assert_eq!(diagnoses, 3, "three diagnoses across the clippy report");
    // Two distinct files cited (src/a.rs, src/b.rs).
    assert_eq!(facts.len(), 2);
    let a = facts
        .iter()
        .find(|sf| sf.file == "src/a.rs")
        .expect("src/a.rs");
    assert_eq!(a.facts.len(), 2);
    // Within a file, sorted by line ascending.
    match (&a.facts[0], &a.facts[1]) {
        (Fact::LintDiagnosis { line: l0, .. }, Fact::LintDiagnosis { line: l1, .. }) => {
            assert!(*l0 < *l1);
        }
        _ => panic!("sorted LintDiagnosis facts"),
    }
}

#[test]
fn load_reports_broken_file_is_visible_and_skipped_not_fatal() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("lint");
    std::fs::create_dir_all(&dir).unwrap();
    // Broken JSON + a non-SARIF JSON blob: neither yields facts, neither
    // fails the loader.
    std::fs::write(dir.join("broken.sarif"), "{ not json").unwrap();
    std::fs::write(dir.join("notsarif.json"), r#"{"hello":"world"}"#).unwrap();

    let (facts, reports, diagnoses) = sarif::load_reports(tmp.path(), &["lint".into()]);
    assert!(facts.is_empty());
    assert_eq!(reports, 0, "neither parsed as a SARIF report");
    assert_eq!(diagnoses, 0);
}

#[test]
fn load_reports_absent_path_is_the_norm_and_stays_silent() {
    let tmp = tempfile::tempdir().unwrap();
    // No report deposited — the default state of every project today.
    let (facts, reports, diagnoses) = sarif::load_reports(tmp.path(), &["target/lint".into()]);
    assert!(facts.is_empty());
    assert_eq!(reports, 0);
    assert_eq!(diagnoses, 0);
    // An empty config list is likewise a no-op.
    let (facts, _, _) = sarif::load_reports(tmp.path(), &[]);
    assert!(facts.is_empty());
}

#[test]
fn parsed_diagnoses_flow_into_findings_a_rule_sees() {
    // The whole path the packet asks for, end to end: a report is read,
    // facts appear, a rule sees them. Uses the citation-primitive rule
    // that ships with the engine (LintSuppressionNeedsReason): a
    // suppressed diagnosis WITH a reason surfaces acknowledged (visible,
    // gate-inert — the point-2 mapping), one WITHOUT a reason surfaces
    // Live (a violation); a live diagnosis is citation data this rule
    // does not surface.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("lint");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("clippy.sarif"), clippy_report()).unwrap();

    let (facts, _, _) = sarif::load_reports(tmp.path(), &["lint".into()]);
    let findings = LintSuppressionNeedsReason.check(&facts);
    // src/a.rs:9 (suppressed, reason) -> acknowledged; src/b.rs:2
    // (suppressed, no reason) -> live; src/a.rs:4 (live) -> not surfaced.
    assert_eq!(findings.len(), 2);
    let ack = findings
        .iter()
        .find(|f| f.file.contains("src/a.rs"))
        .expect("the reasoned suppression is surfaced (acknowledged)");
    assert!(matches!(
        ack.status,
        FindingStatus::DeviationAcknowledged { .. }
    ));
    let live = findings
        .iter()
        .find(|f| f.file.contains("src/b.rs"))
        .expect("the reasonless suppression is surfaced (live)");
    assert!(matches!(live.status, FindingStatus::Live));
}
