//! `vibe-index list <data-dir>` — list packages in the index.

use std::path::PathBuf;

use clap::Parser;

use crate::cli::kinds::PackageKind;
use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "List packages in the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Restrict to one package kind.
    #[arg(long, value_enum)]
    pub kind: Option<PackageKind>,

    /// Maximum number of entries to return.
    #[arg(long, default_value_t = 50)]
    pub limit: usize,

    /// Skip the first N entries (for paginating very large indices).
    #[arg(long, default_value_t = 0)]
    pub offset: usize,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("list"))
}
