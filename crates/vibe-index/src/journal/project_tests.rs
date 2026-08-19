//! Unit tests for the journal projector, out-of-line per the
//! file-length budget. Included via `#[cfg(test)] #[path =
//! "project_tests.rs"] mod project_tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use chrono::{DateTime, Utc};
use semver::Version;

use crate::error::Error;
use crate::journal::project::project;
use crate::journal::record::{Event, JournalRecord};
use crate::types::{Group, NamingConvention, PackageKind, VersionEntry};

fn at(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .unwrap()
        .with_timezone(&Utc)
}

fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

fn v(version: &str) -> Version {
    version.parse().unwrap()
}

fn record(at: DateTime<Utc>, event: Event) -> JournalRecord {
    JournalRecord {
        at,
        actor: "vibe-index 0.1.0-dev".into(),
        event,
    }
}

/// The journal's mandatory first fact, with the identity the other
/// helpers assume; tests that vary the identity build their own
/// records inline.
fn init(at: DateTime<Utc>) -> JournalRecord {
    record(
        at,
        Event::Initialised {
            registry: "vibespecs".into(),
            registry_url: "https://example.invalid/vibespecs".into(),
            naming: NamingConvention::Fqdn,
        },
    )
}

/// Boxed because `Event::Published` holds a boxed entry — the helper
/// returns what the variant takes, so no call site repeats the wrapper.
fn sample_entry(name: &str, version: &str, at: DateTime<Utc>) -> Box<VersionEntry> {
    // `name` rides in as `&str` and `minimal` takes `impl Into<String>`
    // directly — an `.into()` here would be ambiguous (E0283).
    let mut entry = VersionEntry::minimal(PackageKind::Flow, org(), name, v(version), at);
    entry.registry = "vibespecs".into();
    Box::new(entry)
}

fn published(name: &str, version: &str, at: DateTime<Utc>) -> JournalRecord {
    record(
        at,
        Event::Published {
            entry: sample_entry(name, version, at),
        },
    )
}

/// Group 1 — a lone `Initialised` folds to an empty catalog that
/// carries exactly the journal's identity, the writer's own constants,
/// and nothing else.
#[test]
fn identity_of_a_lone_initialised_record() {
    let t1 = at("2026-08-01T12:00:00Z");
    let index = project([init(t1)]).unwrap();
    assert_eq!(index.registry, "vibespecs");
    assert_eq!(index.registry_url, "https://example.invalid/vibespecs");
    assert_eq!(index.naming, NamingConvention::Fqdn);
    assert_eq!(index.generated_at, t1);
    // A projection births the catalog from scratch, so it stamps the
    // writer's own constants (F2.2 forbids overwriting a READ version;
    // nothing here is read).
    assert_eq!(index.schema_version, 1);
    assert_eq!(index.generator, crate::index::memory::default_generator());
    assert_eq!(index.package_count(), 0);
    assert!(index.tombstones.is_empty());
    assert!(index.quarantined.is_empty());
}

/// Group 2 — two `Initialised` records: the LAST one wins, exactly as
/// `cli/init.rs` promises the projector's fold will decide.
#[test]
fn last_initialised_wins() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-05T09:30:00Z");
    let index = project([
        init(t1),
        record(
            t2,
            Event::Initialised {
                registry: "second".into(),
                registry_url: "https://example.invalid/second".into(),
                naming: NamingConvention::KindName,
            },
        ),
    ])
    .unwrap();
    assert_eq!(index.registry, "second");
    assert_eq!(index.registry_url, "https://example.invalid/second");
    assert_eq!(index.naming, NamingConvention::KindName);
    assert_eq!(
        index.generated_at, t2,
        "the last applied record stamps the catalog"
    );
}

/// Group 3 — a journal with no identity is refused with the `init`
/// recipe: empty, and non-empty but without a single `Initialised`.
#[test]
fn journal_without_identity_is_refused() {
    let t1 = at("2026-08-01T12:00:00Z");

    let empty = project(Vec::new()).unwrap_err();
    assert!(
        matches!(empty, Error::Unprojectable(_)),
        "an empty journal must refuse, got: {empty}"
    );
    let msg = empty.to_string();
    assert!(
        msg.contains("empty"),
        "the error must say the journal is empty: {msg}"
    );
    assert!(
        msg.contains("init"),
        "the error must carry the `init` recipe: {msg}"
    );

    let no_identity = project([published("wal", "0.1.0", t1)]).unwrap_err();
    assert!(
        matches!(no_identity, Error::Unprojectable(_)),
        "an identity-less journal must refuse, got: {no_identity}"
    );
    let msg = no_identity.to_string();
    assert!(
        msg.contains("Initialised"),
        "the error must name the missing record: {msg}"
    );
    assert!(
        msg.contains("init"),
        "the error must carry the `init` recipe: {msg}"
    );
}

