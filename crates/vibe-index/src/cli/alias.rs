//! `vibe-index alias <data-dir> add|remove|list` — the registry
//! owner's edits to the short spellings this registry offers
//! (PROP-008 §2.6).
//!
//! The table is an authored INPUT (`<data-dir>/aliases.toml`,
//! [`crate::aliases`]), not a journal fact, so this verb does not
//! append anything: it reads the table, changes one row, validates the
//! whole table against the catalog the journal projects, writes the
//! table and reprojects. Table and catalog therefore move together or
//! not at all — a row that reached the file without reaching
//! `by-name/<alias>.json` would be an alias that silently did nothing.
//!
//! **A writer refuses what a reader would merely drop.** The operator
//! is standing right here and the file is theirs: a row this command
//! accepted and the catalog then quietly discarded would surface much
//! later, to somebody else, as a short name that simply does not work.
//! The reader's tolerance lives in [`crate::aliases::attach`] and is
//! for other people's registries.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::aliases::{self, AliasTable};
use crate::error::{Error, Result};
use crate::index::memory::WriteCtx;
use crate::journal::{default_dir, project, replay};
use crate::types::AliasTarget;

#[derive(Debug, Parser)]
#[command(about = "Manage the short spellings this registry offers for its packages.")]
pub struct Args {
    pub data_dir: PathBuf,

    #[command(subcommand)]
    pub action: Action,
}

#[derive(Debug, Subcommand)]
pub enum Action {
    /// Offer `<alias>` as a short spelling of `<group>/<name>`.
    Add {
        /// The short spelling. Obeys the package-name grammar, because
        /// it is typed exactly where a package name is typed.
        alias: String,
        /// The package it stands for — fully qualified, never
        /// versioned.
        target: String,
        /// Replace a row that already offers this spelling. Without it
        /// a second target for one alias is refused, so a table never
        /// changes meaning by accident.
        #[arg(long)]
        replace: bool,
    },
    /// Withdraw the spelling `<alias>`.
    Remove { alias: String },
    /// Print the table, one `alias -> group/name` per line.
    List,
}

pub fn run(args: Args) -> Result<()> {
    let data_dir = args.data_dir.clone();
    let mut table = AliasTable::read(&data_dir)?;
    match args.action {
        Action::List => return list(&table),
        Action::Remove { alias } => {
            if table.remove(&alias).is_none() {
                return Err(Error::InvalidInput(format!(
                    "this registry offers no alias `{alias}` — `vibe-index alias {} list` \
                     shows the ones it does",
                    data_dir.display()
                )));
            }
            println!("withdrew the alias `{alias}`");
        }
        Action::Add {
            alias,
            target,
            replace,
        } => {
            let target = parse_target(&target)?;
            if let Some(held) = table.get(&alias) {
                if held == &target {
                    // A mutation that changes nothing writes nothing
                    // (F2-3): no file, no reprojection, no commit.
                    println!("`{alias}` already means {}", aliases::spell(&target));
                    return Ok(());
                }
                if !replace {
                    return Err(Error::InvalidInput(format!(
                        "`{alias}` already means {} in this registry — one spelling has one \
                         meaning, so re-pointing it is a deliberate act: pass `--replace` to \
                         change it, or pick another spelling",
                        aliases::spell(held),
                    )));
                }
            }
            println!("`{alias}` now means {}", aliases::spell(&target));
            table.insert(alias, target);
        }
    }
    super::refuse_if_server_running(&data_dir)?;

    // The whole table is validated, not just the edited row: a row that
    // was sound when it was written can be made unsound by what has
    // happened since — a package published under its spelling, a target
    // removed — and the operator editing the table now is the one who
    // can act on that.
    let mut index = project(replay(&default_dir(&data_dir))?)?;
    let faults = aliases::check(&index, &table);
    if !faults.is_empty() {
        let lines: Vec<String> = faults.iter().map(ToString::to_string).collect();
        return Err(Error::InvalidInput(format!(
            "the alias table would not stand:\n  {}",
            lines.join("\n  ")
        )));
    }
    // Table and catalog move together: the table lands first because it
    // is the authored truth, and the projection that follows is what
    // every client actually reads.
    table.write(&data_dir)?;
    aliases::attach(&mut index, table);
    // **No clock enters here**, and that is the whole reason this verb
    // appends no journal record either. The catalog's `generated_at`
    // means "this reflects the journal up to that moment", and the
    // journal did not move — so stamping `now()` would make
    // `rebuild --check` fail for ever on any registry that offers a
    // spelling: the rebuild projects the journal's last moment, and a
    // catalog stamped later carries a fact its inputs cannot explain.
    // The stamp therefore stays the fold's own, which is derivable
    // from the inputs exactly as the proof requires (PROP-044 §4.4).
    let at = index.generated_at;
    index.write_to(&data_dir, &WriteCtx { at })
}

fn list(table: &AliasTable) -> Result<()> {
    for (alias, target) in table.iter() {
        println!("{alias} -> {}", aliases::spell(target));
    }
    Ok(())
}

fn parse_target(spelling: &str) -> Result<AliasTarget> {
    let parsed = vibe_core::PackageRef::parse(spelling)
        .map_err(|e| Error::InvalidInput(format!("`{spelling}` is not a pkgref: {e}")))?;
    let Some(group) = parsed.group else {
        return Err(Error::InvalidInput(format!(
            "`{spelling}` carries no group — an alias target is always the fully qualified \
             `<group>/<name>`, because supplying the group is the whole point of the alias"
        )));
    };
    if !matches!(parsed.version, vibe_core::VersionSpec::Latest) {
        return Err(Error::InvalidInput(format!(
            "`{spelling}` pins a version — an alias is a spelling of a PACKAGE, so version \
             selection stays the operator's"
        )));
    }
    Ok(AliasTarget {
        group,
        name: parsed.name.as_str().to_owned(),
    })
}

#[cfg(test)]
#[path = "alias/tests.rs"]
mod tests;
