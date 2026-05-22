//! `vibe-index get <data-dir> <group> <name>` — read one package entry
//! from the index by its `(group, name)` identity (PROP-008 §2.2).

use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::types::{PackageEntry, VersionEntry};

#[derive(Debug, Parser)]
#[command(about = "Read one package entry from the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Reverse-FQDN group qualifier — e.g. `org.vibevm`.
    pub group: Group,

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
    group: &'a Group,
    name: &'a str,
    versions: Vec<&'a VersionEntry>,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let Some(pkg) = index.get(&args.group, &args.name) else {
        if args.json {
            let env = GetEnvelope {
                command: "get",
                found: false,
                group: &args.group,
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
            "package `{}/{}` is not in the index",
            args.group, args.name
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
                group: &args.group,
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
            "package `{}/{}` has no version `{}` in the index",
            args.group,
            args.name,
            args.version.unwrap()
        )));
    }

    if args.json {
        let env = GetEnvelope {
            command: "get",
            found: true,
            group: &args.group,
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
    println!("group         : {}", pkg.group);
    println!("name          : {}", pkg.name);
    if let Some(kind) = pkg.versions.first().map(|v| v.kind) {
        println!("kind          : {kind}");
    }
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
