//! `vibe-index outdated <data-dir>` — compare a `vibe.lock` against
//! the index and report upgrade candidates.

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Compare a vibe.lock against the index and report outdated entries.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Path to the lockfile to compare against.
    #[arg(long, value_name = "PATH", default_value = "vibe.lock")]
    pub lockfile: PathBuf,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("outdated"))
}
