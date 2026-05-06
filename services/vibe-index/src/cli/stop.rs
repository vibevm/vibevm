//! `vibe-index stop <data-dir>` — gracefully stop a running server.

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Gracefully stop a running server (PID-based).")]
pub struct Args {
    pub data_dir: PathBuf,
}

pub fn run(_args: Args) -> Result<()> {
    Err(Error::NotYetImplemented("stop"))
}
