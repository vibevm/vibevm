//! Generated scrape health persistence and shared-row corpus.

use std::path::{Path, PathBuf};

use vibe_wire::generated::format_id::{ForeignParsers, FormatId, UnknownFields};
use vibe_wire::generated::scrape::e2::prepared_health_snapshot::PreparedHealthSnapshot;
use vibe_wire::generated::scrape::e2::verification_health_evidence::{
    ScrapeHealthFinding, ScrapeHealthPhase, ScrapeHealthRow, ScrapeHealthSeverity,
    ScrapeHealthStep, ScrapeHealthStreamWitness, ScrapeHealthTerminalState,
    VerificationHealthEvidence,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus(name: &str) -> PathBuf {
    repo_root().join("formats/corpora").join(name).join("e2")
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| panic!("{} readable: {error}", path.display()))
}

fn without_final_newline(bytes: &[u8]) -> &[u8] {
    bytes
        .strip_suffix(b"\r\n")
        .or_else(|| bytes.strip_suffix(b"\n"))
        .unwrap_or(bytes)
}

#[test]
fn prepared_health_corpus_round_trips_and_refuses_every_strict_red() {
    let root = corpus("scrape-prepared-health-snapshot");
    for name in ["minimal.json", "custom.json"] {
        let authored = read(&root.join("valid").join(name));
        let wire: PreparedHealthSnapshot = serde_json::from_slice(&authored).unwrap();
        assert_eq!(
            serde_json::to_vec(&wire).unwrap(),
            without_final_newline(&authored)
        );
    }
    for name in [
        "root-unknown.json",
        "nested-unknown.json",
        "duplicate.json",
        "missing.json",
        "type.json",
        "tag.json",
        "null.json",
        "count-overflow.json",
    ] {
        assert!(
            serde_json::from_slice::<PreparedHealthSnapshot>(&read(
                &root.join("invalid").join(name)
            ))
            .is_err(),
            "{name} must be refused"
        );
    }
}

#[test]
fn evidence_corpus_round_trips_and_refuses_root_and_nested_drift() {
    let root = corpus("scrape-verification-health-evidence");
    for name in ["minimal.json", "failure.json"] {
        let authored = read(&root.join("valid").join(name));
        let wire: VerificationHealthEvidence = serde_json::from_slice(&authored).unwrap();
        assert_eq!(
            serde_json::to_vec(&wire).unwrap(),
            without_final_newline(&authored)
        );
    }
    for name in [
        "root-unknown.json",
        "nested-unknown.json",
        "duplicate.json",
        "missing.json",
        "type.json",
        "tag.json",
        "null.json",
    ] {
        assert!(
            serde_json::from_slice::<VerificationHealthEvidence>(&read(
                &root.join("invalid").join(name)
            ))
            .is_err(),
            "{name} must be refused"
        );
    }
}

#[test]
fn shared_row_preserves_the_public_epoch1_member_bytes() {
    let empty = ScrapeHealthStreamWitness {
        bytes: "0".to_owned(),
        head: String::new(),
        redacted: true,
        sha256: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            .to_owned(),
        tail: String::new(),
        truncated: false,
        utf8: true,
    };
    let row = ScrapeHealthRow {
        argv: vec!["cargo.exe".to_owned(), "test".to_owned()],
        findings: vec![ScrapeHealthFinding {
            id: "id".to_owned(),
            message: "message".to_owned(),
            severity: ScrapeHealthSeverity::Warning,
            evidence: None,
        }],
        id: "cargo".to_owned(),
        network_verified: false,
        phase: ScrapeHealthPhase::After,
        stderr: empty.clone(),
        stdout: empty,
        step: ScrapeHealthStep::Test,
        terminal: ScrapeHealthTerminalState::Warn,
        tests_skipped: true,
    };
    assert_eq!(
        serde_json::to_string(&row).unwrap(),
        r#"{"argv":["cargo.exe","test"],"findings":[{"id":"id","message":"message","severity":"warning"}],"id":"cargo","network_verified":false,"phase":"after","stderr":{"bytes":"0","head":"","redacted":true,"sha256":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","tail":"","truncated":false,"utf8":true},"stdout":{"bytes":"0","head":"","redacted":true,"sha256":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","tail":"","truncated":false,"utf8":true},"step":"test","terminal":"warn","tests_skipped":true}"#
    );
}