/// Group 4 — publish inserts (and replaces), remove retires: by
/// version, or by whole package.
#[test]
fn publish_inserts_replaces_and_remove_retires() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let t4 = at("2026-08-04T12:00:00Z");

    // Two publishes -> two versions of one package.
    let index = project([
        init(t1),
        published("wal", "0.1.0", t2),
        published("wal", "0.2.0", t3),
    ])
    .unwrap();
    assert_eq!(index.package_count(), 1);
    assert_eq!(index.version_count(), 2);

    // A re-publish of the SAME version replaces it — Index::upsert
    // semantics, carried into the fold verbatim.
    let mut replacement = *sample_entry("wal", "0.1.0", t4);
    replacement.content_hash = "sha256:replaced".into();
    let index = project([
        init(t1),
        published("wal", "0.1.0", t2),
        record(
            t4,
            Event::Published {
                entry: Box::new(replacement),
            },
        ),
    ])
    .unwrap();
    assert_eq!(
        index.version_count(),
        1,
        "same version re-published replaces, not adds"
    );
    let pkg = index.get(&org(), "wal").unwrap();
    assert_eq!(pkg.versions[0].content_hash, "sha256:replaced");

    // Removed{Some} -> that version goes, the package row stays.
    let index = project([
        init(t1),
        published("wal", "0.1.0", t2),
        published("wal", "0.2.0", t3),
        record(
            t4,
            Event::Removed {
                group: org(),
                name: "wal".into(),
                version: Some(v("0.1.0")),
            },
        ),
    ])
    .unwrap();
    assert_eq!(index.version_count(), 1);
    assert_eq!(
        index.package_count(),
        1,
        "an emptied package row is valid and stays"
    );
    let pkg = index.get(&org(), "wal").unwrap();
    assert_eq!(
        pkg.versions
            .iter()
            .map(|e| e.version.to_string())
            .collect::<Vec<_>>(),
        ["0.2.0"]
    );

    // Removed{None} -> the whole package goes.
    let index = project([
        init(t1),
        published("wal", "0.1.0", t2),
        published("wal", "0.2.0", t3),
        record(
            t4,
            Event::Removed {
                group: org(),
                name: "wal".into(),
                version: None,
            },
        ),
    ])
    .unwrap();
    assert!(index.get(&org(), "wal").is_none());
    assert_eq!(index.package_count(), 0);
}

/// Group 5 — the watershed: `EntrySetReplaced` clears the entry set,
/// so only what is re-published AFTER it survives. This is the
/// behaviour the variant exists for.
#[test]
fn watershed_clears_the_entry_set() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let t4 = at("2026-08-04T12:00:00Z");
    let index = project([
        init(t1),
        published("wal", "0.1.0", t2),
        published("swap", "0.3.0", t3),
        record(
            t4,
            Event::EntrySetReplaced {
                source: "from-clones".into(),
            },
        ),
        published("wal", "0.2.0", t4),
    ])
    .unwrap();
    // The pre-watershed entries are gone, including the whole `swap`
    // package; only the post-watershed republish stands.
    assert!(
        index.get(&org(), "swap").is_none(),
        "a package not re-published is gone"
    );
    let pkg = index.get(&org(), "wal").unwrap();
    assert_eq!(
        pkg.versions
            .iter()
            .map(|e| e.version.to_string())
            .collect::<Vec<_>>(),
        ["0.2.0"],
        "the watershed drops every entry asserted before it; only the \
         re-published one survives"
    );
}

/// Group 6 — the watershed does NOT carry the tombstones away. No
/// journal event carries a tombstone carrier yet (see the worker
/// report's Decisions), so the honest form of this test asserts the
/// half that is real: the watershed clears `by_pkgref` and leaves the
/// `tombstones` map it must never touch exactly as it was — empty,
/// because no event feeds it.
#[test]
fn watershed_keeps_tombstones() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let index = project([
        init(t1),
        published("wal", "0.1.0", t2),
        record(
            t3,
            Event::EntrySetReplaced {
                source: "from-clones".into(),
            },
        ),
    ])
    .unwrap();
    assert_eq!(
        index.package_count(),
        0,
        "the watershed clears the entry set"
    );
    assert!(
        index.tombstones.is_empty(),
        "the watershed must not touch tombstones — a scan cannot re-derive \
         them, and dropping them would make state unrecoverable (PROP-044 \
         law 2); the map stays exactly as it was"
    );
}

/// Group 7 — `Yanked` / `Frozen` set their flag on the named version.
#[test]
fn yank_and_freeze_mark_their_versions() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let t4 = at("2026-08-04T12:00:00Z");
    let t5 = at("2026-08-05T12:00:00Z");
    let index = project([
        init(t1),
        published("wal", "0.1.0", t2),
        published("wal", "0.2.0", t3),
        record(
            t4,
            Event::Yanked {
                group: org(),
                name: "wal".into(),
                version: v("0.1.0"),
                reason: "withdrawn by the owner".into(),
            },
        ),
        record(
            t5,
            Event::Frozen {
                group: org(),
                name: "wal".into(),
                version: v("0.2.0"),
                content_hash: "sha256:abc".into(),
            },
        ),
    ])
    .unwrap();
    let pkg = index.get(&org(), "wal").unwrap();
    let by_version = |ver: &str| {
        pkg.versions
            .iter()
            .find(|e| e.version.to_string() == ver)
            .unwrap()
    };
    assert!(by_version("0.1.0").yanked);
    assert!(
        !by_version("0.1.0").frozen,
        "the freeze on 0.2.0 must not leak to 0.1.0"
    );
    assert!(by_version("0.2.0").frozen);
    assert!(
        !by_version("0.2.0").yanked,
        "the yank on 0.1.0 must not leak to 0.2.0"
    );
}

