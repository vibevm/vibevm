//! `vibe-index outdated <data-dir>` — diff a local `vibe.lock`
//! against the index, report upgrade candidates.

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::lockfile;
use crate::types::PackageKind;

#[derive(Debug, Parser)]
#[command(about = "Compare a vibe.lock against the index and report outdated entries.")]
pub struct Args {
    pub data_dir: PathBuf,

    #[arg(long, value_name = "PATH", default_value = "vibe.lock")]
    pub lockfile: PathBuf,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct Envelope {
    command: &'static str,
    lockfile: PathBuf,
    update_available: u32,
    rows: Vec<Row>,
}

#[derive(Debug, Serialize)]
struct Row {
    kind: PackageKind,
    group: Group,
    name: String,
    installed: Version,
    latest: Option<Version>,
    status: Status,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum Status {
    UpToDate,
    UpdateAvailable,
    Unknown,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let lock = lockfile::read(&args.lockfile)?;

    let mut rows = Vec::with_capacity(lock.package.len());
    let mut update_available = 0u32;
    for pkg in &lock.package {
        let latest = index
            .get(&pkg.group, &pkg.name)
            .and_then(|p| p.latest_stable.clone());
        let status = match &latest {
            None => Status::Unknown,
            Some(l) if l > &pkg.version => {
                update_available += 1;
                Status::UpdateAvailable
            }
            Some(_) => Status::UpToDate,
        };
        rows.push(Row {
            kind: pkg.kind,
            group: pkg.group.clone(),
            name: pkg.name.clone(),
            installed: pkg.version.clone(),
            latest,
            status,
        });
    }

    if args.json {
        let env = Envelope {
            command: "outdated",
            lockfile: args.lockfile.clone(),
            update_available,
            rows,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&env)
                .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
        );
    } else {
        println!("lockfile          : {}", args.lockfile.display());
        println!("update available  : {}", update_available);
        for row in &rows {
            let arrow = match row.status {
                Status::UpdateAvailable => "→",
                Status::UpToDate => "=",
                Status::Unknown => "?",
            };
            let latest = row
                .latest
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".to_string());
            println!(
                "  {} {}/{} {} {} {}",
                arrow, row.group, row.name, row.installed, arrow, latest
            );
        }
    }
    Ok(())
}
