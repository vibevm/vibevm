//! `vibe-index reindex <data-dir>` — (re)build the index from
//! authoritative package state. Slice 3 lands the `--from-clones`
//! source (walks a local org-dir of git clones). `--from-github`
//! lands in slice 8.
//!
//! Ф3.2 journal form: the published catalog is never this writer's
//! input (PROP-044 §4.4). The run is `fold → scan → append → project
//! → write_to` — the registry identity comes from the journal's
//! `Initialised` fact, the scan's entries land as `Published`
//! records, and `--full` differs from `--incremental` by exactly one
//! event: the `EntrySetReplaced` watershed asserting the scan's
//! result is the whole published entry set.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clap::{ArgGroup, Parser};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::index::checkpoint::{self, Checkpoint};
use crate::index::memory::{WriteCtx, default_generator};
use crate::journal::{Event, JournalRecord, append, default_dir, project, replay};
use crate::scanner::{
    FromClonesOptions, FromClonesPackageScanner, FromGithubOptions, FromGithubPackageScanner,
    PackageScanner, ScanReport,
};
use crate::types::PackageKind;

#[derive(Debug, Parser)]
#[command(
    about = "(Re)build the index from authoritative package state.",
    group = ArgGroup::new("source").required(true).args(["from_clones", "from_github", "from_gitverse"]),
    group = ArgGroup::new("scope").args(["full", "incremental"]),
    group = ArgGroup::new("cache_mode").args(["cache_org", "no_cache_org"]),
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

    /// Consult the org-image cache for `--from-github` (default ON,
    /// Р1). On a second consecutive run with no org change, the host
    /// is asked once with the stored validator and answers `304 Not
    /// Modified` — the repo list is taken from the cache and the org
    /// is NOT re-enumerated (and, on GitHub, the probe costs no
    /// rate-limit token). The cache lives at
    /// `<data-dir>/state/org-cache.json`. This is the named,
    /// help-documented affirmation of the default; turn it OFF with
    /// `--no-cache-org`. Mutually exclusive with `--no-cache-org`.
    /// Only the `--from-github` path uses the cache (Р6).
    #[arg(long)]
    pub cache_org: bool,

    /// Disable the org-image cache for this run: enumerate the org
    /// unconditionally and read / write no image — behaviour
    /// identical to before the cache existed. Use when the
    /// conditional probe might lie (a host that mangles validators)
    /// or to force a plain walk for comparison. Mutually exclusive
    /// with `--cache-org`.
    #[arg(long)]
    pub no_cache_org: bool,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: Args) -> Result<()> {
    if let Some(org) = args.from_gitverse.as_deref() {
        return emit_gitverse_stub(org, &args);
    }

    // Р1 — `--cache-org` is the default; `--no-cache-org` opts out.
    // The two are mutually exclusive (ArgGroup `cache_mode`); the
    // positive form is the named, help-documented affirmation, the
    // negation the actual switch.
    let cache_org = args.cache_org || !args.no_cache_org;

    // Select the scanner cell — the one construction site (R-001): the
    // source flag decides the variant here and nowhere else.
    // `--from-gitverse` stays the error-returning stub above.
    let (scanner, source, temp_guard) = if let Some(path) = args.from_clones.clone() {
        // Р6 — the cache is a from-github concern; from-clones never
        // sets an org-cache path.
        (
            Box::new(FromClonesPackageScanner { org_dir: path }) as Box<dyn PackageScanner>,
            "clones",
            None,
        )
    } else if let Some(org) = args.from_github.clone() {
        let cache_path = if cache_org {
            Some(crate::scanner::org_cache::path(&args.data_dir))
        } else {
            None
        };
        let (scanner, guard) = build_github_scanner(
            args.token_file.as_deref(),
            &args.api_base,
            &org,
            args.clone_cache.clone(),
            cache_path,
            cache_org,
        )?;
        (scanner, "github", guard)
    } else {
        return Err(Error::InvalidInput(
            "missing --from-clones / --from-github / --from-gitverse".into(),
        ));
    };

    let mode = if args.incremental {
        "incremental"
    } else {
        "full"
    };

    run_plan(Plan {
        data_dir: args.data_dir.clone(),
        scanner,
        source,
        mode,
        json: args.json,
        _temp_guard: temp_guard,
    })
}

/// Construct the from-github scanner cell with a given cache policy.
/// Shared by `reindex` and `rescan-org` so the token read, scratch
/// clone dir, and option assembly live in one place. Returns the
/// scanner plus the tempdir guard (when a scratch clone dir was
/// created) the caller must hold alive for the scan's duration.
pub(crate) fn build_github_scanner(
    token_file: Option<&std::path::Path>,
    api_base: &str,
    org: &str,
    clone_cache: Option<PathBuf>,
    org_cache_path: Option<PathBuf>,
    probe_freshness: bool,
) -> Result<(Box<dyn PackageScanner>, Option<tempfile::TempDir>)> {
    let token = match token_file {
        Some(path) => Some(read_token(path)?),
        None => None,
    };
    let (clone_into, guard) = match clone_cache {
        Some(p) => (p, None),
        None => {
            let dir = tempfile::tempdir().map_err(|e| Error::Io {
                path: PathBuf::new(),
                message: format!("could not create scratch clone dir: {e}"),
            })?;
            (dir.path().to_path_buf(), Some(dir))
        }
    };
    let scanner = Box::new(FromGithubPackageScanner {
        opts: FromGithubOptions {
            api_base: api_base.to_string(),
            org: org.to_string(),
            token,
            clone_into,
            timeout: std::time::Duration::from_secs(60),
            skip_forks: true,
            org_cache_path,
            probe_freshness,
        },
    });
    Ok((scanner, guard))
}

