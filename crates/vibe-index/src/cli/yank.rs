//! `vibe-index yank <data-dir> <group> <name> --version <SEMVER>
//! --reason <TEXT>` — withdraw ONE version of a package, addressed by
//! its `(group, name)` identity (PROP-008 §2.2). The withdrawal trio's
//! middle arm: the entry STAYS (a build that pinned this version keeps
//! working), the wire carries `yanked`, and fresh resolution stops
//! choosing it — where `remove` deletes and `bury` closes the name.
//!
//! Ф3.2 journal form: the published catalog is never this writer's
//! input (PROP-044 §4.4). "Is there something to yank" is answered by
//! folding the journal BEFORE any record is appended — the journal is
//! the truth, and a `Yanked` record for a version that never stood in
//! the projection would be a fact that never held. A REPEATED yank is
//! refused for the other reason: phase 2 established that a mutation
//! which changes nothing leaves no trace, and yanking an already-yanked
//! version changes nothing — so the second record must not exist.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use std::path::PathBuf;

use chrono::Utc;
use clap::Parser;
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::memory::{WriteCtx, default_generator};
use crate::journal::{Event, JournalRecord, append, default_dir, project, replay};

#[derive(Debug, Parser)]
#[command(
    about = "Yank one version of a package: the entry stays, fresh resolution stops choosing it."
)]
pub struct Args {
    pub data_dir: PathBuf,

    /// Reverse-FQDN group qualifier — e.g. `org.vibevm`.
    pub group: Group,

    pub name: String,

    /// The version to yank. Required, unlike `remove`'s optional flag:
    /// a yank withdraws exactly one version — "yank everything" is not
    /// a yank.
    #[arg(long, value_name = "SEMVER")]
    pub version: String,

    /// Why the version is withdrawn. Required: the journal fact carries
    /// `reason` with no optionality, and none may be synthesised on the
    /// operator's behalf.
    #[arg(long, value_name = "TEXT")]
    pub reason: String,
}

