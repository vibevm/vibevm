//! `vibe search` — full-text query over every configured registry's
//! optional package index. Read-only; no lockfile or cache mutation.
//!
//! Spec: [ROADMAP §M2.10](../../../../ROADMAP.md) and
//! [PROP-005 §2.10](../../../../vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#index-routes)
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

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_core::PackageKind;
use vibe_core::manifest::Manifest;
use vibe_registry::search::cache::{self as search_cache, CacheKey};
use vibe_registry::search::full_scan::{self as search_full_scan, FullScanHit};
use vibe_registry::search::query;
use vibe_registry::{
    IndexAuth, IndexClient, IndexUrlResolution, IndexUrlSource, ProbeOutcome, SearchHit,
    resolve_index_url,
};

use crate::cli::SearchArgs;
use crate::output;

mod layout;
mod purl;
mod report;
use report::*;

/// Override for the GitHub REST API root used by `--full-scan`. Set
/// in tests to a local mock; defaults to the live API when unset. The
/// env read happens at the composition root (`main.rs`); this const
/// names the variable so main and the tests agree on it.
pub const GITHUB_API_BASE_ENV: &str = "VIBEVM_GITHUB_API_BASE";

/// The live GitHub REST API root, used when no override is supplied.
const GITHUB_API_DEFAULT: &str = "https://api.github.com";

/// Environment-derived configuration for `vibe search`, resolved once
/// at the composition root and threaded in so the command's domain
/// never reads the ambient environment itself (CONVERT-PLAN v0.1 §1
/// item 0.4; the Phase-5 `ambient-env` rule). Each field carries the
/// raw override or `None`; the defaults live here in the domain.
#[derive(Debug, Default)]
pub struct SearchEnv {
    /// `VIBEVM_GITHUB_API_BASE` — GitHub REST root for `--full-scan`.
    pub github_api_base: Option<String>,
    /// `VIBEVM_SEARCH_CACHE_DIR` — overrides `~/.vibe/search-cache/`.
    pub cache_dir: Option<String>,
}

