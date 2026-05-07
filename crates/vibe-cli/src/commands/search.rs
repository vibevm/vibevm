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
use semver::Version;
use vibe_core::PackageKind;
use vibe_core::manifest::ProjectManifest;
use vibe_registry::{
    BindingSite, IndexClient, PurlLookupHit, SearchHit, index_url_for,
};

use crate::cli::SearchArgs;
use crate::commands::search_cache::{self, CacheKey};
use crate::commands::search_full_scan::{self, FullScanHit};
use crate::output;

/// Override for the GitHub REST API root used by `--full-scan`. Set
/// in tests to a local mock; defaults to the live API when unset.
const GITHUB_API_BASE_ENV: &str = "VIBEVM_GITHUB_API_BASE";

#[derive(Debug, Serialize)]
struct SearchReport {
    ok: bool,
    command: &'static str,
    project: String,
    query: String,
    registries_searched: Vec<String>,
    registries_unconfigured: Vec<String>,
    registries_unreachable: Vec<UnreachableRegistry>,
    /// Registries scanned via `--full-scan` (org-walk fallback).
    /// Always empty when `--full-scan` is off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    registries_full_scanned: Vec<String>,
    /// Registries `--full-scan` could not handle (non-GitHub host).
    /// Always empty when `--full-scan` is off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    registries_full_scan_unsupported: Vec<UnreachableRegistry>,
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
    /// `"index"` (served from the registry's `vibe-index` server) or
    /// `"full-scan"` (built by walking the org's repos directly).
    /// Lets a CI consumer tell whether the search ran against a
    /// curated index or the rate-limited fallback.
    source: &'static str,
}