/// A resolved reindex job — scanner already constructed, cache policy
/// already baked into the from-github options. Shared by `reindex`
/// (which parses CLI flags into a plan) and `rescan-org` (which builds
/// the plan directly with `probe_freshness = false`).
pub(crate) struct Plan {
    pub data_dir: PathBuf,
    pub scanner: Box<dyn PackageScanner>,
    pub source: &'static str,
    pub mode: &'static str,
    pub json: bool,
    /// Scratch clone dir; held alive until the scan returns.
    pub _temp_guard: Option<tempfile::TempDir>,
}

/// The shared reindex core: fold the journal, scan, append the run's
/// facts, re-project, persist the catalog + checkpoint, emit the
/// summary. Both `reindex` and `rescan-org` reduce to this.
pub(crate) fn run_plan(plan: Plan) -> Result<()> {
    // F2-1 — the clock enters here, once per command: the same `at`
    // stamps the scanner's `indexed_at`, every journal record this
    // run appends, the checkpoint, and the written manifest.
    let at = Utc::now();

    // Ф3.2c3 — the catalog is never this writer's input (PROP-044
    // §4.4): the registry identity the scanner is stamped with comes
    // from folding the journal, not from reading back a published
    // catalog. A data-dir that was never `init`-ed folds into
    // `Error::Unprojectable`, whose recipe names that fix — the same
    // guidance the failed catalog read used to give.
    let journal_dir = default_dir(&plan.data_dir);
    let mut records = replay(&journal_dir)?;
    let existing = project(records.iter().cloned())?;

    let opts = FromClonesOptions {
        registry: existing.registry.clone(),
        registry_url: existing.registry_url.clone(),
        naming: existing.naming,
        generator: default_generator(),
        indexed_at: at,
    };

    let prior = if plan.mode == "incremental" {
        Some(checkpoint::load(&plan.data_dir)?)
    } else {
        None
    };

    let mut report = plan.scanner.scan(&opts, prior.as_ref())?;

    // The run's facts. `--full` differs from `--incremental` by
    // exactly one event: the watershed asserting the scan's result is
    // the whole published entry set. Incremental publishes only what
    // it re-walked — entries the scanner skipped as unchanged stay in
    // the journal as their original `Published` records, and the
    // projection holds them with no merge step here.
    let mut appended: Vec<JournalRecord> = Vec::with_capacity(report.entries.len() + 1);
    if plan.mode == "full" {
        appended.push(fact(
            at,
            Event::EntrySetReplaced {
                source: plan.source.to_string(),
            },
        ));
    }
    for entry in std::mem::take(&mut report.entries) {
        appended.push(fact(
            at,
            Event::Published {
                entry: Box::new(entry),
            },
        ));
    }
    // Truth first (PROP-044 `##LAW-NO-UNRECOVERABLE`), the `init`
    // order: every fact is durable in the journal before the derived
    // catalog is written, so a failed `write_to` leaves a journal the
    // next run re-projects — never a catalog whose truth never
    // existed.
    for record in &appended {
        append(&journal_dir, record)?;
    }
    records.extend(appended);
    let next = project(records)?;
    next.write_to(&plan.data_dir, &WriteCtx { at })?;

    // Persist the new checkpoint regardless of mode — incremental
    // walks pick it up next time, full walks reset it. The checkpoint
    // is the SCANNER's memory, not a catalog fact: it lives under the
    // gitignored `state/` and never rides the journal.
    let new_checkpoint = Checkpoint {
        schema_version: 1,
        generated_at: Some(opts.indexed_at),
        repos: report.snapshots.clone(),
    };
    checkpoint::save(&plan.data_dir, &new_checkpoint)?;

    let summary = Summary::from_report(
        &report,
        &plan.data_dir,
        &existing.registry,
        &next,
        plan.source,
        plan.mode,
    );
    if plan.json {
        let envelope = serde_json::to_string_pretty(&summary)
            .map_err(|e| Error::Malformed(format!("could not serialise reindex summary: {e}")))?;
        println!("{envelope}");
    } else {
        render_text(&summary);
    }
    Ok(())
}

/// One journal fact of this run: stamped by the command's single
/// clock reading (F2-1) and authored by this binary's generator
/// label, so the journal and the catalog it projects never disagree
/// about who wrote them.
fn fact(at: DateTime<Utc>, event: Event) -> JournalRecord {
    JournalRecord {
        at,
        actor: default_generator(),
        event,
    }
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub command: &'static str,
    pub data_dir: PathBuf,
    pub registry: String,
    pub source: &'static str,
    pub mode: &'static str,
    /// Org-image cache outcome, surfaced so an operator can tell a
    /// cache hit from a re-enumeration (Р5). `Some("hit")` = served
    /// from a fresh cache; `Some("miss")` = re-enumerated; `None` =
    /// caching not in use (`--from-clones`, or `--no-cache-org`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_cache: Option<&'static str>,
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

        let org_cache = match report.org_cache_hit {
            Some(true) => Some("hit"),
            Some(false) => Some("miss"),
            None => None,
        };

        Summary {
            command: "reindex",
            data_dir: data_dir.to_path_buf(),
            registry: registry.to_string(),
            source,
            mode,
            org_cache,
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
    // Р5 — a hit and a miss must be distinguishable in the output.
    if let Some(c) = summary.org_cache {
        println!("cache     : {c}");
    }
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
