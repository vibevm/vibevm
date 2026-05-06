//! `vibe-index get <data-dir> <kind> <name>` — read one package entry.

use std::path::PathBuf;

use clap::Parser;

use crate::cli::kinds::PackageKind;
use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Read one package entry from the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Package kind: flow, feat, stack, or tool.
    #[arg(value_enum)]
    pub kind: PackageKind,

    /// Package name.
    pub name: String,

    /// Specific version. If omitted, returns all versions of the package.
    #[arg(long, value_name = "SEMVER")]
    pub version: Option<String>,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("get"))
}
