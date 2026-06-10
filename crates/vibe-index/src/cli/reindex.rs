//! `vibe-index reindex <data-dir>` — (re)build the index from
//! authoritative package state. Slice 3 lands the `--from-clones`
//! source (walks a local org-dir of git clones). `--from-github`
//! lands in slice 8.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use chrono::Utc;
use clap::{ArgGroup, Parser};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::index::checkpoint::{self, Checkpoint};
use crate::scanner::from_clones::{
    FromClonesOptions, ScanReport, scan_org_dir, scan_org_dir_with_filter,
};
use crate::scanner::from_github::{FromGithubOptions, clone_org as github_clone_org};
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

    /// GitHub REST API base URL. Defaults to `https://api.github.com`.
    /// Override for tests or self-hosted GitHub Enterprise instances.
    #[arg(long, value_name = "URL", default_value = "https://api.github.com")]
    pub api_base: String,

    /// Where the `--from-github` scanner clones repos. Defaults to a
    /// fresh tempdir that is removed at the end of the run. Pass an
    /// explicit path to keep a warm cache (subsequent runs reuse it).
    #[arg(long, value_name = "DIR")]
    pub clone_cache: Option<PathBuf>,

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
    if let Some(org) = args.from_gitverse.as_deref() {
        return emit_gitverse_stub(org, &args);
    }

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

    // Resolve the org-dir for the scanner. --from-clones uses the
    // path verbatim; --from-github clones the org first into a temp
    // (or operator-supplied) directory and then proceeds as if we
    // had been pointed at it directly. Hold the TempDir alive until
    // the function returns so the directory survives the scan.
    let mut _temp_guard: Option<tempfile::TempDir> = None;
    let org_dir: PathBuf = if let Some(path) = args.from_clones.clone() {
        path
    } else if let Some(org) = args.from_github.clone() {
        let token = match args.token_file.as_deref() {
            Some(path) => Some(read_token(path)?),
            None => None,
        };
        let clone_into = if let Some(p) = args.clone_cache.clone() {
            p
        } else {
            let dir = tempfile::tempdir().map_err(|e| Error::Io {
                path: args.data_dir.clone(),
                message: format!("could not create scratch clone dir: {e}"),
            })?;
            let path = dir.path().to_path_buf();
            _temp_guard = Some(dir);
            path
        };
        let opts = FromGithubOptions {
            api_base: args.api_base.clone(),
            org: org.clone(),
            token,
            clone_into: clone_into.clone(),
            timeout: std::time::Duration::from_secs(60),
            skip_forks: true,
        };
        github_clone_org(&opts)?
    } else {
        return Err(Error::InvalidInput(
            "missing --from-clones / --from-github / --from-gitverse".into(),
        ));
    };

    let opts = FromClonesOptions {
        registry: existing.registry.clone(),
        registry_url: existing.registry_url.clone(),
        naming: existing.naming,
        generator: format!("vibe-index {}", env!("CARGO_PKG_VERSION")),
        indexed_at: Utc::now(),
    };

    let prior = if args.incremental {
        Some(checkpoint::load(&args.data_dir)?)
    } else {
        None
    };

    let report = if args.incremental {
        scan_org_dir_with_filter(&org_dir, &opts, prior.as_ref())?
    } else {
        scan_org_dir(&org_dir, &opts)?
    };

    // For incremental, retain entries for repos that the scanner
    // skipped due to "unchanged since last checkpoint". For full,
    // start fresh.
    let mut next = Index::new(&existing.registry, &existing.registry_url, existing.naming);
    next.generator = opts.generator.clone();

    if args.incremental {
        for entry in existing.iter_versions() {
            // Map entry → repo name via the registry's naming
            // convention; if that repo's snapshot was skipped (i.e.
            // not in the new scan's `entries`), keep the entry.
            let repo_name = existing
                .naming
                .repo_name(entry.kind, &entry.group, &entry.name);
            let scanned_now = report
                .snapshots
                .get(&repo_name)
                .map(|_| {
                    // Repo is present in the scan; if entries from this
                    // scan ALSO carry an entry for the same (group, name)
                    // identity, that's the freshly walked source.
                    // Otherwise the repo was skipped as unchanged — keep
                    // the existing entry.
                    report
                        .entries
                        .iter()
                        .any(|e| e.group == entry.group && e.name == entry.name)
                })
                .unwrap_or(false);
            let kept_unchanged = report.snapshots.contains_key(&repo_name) && !scanned_now;
            if kept_unchanged {
                next.upsert(entry.clone());
            }
        }
    }
    for entry in &report.entries {
        next.upsert(entry.clone());
    }
    next.write_to(&args.data_dir)?;

    // Persist the new checkpoint regardless of mode — incremental
    // walks pick it up next time, full walks reset it.
    let new_checkpoint = Checkpoint {
        schema_version: 1,
        generated_at: Some(opts.indexed_at),
        repos: report.snapshots.clone(),
    };
    checkpoint::save(&args.data_dir, &new_checkpoint)?;

    let source = if args.from_github.is_some() {
        "github"
    } else {
        "clones"
    };
    let summary = Summary::from_report(
        &report,
        &args.data_dir,
        &existing.registry,
        &next,
        source,
        if args.incremental {
            "incremental"
        } else {
            "full"
        },
    );
    if args.json {
        let envelope = serde_json::to_string_pretty(&summary)
            .map_err(|e| Error::Malformed(format!("could not serialise reindex summary: {e}")))?;
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
    pub mode: &'static str,
    pub package_count: u32,
    pub version_count: u32,
    pub skipped: Vec<SkippedSummary>,
    pub by_kind: Vec<KindCount>,
}

