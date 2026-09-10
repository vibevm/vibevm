//! `vibe-index rebuild <data-dir> --check` — prove that the on-disk
//! catalog is the byte-exact projection of its journal.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#persistence");

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Prove the catalog is byte-identical to its journal's projection.")]
pub struct Args {
    /// Index data directory whose journal and catalog are compared.
    pub data_dir: PathBuf,

    /// Reproject into a disposable scratch directory and byte-compare.
    /// This command intentionally has no in-place repair mode.
    #[arg(long)]
    pub check: bool,
}

pub fn run(args: Args) -> Result<()> {
    if !args.check {
        return Err(Error::InvalidInput(
            "rebuild: only `--check` exists — repairing the catalog from its journal in place is \
             a separate decision; this verb ships the proof only"
                .to_string(),
        ));
    }
    crate::rebuild::run_check(&args.data_dir)
}
