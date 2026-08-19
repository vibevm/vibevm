//! Differential wire-parity oracle for the journal schema
//! (`schemas/journal/e1/journal.jtd.json` →
//! `vibe_wire::generated::journal::e1::journal`).
//!
//! Same law as the other wire-parity oracles: the hand-written
//! `JournalRecord` and the JTD-generated one are *meant* to differ as Rust
//! types — the generated union comes out as newtype variants over boxed
//! per-arm structs (`Frozen(Box<EventFrozen>)`, …), which serde's
//! internally-tagged representation puts on the wire exactly like the
//! hand-written struct variants — but the **wire** must not differ. The
//! specific risk this oracle guards is an ELEVEN-arm tagged union: a
//! mapping arm the schema forgot, or a field inside an arm, is invisible
//! to any single-arm spot check, so the fixture exercises ALL eleven arms
//! and the asserts count keys on each.
//!
//! ONE place where the schema is deliberately wider than the Rust type,
//! recorded in the schema and re-stated here so the fixture's choice is
//! not read as an accident:
//!
//! * `removed.version` is `Option<Version>` with no `skip_serializing_if`
//!   — the writer ALWAYS emits the key (`null` for a whole-package
//!   removal), while the generated `Option` skips on `None`. The two forms
//!   agree only for `Some`, so the fixture carries `Some` — exactly the
//!   value a parity oracle can and should prove.
//!
//! There were two. The second was `renamed.from`/`to`, Rust pairs
//! `(Group, String)` that JTD (RFC 8927) can only describe as string
//! arrays of ANY length, so the schema could not check arity and this
//! oracle pinned it by hand. That arm left the vocabulary with the
//! retirement collapse, and the widening left with it — nothing here
//! replaces the pin because nothing carries a tuple any more.
//!
//! Its successor is the counter-example worth naming: `buried` holds an
//! optional `superseded_by` and BOTH sides skip it on `None`, so the two
//! forms agree on either value rather than only on one. That is the shape
//! `removed.version` still owes (`BACKLOG.md` B-078), and the fixture
//! below carries `Some` for the same reason `removed` does — a present
//! value is what a parity oracle can prove — not because absence would
//! diverge.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use semver::Version;

use vibe_index::journal::{Event, JournalRecord};
use vibe_index::types::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, FeaturesEntry, Group,
    I18nEntry, NamingConvention, ObsoletesEntry, PackageKind, ProvidesEntry, RequiresAnyEntry,
    RequiresEntry, SubskillEntry, VersionEntry, WorkspaceOriginEntry,
};
// Both sides are now called `JournalRecord` — the schema's
// `x-rust-type` says so and the generator obeys it — so the alias is
// what keeps the two apart, and it is load-bearing rather than
// stylistic: dropping it collides with the hand-written import above.
use vibe_wire::generated::journal::e1::journal::JournalRecord as GeneratedJournal;

/// How many variants `Event` carries, counted from `journal/record.rs`
/// (the harvest's §6 count agrees). Declared as a constant so the
/// fixture's exhaustiveness is asserted, not assumed: a fixture that
/// exercised ten of eleven would prove nothing about the eleventh.
const EVENT_VARIANT_COUNT: usize = 11;

/// Every record puts exactly three keys on the wire: `at`, `actor`,
/// `event` — no optionals at the root.
const RECORD_KEY_COUNT: usize = 3;

/// A fully populated `published` entry carries all thirty-three of the
/// `version_entry` vocabulary's wire keys (12 required + 21 optional) —
/// the same count the entry oracle asserts, re-asserted here because the
/// arm must tow the WHOLE record, not the entry oracle's business alone.
const PUBLISHED_ENTRY_KEY_COUNT: usize = 33;

