//! `vibe-index purls <data-dir> <purl>` — describes-index.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::index::quarantine::{self, Unavailable};
use crate::index::{Index, search};
use crate::types::PackageKind;
use crate::wire_count::checked_u32;

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
    hit_count: u32,
    hits: Vec<Row>,
    /// Unusable versions that WOULD have matched the requested PURL —
    /// named, not hidden (PROP-044 §4.5). Absent when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    unavailable: Vec<Unavailable>,
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
    let purl_norm = args.purl.trim().to_string();
    let unavailable = quarantine::refused_where(&index, |v| search::describes_purl(v, &purl_norm));
    let rows: Vec<Row> = entries
        .iter()
        .map(|e| {
            let binding_site = if e.describes.as_deref() == Some(args.purl.trim()) {
                "package"
            } else {
                "subskill"
            };
            Row {
                kind: e.kind.clone(),
                name: e.name.clone(),
                version: e.version.clone(),
                binding_site,
            }
        })
        .collect();

    if args.json {
        let hit_count = checked_u32("hit_count", rows.len())?;
        let env = Envelope {
            command: "purls",
            purl: args.purl.clone(),
            hit_count,
            hits: rows,
            unavailable,
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
        if !unavailable.is_empty() {
            println!("unavailable: {}", unavailable.len());
            for u in &unavailable {
                println!(
                    "  - {}:{}@{}  missing: {}",
                    u.group,
                    u.name,
                    u.version,
                    u.missing.join(",")
                );
                println!("    {}", u.recipe);
            }
        }
    }
    Ok(())
}
