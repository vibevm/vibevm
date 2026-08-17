//! Wire-parity oracle for the catalog entry schema
//! (`schemas/index/e1/entry.jtd.json` → `vibe_wire::generated::index::e1::entry`).
//!
//! The writer's `VersionEntry` and the schema's root used to be two types —
//! hand-written here, JTD-generated there — and this oracle compared their
//! wire forms. The re-export unified them: the writer's type IS the schema's
//! type now, so the differential collapsed by design and the oracle's teeth
//! moved to what still has two sides. What it guards after the step:
//!
//! * the census — a fully populated record puts exactly
//!   `FULLY_POPULATED_KEY_COUNT` keys on the wire, so a schema edit that
//!   thins the `version_entry` vocabulary (a field dropped, an optional made
//!   required and skipped) changes the count here before anywhere else;
//! * the serde layer the codegen stamps — renames, skip guards and defaults
//!   must round-trip: an asymmetric attribute (skipped on write, required on
//!   read) fails `from_value`, a wrong rename drifts the key set, and the
//!   `Value`-level comparison names the drift.
//!
//! Comparison is on `serde_json::Value`, not on strings: key order is not
//! part of the contract, and `Value` equality is order-insensitive.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use semver::Version;

use vibe_index::types::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, FeaturesEntry, Group,
    I18nEntry, ObsoletesEntry, PackageKind, ProvidesEntry, RequiresAnyEntry, RequiresEntry,
    SubskillEntry, VersionEntry, WorkspaceOriginEntry,
};
use vibe_wire::generated::index::e1::entry::Entry;

/// How many keys a fully populated entry puts on the wire: 12 required
/// fields plus 21 optional ones, every optional present. Declared as a
/// constant so the fixture's own exhaustiveness is asserted, not assumed —
/// a fixture whose optional field stayed empty would prove nothing about
/// that field.
const FULLY_POPULATED_KEY_COUNT: usize = 33;

fn fixed_instant() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-15T10:15:30Z")
        .expect("the fixture timestamp parses")
        .with_timezone(&Utc)
}

/// A `VersionEntry` with every `Option` in `Some`, every collection
/// non-empty, every nested subsection filled, and both flags `true`.
fn fully_populated_entry() -> VersionEntry {
    let mut features: BTreeMap<String, Vec<String>> = BTreeMap::new();
    features.insert(
        "selene".to_string(),
        vec!["athena".to_string(), "hera".to_string()],
    );
    // An empty activation list inside a non-empty table: the map is the
    // collection, the list is its value — both shapes must survive.
    features.insert("empty-feature".to_string(), Vec::new());
    let mut exclusive: BTreeMap<String, Vec<String>> = BTreeMap::new();
    exclusive.insert(
        "pantheon".to_string(),
        vec!["zeus".to_string(), "poseidon".to_string()],
    );

    VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind: PackageKind::Flow,
        group: Group::parse("org.vibevm").expect("group parses"),
        name: "wal".to_string(),
        version: Version::parse("1.2.3").expect("version parses"),
        content_hash: "sha256:9f2c".to_string(),
        source_url: "https://gitverse.ru/vibevm/vibevm.git".to_string(),
        source_ref: "v1.2.3".to_string(),
        resolved_commit: Some("0123456789abcdef0123".to_string()),
        registry: "upstream".to_string(),
        workspace_origin: Some(WorkspaceOriginEntry {
            upstream: "https://gitverse.ru/vibevm/monorepo.git".to_string(),
            path: "packages/org.vibevm/wal".to_string(),
            commit: Some("fedcba9876543210fedc".to_string()),
            generated_by: "vibe 0.1.0".to_string(),
            generated_at: "2026-08-01T00:00:00Z".to_string(),
        }),
        license: Some("UPL-1.0".to_string()),
        authors: vec!["Oleg Chirukhin".to_string()],
        description: Some("Write-ahead log discipline".to_string()),
        homepage: Some("https://gitverse.ru/vibevm/vibevm".to_string()),
        keywords: vec!["wal".to_string(), "checkpoint".to_string()],
        describes: Some("pkg:generic/wal@1.2.3".to_string()),
        compatibility: Some(CompatibilityEntry {
            min_vibe_version: Some("0.1.0".to_string()),
            requires_kinds: vec![PackageKind::Stack],
        }),
        provides: Some(ProvidesEntry {
            capabilities: vec!["org.vibevm/wal/checkpoint".to_string()],
        }),
        requires: Some(RequiresEntry {
            packages: vec!["org.vibevm/core-ai-native".to_string()],
            capabilities: vec!["org.vibevm/wal/replay".to_string()],
        }),
        requires_any: vec![RequiresAnyEntry {
            one_of: vec![
                "org.vibevm/wal-specspaces".to_string(),
                "org.vibevm/wal".to_string(),
            ],
        }],
        obsoletes: Some(ObsoletesEntry {
            packages: vec!["org.vibevm/wal-legacy".to_string()],
        }),
        conflicts: Some(ConflictsEntry {
            packages: vec!["org.vibevm/wal-fork".to_string()],
        }),
        features: Some(FeaturesEntry {
            features,
            exclusive,
        }),
        subskills: vec![SubskillEntry {
            path: "skills/wal/v08".to_string(),
            delivery: DeliveryMode::LazyPull,
            describes: Some("pkg:generic/wal-skill@2.0.0".to_string()),
            description: Some("The v0.8 subskill".to_string()),
            channels: vec!["stable".to_string()],
        }],
        i18n: Some(I18nEntry {
            available: vec!["en".to_string(), "ru".to_string()],
            default: Some("en".to_string()),
        }),
        boot_snippet: Some(BootSnippetEntry {
            source: "boot/10-flow-wal.md".to_string(),
            category: Some("foundation".to_string()),
        }),
        files_count: 281,
        must_understand: vec!["org.vibevm/wal/tombstone@1".to_string()],
        yanked: true,
        frozen: true,
        indexed_at: fixed_instant(),
        indexed_by: "vibe-index".to_string(),
    }
}

/// The writer's output survives the schema unchanged: what the record
/// emits, the schema root accepts and hands back byte-for-byte at the
/// `Value` level — the two were independent implementations once, and
/// the comparison stays so that the day they drift apart again (a new
/// hand-written field, a schema edit) the assert names the drift.
#[test]
fn fully_populated_entry_round_trips_through_the_generated_type() {
    let handwritten = fully_populated_entry();

    let j1 = serde_json::to_value(&handwritten).expect("the writer's type serialises");
    // The fixture must not be sparse: every one of the 33 fields on the wire.
    assert_eq!(
        j1.as_object().map(|object| object.len()),
        Some(FULLY_POPULATED_KEY_COUNT),
        "the fixture must exercise every field — a sparse fixture proves nothing"
    );

    // The schema accepts everything the writer emits…
    let generated: Entry =
        serde_json::from_value(j1.clone()).expect("the generated type parses the writer's output");
    // …and nothing is lost or added on the way back.
    let j2 = serde_json::to_value(&generated).expect("the generated type serialises");
    assert_eq!(
        j1, j2,
        "wire drift between the writer's record and the schema's root — a \
         field the schema misses is silently dropped by the permissive reader"
    );
}
