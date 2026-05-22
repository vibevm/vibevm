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
    write_gitignore(&args.data_dir)?;
    write_readme(&args.data_dir, &index.registry, &index.registry_url)?;
    println!(
        "Initialised empty index for `{}` at `{}` ({}, naming = {})",
        index.registry,
        args.data_dir.display(),
        index.registry_url,
        index.naming,
    );
    Ok(())
}

fn write_gitignore(data_dir: &std::path::Path) -> Result<()> {
    let path = data_dir.join(".gitignore");
    if path.exists() {
        return Ok(());
    }
    let body = "# vibe-index — local server / runtime state.\n\
        # Index files (repomd.json, primary.jsonl[.gz],\n\
        # by-name/, by-cap/, by-purl/) are tracked; everything\n\
        # under state/ is per-host runtime data and stays out of\n\
        # the source tree.\n\
        /state/\n";
    std::fs::write(&path, body).map_err(|e| Error::Io {
        path,
        message: e.to_string(),
    })
}

fn write_readme(data_dir: &std::path::Path, registry: &str, registry_url: &str) -> Result<()> {
    let path = data_dir.join("README.md");
    if path.exists() {
        return Ok(());
    }
    let body = format!(
        "# vibe-index — `{registry}`\n\
        \n\
        Metadata index for the vibevm registry `{registry}` (`{registry_url}`).\n\
        Format: [PROP-005](https://gitverse.ru/anarchic/vibevm/raw/branch/main/spec/modules/vibe-index/PROP-005-package-index.md).\n\
        \n\
        ## Files\n\
        \n\
        - `repomd.json` — manifest with sha256 of every other file.\n\
        - `primary.jsonl` / `primary.jsonl.gz` — one `VersionEntry` per line.\n\
        - `by-name/<kind>/<name>.json` — cargo-sparse-style per-package.\n\
        - `by-cap/<slug>.jsonl` — inverted index by advertised capability.\n\
        - `by-purl/<slug>.jsonl` — inverted index by `describes` PURL.\n\
        - `state/` — gitignored runtime data (server PID, admin tokens,\n\
          incremental-reindex checkpoint).\n\
        \n\
        ## Maintenance\n\
        \n\
        Refresh from the authoritative org clones with:\n\
        \n\
        ```sh\n\
        vibe-index reindex . --from-clones <org-dir> --incremental\n\
        ```\n\
        \n\
        Or walk a GitHub org directly:\n\
        \n\
        ```sh\n\
        vibe-index reindex . --from-github <org> --token-file <pat-file>\n\
        ```\n\
        \n\
        See `services/vibe-index/docs/` in the vibevm source tree for\n\
        the full operator handbook + consumer protocol + format reference.\n"
    );
    std::fs::write(&path, body).map_err(|e| Error::Io {
        path,
        message: e.to_string(),
    })
}
