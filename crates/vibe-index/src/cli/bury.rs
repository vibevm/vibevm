//! `vibe-index bury <data-dir> <name> --reason <TEXT>
//! [--superseded-by <GROUP/NAME>]` — close a BARE NAME, the withdrawal
//! trio's last arm: where `remove` deletes and `yank` withdraws one
//! version, a burial leaves a TOMBSTONE — why the name is gone and,
//! when there is one, the successor to move to.
//!
//! The command takes the bare `name` and NO group — unlike every other
//! writing verb of this crate — and that is the file layout speaking,
//! not an omission to fix: the tombstone rides on
//! `by-name/<name>.json`, the candidate-set file that spans every group
//! at once (PROP-005 §2.4), so per-group tombstones do not exist and a
//! `--group` flag would promise a narrowing the format cannot express.
//! A burial closes the name for ALL of its groups.
//!
//! Ф3.2 journal form: the published catalog is never this writer's
//! input (PROP-044 §4.4). "Is there something to bury" is answered by
//! folding the journal BEFORE any record is appended — the journal is
//! the truth, and a burial of a name that neither stands in any group
//! nor already carries a tombstone would assert a state that never
//! existed. A REPEATED burial is refused for the other reason: phase 2
//! established that a mutation which changes nothing leaves no trace,
//! and burying an already-buried name changes nothing.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use std::path::PathBuf;

use chrono::Utc;
use clap::Parser;

use crate::error::{Error, Result};
use crate::index::memory::{WriteCtx, default_generator};
use crate::journal::{Event, JournalRecord, append, default_dir, project, replay};

#[derive(Debug, Parser)]
#[command(about = "Close a bare name for every group, leaving a tombstone behind.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// The bare name to close — across every group it stands in; the
    /// tombstone lives on the name's candidate-set file, which spans
    /// them all.
    pub name: String,

    /// Why the name is closed. Required: a tombstone cannot exist
    /// without a reason, and none may be synthesised on the operator's
    /// behalf.
    #[arg(long, value_name = "TEXT")]
    pub reason: String,

    /// The successor this name's consumers should move to — a redirect
    /// pointer, never an automatic rewrite. Not validated: it may name
    /// a package from another registry or one not yet published, which
    /// are the ordinary cases a check would forbid. Recorded verbatim.
    #[arg(long, value_name = "GROUP/NAME")]
    pub superseded_by: Option<String>,
}

