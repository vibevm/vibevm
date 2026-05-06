//! `vibe-index dump <data-dir>` — dump the entire index to stdout.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum DumpFormat {
    Jsonl,
    Json,
    Toml,
}

#[derive(Debug, Parser)]
#[command(about = "Dump the entire index to stdout.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Output format. Defaults to JSON Lines (the on-disk shape of `primary.jsonl`).
    #[arg(long, value_enum, default_value_t = DumpFormat::Jsonl)]
    pub format: DumpFormat,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("dump"))
}
