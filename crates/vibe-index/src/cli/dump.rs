//! `vibe-index dump <data-dir>` — emit the index contents to stdout.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::error::{Error, Result};
use crate::index::Index;
use crate::index::quarantine::{
    Unavailable, unavailable_for, usable_entries, usable_version_count,
};
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

/// The JSONL stream stays one shape — a `VersionEntry` per line: a
/// line of any other shape in this stream is a break in the wire, and
/// `dump` is bulk export, not an answer by NAME, which is whom
/// PROP-044's no-silence law addresses. The unusable set is visible in
/// `--format json`, and the loader has already named every such
/// version with a WARN line on stderr in this very run.
fn dump_jsonl(index: &Index) -> Result<()> {
    for entry in usable_entries(index) {
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
    let entries: Vec<&VersionEntry> = usable_entries(index).collect();
    let unavailable: Vec<Unavailable> =
        index.by_pkgref.values().flat_map(unavailable_for).collect();
    let mut payload = serde_json::json!({
        "schema_version": index.schema_version,
        "registry": index.registry,
        "registry_url": index.registry_url,
        "naming": index.naming,
        "generated_at": index.generated_at,
        "generator": index.generator,
        "package_count": index.package_count(),
        "version_count": usable_version_count(index),
        "entries": entries,
    });
    // The refusal rows ride NEXT TO the entries and vanish when empty —
    // the same skip-empty rule every surface's `unavailable` field
    // follows; `json!` cannot express it, so the field is inserted
    // after the fact.
    if !unavailable.is_empty()
        && let Some(object) = payload.as_object_mut()
    {
        object.insert("unavailable".to_string(), serde_json::json!(unavailable));
    }
    let pretty = serde_json::to_string_pretty(&payload)
        .map_err(|e| Error::Malformed(format!("could not serialise dump payload: {e}")))?;
    println!("{pretty}");
    Ok(())
}
