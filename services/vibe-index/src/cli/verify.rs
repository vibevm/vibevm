//! `vibe-index verify <data-dir>` — recompute file hashes and check
//! `repomd.json` integrity.

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Recompute file hashes and check repomd.json integrity.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("verify"))
}