#[test]
fn report_reader_projects_unknown_nested_health_fields_but_evidence_is_strict() {
    let path = repo_root().join("formats/corpora/scrape/e1/valid/report-minimal.json");
    let mut report: serde_json::Value = serde_json::from_slice(&read(&path)).unwrap();
    let mut row: serde_json::Value = serde_json::from_slice(&read(
        &corpus("scrape-verification-health-evidence").join("valid/failure.json"),
    ))
    .unwrap();
    let mut row = row["rows"].as_array_mut().unwrap().remove(0);
    row["future"] = serde_json::json!(true);
    report["health"] = serde_json::json!([row.clone()]);
    assert!(
        serde_json::from_value::<vibe_wire::generated::scrape::e1::report::Report>(report).is_ok(),
        "public report reader prunes future nested health members"
    );
    assert!(
        serde_json::from_value::<VerificationHealthEvidence>(
            serde_json::json!({"schema": 2, "rows": [row]})
        )
        .is_err(),
        "durable internal evidence remains strict"
    );
}

#[test]
fn registry_exposes_both_closed_epoch2_internal_formats() {
    for (format, id) in [
        (
            FormatId::ScrapePreparedHealthSnapshot,
            "scrape-prepared-health-snapshot",
        ),
        (
            FormatId::ScrapeVerificationHealthEvidence,
            "scrape-verification-health-evidence",
        ),
    ] {
        assert_eq!(format.id(), id);
        assert_eq!(format.epoch(), 2);
        assert!(!format.recoverable());
        assert_eq!(format.foreign_parsers(), ForeignParsers::None);
        assert_eq!(format.unknown_fields(), UnknownFields::Deny);
    }
}

#[test]
fn public_epoch1_health_type_names_and_report_construction_remain_compatible() {
    use vibe_wire::generated::scrape::e1::health_result as protocol;
    use vibe_wire::generated::scrape::e1::report as public;

    let stream: public::StreamWitness = public::StreamWitness {
        bytes: "0".to_owned(),
        head: String::new(),
        redacted: true,
        sha256: "sha256:empty".to_owned(),
        tail: String::new(),
        truncated: false,
        utf8: true,
    };
    let finding: public::Finding = public::Finding {
        id: "finding".to_owned(),
        message: "message".to_owned(),
        severity: public::Severity::Info,
        evidence: None,
    };
    let health: public::HealthResult = public::HealthResult {
        argv: Vec::new(),
        findings: vec![finding],
        id: "check".to_owned(),
        network_verified: true,
        phase: public::Phase::Before,
        stderr: stream.clone(),
        stdout: stream,
        step: public::HealthResultStep::None,
        terminal: public::TerminalState::Pass,
        tests_skipped: false,
    };
    let report = public::Report {
        after_tree_digest: None,
        apply: Vec::new(),
        assurance: public::ReportAssurance::Full,
        before_tree_digest: "sha256:before".to_owned(),
        cleanup: public::ReportCleanup::Complete,
        command: public::ReportCommand::Scrape,
        deleted_artifacts: Vec::new(),
        dependency_graphs: Vec::new(),
        events: Vec::new(),
        health: vec![health],
        mode: public::ReportMode::InPlace,
        outcome: public::ReportOutcome::Verified,
        plan_id: "sha256:plan".to_owned(),
        project_display_root: "C:/project".to_owned(),
        recovery: Vec::new(),
        relocations: Vec::new(),
        residuals: Vec::new(),
        rewrites: Vec::new(),
        rollback: Vec::new(),
        schema: 1,
        transaction_id: "transaction".to_owned(),
        unchanged_files: Vec::new(),
    };
    assert_eq!(
        serde_json::to_string(&report).unwrap(),
        r#"{"after_tree_digest":null,"apply":[],"assurance":"full","before_tree_digest":"sha256:before","cleanup":"complete","command":"scrape","deleted_artifacts":[],"dependency_graphs":[],"events":[],"health":[{"argv":[],"findings":[{"id":"finding","message":"message","severity":"info"}],"id":"check","network_verified":true,"phase":"before","stderr":{"bytes":"0","head":"","redacted":true,"sha256":"sha256:empty","tail":"","truncated":false,"utf8":true},"stdout":{"bytes":"0","head":"","redacted":true,"sha256":"sha256:empty","tail":"","truncated":false,"utf8":true},"step":"none","terminal":"pass","tests_skipped":false}],"mode":"in-place","outcome":"verified","plan_id":"sha256:plan","project_display_root":"C:/project","recovery":[],"relocations":[],"residuals":[],"rewrites":[],"rollback":[],"schema":1,"transaction_id":"transaction","unchanged_files":[]}"#
    );

    let _: protocol::Finding = protocol::Finding {
        id: "protocol".to_owned(),
        message: "message".to_owned(),
        severity: protocol::Severity::Warning,
        evidence: None,
    };
}
