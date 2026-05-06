//! `vibe search` — full-text query over every configured registry's
//! optional package index. Read-only; no lockfile or cache mutation.
//!
//! Spec: [ROADMAP §M2.10](../../../../ROADMAP.md) and
//! [PROP-005 §2.10](../../../../spec/modules/vibe-index/PROP-005-package-index.md#index-routes)
//! for the wire shape served by the index server.
//!
//! Resolution path: walk every `[[registry]]` (or just one with
//! `--registry NAME`), try `VIBEVM_INDEX_URL_<R>`-resolved index URL,
//! probe and search. Registries without a configured index URL or
//! whose probe fails are reported in the envelope but never abort the
//! run — search degrades gracefully when half the world is down.
//!
//! Hits from multiple registries are deduplicated by `(kind, name)`,
//! keeping the highest-score variant; ties break on registry priority
//! order (the order in `vibe.toml`). Identity invariants are not
//! affected — a search hit is metadata only; consumers still install
//! through `MultiRegistryResolver`, which re-verifies `content_hash`
//! at fetch time per [PROP-002 §2.1].

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::PackageKind;
use vibe_core::manifest::ProjectManifest;
use vibe_registry::{IndexClient, SearchHit, index_url_for};

use crate::cli::SearchArgs;
use crate::output;

#[derive(Debug, Serialize)]
struct SearchReport {
    ok: bool,
    command: &'static str,
    project: String,
    query: String,
    registries_searched: Vec<String>,
    registries_unconfigured: Vec<String>,
    registries_unreachable: Vec<UnreachableRegistry>,
    hit_count: usize,
    hits: Vec<HitRow>,
}

#[derive(Debug, Clone, Serialize)]
struct UnreachableRegistry {
    name: String,
    reason: String,
}

#[derive(Debug, Serialize, Clone)]
struct HitRow {
    kind: String,
    name: String,
    latest_stable: Option<String>,
    score: u32,
    matched_tokens: Vec<String>,
    description: Option<String>,
    registry: String,
}

