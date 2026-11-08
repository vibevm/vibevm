//! `vibe search --purl` — reverse PURL lookup across every configured
//! registry index (PROP-005 §2.10 `/purls` route).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use semver::Version;
use serde::Serialize;
use vibe_core::PackageKind;
use vibe_registry::{
    BindingSite, IndexAuth, IndexClient, IndexUrlResolution, IndexUrlSource, ProbeOutcome,
    PurlLookupHit, resolve_index_url,
};

use crate::output;

use super::{PolicySkippedRegistry, UnreachableRegistry, truncate};

#[derive(Debug, Serialize)]
struct PurlReport {
    ok: bool,
    command: &'static str,
    project: String,
    purl: String,
    registries_searched: Vec<String>,
    registries_unconfigured: Vec<String>,
    registries_unreachable: Vec<UnreachableRegistry>,
    registries_policy_skipped: Vec<PolicySkippedRegistry>,
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

pub(super) fn run_purl_lookup(
    ctx: &output::Context,
    project_root: &Path,
    target_registries: &[&vibe_core::manifest::RegistrySection],
    purl: &str,
    operation: &vibe_package_source::SourceOperation,
) -> Result<()> {
    let mut policy_skipped = Vec::new();
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
        // The B-083 ladder: env → `[[registry]].index_url` → the
        // `<registry-url>/index` default. Explicit `none` is a
        // deliberate index-off — the indexless bucket, same as before
        // the ladder existed.
        let (base, source) = match resolve_index_url(reg) {
            IndexUrlResolution::Url { base, source } => (base, source),
            IndexUrlResolution::Disabled => {
                unconfigured.push(reg.name.clone());
                continue;
            }
        };
        if source == IndexUrlSource::Default
            && vibe_registry::resolve_registry_auth(
                vibe_registry::RegistryAuthInput::SelectedRegistry(reg),
            )?
            .is_ssh_source()
        {
            unconfigured.push(reg.name.clone());
            continue;
        }
        // A2-INDEXAUTH — authenticate with the registry's own
        // credentials, and tell a refused (401/403) probe apart from a
        // missing index so the operator sees the real reason in
        // `UnreachableRegistry.reason`.
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
                continue;
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
                continue;
            }
            // The default rung is a guess the spec prescribes trying —
            // nothing there means "this registry has no index", not an
            // error worth an unreachable row.
            ProbeOutcome::Absent if source == IndexUrlSource::Default => {
                unconfigured.push(reg.name.clone());
                continue;
            }
            ProbeOutcome::Absent => {
                unreachable.push(UnreachableRegistry {
                    name: reg.name.clone(),
                    reason: format!(
                        "probe of `{base}/repomd.json` failed (server down or wrong URL)"
                    ),
                });
                continue;
            }
        };
        match client.lookup_purl(purl) {
            Ok(results) => {
                searched.push(reg.name.clone());
                for hit in results.hits {
                    let key = (hit.kind, hit.name.clone(), hit.version.clone());
                    by_kvn
                        .entry(key)
                        .or_insert_with(|| make_purl_row(&hit, &reg.name));
                }
            }
            Err(vibe_registry::IndexError::ContactRefused(reason)) => {
                if matches!(reason, vibe_registry::ContactRefusal::OperationCancelled) {
                    return Err(reason.into());
                }
                policy_skipped.push(PolicySkippedRegistry::refused(&reg.name, &reason));
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
        registries_policy_skipped: policy_skipped.clone(),
        hit_count: hits.len(),
        hits: hits.clone(),
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }

    for skipped in &policy_skipped {
        eprintln!(
            "policy skipped {} (availability unknown): {}",
            skipped.name, skipped.reason
        );
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

    println!("KIND    NAME                          VERSION       BINDING-SITE  REGISTRY");
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
