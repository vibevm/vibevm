//! `vibe-index reindex <data-dir>` — (re)build the index from the
//! authoritative package state. Slice 1 stub.

use std::path::PathBuf;

use clap::{ArgGroup, Parser};

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(
    about = "(Re)build the index from authoritative package state.",
    group = ArgGroup::new("source").required(true).args(["from_clones", "from_github", "from_gitverse"]),
    group = ArgGroup::new("scope").args(["full", "incremental"]),
)]
pub struct Args {
    pub data_dir: PathBuf,

    /// Walk a local directory of org clones (bare or regular).
    #[arg(long, value_name = "ORG-DIR")]
    pub from_clones: Option<PathBuf>,

    /// Walk a GitHub org via the REST API.
    #[arg(long, value_name = "ORG")]
    pub from_github: Option<String>,

    /// Walk a GitVerse org (stub today; emits not-implemented).
    #[arg(long, value_name = "ORG")]
    pub from_gitverse: Option<String>,

    /// File containing the host API token (one line, no trailing newline).
    #[arg(long, value_name = "FILE")]
    pub token_file: Option<PathBuf>,

    /// Force a full rebuild even if a checkpoint exists.
    #[arg(long)]
    pub full: bool,

    /// Apply only the diff against the previous checkpoint.
    #[arg(long, conflicts_with = "full")]
    pub incremental: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("reindex"))
}
