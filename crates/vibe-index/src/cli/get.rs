//! `vibe-index get <data-dir> <kind> <name>` — read one package
//! entry from the index.

use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;

use crate::cli::kinds::PackageKind;
use crate::error::{Error, Result};
use crate::index::Index;
use crate::types::{PackageEntry, VersionEntry};

#[derive(Debug, Parser)]
#[command(about = "Read one package entry from the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    #[arg(value_enum)]
    pub kind: PackageKind,

    pub name: String,

    /// Specific version. If omitted, prints every version.
    #[arg(long, value_name = "SEMVER")]
    pub version: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct GetEnvelope<'a> {
    command: &'static str,
    found: bool,
    kind: PackageKind,
    name: &'a str,
    versions: Vec<&'a VersionEntry>,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let Some(pkg) = index.get(args.kind, &args.name) else {
        if args.json {
            let env = GetEnvelope {
                command: "get",
                found: false,
                kind: args.kind,
                name: &args.name,
                versions: vec![],
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&env)
                    .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
            );
            return Ok(());
        }
        return Err(Error::InvalidInput(format!(
            "package `{}:{}` is not in the index",
            args.kind, args.name
        )));
    };

    let versions: Vec<&VersionEntry> = match &args.version {
        Some(v) => {
            let req: semver::Version = v.parse().map_err(|e| {
                Error::InvalidInput(format!("`--version {v}` is not valid semver: {e}"))
            })?;
            pkg.versions.iter().filter(|ve| ve.version == req).collect()
        }
        None => pkg.versions.iter().collect(),
    };
    if versions.is_empty() {
        if args.json {
            let env = GetEnvelope {
                command: "get",
                found: false,
                kind: args.kind,
                name: &args.name,
                versions,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&env)
                    .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
            );
            return Ok(());
        }
        return Err(Error::InvalidInput(format!(
            "package `{}:{}` has no version `{}` in the index",
            args.kind,
            args.name,
            args.version.unwrap()
        )));
    }

    if args.json {
        let env = GetEnvelope {
            command: "get",
            found: true,
            kind: args.kind,
            name: &args.name,
            versions,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&env)
                .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
        );
    } else {
        render_text(pkg, &versions);
    }
    Ok(())
}

fn render_text(pkg: &PackageEntry, versions: &[&VersionEntry]) {
    println!("kind          : {}", pkg.kind);
    println!("name          : {}", pkg.name);
    if let Some(latest) = &pkg.latest_stable {
        println!("latest stable : {latest}");
    }
    println!("versions      : {}", versions.len());
    for v in versions {
        println!(
            "  - {} (commit {})",
            v.version,
            v.resolved_commit.as_deref().unwrap_or("-")
        );
        if let Some(d) = &v.description {
            println!("    {d}");
        }
        println!("    content_hash: {}", v.content_hash);
        println!("    source_url  : {}", v.source_url);
    }
}
