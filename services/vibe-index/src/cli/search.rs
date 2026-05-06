//! `vibe-index search <data-dir> <query>` — full-text search.

use std::path::PathBuf;

use clap::Parser;

use crate::cli::kinds::PackageKind;
use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Full-text search across the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Free-form query string. Tokenised on whitespace; matched against
    /// package name, description, keywords, capabilities, and PURLs.
    pub query: String,

    /// Restrict to one package kind.
    #[arg(long, value_enum)]
    pub kind: Option<PackageKind>,

    /// Maximum number of hits to return.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("search"))
}
