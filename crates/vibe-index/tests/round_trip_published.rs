//! Round-trip oracles for the three published surfaces that had a
//! writer but no reader until now: `by-cap/<slug>.jsonl`,
//! `by-purl/<slug>.jsonl` and `primary.jsonl.gz`. G11 (PROP-044 §8):
//! a published format carries a reader minted from its schema and
//! proven by a round-trip — a format that is only written drifts
//! silently, because nobody is left to complain.
//!
//! The two inverted surfaces are still WRITTEN through their
//! hand-written twins (`CapabilityRow` / `PurlRow`) and READ into the
//! schema-generated `ByCap` / `ByPurl`, so each comparison is field by
//! field — different structs by name, and that seam is exactly what
//! the round-trip guards. The gz surface wraps the plain one; its test
//! asserts both surfaces give the SAME records, because that equality
//! is what makes `.gz` a transport envelope rather than a second
//! format.

use semver::Version;

use vibe_index::Error;
use vibe_index::index::inverted::{
    BindingSite, CapabilityRow, PurlRow, capability_slug, parse_capability, parse_purl, purl_file,
    purl_slug, read_capability, read_purl, write_capability, write_purl,
};
use vibe_index::index::primary::{FILENAME, parse_gz, read, read_gz, write};
use vibe_index::types::{
    BootSnippetEntry, DeliveryMode, Group, PackageKind, ProvidesEntry, SubskillEntry, VersionEntry,
};

/// Every key a by-cap row puts on the wire — all-required schema, so
/// this is also the exhaustive field census the fixture must fill.
const CAPABILITY_ROW_KEY_COUNT: usize = 5;
/// Same census for a by-purl row (one more field: `binding_site`).
const PURL_ROW_KEY_COUNT: usize = 6;

fn fully_populated_capability_rows() -> Vec<CapabilityRow> {
    vec![
        CapabilityRow {
            kind: PackageKind::Feat,
            group: Group::parse("org.vibevm").expect("group parses"),
            name: "wal".to_string(),
            version: Version::parse("0.1.0").expect("version parses"),
            capability: "interface:wal".to_string(),
        },
        CapabilityRow {
            kind: PackageKind::Stack,
            group: Group::parse("org.vibevm").expect("group parses"),
            name: "rust".to_string(),
            version: Version::parse("0.2.0").expect("version parses"),
            capability: "interface:wal".to_string(),
        },
    ]
}

fn fully_populated_purl_rows() -> Vec<PurlRow> {
    // Both vocabulary values in one file: a package-level binding and
    // a subskill-level binding of the same PURL.
    vec![
        PurlRow {
            kind: PackageKind::Flow,
            group: Group::parse("org.vibevm").expect("group parses"),
            name: "sqlx-skin".to_string(),
            version: Version::parse("0.1.0").expect("version parses"),
            purl: "pkg:cargo/sqlx@0.8.0".to_string(),
            binding_site: BindingSite::Package,
        },
        PurlRow {
            kind: PackageKind::Stack,
            group: Group::parse("org.vibevm").expect("group parses"),
            name: "rust".to_string(),
            version: Version::parse("0.2.0").expect("version parses"),
            purl: "pkg:cargo/sqlx@0.8.0".to_string(),
            binding_site: BindingSite::Subskill,
        },
    ]
}

fn fully_populated_entry(name: &str, version: &str) -> VersionEntry {
    VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind: PackageKind::Flow,
        group: Group::parse("org.vibevm").expect("group parses"),
        name: name.to_string(),
        version: Version::parse(version).expect("version parses"),
        content_hash: format!("sha256:{name}-{version}"),
        source_url: format!("https://example.invalid/{name}.git"),
        source_ref: format!("v{version}"),
        resolved_commit: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
        registry: "vibespecs".to_string(),
        workspace_origin: None,
        license: Some("UPL-1.0".to_string()),
        authors: vec!["Oleg Chirukhin".to_string()],
        description: Some(format!("{name} — round-trip fixture")),
        homepage: Some("https://example.invalid/".to_string()),
        keywords: vec!["fixture".to_string()],
        describes: Some("pkg:cargo/sqlx@0.8.0".to_string()),
        compatibility: None,
        provides: Some(ProvidesEntry {
            capabilities: vec!["interface:wal".to_string()],
        }),
        requires: None,
        requires_any: vec![],
        obsoletes: None,
        conflicts: None,
        features: None,
        subskills: vec![SubskillEntry {
            path: "sub/reader".to_string(),
            delivery: DeliveryMode::LazyPull,
            describes: Some("pkg:cargo/sqlx@0.8.1".to_string()),
            description: None,
            channels: vec![],
        }],
        i18n: None,
        boot_snippet: Some(BootSnippetEntry {
            source: format!("boot/{name}.md"),
            category: Some("flow".to_string()),
        }),
        files_count: 3,
        must_understand: vec![],
        yanked: false,
        frozen: false,
        indexed_at: chrono::DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .expect("timestamp parses")
            .with_timezone(&chrono::Utc),
        indexed_by: "round-trip fixture".to_string(),
    }
}

