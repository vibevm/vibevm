//! `vibe-index purls <data-dir> <purl>` — describes-index lookup.

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "List packages whose `describes` matches a given PURL.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Package URL (e.g. `pkg:cargo/sqlx@0.8.0`).
    pub purl: String,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("purls"))
}