pub fn run(args: Args) -> Result<()> {
    // F2-1 — the clock enters here, once per command: the same `at`
    // stamps the fact and the written catalog, so the two records born
    // of one command never differ by a millisecond.
    let at = Utc::now();
    super::refuse_if_server_running(&args.data_dir)?;

    // Ф3.2 — the catalog is never this writer's input (PROP-044 §4.4):
    // the journal is read from disk exactly once, folded into the
    // projection that answers "is there something to yank", and the
    // record list then carries the appended fact in memory and is
    // re-folded below for the write.
    let journal_dir = default_dir(&args.data_dir);
    let mut records = replay(&journal_dir)?;
    // The probe fold is the gate, and it is only ever READ here —
    // unlike `remove`, whose gate mutates via `remove_version`, a yank
    // asks two questions of the standing state and changes nothing
    // until the fact is appended below.
    let probe = project(records.iter().cloned())?;
    let version: semver::Version = args.version.parse().map_err(|e| {
        Error::InvalidInput(format!(
            "`--version {}` is not valid semver: {e}",
            args.version
        ))
    })?;
    // R1 — the journal carries no false facts: yanking a version that
    // does not stand in the projection is refused BEFORE any record is
    // appended, so the journal does not grow here.
    let standing = probe
        .get(&args.group, &args.name)
        .and_then(|pkg| pkg.versions.iter().find(|v| v.version == version));
    let Some(entry) = standing else {
        return Err(Error::InvalidInput(format!(
            "`{}/{}@{}` is not in the index — nothing to yank",
            args.group, args.name, version
        )));
    };
    // R2 — a mutation that changes nothing leaves no trace: a repeated
    // yank is refused with its OWN message, because "nothing to yank"
    // shown to a version that IS yanked would send the operator hunting
    // a problem that does not exist.
    if entry.yanked {
        return Err(Error::InvalidInput(format!(
            "`{}/{}@{}` is already yanked — a repeated yank changes nothing, \
             so the journal records no second fact",
            args.group, args.name, version
        )));
    }
    let record = JournalRecord {
        at,
        actor: default_generator(),
        event: Event::Yanked {
            group: args.group.clone(),
            name: args.name.clone(),
            version,
            reason: args.reason.clone(),
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
        "yanked {}/{} @ {} ({})",
        args.group, args.name, args.version, args.reason
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NamingConvention, PackageKind, VersionEntry};
    use std::path::Path;

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

    fn publish(data: &Path, name: &str, version: &str) {
        append(
            &default_dir(data),
            &JournalRecord {
                at: Utc::now(),
                actor: default_generator(),
                event: Event::Published {
                    entry: Box::new(VersionEntry::minimal(
                        PackageKind::Flow,
                        "org.vibevm".parse().unwrap(),
                        name,
                        version.parse().unwrap(),
                        Utc::now(),
                    )),
                },
            },
        )
        .unwrap();
    }

    fn yank_args(data: &Path, name: &str, version: &str, reason: &str) -> Args {
        Args {
            data_dir: data.to_path_buf(),
            group: "org.vibevm".parse().unwrap(),
            name: name.to_string(),
            version: version.to_string(),
            reason: reason.to_string(),
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

    /// The `yanked` flag of one version, read off the journal's own
    /// projection — the same fold the command's probe runs.
    fn yanked_flag(data: &Path, name: &str, version: &str) -> bool {
        let group: Group = "org.vibevm".parse().unwrap();
        let idx = project(replay(&default_dir(data)).unwrap()).unwrap();
        let pkg = idx.get(&group, name).unwrap();
        let want: semver::Version = version.parse().unwrap();
        pkg.versions
            .iter()
            .find(|e| e.version == want)
            .unwrap()
            .yanked
    }

    #[test]
    fn yank_marks_the_version_and_appends_exactly_one_fact() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "wal", "0.1.0");
        let before = replay(&default_dir(&data)).unwrap().len();

        run(yank_args(&data, "wal", "0.1.0", "broken build")).unwrap();

        let records = replay(&default_dir(&data)).unwrap();
        assert_eq!(
            records.len(),
            before + 1,
            "a yank must append exactly one record, got {} before {} after",
            before,
            records.len()
        );
        match &records[records.len() - 1].event {
            Event::Yanked {
                group,
                name,
                version,
                reason,
            } => {
                assert_eq!(group.to_string(), "org.vibevm");
                assert_eq!(name, "wal");
                assert_eq!(version.to_string(), "0.1.0");
                assert_eq!(reason, "broken build");
            }
            other => panic!("expected `Yanked`, got {other:?}"),
        }
        assert!(
            yanked_flag(&data, "wal", "0.1.0"),
            "the projection must carry yanked = true for the yanked version"
        );
    }

    #[test]
    fn yank_of_missing_version_is_refused_and_the_journal_does_not_grow() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "wal", "0.1.0");
        let before = raw_journal(&data);

        let err = run(yank_args(&data, "wal", "9.9.9", "typo")).unwrap_err();
        match err {
            Error::InvalidInput(msg) => assert!(
                msg.contains("nothing to yank"),
                "R1 must say there is nothing to yank, got: {msg}"
            ),
            other => panic!("expected `InvalidInput`, got {other:?}"),
        }
        assert_eq!(
            before,
            raw_journal(&data),
            "a refused yank must not grow the journal — a `Yanked` record for \
             a version that never stood in the projection would be a fact \
             that never held"
        );
    }

    #[test]
    fn re_yank_of_a_yanked_version_is_refused_and_the_journal_does_not_grow() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "wal", "0.1.0");
        run(yank_args(&data, "wal", "0.1.0", "broken build")).unwrap();
        let before = raw_journal(&data);

        let err = run(yank_args(&data, "wal", "0.1.0", "again")).unwrap_err();
        match err {
            Error::InvalidInput(msg) => assert!(
                msg.contains("already yanked"),
                "R2 must say the version is already yanked — `nothing to \
                 yank` here would send the operator hunting a problem that \
                 does not exist, got: {msg}"
            ),
            other => panic!("expected `InvalidInput`, got {other:?}"),
        }
        assert_eq!(
            before,
            raw_journal(&data),
            "a repeated yank changes nothing, and a mutation that changes \
             nothing leaves no trace — there must be no second record"
        );
    }

    #[test]
    fn yank_touches_only_its_own_version() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        initialised(&data);
        publish(&data, "wal", "0.1.0");
        publish(&data, "wal", "0.2.0");
        publish(&data, "redbook", "1.0.0");

        run(yank_args(&data, "wal", "0.1.0", "broken build")).unwrap();

        assert!(yanked_flag(&data, "wal", "0.1.0"));
        assert!(
            !yanked_flag(&data, "wal", "0.2.0"),
            "the sibling version of the same package keeps standing unyanked"
        );
        assert!(
            !yanked_flag(&data, "redbook", "1.0.0"),
            "another package is not touched"
        );
        // A yank is a withdrawal, not a removal: the version REMAINS in
        // the projection, so a build that pinned it keeps resolving.
        let group: Group = "org.vibevm".parse().unwrap();
        let idx = project(replay(&default_dir(&data)).unwrap()).unwrap();
        assert_eq!(
            idx.get(&group, "wal").unwrap().versions.len(),
            2,
            "the yanked version must still stand in the projection"
        );
    }
}
