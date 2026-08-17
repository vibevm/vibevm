//! `vibe-index list <data-dir>` — list packages.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;
use vibe_core::Group;

use crate::cli::kinds;
use crate::error::{Error, Result};
use crate::index::Index;
use crate::index::quarantine::{
    Unavailable, unavailable_for, usable_latest_stable, usable_versions,
};
use crate::types::PackageKind;

#[derive(Debug, Parser)]
#[command(about = "List packages in the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Keep only packages of this kind: flow, feat, stack, tool, mcp,
    /// lang. The wire vocabulary is open, but the ARGUMENT speaks: a
    /// kind this build does not know is refused with a message, not
    /// filtered away in silence.
    #[arg(long, value_name = "KIND")]
    pub kind: Option<String>,

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
    /// Versions of this package this build refuses to act on — named,
    /// not hidden (PROP-044 §4.5). Absent when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    unavailable: Vec<Unavailable>,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let kind_filter = match &args.kind {
        Some(raw) => Some(kinds::parse_kind_flag(raw)?),
        None => None,
    };
    let mut rows: Vec<PackageRow> = index
        .by_pkgref
        .values()
        .filter(|p| {
            kind_filter
                .as_ref()
                .is_none_or(|k| usable_versions(p).any(|v| v.kind == *k))
        })
        .map(|p| {
            let description = usable_versions(p)
                .next_back()
                .and_then(|v| v.description.clone());
            PackageRow {
                kind: usable_versions(p).next().map(|v| v.kind.clone()),
                group: p.group.clone(),
                name: p.name.clone(),
                versions: usable_versions(p).map(|v| v.version.clone()).collect(),
                latest_stable: usable_latest_stable(p).cloned(),
                description,
                unavailable: unavailable_for(p),
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
            if !row.unavailable.is_empty() {
                let listed = row
                    .unavailable
                    .iter()
                    .map(|u| format!("{} (missing: {})", u.version, u.missing.join(",")))
                    .collect::<Vec<_>>()
                    .join("; ");
                println!("    unavailable : {listed}");
            }
        }
    }
    Ok(())
}
