//! `vibe-index purls <data-dir> <purl>` — describes-index.

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::index::{Index, search};
use crate::types::PackageKind;

#[derive(Debug, Parser)]
#[command(about = "List packages whose `describes` matches a given PURL.")]
pub struct Args {
    pub data_dir: PathBuf,
    pub purl: String,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct Envelope {
    command: &'static str,
    purl: String,
    hit_count: usize,
    hits: Vec<Row>,
}

#[derive(Debug, Serialize)]
struct Row {
    kind: PackageKind,
    name: String,
    version: Version,
    /// `package` if the package as a whole declared `describes`,
    /// `subskill` if a subskill did.
    binding_site: &'static str,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let entries = search::lookup_purl(&index, &args.purl);
    let rows: Vec<Row> = entries
        .iter()
        .map(|e| {
            let binding_site = if e.describes.as_deref() == Some(args.purl.trim()) {
                "package"
            } else {
                "subskill"
            };
            Row {
                kind: e.kind,
                name: e.name.clone(),
                version: e.version.clone(),
                binding_site,
            }
        })
        .collect();

    if args.json {
        let env = Envelope {
            command: "purls",
            purl: args.purl.clone(),
            hit_count: rows.len(),
            hits: rows,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&env)
                .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
        );
    } else {
        println!("purl      : {}", args.purl);
        println!("hits      : {}", rows.len());
        for r in &rows {
            println!(
                "  {}:{} @ {}  ({})",
                r.kind, r.name, r.version, r.binding_site
            );
        }
    }
    Ok(())
}
