//! `vibe-index capabilities <data-dir> <capability>` — provides-index.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::index::quarantine::{self, Unavailable};
use crate::index::{Index, search};
use crate::types::PackageKind;

#[derive(Debug, Parser)]
#[command(about = "List packages providing a given capability.")]
pub struct Args {
    pub data_dir: PathBuf,
    pub capability: String,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct Envelope {
    command: &'static str,
    capability: String,
    hit_count: usize,
    hits: Vec<Row>,
    /// Unusable versions that WOULD have matched the requested
    /// capability — named, not hidden (PROP-044 §4.5). Absent when
    /// there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    unavailable: Vec<Unavailable>,
}

#[derive(Debug, Serialize)]
struct Row {
    kind: PackageKind,
    name: String,
    version: Version,
    capability_advertised: Option<String>,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let entries = search::lookup_capability(&index, &args.capability);
    let unavailable = unavailable_providing(&index, &args.capability);
    let rows: Vec<Row> = entries
        .iter()
        .map(|e| Row {
            kind: e.kind.clone(),
            name: e.name.clone(),
            version: e.version.clone(),
            capability_advertised: e.provides.as_ref().and_then(|p| {
                p.capabilities
                    .iter()
                    .find(|c: &&String| {
                        c.starts_with(&args.capability) || args.capability.starts_with(c.as_str())
                    })
                    .cloned()
            }),
        })
        .collect();

    if args.json {
        let env = Envelope {
            command: "capabilities",
            capability: args.capability.clone(),
            hit_count: rows.len(),
            hits: rows,
            unavailable,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&env)
                .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
        );
    } else {
        println!("capability: {}", args.capability);
        println!("hits      : {}", rows.len());
        for r in &rows {
            print!("  {}:{} @ {}", r.kind, r.name, r.version);
            if let Some(c) = &r.capability_advertised {
                print!("  ({c})");
            }
            println!();
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

/// The refusal pass: unusable versions that WOULD have matched the
/// requested capability. `search::lookup_capability` walks only the
/// versions this build can act on (the answering default since F62A),
/// so naming the refused ones takes a second pass over the RAW stored
/// vector.
///
/// The pass lives in the verb, but the QUESTION it asks does not: both
/// passes call `search::provides_capability`, so they cannot drift into
/// answering different things about the same capability.
fn unavailable_providing(index: &Index, capability: &str) -> Vec<Unavailable> {
    let cap_norm = capability.trim();
    let mut out = Vec::new();
    for pkg in index.by_pkgref.values() {
        for v in &pkg.versions {
            if quarantine::is_usable(v) || !search::provides_capability(v, cap_norm) {
                continue;
            }
            let missing = quarantine::missing_capabilities(&v.must_understand);
            out.push(Unavailable {
                group: pkg.group.clone(),
                name: pkg.name.clone(),
                version: v.version.clone(),
                recipe: quarantine::recipe_for(&missing),
                missing,
            });
        }
    }
    out
}
