//! `vibe-index remove <data-dir> <kind> <name>` — drop one or all
//! versions of a package from the index.

use std::path::PathBuf;

use clap::Parser;

use crate::cli::kinds::PackageKind;
use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Remove one or all versions of a package from the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    #[arg(value_enum)]
    pub kind: PackageKind,

    pub name: String,

    /// Specific version to remove. If omitted, every version of the
    /// package is removed.
    #[arg(long, value_name = "SEMVER")]
    pub version: Option<String>,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("remove"))
}
