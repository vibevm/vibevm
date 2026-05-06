//! `vibe-index capabilities <data-dir> <capability>` — provides-index lookup.

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "List packages providing a given capability.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Capability string (e.g. `ui:landing-page` or `interface:wal`).
    pub capability: String,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("capabilities"))
}
