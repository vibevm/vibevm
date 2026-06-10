//! `vibe-index list <data-dir>` — list packages.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;
use vibe_core::Group;

use crate::cli::kinds::PackageKind;
use crate::error::{Error, Result};
use crate::index::Index;

#[derive(Debug, Parser)]
#[command(about = "List packages in the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    #[arg(long, value_enum)]
    pub kind: Option<PackageKind>,

    #[arg(long, default_value_t = 50)]
    pub limit: usize,

    #[arg(long, default_value_t = 0)]
    pub offset: usize,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct Envelope {
    command: &'static str,
    registry: String,
    package_count: u32,
    returned: usize,
    offset: usize,
    limit: usize,
    packages: Vec<PackageRow>,
}

#[derive(Debug, Serialize)]
struct PackageRow {
    /// `kind` is metadata (PROP-008 §2.3) — read from the package's
    /// versions; `None` only for the (rare) zero-version package row.
    kind: Option<PackageKind>,
    group: Group,
    name: String,
    versions: Vec<Version>,
    latest_stable: Option<Version>,
    description: Option<String>,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let mut rows: Vec<PackageRow> = index
        .by_pkgref
        .values()
        .filter(|p| {
            args.kind
                .is_none_or(|k| p.versions.iter().any(|v| v.kind == k))
        })
        .map(|p| {
            let description = p.versions.last().and_then(|v| v.description.clone());
            PackageRow {
                kind: p.versions.first().map(|v| v.kind),
                group: p.group.clone(),
                name: p.name.clone(),
                versions: p.versions.iter().map(|v| v.version.clone()).collect(),
                latest_stable: p.latest_stable.clone(),
                description,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.group.cmp(&b.group).then(a.name.cmp(&b.name)));
    let package_count = rows.len() as u32;
    let returned: Vec<PackageRow> = rows
        .into_iter()
        .skip(args.offset)
        .take(args.limit)
        .collect();

    if args.json {
        let env = Envelope {
            command: "list",
            registry: index.registry.clone(),
            package_count,
            returned: returned.len(),
            offset: args.offset,
            limit: args.limit,
            packages: returned,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&env)
                .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
        );
    } else {
        println!("registry  : {}", index.registry);
        println!(
            "packages  : {} ({} returned)",
            package_count,
            returned.len()
        );
        for row in returned {
            print!("  {}/{}", row.group, row.name);
            if let Some(latest) = &row.latest_stable {
                print!(" @ {latest}");
            }
            println!();
            if let Some(d) = &row.description {
                println!("    {d}");
            }
        }
    }
    Ok(())
}
