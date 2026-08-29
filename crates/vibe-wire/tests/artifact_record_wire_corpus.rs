//! Authored golden documents for the epoch-1 artifact record
//! (`schemas/artifact_record.jtd.json`, R8A2 — the durable record of
//! one produced artifact, packages-2026-09 architecture §4). The
//! registry names the format `[format.artifact-record]` with
//! `foreign_parsers = "none"`, so the generated reader is STRICT: an
//! unknown member is a wire bug, and every refusal below names the
//! member it refused.
//!
//! Two kinds of check sit beside each other, and they are not the same
//! kind. READER checks prove the strict generated reader refuses an
//! unknown member while naming it; CELL checks prove the hand-written
//! validator (`behaviour::artifact_record`) refuses a broken scalar
//! with the typed error naming the member — the corpus documents are
//! the shared fixtures both halves cite.

use std::path::PathBuf;

use vibe_wire::behaviour::artifact_record::{
    AbsolutePathUnsafety, ArtifactRecordError, MechanismDefect, validate,
};
use vibe_wire::generated::artifact_record::{ArtifactRecord, DigestAlgorithm, VerificationStatus};
use vibe_wire::generated::format_id::{ForeignParsers, FormatId};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus() -> PathBuf {
    repo_root().join("formats/corpora/artifact-record/e1")
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

fn parse_and_validate(doc: serde_json::Value) -> ArtifactRecord {
    let record: ArtifactRecord =
        serde_json::from_value(doc).expect("parses through the generated reader");
    validate(&record).unwrap_or_else(|error| panic!("corpus document violates a law: {error}"));
    record
}

#[test]
fn every_valid_document_round_trips_and_validates() {
    for name in valid_names() {
        let authored = read_corpus(&format!("valid/{name}"));
        let record: ArtifactRecord = serde_json::from_value(authored.clone())
            .unwrap_or_else(|e| panic!("{name} parses: {e}"));
        validate(&record).unwrap_or_else(|error| panic!("{name} violates a law: {error}"));
        let round_trip = serde_json::to_value(&record).unwrap();
        assert_eq!(
            round_trip, authored,
            "{name} loses data on generated round-trip"
        );
    }
}

#[test]
fn the_minimal_record_carries_exactly_what_the_wire_demands() {
    let record = parse_and_validate(read_corpus("valid/minimal.json"));
    assert_eq!(record.schema, 1);
    assert_eq!(record.id, "vibe-helper.exe");
    // The provider is reduced to its key and the freshness object is
    // present but empty — the provider-fresh record — while the
    // round-trip keeps BOTH exactly (an empty object is a statement,
    // not an omission).
    assert_eq!(record.producer.provider.version, None);
    assert_eq!(record.freshness.inputs, None);
    assert_eq!(record.freshness.config, None);
    assert_eq!(record.freshness.toolchain, None);
    let wire = serde_json::to_value(&record).unwrap();
    assert!(wire.get("media_type").is_none());
    assert!(wire.get("platform").is_none());
    assert_eq!(wire["freshness"], serde_json::json!({}));
}

#[test]
fn the_full_record_carries_every_optional_member() {
    let record = parse_and_validate(read_corpus("valid/full.json"));
    assert_eq!(record.media_type.as_deref(), Some("application/zip"));
    assert_eq!(record.platform.as_deref(), Some("x86_64-pc-windows-msvc"));
    assert!(record.freshness.inputs.is_some());
    assert!(record.freshness.config.is_some());
    assert!(record.freshness.toolchain.is_some());
    assert_eq!(record.verification.status, VerificationStatus::Verified);
    assert!(record.verification.evidence.is_some());
    assert!(record.producer.provider.version.is_some());
    assert!(record.producer.provider.content_hash.is_some());
}

#[test]
fn a_directory_artifact_carries_the_tree_digest() {
    let record = parse_and_validate(read_corpus("valid/skill_directory.json"));
    assert_eq!(record.digest.algorithm, DigestAlgorithm::Sha256Tree);
    let wire = serde_json::to_value(&record).unwrap();
    assert_eq!(wire["digest"]["algorithm"], "sha256-tree/1");
    // A directory carries no media type — the tree IS the artifact.
    assert!(wire.get("media_type").is_none());
}

#[test]
fn the_reader_refuses_an_unknown_member_and_names_it() {
    let doc = read_corpus("invalid/unknown_field.json");
    let error = serde_json::from_value::<ArtifactRecord>(doc)
        .expect_err("an unknown member must be refused");
    let text = format!("{error}");
    assert!(text.contains("unknown field"), "reader says: {text}");
    assert!(text.contains("checksum"), "reader names the member: {text}");
}

#[test]
fn a_short_digest_is_a_typed_refusal_naming_the_member() {
    let doc = read_corpus("invalid/digest_short.json");
    let record: ArtifactRecord =
        serde_json::from_value(doc).expect("a short digest parses; the cell refuses it");
    assert_eq!(
        validate(&record),
        Err(ArtifactRecordError::DigestValueNotHex {
            value: vibe_wire::behaviour::compiler_trace_index::ScalarPreview::of(
                "00112233445566778899aabbccddeeff00112233445566778899aabbccddee"
            ),
        })
    );
}

#[test]
fn a_blank_required_scalar_is_a_typed_refusal_naming_the_member() {
    let doc = read_corpus("invalid/blank_id.json");
    let record: ArtifactRecord =
        serde_json::from_value(doc).expect("a blank id parses; the cell refuses it");
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::IdNotPortableToken { .. })
    ));
}