pub fn run(ctx: &output::Context, args: SearchArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest = load_project_manifest(&project_root)?;

    if manifest.registries.is_empty() {
        bail!(
            "no registry configured. Add a `[[registry]]` entry to `vibe.toml` or run `vibe search` against a project that has one."
        );
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

    if let Some(purl) = args.purl.as_deref() {
        let purl_norm = purl.trim();
        if !purl_norm.starts_with("pkg:") {
            bail!(
                "`--purl` value `{purl_norm}` does not start with `pkg:` — see https://github.com/package-url/purl-spec for the canonical PURL syntax"
            );
        }
        return run_purl_lookup(ctx, &project_root, &target_registries, purl_norm);
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

    let full_scan = args.full_scan;
    let cache_ttl = args.cache_ttl.unwrap_or(search_cache::DEFAULT_TTL_SECS);
    let cache_root = if args.no_cache {
        None
    } else {
        search_cache::cache_root()
    };
    let kind_str: Option<String> = kind_filter.map(|k| k.as_str().to_string());

    // Walk registries and aggregate. Track per-registry status so the
    // report distinguishes "no index configured" (operator action
    // required) from "index URL set but server down" (transient) and,
    // when --full-scan is on, "scanned via org-walk" from "host not
    // supported by full-scan".
    let mut searched: Vec<String> = Vec::new();
    let mut unconfigured: Vec<String> = Vec::new();
    let mut unreachable: Vec<UnreachableRegistry> = Vec::new();
    let mut full_scanned: Vec<String> = Vec::new();
    let mut full_scan_unsupported: Vec<UnreachableRegistry> = Vec::new();
    let mut by_pkg: HashMap<(PackageKind, String), HitRow> = HashMap::new();

    for reg in &target_registries {
        match index_url_for(&reg.name) {
            Some(base) => {
                let cache_key = CacheKey {
                    registry: &reg.name,
                    query: &query,
                    kind: kind_str.as_deref(),
                    limit: args.limit,
                };
                if let Some(root) = cache_root.as_ref()
                    && let Ok(Some(cached)) =
                        search_cache::load_if_fresh(root, &cache_key, cache_ttl)
                {
                    searched.push(reg.name.clone());
                    for hit in cached.hits {
                        insert_hit_keep_highest(&mut by_pkg, make_hit_row(&hit, &reg.name));
                    }
                    continue;
                }
                let Some(client) = IndexClient::probe(&base) else {
                    unreachable.push(UnreachableRegistry {
                        name: reg.name.clone(),
                        reason: format!(
                            "probe of `{base}/repomd.json` failed (server down or wrong URL)"
                        ),
                    });
                    continue;
                };
                match client.search(&query, kind_filter, Some(args.limit)) {
                    Ok(results) => {
                        searched.push(reg.name.clone());
                        if let Some(root) = cache_root.as_ref()
                            && let Err(e) = search_cache::save(root, &cache_key, &results)
                        {
                            tracing::debug!(
                                target: "vibe_cli::search_cache",
                                "could not write cache entry for {}: {e}",
                                reg.name
                            );
                        }
                        for hit in results.hits {
                            insert_hit_keep_highest(
                                &mut by_pkg,
                                make_hit_row(&hit, &reg.name),
                            );
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
            None if full_scan => {
                match run_full_scan_for_registry(&query, kind_filter, reg) {
                    Ok(hits) => {
                        full_scanned.push(reg.name.clone());
                        for h in hits {
                            insert_hit_keep_highest(
                                &mut by_pkg,
                                make_full_scan_hit_row(&h, &reg.name),
                            );
                        }
                    }
                    Err(reason) => {
                        full_scan_unsupported.push(UnreachableRegistry {
                            name: reg.name.clone(),
                            reason,
                        });
                    }
                }
            }
            None => {
                unconfigured.push(reg.name.clone());
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
        registries_full_scanned: full_scanned.clone(),
        registries_full_scan_unsupported: full_scan_unsupported.clone(),
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
        "registries: {} searched, {} unreachable, {} without index URL{}",
        searched.len(),
        unreachable.len(),
        unconfigured.len(),
        if full_scan {
            format!(
                ", {} full-scanned, {} full-scan unsupported",
                full_scanned.len(),
                full_scan_unsupported.len()
            )
        } else {
            String::new()
        },
    );
    if !searched.is_empty() {
        println!("  searched: {}", searched.join(", "));
    }
    if !full_scanned.is_empty() {
        println!("  full-scanned: {}", full_scanned.join(", "));
    }
    for u in &unreachable {
        println!("  unreachable: {} — {}", u.name, u.reason);
    }
    for u in &full_scan_unsupported {
        println!("  full-scan unsupported: {} — {}", u.name, u.reason);
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
            ctx.summary(
                "(no registry has VIBEVM_INDEX_URL_<R> configured; search returns empty.\n\
                 To install a known package, run `vibe install <kind>:<name>` directly —\n\
                 install resolves through `[[registry]]` over git and does not need an\n\
                 index. The index is a discovery optimisation, not a runtime dependency.\n\
                 See docs/commands/search.md for setting up an index server.)",
            );
        } else {
            ctx.summary("(no matches)");
        }
        return Ok(());
    }

    println!(
        "KIND    NAME                          LATEST       SCORE  SOURCE      REGISTRY              DESCRIPTION"
    );
    for h in &hits {
        let latest = h.latest_stable.as_deref().unwrap_or("-");
        let desc = h.description.as_deref().unwrap_or("");
        println!(
            "{:<6}  {:<28}  {:<11}  {:<5}  {:<10}  {:<20}  {}",
            h.kind,
            truncate(&h.name, 28),
            latest,
            h.score,
            h.source,
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
        source: "index",
    }
}

fn make_full_scan_hit_row(hit: &FullScanHit, registry: &str) -> HitRow {
    HitRow {
        kind: hit.kind.to_string(),
        name: hit.name.clone(),
        // Full-scan reads HEAD, not the latest tag — so what we
        // report is the manifest's declared version. Operators see
        // it under `latest_stable` for shape parity with the index
        // path; the underlying semver is still correct.
        latest_stable: Some(hit.version.to_string()),
        score: hit.score,
        matched_tokens: hit.matched_tokens.clone(),
        description: hit.description.clone(),
        registry: registry.to_string(),
        source: "full-scan",
    }
}

fn insert_hit_keep_highest(
    by_pkg: &mut HashMap<(PackageKind, String), HitRow>,
    row: HitRow,
) {
    let kind: PackageKind = row.kind.parse().unwrap_or(PackageKind::Flow);
    let key = (kind, row.name.clone());
    match by_pkg.get_mut(&key) {
        Some(existing) => {
            if row.score > existing.score {
                *existing = row;
            }
        }
        None => {
            by_pkg.insert(key, row);
        }
    }
}

fn run_full_scan_for_registry(
    query: &str,
    kind_filter: Option<PackageKind>,
    reg: &vibe_core::manifest::RegistrySection,
) -> std::result::Result<Vec<FullScanHit>, String> {
    let Some(org) = search_full_scan::detect_github_org(&reg.url) else {
        return Err(format!(
            "host of `{}` is not `github.com` (only GitHub orgs supported by --full-scan v0)",
            reg.url
        ));
    };
    let api_base = std::env::var(GITHUB_API_BASE_ENV)
        .unwrap_or_else(|_| "https://api.github.com".to_string());
    let token = vibe_publish::token::load_token_for_host("github.com")
        .ok()
        .map(|t| t.value().to_string());
    let token_ref = token.as_deref();
    let query_tokens = search_full_scan::tokenise_query(query);
    if query_tokens.is_empty() {
        return Err("query has no searchable tokens after stopword filtering".into());
    }
    search_full_scan::full_scan_github_org(&org, &api_base, token_ref, &query_tokens, kind_filter)
        .map_err(|e| format!("{e}"))
}

#[derive(Debug, Serialize)]
struct PurlReport {
    ok: bool,
    command: &'static str,
    project: String,
    purl: String,
    registries_searched: Vec<String>,
    registries_unconfigured: Vec<String>,
    registries_unreachable: Vec<UnreachableRegistry>,
    hit_count: usize,
    hits: Vec<PurlHitRow>,
}

#[derive(Debug, Clone, Serialize)]
struct PurlHitRow {
    kind: String,
    name: String,
    version: String,
    binding_site: BindingSite,
    registry: String,
}

fn run_purl_lookup(
    ctx: &output::Context,
    project_root: &Path,
    target_registries: &[&vibe_core::manifest::RegistrySection],
    purl: &str,
) -> Result<()> {
    let mut searched: Vec<String> = Vec::new();
    let mut unconfigured: Vec<String> = Vec::new();
    let mut unreachable: Vec<UnreachableRegistry> = Vec::new();
    // Dedup key: (kind, name, version). Server returns at most one
    // hit per `(kind, name, version)` for a given PURL — package vs
    // subskill match never coexist for the same entry, so the key
    // is monomorphic. Earlier registries (vibe.toml priority order)
    // win on cross-registry duplicates.
    let mut by_kvn: HashMap<(PackageKind, String, Version), PurlHitRow> = HashMap::new();

    for reg in target_registries {
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
        match client.lookup_purl(purl) {
            Ok(results) => {
                searched.push(reg.name.clone());
                for hit in results.hits {
                    let key = (hit.kind, hit.name.clone(), hit.version.clone());
                    by_kvn.entry(key).or_insert_with(|| make_purl_row(&hit, &reg.name));
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

    let mut hits: Vec<PurlHitRow> = by_kvn.into_values().collect();
    hits.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.name.cmp(&b.name))
            .then(a.version.cmp(&b.version))
    });

    let report = PurlReport {
        ok: true,
        command: "search:purl",
        project: project_root.display().to_string(),
        purl: purl.to_string(),
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
            "vibe search --purl: {} hit{} across {} registry{}",
            hits.len(),
            if hits.len() == 1 { "" } else { "s" },
            searched.len(),
            if searched.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }

    println!("purl      : {purl}");
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
        println!("  no VIBEVM_INDEX_URL_<R> set: {}", unconfigured.join(", "));
    }
    println!();

    if hits.is_empty() {
        if searched.is_empty() {
            ctx.summary(
                "(no registry has VIBEVM_INDEX_URL_<R> configured; search returns empty.\n\
                 To install a known package, run `vibe install <kind>:<name>` directly —\n\
                 install resolves through `[[registry]]` over git and does not need an\n\
                 index. The index is a discovery optimisation, not a runtime dependency.\n\
                 See docs/commands/search.md for setting up an index server.)",
            );
        } else {
            ctx.summary("(no matches)");
        }
        return Ok(());
    }

    println!(
        "KIND    NAME                          VERSION       BINDING-SITE  REGISTRY"
    );
    for h in &hits {
        println!(
            "{:<6}  {:<28}  {:<12}  {:<12}  {}",
            h.kind,
            truncate(&h.name, 28),
            truncate(&h.version, 12),
            h.binding_site,
            truncate(&h.registry, 20),
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

fn make_purl_row(hit: &PurlLookupHit, registry: &str) -> PurlHitRow {
    PurlHitRow {
        kind: hit.kind.to_string(),
        name: hit.name.clone(),
        version: hit.version.to_string(),
        binding_site: hit.binding_site,
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
