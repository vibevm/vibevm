//! Unit tests for the ONE arm of the projector that produces a carrier
//! rather than folding a version or refusing — `Event::Buried` and the
//! tombstone it leaves behind. Split out of `project_tests.rs` along the
//! producer/folder seam rather than shaved, because the file-length
//! budget counts every file and that one already sits at its edge.
//!
//! What these tests exist to catch is named in PROP-005 §2.11: a
//! tombstone placed by anything other than a journal fact is erased by
//! the next unrelated mutation, silently and with nothing going red,
//! because a mutation builds its state from the fold and writes THAT
//! out. `survives_an_unrelated_publish` below is the guard — it is the
//! test a field-writing implementation fails and a fact-appending one
//! passes, and it is the reason the arm was built as an append.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use chrono::{DateTime, Utc};
use semver::Version;

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

fn other_org() -> Group {
    Group::parse("org.example").unwrap()
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

fn publish(group: Group, name: &str, version: &str, at: DateTime<Utc>) -> JournalRecord {
    let mut entry = VersionEntry::minimal(PackageKind::Flow, group, name, v(version), at);
    entry.registry = "vibespecs".into();
    record(
        at,
        Event::Published {
            entry: Box::new(entry),
        },
    )
}

fn bury(name: &str, reason: &str, successor: Option<&str>) -> Event {
    Event::Buried {
        name: name.into(),
        reason: reason.into(),
        superseded_by: successor.map(Into::into),
    }
}

/// The arm produces what nothing produced before: a tombstone, carrying
/// the reason and the successor, in place of the name's packages.
#[test]
fn a_burial_leaves_a_tombstone_where_the_packages_were() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");

    let index = project([
        init(t1),
        publish(org(), "wal-old", "0.1.0", t2),
        record(
            t3,
            bury("wal-old", "renamed to `wal`", Some("org.vibevm/wal")),
        ),
    ])
    .unwrap();

    let stone = index
        .tombstones
        .get("wal-old")
        .expect("the burial must leave a tombstone under the bare name");
    assert_eq!(stone.reason, "renamed to `wal`");
    assert_eq!(stone.superseded_by.as_deref(), Some("org.vibevm/wal"));
    assert!(
        index.get(&org(), "wal-old").is_none(),
        "a buried name keeps no packages — its candidate file carries an empty list"
    );
}

/// A name with nowhere to go leaves a tombstone all the same: the
/// successor is optional, the reason is not.
#[test]
fn a_burial_without_a_successor_still_answers() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");

    let index = project([
        init(t1),
        record(t2, bury("gone", "withdrawn by author", None)),
    ])
    .unwrap();

    let stone = index
        .tombstones
        .get("gone")
        .expect("a reason alone is a complete tombstone");
    assert_eq!(stone.reason, "withdrawn by author");
    assert_eq!(
        stone.superseded_by, None,
        "no successor means the field is absent, not empty"
    );
}

/// The fact carries a BARE name, and the projection honours that: two
/// groups holding the same name are closed by one burial, because the
/// tombstone rides on a candidate-set file that spans every group.
#[test]
fn a_burial_closes_the_name_across_every_group() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let t4 = at("2026-08-04T12:00:00Z");

    let index = project([
        init(t1),
        publish(org(), "twin", "0.1.0", t2),
        publish(other_org(), "twin", "0.2.0", t3),
        record(t4, bury("twin", "the name is retired", None)),
    ])
    .unwrap();

    assert!(index.get(&org(), "twin").is_none());
    assert!(index.get(&other_org(), "twin").is_none());
    assert!(index.tombstones.contains_key("twin"));
    assert_eq!(
        index.version_count(),
        0,
        "both groups' versions go with the name"
    );
}

/// **The mine, guarded.** A tombstone written as a FIELD would be erased
/// by the next unrelated publish, because a mutation builds its state
/// from this fold and writes that out. Written as a FACT it survives,
/// and this asserts exactly that: bury, then publish something with
/// nothing to do with the buried name, and the stone is still there.
#[test]
fn a_tombstone_survives_an_unrelated_publish() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let t4 = at("2026-08-04T12:00:00Z");

    let index = project([
        init(t1),
        publish(org(), "wal-old", "0.1.0", t2),
        record(
            t3,
            bury("wal-old", "renamed to `wal`", Some("org.vibevm/wal")),
        ),
        publish(org(), "unrelated", "1.0.0", t4),
    ])
    .unwrap();

    assert!(
        index.tombstones.contains_key("wal-old"),
        "the tombstone is a journal fact, so a later unrelated publish cannot erase it"
    );
    assert!(
        index.get(&org(), "unrelated").is_some(),
        "and the unrelated publish landed normally"
    );
}

/// Order is fold order here as everywhere, and the two states of a
/// candidate file stay disjoint: a publish AFTER a burial re-opens the
/// name and takes the stone with it. §2.4 carries a tombstone «only
/// when the bare name is buried» and shows an empty package list beside
/// it, so a name holding packages and a tombstone at once would be a
/// shape the contract never describes. The journal keeps the burial
/// forever regardless — this is the projection's answer, not a deletion
/// of the fact.
#[test]
fn a_publish_after_a_burial_re_opens_the_name_and_clears_the_stone() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");

    let index = project([
        init(t1),
        record(t2, bury("wal-old", "closed by mistake", None)),
        publish(org(), "wal-old", "0.2.0", t3),
    ])
    .unwrap();

    assert!(
        index.get(&org(), "wal-old").is_some(),
        "the later publish stands — the fold has no veto, only order"
    );
    assert!(
        !index.tombstones.contains_key("wal-old"),
        "a name with packages is not buried, so it carries no stone"
    );
}

/// The clearing is keyed by NAME, not by identity — the same asymmetry
/// the burial itself has. Re-publishing under one group lifts the stone
/// the other group's twin was buried under too, because there was only
/// ever one stone: the candidate file is per-name.
#[test]
fn re_opening_is_per_name_exactly_as_burial_is() {
    let t1 = at("2026-08-01T12:00:00Z");
    let t2 = at("2026-08-02T12:00:00Z");
    let t3 = at("2026-08-03T12:00:00Z");
    let t4 = at("2026-08-04T12:00:00Z");

    let index = project([
        init(t1),
        publish(org(), "twin", "0.1.0", t2),
        record(t3, bury("twin", "the name is retired", None)),
        publish(other_org(), "twin", "0.2.0", t4),
    ])
    .unwrap();

    assert!(
        !index.tombstones.contains_key("twin"),
        "one stone per name means one publish lifts it"
    );
    assert!(
        index.get(&org(), "twin").is_none(),
        "the burial still took the first group's versions"
    );
    assert!(index.get(&other_org(), "twin").is_some());
}
