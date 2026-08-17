//! `vibe-index init <data-dir>` — initialise an empty index data
//! directory.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use chrono::Utc;
use clap::Parser;

use crate::cli::kinds::NamingConvention;
use crate::error::{Error, Result};
use crate::index::memory::{WriteCtx, default_generator};
use crate::index::{Index, repomd};
use crate::journal::{Event, JournalRecord, append, default_dir};

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

    /// Naming convention used by this org for package repo names:
    /// `fqdn` (the default — the reverse-FQDN `<group>.<name>` shape
    /// every group-native registry uses, PROP-008 §2.5), `kind-name`,
    /// `name`, or `kind/name`. A closed vocabulary: repository paths
    /// are built from it, so an unfamiliar value is refused, not
    /// guessed.
    #[arg(long, value_name = "NAMING", default_value = "fqdn")]
    pub naming: String,

    /// Force initialisation even when the data directory already
    /// carries a repomd.json. The existing files are overwritten.
    #[arg(long)]
    pub force: bool,
}

/// Parse the `--naming` flag at the argument boundary. Unlike
/// `--kind` this vocabulary is closed — there is no `Unknown` to
/// carry — so an unfamiliar string is simply not a convention this
/// build can build paths from.
fn parse_naming_flag(value: &str) -> Result<NamingConvention> {
    match value {
        "fqdn" => Ok(NamingConvention::Fqdn),
        "kind-name" => Ok(NamingConvention::KindName),
        "name" => Ok(NamingConvention::Name),
        "kind/name" => Ok(NamingConvention::KindSlashName),
        other => Err(Error::InvalidInput(format!(
            "naming convention `{other}` is unknown — expected one of: fqdn, kind-name, name, kind/name"
        ))),
    }
}

pub fn run(args: Args) -> Result<()> {
    // F2-1 — the clock enters here, once per command, at the edge.
    // `index/` never calls `now()` itself: one state must produce one
    // byte sequence, or "rebuild and compare" measures nothing.
    let at = Utc::now();
    if repomd::exists(&args.data_dir) && !args.force {
        return Err(Error::InvalidInput(format!(
            "data directory `{}` already carries an index (use --force to overwrite)",
            args.data_dir.display()
        )));
    }
    let naming = parse_naming_flag(&args.naming)?;
    let index = Index::new(&args.registry, &args.registry_url, naming.clone(), at);

    // Truth first (PROP-044 `##LAW-NO-UNRECOVERABLE`): the journal
    // record lands BEFORE the catalog write. A failed `write_to` then
    // leaves a journal without a catalog — recoverable, a re-run of
    // the command rebuilds the derived side — while the reverse order
    // could leave a catalog whose truth never existed. A repeated
    // `init --force` appends a SECOND Initialised record on purpose:
    // re-initialisation with a different identity is a real fact, and
    // last-one-wins is the projector's fold, not this writer's call.
    append(
        &default_dir(&args.data_dir),
        &JournalRecord {
            at,
            actor: default_generator(),
            event: Event::Initialised {
                registry: args.registry.clone(),
                registry_url: args.registry_url.clone(),
                naming,
            },
        },
    )?;
    index.write_to(&args.data_dir, &WriteCtx { at })?;
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
        # Index files (hello.json, repomd.json, primary.jsonl[.gz],\n\
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
        Format: [PROP-005](https://gitverse.ru/vibevm/vibevm/raw/branch/main/spec/modules/vibe-index/PROP-005-package-index.md).\n\
        \n\
        ## Files\n\
        \n\
        - `hello.json` — the eternal handshake: the client's entry point,\n\
          naming the worlds that exist and where each lives.\n\
        - `repomd.json` — manifest with sha256 of every other file of the\n\
          catalog.\n\
        - `primary.jsonl` / `primary.jsonl.gz` — one `VersionEntry` per line.\n\
        - `by-name/<name>.json` — candidate set for one bare name (every group).\n\
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
        See `crates/vibe-index/docs/` in the vibevm source tree for\n\
        the full operator handbook + consumer protocol + format reference.\n"
    );
    std::fs::write(&path, body).map_err(|e| Error::Io {
        path,
        message: e.to_string(),
    })
}