#[test]
fn by_cap_round_trips_through_the_generated_reader() {
    let dir = tempfile::tempdir().expect("tempdir");
    let rows = fully_populated_capability_rows();

    // Census first: the fixture must put every field of the row on the
    // wire, or the round-trip below would quietly prove nothing.
    for row in &rows {
        let wire = serde_json::to_value(row).expect("the hand-written row serialises");
        assert_eq!(
            wire.as_object().map(|object| object.len()),
            Some(CAPABILITY_ROW_KEY_COUNT),
            "the fixture must exercise every field of the row"
        );
    }

    let slug = capability_slug("interface:wal");
    write_capability(dir.path(), &slug, &rows).expect("by-cap writes");
    let back =
        read_capability(dir.path(), &slug).expect("the generated reader reads its writer's output");
    assert_eq!(back.len(), rows.len(), "one line in, one row out");
    for (written, read) in rows.iter().zip(&back) {
        assert_eq!(read.kind, written.kind);
        assert_eq!(read.group, written.group);
        assert_eq!(read.name, written.name);
        assert_eq!(read.version, written.version);
        assert_eq!(read.capability, written.capability);
    }
}

#[test]
fn by_purl_round_trips_both_binding_sites() {
    let dir = tempfile::tempdir().expect("tempdir");
    let rows = fully_populated_purl_rows();

    for row in &rows {
        let wire = serde_json::to_value(row).expect("the hand-written row serialises");
        assert_eq!(
            wire.as_object().map(|object| object.len()),
            Some(PURL_ROW_KEY_COUNT),
            "the fixture must exercise every field of the row"
        );
    }

    let slug = purl_slug("pkg:cargo/sqlx@0.8.0");
    write_purl(dir.path(), &slug, &rows).expect("by-purl writes");

    // Pin the writer's dictionary wire before reading it back: the
    // binding-site vocabulary travels `kebab-case`, and both of its
    // values are in this file.
    let raw = std::fs::read_to_string(purl_file(dir.path(), &slug)).expect("by-purl file reads");
    assert!(
        raw.contains("\"binding_site\":\"package\"")
            && raw.contains("\"binding_site\":\"subskill\""),
        "both binding-site wire strings must be on disk, was: {raw}"
    );

    let back =
        read_purl(dir.path(), &slug).expect("the generated reader reads its writer's output");
    assert_eq!(back.len(), rows.len(), "one line in, one row out");
    assert_eq!(back[0].binding_site, BindingSite::Package);
    assert_eq!(back[1].binding_site, BindingSite::Subskill);
    for (written, read) in rows.iter().zip(&back) {
        assert_eq!(read.kind, written.kind);
        assert_eq!(read.group, written.group);
        assert_eq!(read.name, written.name);
        assert_eq!(read.version, written.version);
        assert_eq!(read.purl, written.purl);
        assert_eq!(read.binding_site, written.binding_site);
    }
}

#[test]
fn gz_surface_round_trips_and_matches_the_plain_surface() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut entries = vec![
        fully_populated_entry("wal", "0.1.0"),
        fully_populated_entry("atomic-commits", "0.2.0"),
    ];
    write(dir.path(), &mut entries).expect("primary writes both surfaces");
    let plain = read(dir.path()).expect("the plain surface reads");

    // The envelope must stand on its own bytes: with the plain file
    // gone, a passing read_gz proves it decompressed the `.gz` file,
    // not its sibling's.
    std::fs::remove_file(dir.path().join(FILENAME)).expect("the plain file is removed");
    let gz = read_gz(dir.path()).expect("the gz reader reads its writer's output");
    assert_eq!(gz.len(), entries.len(), "one line in, one entry out");
    assert_eq!(
        gz, entries,
        "the gz surface must parse back to the records that were written"
    );

    // THE envelope claim, asserted directly: compressed and plain
    // surfaces give the same records — that equality is what makes
    // `.gz` a transport wrapper, not a second format.
    assert_eq!(
        gz, plain,
        "the gz surface must equal the plain surface record for record"
    );
}

#[test]
fn inverted_parsers_name_the_offending_line() {
    // Line 1 parses, line 2 does not — the refusal must carry "line 2"
    // (the primary surface's message form, borrowed not reinvented).
    let good =
        serde_json::to_string(&fully_populated_capability_rows()[0]).expect("the row serialises");
    let bytes = format!("{good}\n{{\"not a valid row\":true}}\n");
    let err = parse_capability(bytes.as_bytes()).expect_err("line 2 must be refused");
    match err {
        Error::Malformed(message) => assert!(
            message.contains("line 2"),
            "the refusal must name the line, was: {message}"
        ),
        other => panic!("unexpected error: {other:?}"),
    }

    // Same law for by-purl, with a blank line 2 in between — blanks
    // are skipped (the neighbour's rule), so the refusal is line 3.
    let good = serde_json::to_string(&fully_populated_purl_rows()[0]).expect("the row serialises");
    let bytes = format!("{good}\n\n{{\"not a valid row\":true}}\n");
    let err = parse_purl(bytes.as_bytes()).expect_err("line 3 must be refused");
    match err {
        Error::Malformed(message) => assert!(
            message.contains("line 3"),
            "the refusal must name the line, was: {message}"
        ),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn gz_reader_refuses_non_gzip_bytes() {
    let err = parse_gz(b"not gzip at all\n").expect_err("garbage must be refused, not guessed");
    assert!(
        matches!(err, Error::Malformed(_)),
        "unexpected error: {err:?}"
    );
}
