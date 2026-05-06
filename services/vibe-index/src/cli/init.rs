//! `vibe-index init <data-dir>` — initialise an empty index data
//! directory.

use std::path::PathBuf;

use clap::Parser;

use crate::cli::kinds::NamingConvention;
use crate::error::{Error, Result};
use crate::index::{Index, repomd};

#[derive(Debug, Parser)]
#[command(about = "Initialise an empty index data directory.")]
pub struct Args {
    /// Path to the data directory to initialise. Created if missing.
    pub data_dir: PathBuf,

    /// Registry name (the `[[registry]].name` value this index serves).
    #[arg(long, value_name = "NAME")]
    pub registry: String,

    /// Registry URL — the org root URL the package repos sit under.
    #[arg(long, value_name = "URL")]
    pub registry_url: String,

    /// Naming convention used by this org for package repo names.
    #[arg(long, value_enum, default_value_t = NamingConvention::KindName)]
    pub naming: NamingConvention,

    /// Force initialisation even when the data directory already
    /// carries a repomd.json. The existing files are overwritten.
    #[arg(long)]
    pub force: bool,
}

pub fn run(args: Args) -> Result<()> {
    if repomd::exists(&args.data_dir) && !args.force {
        return Err(Error::InvalidInput(format!(
            "data directory `{}` already carries an index (use --force to overwrite)",
            args.data_dir.display()
        )));
    }
    let index = Index::new(&args.registry, &args.registry_url, args.naming);
    index.write_to(&args.data_dir)?;
    println!(
        "Initialised empty index for `{}` at `{}` ({}, naming = {})",
        index.registry,
        args.data_dir.display(),
        index.registry_url,
        index.naming,
    );
    Ok(())
}