#[test]
fn the_three_scalar_laws_are_three_separate_refusals() {
    let base = read_corpus("valid/minimal.json");

    // (1) A blank free-text scalar.
    let mut blank = base.clone();
    blank["media_type"] = serde_json::json!("  ");
    let record: ArtifactRecord = serde_json::from_value(blank).unwrap();
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::UnsafeScalar { field, .. }) if field == "media_type"
    ));

    // (2) A 63-hex digest.
    let mut short = base.clone();
    short["digest"]["value"] =
        serde_json::json!("00112233445566778899aabbccddeeff00112233445566778899aabbccddee");
    let record: ArtifactRecord = serde_json::from_value(short).unwrap();
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::DigestValueNotHex { .. })
    ));

    // (3) A backslash in the relative path.
    let mut backslashed = base;
    backslashed["path_relative"]["path"] = serde_json::json!("target\\release\\vibe-helper.exe");
    let record: ArtifactRecord = serde_json::from_value(backslashed).unwrap();
    let error = validate(&record).expect_err("a backslashed relative path must refuse");
    assert!(matches!(
        &error,
        ArtifactRecordError::UnsafeRelativePath { path, .. }
            if path.head() == "target\\release\\vibe-helper.exe"
    ));
    // The refusal reads as the backslash arm, not a neighbour arm.
    let text = format!("{error}");
    assert!(
        text.contains("backslash"),
        "the refusal names the separator law: {text}"
    );
    // …and the same value with forward slashes validates — the law is
    // the separator spelling, not the path itself.
}

#[test]
fn the_absolute_path_and_mechanism_laws_are_cited_from_the_corpus() {
    let base = read_corpus("valid/minimal.json");

    let mut relative = base.clone();
    relative["path_absolute"] = serde_json::json!("target/release/vibe-helper.exe");
    let record: ArtifactRecord = serde_json::from_value(relative).unwrap();
    assert_eq!(
        validate(&record),
        Err(ArtifactRecordError::UnsafeAbsolutePath {
            path: vibe_wire::behaviour::compiler_trace_index::ScalarPreview::of(
                "target/release/vibe-helper.exe"
            ),
            reason: AbsolutePathUnsafety::NotAbsolute,
        })
    );

    for (mechanism, reason) in [
        ("cargo", MechanismDefect::MissingRolePrefix),
        ("deploy:vibe-bin", MechanismDefect::UnknownRole),
    ] {
        let mut bad = base.clone();
        bad["producer"]["mechanism"] = serde_json::json!(mechanism);
        let record: ArtifactRecord = serde_json::from_value(bad).unwrap();
        assert_eq!(
            validate(&record),
            Err(ArtifactRecordError::BadMechanismKey {
                mechanism: vibe_wire::behaviour::compiler_trace_index::ScalarPreview::of(mechanism),
                reason,
            }),
            "{mechanism} must refuse as {reason:?}"
        );
    }
}

#[test]
fn registry_record_is_pinned() {
    let text = std::fs::read_to_string(repo_root().join("formats/REGISTRY.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    let formats = parsed
        .get("format")
        .and_then(|v| v.as_table())
        .expect("formats/REGISTRY.toml has a [format.*] table");
    let record = &formats["artifact-record"];
    assert_eq!(record.get("epoch").unwrap().as_integer(), Some(1));
    assert_eq!(
        record.get("schema").unwrap().as_str(),
        Some("schemas/artifact_record.jtd.json")
    );
    assert_eq!(record.get("recoverable").unwrap().as_bool(), Some(false));
    assert_eq!(
        record.get("foreign_parsers").unwrap().as_str(),
        Some("none")
    );
    assert_eq!(
        record.get("corpus").unwrap().as_str(),
        Some("formats/corpora/artifact-record/e1")
    );
    assert_eq!(record.get("sunset").unwrap().as_str(), Some("none"));

    // The pinned paths exist on disk.
    assert!(
        repo_root()
            .join("schemas/artifact_record.jtd.json")
            .is_file()
    );
    assert!(corpus().join("valid").is_dir());
    assert!(corpus().join("invalid").is_dir());

    // The generated FormatId agrees with the record.
    let variant = FormatId::ALL
        .iter()
        .copied()
        .find(|id| id.id() == "artifact-record")
        .expect("FormatId carries the artifact-record variant");
    assert_eq!(variant.epoch(), 1);
    assert!(!variant.recoverable());
    assert_eq!(variant.foreign_parsers(), ForeignParsers::None);
}
