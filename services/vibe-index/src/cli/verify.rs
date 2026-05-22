//! `vibe-index verify <data-dir>` — recompute file hashes and check
//! `repomd.json` integrity.

use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::index::{by_name, persistence, repomd};
use crate::types::{Repomd, RepomdFileEntry};

#[derive(Debug, Parser)]
#[command(about = "Recompute file hashes and check repomd.json integrity.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: Args) -> Result<()> {
    let manifest = repomd::read(&args.data_dir)?;
    let report = check(&args.data_dir, &manifest)?;
    if args.json {
        let envelope = serde_json::to_string_pretty(&report)
            .map_err(|e| Error::Malformed(format!("could not serialise verify report: {e}")))?;
        println!("{envelope}");
    } else {
        render_text(&report);
    }
    if report.has_failures() {
        Err(Error::Malformed(format!(
            "{} file(s) failed integrity check",
            report.mismatches.len() + report.missing.len()
        )))
    } else {
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub command: &'static str,
    pub data_dir: PathBuf,
    pub registry: String,
    pub package_count: u32,
    pub version_count: u32,
    pub files_checked: u32,
    pub mismatches: Vec<Mismatch>,
    pub missing: Vec<String>,
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct Mismatch {
    pub path: String,
    pub expected_size: u64,
    pub actual_size: u64,
    pub expected_sha256: String,
    pub actual_sha256: String,
}

impl Report {
    pub fn has_failures(&self) -> bool {
        !self.mismatches.is_empty() || !self.missing.is_empty()
    }
}

fn check(data_dir: &std::path::Path, manifest: &Repomd) -> Result<Report> {
    let mut mismatches = Vec::new();
    let mut missing = Vec::new();
    let mut files_checked = 0;

    for (rel_path, entry) in &manifest.files {
        match entry {
            RepomdFileEntry::File { size, sha256 } => {
                let path = data_dir.join(rel_path);
                let bytes = match std::fs::read(&path) {
                    Ok(b) => b,
                    Err(_) => {
                        missing.push(rel_path.clone());
                        continue;
                    }
                };
                let actual_sha = persistence::sha256_of_bytes(&bytes);
                let actual_size = bytes.len() as u64;
                if actual_size != *size || &actual_sha != sha256 {
                    mismatches.push(Mismatch {
                        path: rel_path.clone(),
                        expected_size: *size,
                        actual_size,
                        expected_sha256: sha256.clone(),
                        actual_sha256: actual_sha,
                    });
                }
                files_checked += 1;
            }
            RepomdFileEntry::Directory { entries, .. } => {
                if rel_path == by_name::DIRNAME {
                    let observed = by_name::entry_count(data_dir);
                    if observed != *entries {
                        mismatches.push(Mismatch {
                            path: rel_path.clone(),
                            expected_size: u64::from(*entries),
                            actual_size: u64::from(observed),
                            expected_sha256: "directory".into(),
                            actual_sha256: "directory".into(),
                        });
                    }
                    files_checked += 1;
                }
            }
        }
    }

    Ok(Report {
        command: "verify",
        data_dir: data_dir.to_path_buf(),
        registry: manifest.registry.clone(),
        package_count: manifest.package_count,
        version_count: manifest.version_count,
        files_checked,
        ok: mismatches.is_empty() && missing.is_empty(),
        mismatches,
        missing,
    })
}

fn render_text(report: &Report) {
    println!("registry  : {}", report.registry);
    println!("packages  : {}", report.package_count);
    println!("versions  : {}", report.version_count);
    println!(
        "files     : {} checked, {} mismatch, {} missing",
        report.files_checked,
        report.mismatches.len(),
        report.missing.len()
    );
    for m in &report.mismatches {
        println!(
            "  ✗ {} — size {} vs {}, sha256 {} vs {}",
            m.path, m.expected_size, m.actual_size, m.expected_sha256, m.actual_sha256
        );
    }
    for m in &report.missing {
        println!("  ✗ {m} — file missing");
    }
    if report.ok {
        println!("status    : OK");
    } else {
        println!("status    : FAILED");
    }
}