/// `--from-gitverse` is a deliberate stub today — GitVerse's public
/// REST API does not yet expose org-scoped repository enumeration
/// (the same upstream gap that keeps `vibe registry publish
/// --registry vibespecs-gitverse` stub-bound). Emit a structured
/// envelope so consumers can detect the stub without parsing
/// stderr; mirror shape per `vibe-publish` GitVerse stub.
fn emit_gitverse_stub(org: &str, args: &Args) -> Result<()> {
    let reason = format!(
        "`--from-gitverse {org}` is not implemented yet — the GitVerse public API does \
         not expose org-scoped repository enumeration (same upstream gap that keeps \
         `vibe registry publish --registry <gitverse>` stub-bound). Use `--from-clones \
         <org-dir>` against a local mirror of the GitVerse org, or `--from-github` if \
         the org has a GitHub mirror. This branch flips back to a real implementation \
         the moment the upstream API exposes the equivalent of \
         `GET /orgs/<org>/repos`."
    );
    let envelope = GitVerseStubReport {
        ok: false,
        command: "registry:reindex",
        host: "gitverse.ru",
        org: org.to_string(),
        data_dir: args.data_dir.clone(),
        stub: true,
        reason: reason.clone(),
    };
    if args.json {
        let s = serde_json::to_string_pretty(&envelope).map_err(|e| {
            Error::Malformed(format!("could not serialise gitverse stub envelope: {e}"))
        })?;
        println!("{s}");
    } else {
        println!("vibe-index reindex --from-gitverse {org}: {reason}");
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct GitVerseStubReport {
    ok: bool,
    command: &'static str,
    host: &'static str,
    org: String,
    data_dir: PathBuf,
    stub: bool,
    reason: String,
}

fn read_token(path: &std::path::Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    let s = std::str::from_utf8(&bytes)
        .map_err(|e| Error::Malformed(format!("token file is not UTF-8: {e}")))?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidInput(format!(
            "token file `{}` is empty",
            path.display()
        )));
    }
    Ok(trimmed.to_string())
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
        source: &'static str,
        mode: &'static str,
    ) -> Self {
        let mut by_kind: Vec<KindCount> = PackageKind::all()
            .iter()
            .map(|k| KindCount {
                kind: *k,
                // `kind` is per-version metadata (PROP-008 §2.3) — a
                // package's kind is the kind its versions carry.
                count: index
                    .by_pkgref
                    .values()
                    .filter(|p| p.versions.first().map(|v| v.kind) == Some(*k))
                    .count() as u32,
            })
            .collect();
        by_kind.retain(|kc| kc.count > 0);

        Summary {
            command: "reindex",
            data_dir: data_dir.to_path_buf(),
            registry: registry.to_string(),
            source,
            mode,
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
    println!("mode      : {}", summary.mode);
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
