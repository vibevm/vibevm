//! Authored golden documents for the epoch-1 lane analyzer report
//! (`schemas/extensions_analyze.jtd.json`, R4.3 — the attribution
//! evidence of one in-process compile, packages-2026-09 architecture
//! §9). The registry names the format `[format.extensions-analyze]`
//! with `foreign_parsers = "many"`, so the generated reader is
//! permissive — an unknown member is forward compatibility, not a wire
//! bug — and the strictness that matters lives in the hand-written
//! validator (`behaviour::extensions_analyze`): the byte-count,
//! reconciliation, stage, chain and estimator laws below.
//!
//! The corpus home is `formats/corpora/extensions-analyze/e1/`, beside
//! every other corpus; the registry path and the test path are one
//! spelling, pinned together here.

use std::path::PathBuf;

use vibe_wire::behaviour::extensions_analyze::{ExtensionsAnalyzeError, validate};
use vibe_wire::generated::extensions_analyze::ExtensionsAnalyze;
use vibe_wire::generated::format_id::{ForeignParsers, FormatId};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus() -> PathBuf {
    repo_root().join("formats/corpora/extensions-analyze/e1")
}

fn read_corpus(name: &str) -> serde_json::Value {
    let path = corpus().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} readable: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} parses: {e}"))
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus().join("valid"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn parse_and_validate(doc: serde_json::Value) -> ExtensionsAnalyze {
    let report: ExtensionsAnalyze =
        serde_json::from_value(doc).expect("parses through the generated reader");
    validate(&report).unwrap_or_else(|error| panic!("corpus document violates a law: {error}"));
    report
}

#[test]
fn every_valid_document_round_trips_and_validates() {
    for name in valid_names() {
        let authored = read_corpus(&format!("valid/{name}"));
        let report: ExtensionsAnalyze = serde_json::from_value(authored.clone())
            .unwrap_or_else(|e| panic!("{name} parses: {e}"));
        validate(&report).unwrap_or_else(|error| panic!("{name} violates a law: {error}"));
        let round_trip = serde_json::to_value(&report).unwrap();
        assert_eq!(
            round_trip, authored,
            "{name} loses data on generated round-trip"
        );
    }
}

#[test]
fn the_full_document_carries_both_stage_spellings_and_absent_estimates() {
    let report = parse_and_validate(read_corpus("valid/full.json"));
    let artifact = &report.artifacts[0];

    // The two delta members are separately named and separately spelled:
    // a lane row carries the lane pair with a null artifact pair, the
    // emitted row the mirror image.
    let lane_row = &artifact.deltas[0];
    assert_eq!(lane_row.pass, "transform:lane:org.vibevm/vibe#xml-minify");
    assert!(lane_row.lane_byte_delta.is_some());
    assert!(lane_row.artifact_byte_delta.is_none());
    let emitted_row = &artifact.deltas[1];
    assert!(emitted_row.lane_byte_delta.is_none());
    assert!(
        emitted_row.artifact_byte_delta.is_some(),
        "the emitted pair is present, not conflated into the lane member"
    );

    // Reconciliation, spelled out the way a reader recomputes it: 500 +
    // 100 contributions inside a 400-byte frame are the 1000-byte total.
    assert_eq!(artifact.total_emitted_bytes, "1000");
    assert_eq!(artifact.frame_overhead_bytes, "400");
    assert_eq!(artifact.occurrence_count, 4);

    // The absent estimator form: BOTH members null, and the nulls ride
    // the wire (a required-nullable member, not an omitted one).
    assert!(artifact.token_estimate.is_none());
    assert!(artifact.estimator_id.is_none());
    let wire = serde_json::to_value(artifact).unwrap();
    assert_eq!(wire["token_estimate"], serde_json::Value::Null);
    assert_eq!(wire["estimator_id"], serde_json::Value::Null);

    // The provider one-of's coordinate arms, tagged the way the compiler
    // IR wire tags its own provider shape.
    let wire = serde_json::to_value(&artifact.contributions[0].provider).unwrap();
    assert_eq!(wire["kind"], "dependency");
    assert_eq!(wire["group"], "org.vibevm.core");
    assert_eq!(wire["name"], "vibe");
}

#[test]
fn the_minimal_document_is_the_empty_lawful_answer() {
    let report = parse_and_validate(read_corpus("valid/minimal.json"));
    assert!(report.artifacts.is_empty());
    let wire = serde_json::to_value(&report).unwrap();
    // `x-empty: emit` — the empty list is written, not skipped.
    assert_eq!(wire["artifacts"], serde_json::json!([]));
}

#[test]
fn a_token_estimate_without_an_estimator_refuses() {
    let doc = read_corpus("invalid/token_estimate_without_estimator_id.json");
    let report: ExtensionsAnalyze = serde_json::from_value(doc)
        .expect("the permissive reader accepts the member; the cell refuses it");
    assert!(matches!(
        validate(&report),
        Err(ExtensionsAnalyzeError::EstimatorCoupling {
            estimate_is_some: true,
            estimator_id: None,
            ..
        })
    ));
}

#[test]
fn registry_record_is_pinned() {
    let text = std::fs::read_to_string(repo_root().join("formats/REGISTRY.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    let formats = parsed
        .get("format")
        .and_then(|v| v.as_table())
        .expect("formats/REGISTRY.toml has a [format.*] table");
    let record = &formats["extensions-analyze"];
    assert_eq!(record.get("epoch").unwrap().as_integer(), Some(1));
    assert_eq!(
        record.get("schema").unwrap().as_str(),
        Some("schemas/extensions_analyze.jtd.json")
    );
    assert_eq!(record.get("recoverable").unwrap().as_bool(), Some(true));
    assert_eq!(
        record.get("foreign_parsers").unwrap().as_str(),
        Some("many")
    );
    assert_eq!(
        record.get("corpus").unwrap().as_str(),
        Some("formats/corpora/extensions-analyze/e1")
    );
    assert_eq!(record.get("sunset").unwrap().as_str(), Some("none"));

    // The pinned paths exist on disk.
    assert!(
        repo_root()
            .join("schemas/extensions_analyze.jtd.json")
            .is_file()
    );
    assert!(corpus().join("valid").is_dir());
    assert!(corpus().join("invalid").is_dir());

    // The generated FormatId agrees with the record.
    let variant = FormatId::ALL
        .iter()
        .copied()
        .find(|id| id.id() == "extensions-analyze")
        .expect("FormatId carries the extensions-analyze variant");
    assert_eq!(variant.epoch(), 1);
    assert!(variant.recoverable());
    assert_eq!(variant.foreign_parsers(), ForeignParsers::Many);
}
