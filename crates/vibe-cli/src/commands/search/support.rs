//! Search row projection and selected source helpers.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");
use super::*;

pub(super) fn make_hit_row(hit: &SearchHit, registry: &str) -> HitRow {
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

pub(super) fn make_full_scan_hit_row(hit: &FullScanHit, registry: &str) -> HitRow {
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

pub(super) fn insert_hit_keep_highest(
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

pub(super) fn run_full_scan_for_registry(
    query: &str,
    kind_filter: Option<PackageKind>,
    reg: &vibe_core::manifest::RegistrySection,
    api_base: &str,
    operation: &vibe_package_source::SourceOperation,
) -> Result<Vec<FullScanHit>> {
    let Some(org) = search_full_scan::detect_github_org(&reg.url) else {
        bail!(
            "host of `{}` is not `github.com` (only GitHub orgs supported by --full-scan v0)",
            reg.url
        );
    };
    let token = vibe_publish::token::load_token_for_host("github.com")
        .ok()
        .map(|t| t.value().to_string());
    let token_ref = token.as_deref();
    let query_tokens = query::tokenise_query(query);
    if query_tokens.is_empty() {
        bail!("query has no searchable tokens after stopword filtering");
    }
    let result = match operation.http_admission()? {
        Some(binding) => search_full_scan::full_scan_github_org_with_admission(
            &org,
            api_base,
            token_ref,
            &query_tokens,
            kind_filter,
            binding,
        ),
        None => search_full_scan::full_scan_github_org(
            &org,
            api_base,
            token_ref,
            &query_tokens,
            kind_filter,
        ),
    };
    match result {
        Err(search_full_scan::FullScanError::ContactRefused(reason)) => Err(reason.into()),
        other => other.map_err(Into::into),
    }
}

pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// The directory a search runs from. It need not be a project: outside one
/// the machine layers (`~/.vibe/registry.toml` and the applied site profile)
/// are the registries searched (PROP-002 `#registry-layers`).
pub(super) fn resolve_project_root(path: &Path) -> Result<std::path::PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    Ok(crate::commands::init::strip_unc_public(canonical))
}

/// The project manifest, or an empty one outside a project, so the
/// project layer simply contributes no registries.
pub(super) fn load_project_manifest(root: &Path) -> Result<Manifest> {
    let path = root.join(Manifest::FILENAME);
    if !path.exists() {
        return Ok(Manifest::default());
    }
    Ok(Manifest::read(&path)?)
}