/// One row per union arm, in fixture order: the wire tag the writer's
/// `#[serde(tag = "kind", rename_all = "snake_case")]` produces, and how
/// many keys the event object carries on the wire — the tag plus the
/// arm's own fields. Counted per arm so a fixture (or schema) that
/// thinned one arm is caught on each arm independently.
const ARM_WIRE_SHAPES: &[(&str, usize)] = &[
    // Four keys: the tag, `name`, `reason` and the optional
    // `superseded_by` the fixture supplies. With no successor the arm is
    // three — both sides skip the absent key, so that shape is a
    // narrowing of this one and not a second wire form.
    ("buried", 4),
    ("channel_set", 5),
    ("channel_unset", 4),
    ("entry_set_replaced", 2),
    ("force_replaced", 7),
    ("frozen", 5),
    ("initialised", 4),
    ("notice", 4),
    ("published", 2),
    ("removed", 4),
    ("yanked", 5),
];

fn fixed_instant() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-15T10:15:30Z")
        .expect("the fixture timestamp parses")
        .with_timezone(&Utc)
}

fn org() -> Group {
    Group::parse("org.vibevm").expect("the fixture group parses")
}

fn v(version: &str) -> Version {
    version.parse().expect("the fixture version parses")
}

fn record(at: DateTime<Utc>, event: Event) -> JournalRecord {
    JournalRecord {
        at,
        actor: "vibe-index 0.1.0-dev".to_string(),
        event,
    }
}

/// A `VersionEntry` with every `Option` in `Some`, every collection
/// non-empty, every nested subsection filled, and both flags `true` —
/// the same fully populated shape as the entry oracle's fixture, because
/// `published` tows the whole catalog record through the journal.
fn fully_populated_entry() -> Box<VersionEntry> {
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

    Box::new(VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind: PackageKind::Flow,
        group: org(),
        name: "wal".to_string(),
        version: v("1.2.3"),
        content_hash: "sha256:9f2c".to_string(),
        source_url: "https://gitverse.ru/vibevm/vibevm.git".to_string(),
        source_ref: "v1.2.3".to_string(),
        resolved_commit: Some("0123456789abcdef0123".to_string()),
        registry: "vibespecs".to_string(),
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
    })
}

/// One record per `Event` variant — every arm fully populated, `removed`
/// with `version: Some(..)` per the module doc. Order matches
/// [`ARM_WIRE_SHAPES`] row for row; the test zips them.
fn fixture_records() -> Vec<JournalRecord> {
    let stamp = fixed_instant();
    vec![
        record(
            stamp,
            Event::Buried {
                // Distinct strings in every position, so a schema that
                // swapped `name` for `reason` — or dropped the successor
                // into either — cannot pass by symmetry.
                name: "old-name".to_string(),
                reason: "renamed to `new-name`".to_string(),
                superseded_by: Some("org.vibevm.core/new-name".to_string()),
            },
        ),
        record(
            stamp,
            Event::ChannelSet {
                group: org(),
                name: "wal".to_string(),
                channel: "stable".to_string(),
                version: v("0.2.0"),
            },
        ),
        record(
            stamp,
            Event::ChannelUnset {
                group: org(),
                name: "wal".to_string(),
                channel: "stable".to_string(),
            },
        ),
        record(
            stamp,
            Event::EntrySetReplaced {
                source: "full-scan of ../packages".to_string(),
            },
        ),
        record(
            stamp,
            Event::ForceReplaced {
                group: org(),
                name: "wal".to_string(),
                version: v("1.2.3"),
                old_hash: "sha256:old".to_string(),
                new_hash: "sha256:new".to_string(),
                reason: "reupload after tamper".to_string(),
            },
        ),
        record(
            stamp,
            Event::Frozen {
                group: org(),
                name: "wal".to_string(),
                version: v("1.2.3"),
                content_hash: "sha256:9f2c".to_string(),
            },
        ),
        record(
            stamp,
            Event::Initialised {
                registry: "vibespecs".to_string(),
                registry_url: "https://example.invalid/vibespecs".to_string(),
                // A hyphenated convention pins the one wire string a
                // "simplified" rename rule would silently break.
                naming: NamingConvention::KindName,
            },
        ),
        record(
            stamp,
            Event::Notice {
                group: org(),
                name: "wal".to_string(),
                text: "maintainership moved".to_string(),
            },
        ),
        record(
            stamp,
            Event::Published {
                entry: fully_populated_entry(),
            },
        ),
        record(
            stamp,
            Event::Removed {
                group: org(),
                name: "wal".to_string(),
                version: Some(v("1.2.3")),
            },
        ),
        record(
            stamp,
            Event::Yanked {
                group: org(),
                name: "wal".to_string(),
                version: v("0.2.0"),
                reason: "legal hold".to_string(),
            },
        ),
    ]
}

