//! `vibe-index remove <data-dir> <group> <name>` — drop one or all
//! versions of a package from the index, addressed by its `(group,
//! name)` identity (PROP-008 §2.2).
//!
//! Ф3.2 journal form: the published catalog is never this writer's
//! input (PROP-044 §4.4). "Is there something to remove" is answered
//! by folding the journal BEFORE any record is appended — the journal
//! is the truth, and a record of removing what never stood in the
//! projection would be a fact that never held.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use std::path::PathBuf;

use chrono::Utc;
use clap::Parser;
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::memory::{WriteCtx, default_generator};
use crate::journal::{Event, JournalRecord, append, default_dir, project, replay};

#[derive(Debug, Parser)]
#[command(about = "Remove one or all versions of a package from the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Reverse-FQDN group qualifier — e.g. `org.vibevm`.
    pub group: Group,

    pub name: String,

    /// Specific version to remove. If omitted, every version of the
    /// package is removed.
    #[arg(long, value_name = "SEMVER")]
    pub version: Option<String>,
}

pub fn run(args: Args) -> Result<()> {
    // F2-1 — the clock enters here, once per command: the removal's
    // persist is stamped by the command moment, not by the writer.
    let at = Utc::now();
    super::refuse_if_server_running(&args.data_dir)?;

    // Ф3.2 — the catalog is never this writer's input (PROP-044 §4.4):
    // the journal is read from disk exactly once, folded into the
    // projection that answers "is there something to remove", and the
    // record list then carries the appended fact in memory and is
    // re-folded below for the write.
    let journal_dir = default_dir(&args.data_dir);
    let mut records = replay(&journal_dir)?;
    // The probe fold doubles as the gate: `remove_version` /
    // `remove_package` applied to it answer, in their own terms,
    // whether the target stands in the projection — the check is
    // never re-derived here. The mutated probe is discarded: the
    // written catalog comes from re-folding the journal, not from it.
    let mut probe = project(records.iter().cloned())?;
    let version: Option<semver::Version> = match args.version.as_deref() {
        Some(v) => Some(v.parse().map_err(|e| {
            Error::InvalidInput(format!("`--version {v}` is not valid semver: {e}"))
        })?),
        None => None,
    };
    let removed = match &version {
        Some(v) => probe.remove_version(&args.group, &args.name, v),
        None => probe.remove_package(&args.group, &args.name),
    };
    if !removed {
        // The journal carries no false facts: a removal of something
        // that never stood in the projection is refused BEFORE any
        // record is appended, so the journal does not grow here.
        return Err(Error::InvalidInput(match args.version {
            Some(v) => format!(
                "`{}/{}@{}` is not in the index — nothing to remove",
                args.group, args.name, v
            ),
            None => format!(
                "`{}/{}` is not in the index — nothing to remove",
                args.group, args.name
            ),
        }));
    }
    let record = JournalRecord {
        at,
        actor: default_generator(),
        event: Event::Removed {
            group: args.group.clone(),
            name: args.name.clone(),
            version,
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
        "removed {}/{}{}",
        args.group,
        args.name,
        args.version
            .as_deref()
            .map(|v| format!(" @ {v}"))
            .unwrap_or_default()
    );
    Ok(())
}