pub fn run(ctx: &output::Context, args: SearchArgs, env: SearchEnv) -> Result<()> {
    let progress = ctx.progress();
    let preparation = progress.task("Preparing registry search");
    let prepared = (|| -> Result<_> {
        let project_root = resolve_project_root(&args.path)?;
        let manifest = load_project_manifest(&project_root)?;
        Ok((project_root, manifest))
    })();
    let (project_root, manifest) = match prepared {
        Ok(value) => {
            preparation.finish();
            value
        }
        Err(error) => {
            preparation.fail(error.to_string());
            return Err(error);
        }
    };

    let source_options = vibe_package_source::PackageSourceOptions {
        source_policy: args.source_policy.declaration(),
        ..Default::default()
    };
    let operation = vibe_package_source::source_operation(
        &source_options,
        &manifest,
        &project_root,
        vibe_package_source::deferred_user_registry(crate::output::seeds_default_registry()),
    )?;

    // PROP-002 `#registry-layers`: the registries a search reads are the
    // ones resolution reads — `.vibe/registry.local.toml`, `vibe.toml`,
    // then `~/.vibe/registry.toml` (and the applied site profile).
    let effective = crate::commands::registry::effective_registries(&project_root, &manifest)?;
    if effective.registries.is_empty() {
        bail!(
            "no registry configured. Add a `[[registry]]` entry to `vibe.toml` (or \
             `~/.vibe/registry.toml`) or run `vibe search` against a project that has one."
        );
    }

    let target_registries: Vec<&vibe_core::manifest::RegistrySection> =
        match args.registry.as_deref() {
            None => effective.registries.iter().collect(),
            Some(name) => match effective.registries.iter().find(|r| r.name == name) {
                Some(r) => vec![r],
                None => bail!(
                    "no `[[registry]]` named `{name}` in `vibe.toml`, \
                     `.vibe/registry.local.toml` or `~/.vibe/registry.toml`. Configured: {}",
                    effective
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
        let lookup = progress.task("Looking up package URL");
        let result = purl::run_purl_lookup(
            ctx,
            &project_root,
            &target_registries,
            purl_norm,
            &operation,
        );
        if result.is_ok() {
            lookup.finish();
        } else {
            lookup.fail("package URL lookup failed");
        }
        return result;
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
        search_cache::cache_root(env.cache_dir.as_deref())
    };
    let github_api_base = env
        .github_api_base
        .unwrap_or_else(|| GITHUB_API_DEFAULT.to_string());
    let kind_str: Option<String> = kind_filter.map(|k| k.as_str().to_string());

    // Walk registries and aggregate. Track per-registry status so the
    // report distinguishes "no index configured" (operator action
    // required) from "index URL set but server down" (transient) and,
    // when --full-scan is on, "scanned via org-walk" from "host not
    // supported by full-scan".
    let mut searched: Vec<String> = Vec::new();
    let mut unconfigured: Vec<String> = Vec::new();
    let mut unreachable: Vec<UnreachableRegistry> = Vec::new();
    let mut policy_skipped = Vec::new();
    let mut full_scanned: Vec<String> = Vec::new();
    let mut full_scan_unsupported: Vec<UnreachableRegistry> = Vec::new();
    let mut by_pkg: HashMap<(PackageKind, String), HitRow> = HashMap::new();

    let registry_search = progress.task("Searching configured registries");
    registry_search.set_progress(0, Some(target_registries.len() as u64), "registries");
    let mut search_failed = false;
    for (registry_index, reg) in target_registries.iter().enumerate() {
        let mut component_failed = false;
        let component = registry_search
            .progress()
            .task(format!("Searching registry {}", reg.name));
        component.detail(if full_scan {
            "index search with full-scan fallback"
        } else {
            "index search"
        });
        // A directory-layout registry carries its own index in its tree
        // (PROP-002 `#registry-index`): read it there — or compute it in
        // memory when it is absent or stale — instead of probing a URL.
        if layout::is_layout(reg) {
            match layout::search(
                reg,
                &effective.mirrors,
                &operation,
                &query,
                kind_filter,
                args.limit,
                &progress,
            ) {
                Ok(found) => {
                    searched.push(reg.name.clone());
                    for hit in &found.hits {
                        insert_hit_keep_highest(&mut by_pkg, make_hit_row(hit, &reg.name));
                    }
                    if let Some(note) = found.stale {
                        component.detail(note);
                    }
                    component.finish();
                }
                Err(error) => {
                    if let Some(reason) = error.downcast_ref::<vibe_registry::ContactRefusal>() {
                        if matches!(reason, vibe_registry::ContactRefusal::OperationCancelled) {
                            return Err(error);
                        }
                        policy_skipped.push(PolicySkippedRegistry::refused(&reg.name, reason));
                        component.skip("registry contact refused by source policy");
                    } else {
                        unreachable.push(UnreachableRegistry {
                            name: reg.name.clone(),
                            reason: format!("{error:#}"),
                        });
                        component.fail("registry index could not be read");
                        search_failed = true;
                    }
                }
            }
            registry_search.set_progress(
                (registry_index + 1) as u64,
                Some(target_registries.len() as u64),
                "registries",
            );
            continue;
        }
        // The B-083 ladder: env → `[[registry]].index_url` → the
        // `<registry-url>/index` default. `served_by_index` = the index
        // path answered for this registry (hits or a recorded failure);
        // `false` falls through to the indexless path — an explicit
        // `none`, or the default-rung guess finding nothing there (a
        // registry without an index is not an error).
        let served_by_index = match resolve_index_url(reg) {
            IndexUrlResolution::Disabled => false,
            IndexUrlResolution::Url { base, source } => 'index: {
                if source == IndexUrlSource::Default
                    && vibe_registry::resolve_registry_auth(
                        vibe_registry::RegistryAuthInput::SelectedRegistry(reg),
                    )?
                    .is_ssh_source()
                {
                    break 'index false;
                }
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
                    break 'index true;
                }
                // A2-INDEXAUTH — authenticate with the registry's own
                // credentials, and tell a refused (401/403) probe apart
                // from a missing index so the operator sees the real
                // reason in `UnreachableRegistry.reason`.
                let probe = match operation.http_admission()? {
                    Some(binding) => match binding.check_address(&base) {
                        Ok(()) => IndexClient::probe_with_admission(
                            &base,
                            IndexAuth::for_registry(reg, &base)?,
                            binding,
                        ),
                        Err(reason) => Err(vibe_registry::IndexError::ContactRefused(reason)),
                    },
                    None => Ok(IndexClient::probe(
                        &base,
                        IndexAuth::for_registry(reg, &base)?,
                    )),
                };
                let probe = match probe {
                    Ok(value) => value,
                    Err(vibe_registry::IndexError::ContactRefused(reason)) => {
                        if matches!(reason, vibe_registry::ContactRefusal::OperationCancelled) {
                            return Err(reason.into());
                        }
                        policy_skipped.push(PolicySkippedRegistry::refused(&reg.name, &reason));
                        component.skip("index contact refused by source policy");
                        component_failed = true;
                        break 'index true;
                    }
                    Err(error) => return Err(error.into()),
                };
                let client = match probe {
                    ProbeOutcome::Found(c) => *c,
                    ProbeOutcome::Refused { reason } => {
                        unreachable.push(UnreachableRegistry {
                            name: reg.name.clone(),
                            reason,
                        });
                        component.fail("registry index refused the probe");
                        component_failed = true;
                        search_failed = true;
                        break 'index true;
                    }
                    ProbeOutcome::Absent if source == IndexUrlSource::Default => {
                        break 'index false;
                    }
                    ProbeOutcome::Absent => {
                        unreachable.push(UnreachableRegistry {
                            name: reg.name.clone(),
                            reason: format!(
                                "probe of `{base}/repomd.json` failed (server down or wrong URL)"
                            ),
                        });
                        component.fail("configured registry index is unavailable");
                        component_failed = true;
                        search_failed = true;
                        break 'index true;
                    }
                };
                match client.search(&query, kind_filter, Some(args.limit)) {
                    Ok(results) => {
                        searched.push(reg.name.clone());
                        if let Some(root) = cache_root.as_ref()
                            && let Err(e) = search_cache::save(root, &cache_key, &results)
                        {
                            tracing::debug!(
                                target: "vibe_registry::search::cache",
                                "could not write cache entry for {}: {e}",
                                reg.name
                            );
                        }
                        for hit in results.hits {
                            insert_hit_keep_highest(&mut by_pkg, make_hit_row(&hit, &reg.name));
                        }
                    }
                    Err(vibe_registry::IndexError::ContactRefused(reason)) => {
                        if matches!(reason, vibe_registry::ContactRefusal::OperationCancelled) {
                            return Err(reason.into());
                        }
                        policy_skipped.push(PolicySkippedRegistry::refused(&reg.name, &reason));
                        component.skip("index contact refused by source policy");
                        component_failed = true;
                    }
                    Err(e) => {
                        unreachable.push(UnreachableRegistry {
                            name: reg.name.clone(),
                            reason: format!("{e}"),
                        });
                        component.fail("registry index search failed");
                        component_failed = true;
                        search_failed = true;
                    }
                }
                true
            }
        };
        if !served_by_index {
            if full_scan {
                match run_full_scan_for_registry(
                    &query,
                    kind_filter,
                    reg,
                    &github_api_base,
                    &operation,
                ) {
                    Ok(hits) => {
                        full_scanned.push(reg.name.clone());
                        for h in hits {
                            insert_hit_keep_highest(
                                &mut by_pkg,
                                make_full_scan_hit_row(&h, &reg.name),
                            );
                        }
                        component.finish();
                    }
                    Err(error) => {
                        if matches!(
                            error.downcast_ref::<vibe_registry::ContactRefusal>(),
                            Some(vibe_registry::ContactRefusal::OperationCancelled)
                        ) {
                            return Err(error);
                        }
                        if let Some(reason) = error.downcast_ref::<vibe_registry::ContactRefusal>()
                        {
                            policy_skipped.push(PolicySkippedRegistry::refused(&reg.name, reason));
                            component.skip("full scan contact refused by source policy");
                        } else {
                            let reason = error.to_string();
                            full_scan_unsupported.push(UnreachableRegistry {
                                name: reg.name.clone(),
                                reason,
                            });
                            component.fail("registry full scan failed or is unsupported");
                            search_failed = true;
                        }
                    }
                }
            } else {
                unconfigured.push(reg.name.clone());
                component.skip("registry index is not configured");
            }
        } else if !component_failed {
            component.finish();
        }
        registry_search.set_progress(
            (registry_index + 1) as u64,
            Some(target_registries.len() as u64),
            "registries",
        );
    }
    if search_failed {
        registry_search.fail("one or more registry searches failed");
    } else {
        registry_search.finish();
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
        registries_policy_skipped: policy_skipped.clone(),
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
            "vibe search: {} hit{} across {} registr{}",
            hits.len(),
            if hits.len() == 1 { "" } else { "s" },
            searched.len(),
            if searched.len() == 1 { "y" } else { "ies" },
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
    for skipped in &policy_skipped {
        println!(
            "  policy skipped {} (availability unknown): {}",
            skipped.name, skipped.reason
        );
    }
    for u in &full_scan_unsupported {
        println!("  full-scan unsupported: {} — {}", u.name, u.reason);
    }
    if !unconfigured.is_empty() {
        println!("  no VIBEVM_INDEX_URL_<R> set: {}", unconfigured.join(", "),);
    }
    println!();

    if hits.is_empty() {
        if searched.is_empty() {
            ctx.summary(
                "(no registry has VIBEVM_INDEX_URL_<R> configured; search returns empty.\n\
                 To install a known package, run `vibe install <group>/<name>` directly —\n\
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
        "{} hit{} across {} registr{}",
        hits.len(),
        if hits.len() == 1 { "" } else { "s" },
        searched.len(),
        if searched.len() == 1 { "y" } else { "ies" },
    ));
    Ok(())
}

mod support;
use support::*;