pub fn run(ctx: &output::Context, args: SearchArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest = load_project_manifest(&project_root)?;

    if manifest.registries.is_empty() {
        bail!(
            "no registry configured. Add a `[[registry]]` entry to `vibe.toml` or run `vibe search` against a project that has one."
        );
    }

    let kind_filter: Option<PackageKind> = match args.kind.as_deref() {
        None => None,
        Some(s) => Some(
            s.parse::<PackageKind>()
                .with_context(|| format!("`--kind {s}` is not a recognised package kind"))?,
        ),
    };

    let query = args.query.join(" ");
    if query.trim().is_empty() {
        bail!("query is empty after trimming whitespace");
    }

    let target_registries: Vec<&vibe_core::manifest::RegistrySection> =
        match args.registry.as_deref() {
            None => manifest.registries.iter().collect(),
            Some(name) => match manifest.registry_by_name(name) {
                Some(r) => vec![r],
                None => bail!(
                    "no `[[registry]]` named `{name}` in `vibe.toml`. Configured: {}",
                    manifest
                        .registries
                        .iter()
                        .map(|r| r.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
        };

    // Walk registries and aggregate. Track per-registry status so the
    // report distinguishes "no index configured" (operator action
    // required) from "index URL set but server down" (transient).
    let mut searched: Vec<String> = Vec::new();
    let mut unconfigured: Vec<String> = Vec::new();
    let mut unreachable: Vec<UnreachableRegistry> = Vec::new();
    let mut by_pkg: HashMap<(PackageKind, String), HitRow> = HashMap::new();

    for reg in &target_registries {
        let Some(base) = index_url_for(&reg.name) else {
            unconfigured.push(reg.name.clone());
            continue;
        };
        let Some(client) = IndexClient::probe(&base) else {
            unreachable.push(UnreachableRegistry {
                name: reg.name.clone(),
                reason: format!("probe of `{base}/repomd.json` failed (server down or wrong URL)"),
            });
            continue;
        };
        match client.search(&query, kind_filter, Some(args.limit)) {
            Ok(results) => {
                searched.push(reg.name.clone());
                for hit in results.hits {
                    let key = (hit.kind, hit.name.clone());
                    let row = make_hit_row(&hit, &reg.name);
                    match by_pkg.get_mut(&key) {
                        Some(existing) => {
                            // Highest score wins; on tie, registry-priority
                            // order from vibe.toml wins (already ensured by
                            // walking `target_registries` in order — earlier
                            // entries lose only to later if a later entry
                            // has a strictly higher score).
                            if row.score > existing.score {
                                *existing = row;
                            }
                        }
                        None => {
                            by_pkg.insert(key, row);
                        }
                    }
                }
            }
            Err(e) => {
                unreachable.push(UnreachableRegistry {
                    name: reg.name.clone(),
                    reason: format!("{e}"),
                });
            }
        }
    }

    let mut hits: Vec<HitRow> = by_pkg.into_values().collect();
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.kind.cmp(&b.kind))
            .then(a.name.cmp(&b.name))
    });

    let report = SearchReport {
        ok: true,
        command: "search",
        project: project_root.display().to_string(),
        query: query.clone(),
        registries_searched: searched.clone(),
        registries_unconfigured: unconfigured.clone(),
        registries_unreachable: unreachable.clone(),
        hit_count: hits.len(),
        hits: hits.clone(),
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }

    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe search: {} hit{} across {} registry{}",
            hits.len(),
            if hits.len() == 1 { "" } else { "s" },
            searched.len(),
            if searched.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }

    println!("query     : {query}");
    println!(
        "registries: {} searched, {} unreachable, {} without index URL",
        searched.len(),
        unreachable.len(),
        unconfigured.len(),
    );
    if !searched.is_empty() {
        println!("  searched: {}", searched.join(", "));
    }
    for u in &unreachable {
        println!("  unreachable: {} — {}", u.name, u.reason);
    }
    if !unconfigured.is_empty() {
        println!(
            "  no VIBEVM_INDEX_URL_<R> set: {}",
            unconfigured.join(", "),
        );
    }
    println!();

    if hits.is_empty() {
        if searched.is_empty() {
            ctx.summary("(no registry has VIBEVM_INDEX_URL_<R> configured — see docs/commands/search.md)");
        } else {
            ctx.summary("(no matches)");
        }
        return Ok(());
    }

    println!(
        "KIND    NAME                          LATEST       SCORE  REGISTRY              DESCRIPTION"
    );
    for h in &hits {
        let latest = h.latest_stable.as_deref().unwrap_or("-");
        let desc = h.description.as_deref().unwrap_or("");
        println!(
            "{:<6}  {:<28}  {:<11}  {:<5}  {:<20}  {}",
            h.kind,
            truncate(&h.name, 28),
            latest,
            h.score,
            truncate(&h.registry, 20),
            truncate(desc, 60),
        );
    }
    println!();
    ctx.summary(&format!(
        "{} hit{} across {} registry{}",
        hits.len(),
        if hits.len() == 1 { "" } else { "s" },
        searched.len(),
        if searched.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}

fn make_hit_row(hit: &SearchHit, registry: &str) -> HitRow {
    HitRow {
        kind: hit.kind.to_string(),
        name: hit.name.clone(),
        latest_stable: hit.latest_stable.as_ref().map(|v| v.to_string()),
        score: hit.score,
        matched_tokens: hit.matched_tokens.clone(),
        description: hit.description.clone(),
        registry: registry.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn resolve_project_root(path: &Path) -> Result<std::path::PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(ProjectManifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_project_manifest(root: &Path) -> Result<ProjectManifest> {
    let path = root.join(ProjectManifest::FILENAME);
    Ok(ProjectManifest::read(&path)?)
}
