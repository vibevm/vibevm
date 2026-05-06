//! `vibe-index add <data-dir>` — insert/upsert a single index entry
//! from a `vibe-package.toml` manifest.

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Insert/upsert a single index entry from a vibe-package.toml manifest.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Path to the `vibe-package.toml` whose entry should be inserted.
    #[arg(long, value_name = "PATH")]
    pub manifest: PathBuf,

    /// Canonical clone URL recorded on the index entry.
    #[arg(long, value_name = "URL")]
    pub repo_url: Option<String>,

    /// Git ref the content was fetched at (defaults to `v<semver>`).
    #[arg(long, value_name = "REF")]
    pub r#ref: Option<String>,

    /// Commit SHA the ref resolved to.
    #[arg(long, value_name = "SHA")]
    pub commit: Option<String>,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("add"))
}
