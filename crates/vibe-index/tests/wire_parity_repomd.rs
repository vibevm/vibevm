//! Differential wire-parity oracle for the catalog manifest schema
//! (`schemas/index/e1/repomd.jtd.json` →
//! `vibe_wire::generated::index::e1::repomd`).
//!
//! Same law as the other wire-parity oracles: the hand-written `Repomd`
//! and the JTD-generated one are *meant* to differ as Rust types — the
//! generated union comes out as newtype variants over separate structs
//! (`Directory(RepomdFileEntryDirectory)`, `File(RepomdFileEntryFile)`),
//! which serde's internally-tagged representation puts on the wire exactly
//! like the hand-written struct variants — but the **wire** must not
//! differ. The specific risk this oracle guards is the tagged union: a
//! mapping arm the schema forgot, or a field inside an arm, is invisible
//! to any single-arm spot check, so the fixture exercises BOTH arms and
//! the asserts count keys on each.
//!
//! `size` rides the wire as a canonical decimal **string** — the owner's
//! standing rule for integers wider than 32 bits (2026-08-20, BACKLOG
//! `B-091` «вариант (б)»; `formats/breaks/003.md`): JTD has no 64-bit
//! integer, so the schema says `string` and the value carries the number.
//! The schema and the writer agree on this field again — the `uint32` era's
//! recorded divergence is closed. The fixture pins the form with a size
//! ABOVE the old `uint32` ceiling, which the numeric wire could not have
//! carried truthfully at all.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use vibe_index::types::{NamingConvention, Repomd, RepomdFileEntry};
use vibe_wire::generated::index::e1::repomd::Repomd as GeneratedRepomd;

/// How many keys a fully populated manifest puts on the wire: nine
/// required fields, no optionals. Declared as a constant so the
/// fixture's exhaustiveness is asserted, not assumed.
const REPOMD_KEY_COUNT: usize = 9;
/// Key count per union arm on the wire: the `kind` tag plus the arm's
/// own fields — three for `file` (size + sha256), two for `directory`
/// (entries). Counted per arm so a fixture (or schema) that thinned one
/// is caught on each arm independently.
const FILE_ARM_KEY_COUNT: usize = 3;
const DIRECTORY_ARM_KEY_COUNT: usize = 2;
/// The `files` map holds exactly the two fixture entries — one per union
/// arm. An oracle that exercised one arm would prove nothing about the
/// other, so the count is asserted, not assumed.
const FILES_MAP_LEN: usize = 2;

fn fixed_instant() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-15T10:15:30Z")
        .expect("the fixture timestamp parses")
        .with_timezone(&Utc)
}

/// A `Repomd` with every collection non-empty and BOTH union arms
/// exercised: a `file` entry with a size ABOVE the retired `uint32`
/// ceiling and a real hash, and a `directory` entry with a non-zero
/// count.
fn fully_populated_repomd() -> Repomd {
    let mut files: BTreeMap<String, RepomdFileEntry> = BTreeMap::new();
    files.insert(
        "primary.jsonl".to_string(),
        RepomdFileEntry::file(4_294_967_296, "sha256:9f2c"),
    );
    files.insert("by-name".to_string(), RepomdFileEntry::directory(42));
    Repomd {
        schema_version: Repomd::SCHEMA_VERSION,
        registry: "vibespecs".to_string(),
        registry_url: "https://github.com/vibespecs".to_string(),
        // A hyphenated convention pins the one wire string a "simplified"
        // rename rule would silently break.
        naming: NamingConvention::KindName,
        generated_at: fixed_instant(),
        generator: "vibe-index 0.1.0-dev".to_string(),
        package_count: 3,
        version_count: 5,
        files,
    }
}

/// The writer's output survives the schema unchanged: what the
/// hand-written type emits, the generated type accepts and hands back
/// byte-for-byte at the `Value` level — including both union arms. A
/// mapping arm the schema forgot makes `from_value` itself fail (an
/// unknown `kind` tag); a field inside an arm the schema forgot is
/// dropped by the permissive generated reader and caught by the assert.
#[test]
fn fully_populated_repomd_round_trips_through_the_generated_type() {
    let handwritten = fully_populated_repomd();

    let j1 = serde_json::to_value(&handwritten).expect("the hand-written type serialises");
    assert_eq!(
        j1.as_object().map(|object| object.len()),
        Some(REPOMD_KEY_COUNT),
        "the fixture must exercise every top-level field — a sparse fixture proves nothing"
    );
    // Both union arms present, each with the tag plus its own fields.
    let files = j1["files"].as_object().expect("files is a JSON object");
    assert_eq!(
        files.len(),
        FILES_MAP_LEN,
        "both union arms must be exercised"
    );
    assert_eq!(
        j1["naming"].as_str(),
        Some("kind-name"),
        "the fixture pins the hyphenated naming wire string"
    );
    for (path, expected_kind, expected_keys) in [
        ("primary.jsonl", "file", FILE_ARM_KEY_COUNT),
        ("by-name", "directory", DIRECTORY_ARM_KEY_COUNT),
    ] {
        let entry = &files[path];
        assert_eq!(
            entry.as_object().map(|object| object.len()),
            Some(expected_keys),
            "the {expected_kind} arm must carry the tag plus its own fields: {entry:?}"
        );
        assert_eq!(
            entry["kind"].as_str(),
            Some(expected_kind),
            "the union tag must be on the wire for {path}"
        );
    }

    // The schema accepts everything the writer emits…
    let generated: GeneratedRepomd =
        serde_json::from_value(j1.clone()).expect("the generated type parses the writer's output");
    // …and nothing is lost or added on the way back.
    let j2 = serde_json::to_value(&generated).expect("the generated type serialises");
    assert_eq!(
        j1, j2,
        "wire drift between the hand-written and the generated manifest — a \
         mapping arm or field the schema misses is dropped (or rejected) by \
         the generated reader"
    );
}

/// The break itself, refused loudly (formats/breaks/003.md): a document
/// of the OLD wire form — `"size"` as a JSON **number** — is rejected
/// by the generated reader with a TYPE error, not coerced into a
/// string or quietly parsed as `0`. The fixture is a full manifest so
/// the refusal is attributable to the `size` field specifically: the
/// string form of the same document parses (the round-trip test
/// above), and only the number/string distinction differs.
#[test]
fn the_generated_reader_refuses_the_old_numeric_size_form() {
    let handwritten = fully_populated_repomd();
    let mut j = serde_json::to_value(&handwritten).expect("the hand-written type serialises");
    // Rewrite one file entry's size into the pre-2026-08-20 numeric
    // form — the document every older binary wrote.
    j["files"]["primary.jsonl"]["size"] = serde_json::json!(4294967296u64);

    let parsed = serde_json::from_value::<GeneratedRepomd>(j);
    let err = parsed.expect_err("the old numeric size form must be refused");
    let msg = err.to_string();
    // The generated field is `size: String`, so the numeric form dies as
    // a TYPE error at that field — serde's Value-path message names the
    // types, not the field, and that is the load-bearing part: the old
    // form is refused, never coerced into a string or a quiet `0`.
    assert!(
        msg.to_lowercase().contains("invalid type")
            && msg.to_lowercase().contains("expected a string"),
        "the refusal must be a TYPE error, not a coercion: {msg}"
    );
}
