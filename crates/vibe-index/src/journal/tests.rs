//! Unit tests for the journal store, out-of-line per the file-length
//! budget. Included via `#[cfg(test)] #[path = "tests.rs"] mod tests;`,
//! so the module-tree position — and therefore `use super::*` — is
//! unchanged from the inline form.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use chrono::{DateTime, Utc};
use semver::Version;

use crate::journal::record::{Event, JournalRecord};
use crate::journal::store::{append, default_dir, replay};
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

/// Boxed because `Event::Published` holds a boxed entry — the helper
/// returns what the variant takes, so no call site repeats the wrapper.
fn sample_entry(name: &str, version: &str, at: DateTime<Utc>) -> Box<VersionEntry> {
    // `name` rides in as `&str` and `minimal` takes `impl Into<String>`
    // directly — an `.into()` here would be ambiguous (E0283).
    let mut entry = VersionEntry::minimal(PackageKind::Flow, org(), name, v(version), at);
    entry.registry = "vibespecs".into();
    Box::new(entry)
}

#[test]
fn round_trip_preserves_records_in_journal_order() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("journal");
    let records = vec![
        record(
            at("2026-08-01T12:00:00Z"),
            Event::Initialised {
                registry: "vibespecs".into(),
                registry_url: "https://example.invalid/vibespecs".into(),
                naming: NamingConvention::Fqdn,
            },
        ),
        record(
            at("2026-08-02T12:00:00Z"),
            Event::Published {
                entry: sample_entry("wal", "0.1.0", at("2026-08-02T12:00:00Z")),
            },
        ),
        record(
            at("2026-08-03T09:00:00Z"),
            Event::Removed {
                group: org(),
                name: "wal".into(),
                version: Some(v("0.1.0")),
            },
        ),
        record(
            at("2026-08-03T18:00:00Z"),
            Event::Yanked {
                group: org(),
                name: "wal".into(),
                version: v("0.2.0"),
                reason: "withdrawn by the owner".into(),
            },
        ),
    ];
    for r in &records {
        append(&journal, r).unwrap();
    }
    assert_eq!(replay(&journal).unwrap(), records);
}

#[test]
fn records_land_in_monthly_shards_and_replay_is_chronological() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("journal");
    let aug_middle = record(
        at("2026-08-15T12:00:00Z"),
        Event::Notice {
            group: org(),
            name: "wal".into(),
            text: "middle of august".into(),
        },
    );
    let aug_late = record(
        at("2026-08-20T12:00:00Z"),
        Event::Notice {
            group: org(),
            name: "wal".into(),
            text: "late august".into(),
        },
    );
    let sep_first = record(
        at("2026-09-01T12:00:00Z"),
        Event::Notice {
            group: org(),
            name: "wal".into(),
            text: "september".into(),
        },
    );

    // Append OUT of chronological order: replay must still come back
    // in journal order (ascending shard, file order within a shard).
    append(&journal, &sep_first).unwrap();
    append(&journal, &aug_middle).unwrap();
    append(&journal, &aug_late).unwrap();

    let august = journal.join("2026-08.ndjson");
    let september = journal.join("2026-09.ndjson");
    assert!(august.exists(), "expected shard {august:?}");
    assert!(september.exists(), "expected shard {september:?}");
    let august_text = std::fs::read_to_string(&august).unwrap();
    let august_lines: Vec<&str> = august_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(august_lines.len(), 2, "same-month records share one shard");
    assert_eq!(
        replay(&journal).unwrap(),
        vec![aug_middle, aug_late, sep_first]
    );
}

