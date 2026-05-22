//! `vibe-index dump <data-dir>` — emit the index contents to stdout.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::error::{Error, Result};
use crate::index::Index;
use crate::types::VersionEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum DumpFormat {
    /// JSON Lines — same shape as `primary.jsonl` on disk.
    Jsonl,
    /// Single JSON document with the `Index` struct laid out flat.
    Json,
}

#[derive(Debug, Parser)]
#[command(about = "Dump the entire index to stdout.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Output format. Defaults to JSON Lines.
    #[arg(long, value_enum, default_value_t = DumpFormat::Jsonl)]
    pub format: DumpFormat,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    match args.format {
        DumpFormat::Jsonl => dump_jsonl(&index)?,
        DumpFormat::Json => dump_json(&index)?,
    }
    Ok(())
}

fn dump_jsonl(index: &Index) -> Result<()> {
    for entry in index.iter_versions() {
        let line = serde_json::to_string(entry).map_err(|e| {
            Error::Malformed(format!(
                "could not serialise {}:{}@{} — {e}",
                entry.kind, entry.name, entry.version
            ))
        })?;
        println!("{line}");
    }
    Ok(())
}

fn dump_json(index: &Index) -> Result<()> {
    let entries: Vec<&VersionEntry> = index.iter_versions().collect();
    let payload = serde_json::json!({
        "schema_version": index.schema_version,
        "registry": index.registry,
        "registry_url": index.registry_url,
        "naming": index.naming,
        "generated_at": index.generated_at,
        "generator": index.generator,
        "package_count": index.package_count(),
        "version_count": index.version_count(),
        "entries": entries,
    });
    let pretty = serde_json::to_string_pretty(&payload)
        .map_err(|e| Error::Malformed(format!("could not serialise dump payload: {e}")))?;
    println!("{pretty}");
    Ok(())
}
