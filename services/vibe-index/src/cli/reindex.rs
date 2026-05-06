//! `vibe-index reindex <data-dir>` — (re)build the index from
//! authoritative package state. Slice 3 lands the `--from-clones`
//! source (walks a local org-dir of git clones). `--from-github`
//! lands in slice 8.

use std::path::PathBuf;

use chrono::Utc;
use clap::{ArgGroup, Parser};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::scanner::from_clones::{FromClonesOptions, ScanReport, scan_org_dir};
use crate::types::{NamingConvention, PackageKind, VersionEntry};

#[derive(Debug, Parser)]
#[command(
    about = "(Re)build the index from authoritative package state.",
    group = ArgGroup::new("source").required(true).args(["from_clones", "from_github", "from_gitverse"]),
    group = ArgGroup::new("scope").args(["full", "incremental"]),
)]
pub struct Args {
    pub data_dir: PathBuf,

    /// Walk a local directory of org clones (one subdirectory per
    /// package repo).
    #[arg(long, value_name = "ORG-DIR")]
    pub from_clones: Option<PathBuf>,

    /// Walk a GitHub org via the REST API. Lands in slice 8.
    #[arg(long, value_name = "ORG")]
    pub from_github: Option<String>,

    /// Walk a GitVerse org. Stub today (their public API does not yet
    /// expose org-scoped repo enumeration).
    #[arg(long, value_name = "ORG")]
    pub from_gitverse: Option<String>,

    /// File containing the host API token (one line, no trailing newline).
    #[arg(long, value_name = "FILE")]
    pub token_file: Option<PathBuf>,

    /// Force a full rebuild even if a checkpoint exists. Default in slice 3.
    #[arg(long)]
    pub full: bool,

    /// Apply only the diff against the previous checkpoint. Lands in slice 7.
    #[arg(long, conflicts_with = "full")]
    pub incremental: bool,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: Args) -> Result<()> {
    if args.incremental {
        return Err(Error::NotYetImplemented("reindex --incremental"));
    }
    if args.from_github.is_some() {
        return Err(Error::NotYetImplemented("reindex --from-github"));
    }
    if args.from_gitverse.is_some() {
        return Err(Error::NotYetImplemented("reindex --from-gitverse"));
    }
    let Some(org_dir) = args.from_clones.as_deref() else {
        return Err(Error::InvalidInput(
            "missing --from-clones / --from-github / --from-gitverse".into(),
        ));
    };

    // Load existing index manifest to preserve registry name / URL /
    // naming. Refuse if the data dir was never `init`-ed.
    let existing = Index::load_from(&args.data_dir).map_err(|e| match e {
        Error::Io { .. } | Error::Malformed(_) => Error::InvalidInput(format!(
            "data-dir `{}` does not look like an initialised index. \
             Run `vibe-index init` first.",
            args.data_dir.display()
        )),
        other => other,
    })?;

    let opts = FromClonesOptions {
        registry: existing.registry.clone(),
        registry_url: existing.registry_url.clone(),
        naming: existing.naming,
        generator: format!("vibe-index {}", env!("CARGO_PKG_VERSION")),
        indexed_at: Utc::now(),
    };

    let report = scan_org_dir(org_dir, &opts)?;

    let mut next = Index::new(&existing.registry, &existing.registry_url, existing.naming);
    next.generator = opts.generator.clone();
    for entry in &report.entries {
        next.upsert(entry.clone());
    }
    next.write_to(&args.data_dir)?;

    let summary = Summary::from_report(&report, &args.data_dir, &existing.registry, &next);
    if args.json {
        let envelope = serde_json::to_string_pretty(&summary).map_err(|e| {
            Error::Malformed(format!("could not serialise reindex summary: {e}"))
        })?;
        println!("{envelope}");
    } else {
        render_text(&summary);
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub command: &'static str,
    pub data_dir: PathBuf,
    pub registry: String,
    pub source: &'static str,
    pub package_count: u32,
    pub version_count: u32,
    pub skipped: Vec<SkippedSummary>,
    pub by_kind: Vec<KindCount>,
}

#[derive(Debug, Serialize)]
pub struct SkippedSummary {
    pub repo: String,
    pub tag: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct KindCount {
    pub kind: PackageKind,
    pub count: u32,
}

impl Summary {
    fn from_report(
        report: &ScanReport,
        data_dir: &std::path::Path,
        registry: &str,
        index: &Index,
    ) -> Self {
        let mut by_kind: Vec<KindCount> = PackageKind::all()
            .iter()
            .map(|k| KindCount {
                kind: *k,
                count: index
                    .by_pkgref
                    .keys()
                    .filter(|(kk, _)| kk == k)
                    .count() as u32,
            })
            .collect();
        by_kind.retain(|kc| kc.count > 0);

        Summary {
            command: "reindex",
            data_dir: data_dir.to_path_buf(),
            registry: registry.to_string(),
            source: "clones",
            package_count: index.package_count(),
            version_count: index.version_count(),
            skipped: report
                .skipped
                .iter()
                .map(|s| SkippedSummary {
                    repo: s.repo.clone(),
                    tag: s.tag.clone(),
                    reason: s.reason.clone(),
                })
                .collect(),
            by_kind,
        }
    }
}

fn render_text(summary: &Summary) {
    println!("registry  : {}", summary.registry);
    println!("source    : {}", summary.source);
    println!("packages  : {}", summary.package_count);
    println!("versions  : {}", summary.version_count);
    for kc in &summary.by_kind {
        println!("  {} : {}", kc.kind, kc.count);
    }
    if !summary.skipped.is_empty() {
        println!("skipped   : {}", summary.skipped.len());
        for s in &summary.skipped {
            match &s.tag {
                Some(t) => println!("  ⚠ {} @ {} — {}", s.repo, t, s.reason),
                None => println!("  ⚠ {} — {}", s.repo, s.reason),
            }
        }
    }
}

// VersionEntry imported for documentation purposes — referenced by the
// text-render block above is implicit; keep the use to silence unused
// warnings if reorganisation ever drops the explicit reference.
#[allow(dead_code)]
fn _silence_unused(v: &VersionEntry) {
    let _ = v;
}

// `NamingConvention` is referenced by Args via clap-derive on the
// existing flag; importing it explicitly here so the use line above
// reads naturally. Same housekeeping as `_silence_unused`.
#[allow(dead_code)]
fn _silence_naming(_n: NamingConvention) {}