/// The append-only law, guarded: a second append into the same shard
/// must ADD a line, never rewrite the file. Fails on any
/// implementation that writes the shard wholesale (create/truncate).
#[test]
fn append_appends_never_rewrites() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("journal");
    let first = record(
        at("2026-08-01T12:00:00Z"),
        Event::Published {
            entry: sample_entry("wal", "0.1.0", at("2026-08-01T12:00:00Z")),
        },
    );
    let second = record(
        at("2026-08-02T12:00:00Z"),
        Event::Published {
            entry: sample_entry("wal", "0.2.0", at("2026-08-02T12:00:00Z")),
        },
    );
    append(&journal, &first).unwrap();
    append(&journal, &second).unwrap();

    let shard = journal.join("2026-08.ndjson");
    let text = std::fs::read_to_string(&shard).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "two appends into one shard must leave TWO lines — a journal \
         only ever appends, it never rewrites its own past"
    );
    assert!(lines[0].contains("\"wal\""));
    assert!(lines[1].contains("0.2.0"));
}

/// F2-1's instrument applied to the journal: one record + one `at`
/// must produce one byte sequence, or "rebuild and compare" over the
/// journal would measure noise.
#[test]
fn same_record_appends_byte_identical_shards() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let rec = record(
        at("2026-08-14T12:00:00Z"),
        Event::Published {
            entry: sample_entry("wal", "0.1.0", at("2026-08-14T12:00:00Z")),
        },
    );
    append(&left.path().join("journal"), &rec).unwrap();
    append(&right.path().join("journal"), &rec).unwrap();
    let a = std::fs::read(left.path().join("journal/2026-08.ndjson")).unwrap();
    let b = std::fs::read(right.path().join("journal/2026-08.ndjson")).unwrap();
    assert_eq!(a, b, "the same record must serialise to the same bytes");
}

#[test]
fn every_event_variant_survives_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("journal");
    let stamp = at("2026-08-14T12:00:00Z");
    let records = vec![
        record(
            stamp,
            Event::Initialised {
                registry: "vibespecs".into(),
                registry_url: "https://example.invalid/vibespecs".into(),
                naming: NamingConvention::KindName,
            },
        ),
        record(
            stamp,
            Event::Published {
                entry: sample_entry("wal", "0.1.0", stamp),
            },
        ),
        record(
            stamp,
            Event::Frozen {
                group: org(),
                name: "wal".into(),
                version: v("0.1.0"),
                content_hash: "sha256:abc".into(),
            },
        ),
        record(
            stamp,
            Event::Yanked {
                group: org(),
                name: "wal".into(),
                version: v("0.1.0"),
                reason: "legal hold".into(),
            },
        ),
        record(
            stamp,
            Event::Removed {
                group: org(),
                name: "wal".into(),
                version: None,
            },
        ),
        record(
            stamp,
            Event::Renamed {
                from: (org(), "old-name".into()),
                to: (org(), "new-name".into()),
            },
        ),
        record(
            stamp,
            Event::Notice {
                group: org(),
                name: "wal".into(),
                text: "maintainership moved".into(),
            },
        ),
        record(
            stamp,
            Event::ChannelSet {
                group: org(),
                name: "wal".into(),
                channel: "stable".into(),
                version: v("0.2.0"),
            },
        ),
        record(
            stamp,
            Event::ChannelUnset {
                group: org(),
                name: "wal".into(),
                channel: "stable".into(),
            },
        ),
        record(
            stamp,
            Event::ForceReplaced {
                group: org(),
                name: "wal".into(),
                version: v("0.1.0"),
                old_hash: "sha256:old".into(),
                new_hash: "sha256:new".into(),
                reason: "reupload after tamper".into(),
            },
        ),
        record(
            stamp,
            Event::EntrySetReplaced {
                source: "from-clones".into(),
            },
        ),
    ];
    for r in &records {
        append(&journal, r).unwrap();
    }
    assert_eq!(replay(&journal).unwrap(), records);
}

#[test]
fn default_dir_sits_under_the_data_dir_state() {
    let p = default_dir(std::path::Path::new("data"));
    assert_eq!(
        p,
        std::path::Path::new("data").join("state").join("journal")
    );
}

#[test]
fn replay_of_missing_journal_is_empty_history() {
    let dir = tempfile::tempdir().unwrap();
    assert!(replay(&dir.path().join("nowhere")).unwrap().is_empty());
}
