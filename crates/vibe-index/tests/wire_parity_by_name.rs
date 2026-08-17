//! Wire-parity oracle for the `by-name` candidate-set schema
//! (`schemas/index/e1/by_name.jtd.json` →
//! `vibe_wire::generated::index::e1::by_name`).
//!
//! The writer's `NameEntry` and the schema's root used to be two types —
//! hand-written here, JTD-generated `ByName` there — and this oracle
//! compared their wire forms. The re-export unified them (the schema's
//! `x-rust-type` names `NameEntry` and the writer re-exports it), so the
//! differential collapsed by design and the oracle's teeth moved to what
//! still has two sides. What it guards after the step:
//!
//! * the census at every depth — 4 keys on the name entry, 5 on each
//!   package, 33 on each version record, so a schema edit that thins the
//!   aggregate (a `package_entry` field dropped) or the shared
//!   `version_entry` vocabulary changes a count here before anywhere
//!   else; depth is the specific risk, because the record three levels
//!   down is shared vocabulary and a field it lost would surface nowhere
//!   else;
//! * the serde layer the codegen stamps — renames and skip guards must
//!   round-trip through `Value`, and the comparison names any drift.
//!
//! The fixture deliberately carries BOTH a non-empty `packages` list and
//! a filled `tombstone`: the real writer never emits both (a name
//! answers with candidates, a redirect, or a death record — PROP-044 §2),
//! but a parity fixture must exercise every optional branch, and
//! semantics is the writer's business, not the schema's.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use semver::Version;

use vibe_index::types::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, FeaturesEntry, Group,
    I18nEntry, NameEntry, ObsoletesEntry, PackageEntry, PackageKind, ProvidesEntry,
    RequiresAnyEntry, RequiresEntry, SubskillEntry, Tombstone, VersionEntry, WorkspaceOriginEntry,
};
use vibe_wire::generated::index::e1::by_name::NameEntry as GeneratedNameEntry;

/// Key counts a fully populated fixture puts on the wire at each nesting
/// level: 4 on the `NameEntry` itself, 5 on each `PackageEntry`, 33 on
/// each `VersionEntry` (12 required + 21 optional, every optional
/// present — the same closure the entry oracle guards). Declared as
/// constants so the fixture's exhaustiveness is asserted at every depth,
/// not assumed: an optional branch that quietly stayed `None` would
/// prove nothing about the field the schema forgot.
const NAME_ENTRY_KEY_COUNT: usize = 4;
const PACKAGE_ENTRY_KEY_COUNT: usize = 5;
const VERSION_ENTRY_KEY_COUNT: usize = 33;

fn fixed_instant() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-15T10:15:30Z")
        .expect("the fixture timestamp parses")
        .with_timezone(&Utc)
}

/// A `VersionEntry` with every `Option` in `Some`, every collection
/// non-empty, every nested subsection filled, and both flags `true` —
/// the same fully populated record the entry oracle transcribes.
fn fully_populated_version() -> VersionEntry {
    let mut features: BTreeMap<String, Vec<String>> = BTreeMap::new();
    features.insert(
        "selene".to_string(),
        vec!["athena".to_string(), "hera".to_string()],
    );
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

/// A `NameEntry` with both optional branches taken: a candidate package
/// whose own optionals (`latest_stable`, the whole version record) are
/// filled, and a tombstone carrying its own optional successor.
fn fully_populated_name_entry() -> NameEntry {
    NameEntry {
        name: "wal".to_string(),
        indexed_at: fixed_instant(),
        packages: vec![PackageEntry {
            group: Group::parse("org.vibevm").expect("group parses"),
            name: "wal".to_string(),
            indexed_at: fixed_instant(),
            latest_stable: Some(Version::parse("1.2.3").expect("version parses")),
            versions: vec![fully_populated_version()],
        }],
        tombstone: Some(Tombstone {
            reason: "absorbed into the monorepo".to_string(),
            superseded_by: Some("org.vibevm/wal".to_string()),
        }),
    }
}

/// The writer's output survives the schema unchanged at every depth:
/// what the record emits, the schema root accepts and hands back
/// byte-for-byte at the `Value` level. The two were independent
/// implementations once, and the comparison stays so that the day they
/// drift apart again the assert names the drift — at any depth,
/// including inside a version record three levels down.
#[test]
fn fully_populated_name_entry_round_trips_through_the_generated_type() {
    let handwritten = fully_populated_name_entry();

    let j1 = serde_json::to_value(&handwritten).expect("the writer's type serialises");
    assert_eq!(
        j1.as_object().map(|object| object.len()),
        Some(NAME_ENTRY_KEY_COUNT),
        "the fixture must exercise every top-level field — a sparse fixture proves nothing"
    );
    assert_eq!(
        j1["packages"][0].as_object().map(|object| object.len()),
        Some(PACKAGE_ENTRY_KEY_COUNT),
        "the nested package must exercise every field, `latest_stable` included"
    );
    assert_eq!(
        j1["packages"][0]["versions"][0]
            .as_object()
            .map(|object| object.len()),
        Some(VERSION_ENTRY_KEY_COUNT),
        "the nested version record must stay fully populated inside the aggregate"
    );

    // The schema accepts everything the writer emits…
    let generated: GeneratedNameEntry =
        serde_json::from_value(j1.clone()).expect("the generated type parses the writer's output");
    // …and nothing is lost or added on the way back.
    let j2 = serde_json::to_value(&generated).expect("the generated type serialises");
    assert_eq!(
        j1, j2,
        "wire drift between the writer's record and the schema's root — a \
         field the schema misses (at any depth) is silently dropped by the \
         permissive reader"
    );
}
