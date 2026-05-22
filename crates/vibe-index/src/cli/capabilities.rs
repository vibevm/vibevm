//! `vibe-index capabilities <data-dir> <capability>` — provides-index.

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;

use crate::error::{Error, Result};
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
    let rows: Vec<Row> = entries
        .iter()
        .map(|e| Row {
            kind: e.kind,
            name: e.name.clone(),
            version: e.version.clone(),
            capability_advertised: e
                .provides
                .capabilities
                .iter()
                .find(|c: &&String| {
                    c.starts_with(&args.capability) || args.capability.starts_with(c.as_str())
                })
                .cloned(),
        })
        .collect();

    if args.json {
        let env = Envelope {
            command: "capabilities",
            capability: args.capability.clone(),
            hit_count: rows.len(),
            hits: rows,
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
    }
    Ok(())
}
