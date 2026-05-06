//! `vibe-index init <data-dir>` — initialise an empty index data
//! directory. Slice 1 stub. Real impl lands in slice 2.

use std::path::PathBuf;

use clap::Parser;

use crate::cli::kinds::NamingConvention;
use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Initialise an empty index data directory.")]
pub struct Args {
    /// Path to the data directory to initialise. Created if missing.
    pub data_dir: PathBuf,

    /// Registry name (the `[[registry]].name` value this index serves).
    #[arg(long, value_name = "NAME")]
    pub registry: Option<String>,

    /// Registry URL — the org root URL the package repos sit under.
    #[arg(long, value_name = "URL")]
    pub registry_url: Option<String>,

    /// Naming convention used by this org for package repo names.
    #[arg(long, value_enum, default_value_t = NamingConvention::KindName)]
    pub naming: NamingConvention,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("init"))
}
