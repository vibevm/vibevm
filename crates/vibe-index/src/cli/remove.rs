//! `vibe-index remove <data-dir> <group> <name>` — drop one or all
//! versions of a package from the index, addressed by its `(group,
//! name)` identity (PROP-008 §2.2).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-008#identity");

use std::path::PathBuf;

use clap::Parser;
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::server::lock::ServerLock;

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
    refuse_if_server_running(&args.data_dir)?;

    let mut index = Index::load_from(&args.data_dir)?;
    let removed = match args.version.as_deref() {
        Some(v) => {
            let parsed: semver::Version = v.parse().map_err(|e| {
                Error::InvalidInput(format!("`--version {v}` is not valid semver: {e}"))
            })?;
            index.remove_version(&args.group, &args.name, &parsed)
        }
        None => index.remove_package(&args.group, &args.name),
    };
    if !removed {
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
    index.write_to(&args.data_dir)?;
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

fn refuse_if_server_running(data_dir: &std::path::Path) -> Result<()> {
    if let Some(pid) = ServerLock::read_pid(data_dir) {
        return Err(Error::InvalidInput(format!(
            "a vibe-index server is running on this data dir (PID {pid}). \
             Use the HTTP API or stop the server first."
        )));
    }
    Ok(())
}