/// The §2.4 decision, made testable: a `Yanked`/`Frozen` whose target
/// is absent from the projection at that point of the fold is a no-op,
/// not an error — journal order decided the version's fate already
/// (an earlier `Removed` retired it, or it was never published).
#[test]
fn yank_and_freeze_on_a_missing_version_fold_as_noops() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let t4 = at("2026-08-04T12:00:00Z");

    // Removed earlier: the late yank changes nothing observable.
    let index = project([
        init(t1),
        published("wal", "0.1.0", t2),
        record(
            t3,
            Event::Removed {
                group: org(),
                name: "wal".into(),
                version: Some(v("0.1.0")),
            },
        ),
        record(
            t4,
            Event::Yanked {
                group: org(),
                name: "wal".into(),
                version: v("0.1.0"),
                reason: "late fact about a retired version".into(),
            },
        ),
    ])
    .unwrap();
    assert_eq!(index.version_count(), 0);
    assert_eq!(
        index.generated_at, t4,
        "a no-op record still advances the catalog's as-of moment"
    );

    // Never published: nothing to mark, and the fold stays total.
    let index = project([
        init(t1),
        record(
            t4,
            Event::Frozen {
                group: org(),
                name: "wal".into(),
                version: v("0.9.9"),
                content_hash: "sha256:abc".into(),
            },
        ),
    ])
    .unwrap();
    assert_eq!(index.version_count(), 0);
    assert_eq!(index.generated_at, t4);
}

/// Group 8 — each of the four variants without a carrier refuses, and
/// the error names the variant it met. There were five until `Renamed`
/// left the vocabulary: retirement gained a carrier, so its arm folds
/// now instead of refusing (Group 10).
#[test]
fn unprojectable_variants_are_refused_by_name() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let cases: Vec<(&str, Event)> = vec![
        (
            "Notice",
            Event::Notice {
                group: org(),
                name: "wal".into(),
                text: "maintainership moved".into(),
            },
        ),
        (
            "ChannelSet",
            Event::ChannelSet {
                group: org(),
                name: "wal".into(),
                channel: "stable".into(),
                version: v("0.2.0"),
            },
        ),
        (
            "ChannelUnset",
            Event::ChannelUnset {
                group: org(),
                name: "wal".into(),
                channel: "stable".into(),
            },
        ),
        (
            "ForceReplaced",
            Event::ForceReplaced {
                group: org(),
                name: "wal".into(),
                version: v("0.1.0"),
                old_hash: "sha256:old".into(),
                new_hash: "sha256:new".into(),
                reason: "reupload after tamper".into(),
            },
        ),
    ];
    for (variant, event) in cases {
        let err = project([init(t1), record(t2, event)]).unwrap_err();
        assert!(
            matches!(err, Error::Unprojectable(_)),
            "`{variant}` must refuse as Unprojectable, got: {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains(variant),
            "the error must name the variant `{variant}` it met: {msg}"
        );
    }
}

/// Group 9 — journal order is fold order: the same events in a
/// different order yield a different catalog wherever they must.
#[test]
fn journal_order_decides() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let remove_010 = || Event::Removed {
        group: org(),
        name: "wal".into(),
        version: Some(v("0.1.0")),
    };
    let yank_010 = || Event::Yanked {
        group: org(),
        name: "wal".into(),
        version: v("0.1.0"),
        reason: "order probe".into(),
    };

    // Publish-then-remove retires the version; remove-then-publish
    // leaves it standing.
    let gone = project([
        init(t1),
        published("wal", "0.1.0", t2),
        record(t3, remove_010()),
    ])
    .unwrap();
    assert_eq!(gone.version_count(), 0);
    let standing = project([
        init(t1),
        record(t2, remove_010()),
        published("wal", "0.1.0", t3),
    ])
    .unwrap();
    assert_eq!(standing.version_count(), 1);

    // Yank-after-publish marks the version; yank-before-publish is a
    // fact about a version not yet in the projection — a no-op.
    let marked = project([
        init(t1),
        published("wal", "0.1.0", t2),
        record(t3, yank_010()),
    ])
    .unwrap();
    assert!(marked.get(&org(), "wal").unwrap().versions[0].yanked);
    let unmarked = project([
        init(t1),
        record(t2, yank_010()),
        published("wal", "0.1.0", t3),
    ])
    .unwrap();
    assert!(
        !unmarked.get(&org(), "wal").unwrap().versions[0].yanked,
        "journal order decides: the yank that came before the publish \
         found no version to mark"
    );
}