/// The writer's output survives the schema unchanged for ALL ELEVEN arms:
/// what the hand-written type emits, the generated type accepts and hands
/// back byte-for-byte at the `Value` level. A mapping arm the schema
/// forgot makes `from_value` itself fail (an unknown `kind` tag); a field
/// inside an arm the schema forgot is dropped by the permissive generated
/// reader and caught by the per-arm key counts and the value compare.
#[test]
fn every_event_variant_round_trips_through_the_generated_type() {
    let handwritten = fixture_records();

    // The fixture covers exactly the enum: one record per variant, every
    // tag distinct — a fixture that skipped or doubled an arm is caught
    // here, not left to the reader's trust.
    assert_eq!(
        handwritten.len(),
        EVENT_VARIANT_COUNT,
        "the fixture must carry one record per event variant"
    );
    assert_eq!(
        ARM_WIRE_SHAPES.len(),
        EVENT_VARIANT_COUNT,
        "the wire-shape table must carry one row per event variant"
    );
    let tags: BTreeSet<&str> = ARM_WIRE_SHAPES.iter().map(|(tag, _)| *tag).collect();
    assert_eq!(
        tags.len(),
        EVENT_VARIANT_COUNT,
        "every arm's wire tag must be distinct"
    );

    for (index, record) in handwritten.iter().enumerate() {
        let (expected_tag, expected_keys) = ARM_WIRE_SHAPES[index];

        let j1 = serde_json::to_value(record).expect("the hand-written type serialises");
        assert_eq!(
            j1.as_object().map(|object| object.len()),
            Some(RECORD_KEY_COUNT),
            "every record carries `at`, `actor`, `event` — nothing else: {j1:?}"
        );

        let event = j1["event"].as_object().expect("the event is a JSON object");
        assert_eq!(
            event.len(),
            expected_keys,
            "the {expected_tag} arm must carry the tag plus its own fields — \
             a thinned fixture or schema is caught per arm: {event:?}"
        );
        assert_eq!(
            event["kind"].as_str(),
            Some(expected_tag),
            "the union tag must be on the wire for {expected_tag}"
        );

        // The schema is wider than Rust in two named places; these pins
        // hold the Rust side of each gap while the round-trip below holds
        // the wire side.
        match expected_tag {
            "published" => assert_eq!(
                event["entry"].as_object().map(|entry| entry.len()),
                Some(PUBLISHED_ENTRY_KEY_COUNT),
                "the published arm must tow the whole version_entry — all \
                 thirty-three keys"
            ),
            "removed" => assert_eq!(
                event["version"],
                serde_json::json!("1.2.3"),
                "the removed fixture carries `Some` — the only value both \
                 the always-present writer and the skipping generated type \
                 render identically"
            ),
            "buried" => {
                assert_eq!(
                    event["name"],
                    serde_json::json!("old-name"),
                    "`name` is a bare string, not a `(group, name)` pair — \
                     the one identity-bearing arm shaped that way, because \
                     a tombstone rides on a per-name candidate file"
                );
                assert_eq!(
                    event["superseded_by"],
                    serde_json::json!("org.vibevm.core/new-name"),
                    "the successor rides as the same string the tombstone \
                     carries, so the projection copies it across untouched"
                );
            }
            _ => {}
        }

        // The schema accepts everything the writer emits…
        let generated: GeneratedJournal = serde_json::from_value(j1.clone())
            .unwrap_or_else(|e| panic!("the generated type parses the {expected_tag} arm: {e}"));
        // …and nothing is lost or added on the way back.
        let j2 = serde_json::to_value(&generated).expect("the generated type serialises");
        assert_eq!(
            j1, j2,
            "wire drift between the hand-written and the generated journal \
             record for the {expected_tag} arm — a mapping arm or field the \
             schema misses is dropped (or rejected) by the generated reader"
        );
    }
}
