//! `vibe-index serve <data-dir>` — boot the HTTP server.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Run the HTTP server.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Address to bind. Default: `127.0.0.1:8412` (local-only).
    #[arg(long, value_name = "ADDR", default_value = "127.0.0.1:8412")]
    pub bind: SocketAddr,

    /// File containing one bearer token per line (gitignored, mode 0600).
    #[arg(long, value_name = "FILE")]
    pub auth_tokens_file: Option<PathBuf>,

    /// Refuse every mutating endpoint regardless of auth.
    #[arg(long)]
    pub read_only: bool,

    /// After every successful mutation, `git add -A && git commit && git push`
    /// in the data directory. Requires the data directory to be a git
    /// working tree with a configured `origin`. Off by default; v0
    /// expects the operator to commit/push manually.
    #[arg(long)]
    pub auto_commit_push: bool,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("serve"))
}