pub fn run(args: Args) -> Result<()> {
    // F2-1 — the clock enters here, once per command: the same `at`
    // stamps the fact and the written catalog, so the two records born
    // of one command never differ by a millisecond.
    let at = Utc::now();
    super::refuse_if_server_running(&args.data_dir)?;

    // Ф3.2 — the catalog is never this writer's input (PROP-044 §4.4):
    // the journal is read from disk exactly once, folded into the
    // projection that answers "is there something to bury", and the
    // record list then carries the appended fact in memory and is
    // re-folded below for the write.
    let journal_dir = default_dir(&args.data_dir);
    let mut records = replay(&journal_dir)?;
    // The probe fold is the gate, and it is only ever READ here — the
    // verb asks the standing state two questions and changes nothing
    // until the fact is appended below.
    let probe = project(records.iter().cloned())?;
    // R1 — the journal carries no false facts: burying a name that
    // stands in no group and carries no tombstone is refused BEFORE any
    // record is appended, so the journal does not grow here.
    let standing = probe.candidates_for(&args.name);
    let buried = probe.tombstones.contains_key(&args.name);
    if standing.is_empty() && !buried {
        return Err(Error::InvalidInput(format!(
            "`{}` is not in the index — nothing to bury",
            args.name
        )));
    }
    // R2 — a mutation that changes nothing leaves no trace: a repeated
    // burial is refused with its OWN message, because "nothing to bury"
    // shown to a name that IS buried would send the operator hunting a
    // problem that does not exist.
    if buried {
        return Err(Error::InvalidInput(format!(
            "`{}` is already buried — a repeated burial changes nothing, \
             so the journal records no second fact",
            args.name
        )));
    }
    let record = JournalRecord {
        at,
        actor: default_generator(),
        event: Event::Buried {
            name: args.name.clone(),
            reason: args.reason.clone(),
            superseded_by: args.superseded_by.clone(),
        },
    };
    // Truth first (PROP-044 `##LAW-NO-UNRECOVERABLE`), the `init`
    // order: the fact lands in the journal before the derived catalog
    // is written, so a failed `write_to` leaves a journal without a
    // catalog — recoverable by re-running the command — never a
    // catalog whose truth never existed.
    append(&journal_dir, &record)?;
    records.push(record);
    project(records)?.write_to(&args.data_dir, &WriteCtx { at })?;
    println!(
        "buried {} ({}){}",
        args.name,
        args.reason,
        args.superseded_by
            .as_deref()
            .map(|s| format!(" -> {s}"))
            .unwrap_or_default()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;
    use crate::types::{NamingConvention, PackageKind, VersionEntry};
    use std::path::Path;
    use vibe_core::Group;

    /// Setup writes the FACTS an `init` / `add` would (Ф3.2: the
    /// journal is the truth) — no catalog file is ever built by hand,
    /// the probe answers from the fold of these records alone.
    fn initialised(data: &Path) {
        append(
            &default_dir(data),
            &JournalRecord {
                at: Utc::now(),
                actor: default_generator(),
                event: Event::Initialised {
                    registry: "vibespecs".to_string(),
                    registry_url: "https://example.invalid/vibespecs".to_string(),
                    naming: NamingConvention::Fqdn,
                },
            },
        )
        .unwrap();
    }

    /// The group is a parameter: a burial answers for the bare NAME,
    /// so the fixtures must be able to stand one name in SEVERAL
    /// groups — that is the distinction this verb lives on.
    fn publish(data: &Path, group: &str, name: &str, version: &str) {
        append(
            &default_dir(data),
            &JournalRecord {
                at: Utc::now(),
                actor: default_generator(),
                event: Event::Published {
                    entry: Box::new(VersionEntry::minimal(
                        PackageKind::Flow,
                        group.parse().unwrap(),
                        name,
                        version.parse().unwrap(),
                        Utc::now(),
                    )),
                },
            },
        )
        .unwrap();
    }

    fn bury_args(data: &Path, name: &str, reason: &str) -> Args {
        Args {
            data_dir: data.to_path_buf(),
            name: name.to_string(),
            reason: reason.to_string(),
            superseded_by: None,
        }
    }

    /// The raw journal bytes — shard files concatenated in journal
    /// order. Comparing before/after is the strongest form of "the
    /// journal did not grow": no appended line, no rewritten line.
    fn raw_journal(data: &Path) -> String {
        let dir = default_dir(data);
        let mut shards: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "ndjson"))
            .collect();
        shards.sort();
        let mut out = String::new();
        for shard in shards {
            out.push_str(&std::fs::read_to_string(&shard).unwrap());
        }
        out
    }

    /// The journal's own projection — the same fold the command's
    /// probe runs.
    fn projection(data: &Path) -> Index {
        project(replay(&default_dir(data)).unwrap()).unwrap()
    }

    #[test]
    fn bury_erects_a_tombstone_and_drops_the_name_in_every_group() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "org.vibevm", "wal", "0.1.0");
        publish(&data, "org.other", "wal", "1.2.3");
        let before = replay(&default_dir(&data)).unwrap().len();

        let mut args = bury_args(&data, "wal", "renamed");
        args.superseded_by = Some("org.vibevm/redbook".to_string());
        run(args).unwrap();

        // Exactly one new fact, and it is the burial itself.
        let records = replay(&default_dir(&data)).unwrap();
        assert_eq!(
            records.len(),
            before + 1,
            "a burial must append exactly one record"
        );
        match &records[records.len() - 1].event {
            Event::Buried {
                name,
                reason,
                superseded_by,
            } => {
                assert_eq!(name, "wal");
                assert_eq!(reason, "renamed");
                assert_eq!(superseded_by.as_deref(), Some("org.vibevm/redbook"));
            }
            other => panic!("expected `Buried`, got {other:?}"),
        }
        // The tombstone carries the reason and the successor.
        let idx = projection(&data);
        let ts = idx.tombstones.get("wal").unwrap();
        assert_eq!(ts.reason, "renamed");
        assert_eq!(ts.superseded_by.as_deref(), Some("org.vibevm/redbook"));
        // The name's packages are gone in EVERY group — checked one
        // group at a time, or the test could not tell a burial by name
        // from a removal by identity.
        let g1: Group = "org.vibevm".parse().unwrap();
        let g2: Group = "org.other".parse().unwrap();
        assert!(
            idx.get(&g1, "wal").is_none(),
            "the name must close in org.vibevm too, not just one group"
        );
        assert!(
            idx.get(&g2, "wal").is_none(),
            "the name must close in org.other as well"
        );
        assert!(idx.candidates_for("wal").is_empty());
    }

    #[test]
    fn bury_without_successor_leaves_the_pointer_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "org.vibevm", "wal", "0.1.0");

        run(bury_args(&data, "wal", "gone for good")).unwrap();

        let idx = projection(&data);
        let ts = idx.tombstones.get("wal").unwrap();
        assert_eq!(ts.reason, "gone for good");
        assert!(
            ts.superseded_by.is_none(),
            "no `--superseded-by` given — the tombstone must carry no successor"
        );
    }

    #[test]
    fn bury_of_missing_name_is_refused_and_the_journal_does_not_grow() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "org.vibevm", "wal", "0.1.0");
        let before = raw_journal(&data);

        let err = run(bury_args(&data, "ghost", "typo")).unwrap_err();
        match err {
            Error::InvalidInput(msg) => assert!(
                msg.contains("nothing to bury"),
                "R1 must say there is nothing to bury, got: {msg}"
            ),
            other => panic!("expected `InvalidInput`, got {other:?}"),
        }
        assert_eq!(
            before,
            raw_journal(&data),
            "a refused burial must not grow the journal — a `Buried` record \
             for a name that never stood in any group would be a fact that \
             never held"
        );
    }

    #[test]
    fn re_bury_of_a_buried_name_is_refused_and_the_journal_does_not_grow() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "org.vibevm", "wal", "0.1.0");
        run(bury_args(&data, "wal", "renamed")).unwrap();
        let before = raw_journal(&data);

        let err = run(bury_args(&data, "wal", "again")).unwrap_err();
        match err {
            Error::InvalidInput(msg) => assert!(
                msg.contains("already buried"),
                "R2 must say the name is already buried — `nothing to bury` \
                 here would send the operator hunting a problem that does \
                 not exist, got: {msg}"
            ),
            other => panic!("expected `InvalidInput`, got {other:?}"),
        }
        assert_eq!(
            before,
            raw_journal(&data),
            "a repeated burial changes nothing, and a mutation that changes \
             nothing leaves no trace — there must be no second record"
        );
    }

    #[test]
    fn bury_touches_only_its_own_name() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "org.vibevm", "wal", "0.1.0");
        publish(&data, "org.other", "wal", "1.2.3");
        publish(&data, "org.vibevm", "redbook", "1.0.0");

        run(bury_args(&data, "wal", "renamed")).unwrap();

        let idx = projection(&data);
        let group: Group = "org.vibevm".parse().unwrap();
        assert_eq!(
            idx.get(&group, "redbook").unwrap().versions.len(),
            1,
            "another package is not touched"
        );
        assert!(
            !idx.tombstones.contains_key("redbook"),
            "another name must not gain a tombstone"
        );
        assert_eq!(idx.candidates_for("redbook").len(), 1);
    }
}
