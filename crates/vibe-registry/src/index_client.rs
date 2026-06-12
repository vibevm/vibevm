//! Optional HTTP client that lets `GitPackageRegistry` consult an
//! upstream index (PROP-005 §2.10) for cheap version enumeration
//! before falling back to `git ls-remote`. Slice 10.
//!
//! The client is resilient: any failure (4xx, 5xx, connect-fail,
//! malformed JSON) returns an error that the caller treats as a
//! fall-through trigger. Identity (`content_hash`) is verified at
//! fetch time per [PROP-002 §2.1] regardless of how versions were
//! enumerated, so a compromised index can at worst mislead the
//! version selector — never substitute content.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#http");

use std::time::Duration;

use semver::Version;
use serde::Deserialize;
use specmark::spec;
use thiserror::Error;
use vibe_core::{Group, PackageKind};

const PROBE_TIMEOUT_SECS: u64 = 5;
const FETCH_TIMEOUT_SECS: u64 = 10;

/// Resolved client.
///
/// `file_base` is the URL prefix that, when joined with `repomd.json`
/// or `by-name/<name>.json`, addresses the per-file endpoints
/// (the static-mirror-friendly read surface from PROP-005 §2.4).
/// `server_base` is the URL prefix for structured live-server routes
/// (`/v1/packages`, `/v1/capabilities/{cap}`, etc. from PROP-005
/// §2.10). Built via [`IndexClient::probe`] which auto-detects
/// whether the supplied operator URL points at a vibe-index server
/// (`<base>/v1/index/...`) or a static raw-file root (`<base>/...`)
/// — `server_base` is always the bare `<base>` regardless, since the
/// structured routes only exist on a live server and never on a
/// static mirror.
#[derive(Debug, Clone)]
pub struct IndexClient {
    file_base: String,
    server_base: String,
}

#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-index/PROP-005#http")]
pub enum IndexError {
    #[error(
        "HTTP request to `{url}` failed \
         (violates spec://vibevm/modules/vibe-index/PROP-005#http; \
          fix: check the index URL and network reachability): {message}"
    )]
    Http { url: String, message: String },
    #[error(
        "index at `{url}` returned status {status} \
         (violates spec://vibevm/modules/vibe-index/PROP-005#http; \
          fix: check the index server health at that URL)"
    )]
    Status { url: String, status: u16 },
    #[error(
        "index at `{url}` returned malformed JSON \
         (violates spec://vibevm/modules/vibe-index/PROP-005#http; \
          fix: regenerate the index via reindex): {message}"
    )]
    Malformed { url: String, message: String },
}

impl IndexClient {
    /// Probe the operator-supplied base URL. Returns `Some(client)`
    /// if `<base>/repomd.json` OR `<base>/v1/index/repomd.json`
    /// responds with HTTP 200; `None` otherwise (no index there).
    /// Probe timeout is short (5s) so a misconfigured URL does not
    /// stall every install.
    pub fn probe(base: &str) -> Option<IndexClient> {
        let trimmed = base.trim_end_matches('/');
        let client = match Self::build_client(Duration::from_secs(PROBE_TIMEOUT_SECS)) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(target: "vibe_registry::index_client", "could not build probe client: {e}");
                return None;
            }
        };
        for candidate in [format!("{trimmed}/v1/index"), trimmed.to_string()] {
            let url = format!("{candidate}/repomd.json");
            if let Ok(resp) = client.get(&url).send()
                && resp.status().is_success()
            {
                tracing::debug!(target: "vibe_registry::index_client", "probe succeeded at {url}");
                return Some(IndexClient {
                    file_base: candidate,
                    server_base: trimmed.to_string(),
                });
            }
        }
        tracing::debug!(target: "vibe_registry::index_client", "no index found at base `{base}`");
        None
    }

    /// Construct directly without probing. Used by tests where the
    /// caller has set up the server and knows its layout. Both
    /// `file_base` and `server_base` are set to the supplied URL —
    /// suitable for the in-tree `tests/` mock servers that mount
    /// raw-file routes (`/repomd.json`, `/by-name/...`) and the
    /// structured server routes (`/v1/packages`) on the same root.
    pub fn at(base: impl Into<String>) -> IndexClient {
        let trimmed = base.into().trim_end_matches('/').to_string();
        IndexClient {
            file_base: trimmed.clone(),
            server_base: trimmed,
        }
    }

    pub fn file_base(&self) -> &str {
        &self.file_base
    }

    pub fn server_base(&self) -> &str {
        &self.server_base
    }

    /// Fetch the `by-name/<name>.json` candidate set and return the
    /// versions of the `(group, name)` package in ascending semver
    /// order. Returns `Ok(None)` when the file is absent (404) **or**
    /// the candidate set carries no package for `group` — both mean
    /// "fall through to `git ls-remote`". `Ok(Some(versions))` on a
    /// hit; `Err(...)` for any other failure.
    ///
    /// The `by-name/` layer is keyed by bare `name` and holds the whole
    /// candidate set — every group that publishes a package of that
    /// name (PROP-008 §2.8). The lookup selects the candidate whose
    /// `group` matches the requested `(group, name)` identity.
    pub fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Option<Vec<Version>>, IndexError> {
        let url = format!("{}/by-name/{}.json", self.file_base, name);
        let client = Self::build_client(Duration::from_secs(FETCH_TIMEOUT_SECS)).map_err(|e| {
            IndexError::Http {
                url: url.clone(),
                message: e.to_string(),
            }
        })?;
        let resp = client.get(&url).send().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(IndexError::Status {
                url,
                status: status.as_u16(),
            });
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let parsed: NameEntryView =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        let Some(pkg) = parsed.packages.into_iter().find(|p| &p.group == group) else {
            return Ok(None);
        };
        let mut versions: Vec<Version> = pkg.versions.into_iter().map(|v| v.version).collect();
        versions.sort();
        Ok(Some(versions))
    }

    /// Fetch the `by-name/<name>.json` candidate set and return every
    /// `group` that publishes a package of this bare name (PROP-008
    /// §2.8). This is the primitive index-backed short-name resolution
    /// (PROP-008 §2.6) walks: one GET per registry enumerates the
    /// `(*, name)` candidates, so a collision (PROP-008 §2.7) — two
    /// groups under one bare name — is visible at once.
    ///
    /// `Ok(vec![])` when the file is absent (404) — the name is simply
    /// not carried by this index. `Err(...)` for any other failure;
    /// the caller decides whether to treat it as fatal or skip the
    /// registry. Groups are returned in on-disk order; de-duplication
    /// and sorting are the caller's job (it unions across registries).
    pub fn name_candidates(&self, name: &str) -> Result<Vec<Group>, IndexError> {
        let url = format!("{}/by-name/{}.json", self.file_base, name);
        let client = Self::build_client(Duration::from_secs(FETCH_TIMEOUT_SECS)).map_err(|e| {
            IndexError::Http {
                url: url.clone(),
                message: e.to_string(),
            }
        })?;
        let resp = client.get(&url).send().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(Vec::new());
        }
        if !status.is_success() {
            return Err(IndexError::Status {
                url,
                status: status.as_u16(),
            });
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let parsed: NameEntryView =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        Ok(parsed.packages.into_iter().map(|p| p.group).collect())
    }

    /// Direct PURL lookup against the live-server route
    /// `<server_base>/v1/purls/{purl}` from PROP-005 §2.10. Returns
    /// every package whose top-level `describes` or any subskill's
    /// `describes` equals the supplied PURL, with the `binding_site`
    /// surfaced so consumers see whether the match originated at the
    /// package or subskill level.
    ///
    /// Non-2xx surfaces as [`IndexError::Status`]; 404 here means the
    /// URL points at a raw-file mirror without the live server. Empty
    /// `hits` is the "no match" case (HTTP 200 with 0-length list),
    /// not 404. Path-segment encoding is delegated to `reqwest::Url`
    /// so PURL punctuation (`:`, `/`, `@`) is escaped correctly.
    pub fn lookup_purl(&self, purl: &str) -> Result<PurlLookupResults, IndexError> {
        let base_url = format!("{}/v1/purls/", self.server_base);
        let mut parsed = reqwest::Url::parse(&base_url).map_err(|e| IndexError::Http {
            url: base_url.clone(),
            message: e.to_string(),
        })?;
        parsed
            .path_segments_mut()
            .map_err(|_| IndexError::Http {
                url: base_url.clone(),
                message: "base URL is not hierarchical".into(),
            })?
            .pop_if_empty()
            .push(purl);
        let url = parsed.to_string();
        let client = Self::build_client(Duration::from_secs(FETCH_TIMEOUT_SECS)).map_err(|e| {
            IndexError::Http {
                url: url.clone(),
                message: e.to_string(),
            }
        })?;
        let resp = client.get(&url).send().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(IndexError::Status {
                url,
                status: status.as_u16(),
            });
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let parsed: PurlLookupResults =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        Ok(parsed)
    }

    /// Run a full-text search against the live-server route
    /// `<server_base>/v1/packages?q=<query>[&kind=&limit=]` from
    /// PROP-005 §2.10. Returns the structured response on 200; any
    /// non-2xx status surfaces as [`IndexError::Status`] so the
    /// caller can decide whether to fall through to another registry
    /// or surface the error. A 404 here means the URL is a raw-file
    /// mirror (no live server), not "package absent" — there is no
    /// "package absent" case for this endpoint, since search returns
    /// an empty `hits` array on no matches. Identity / integrity
    /// invariants are unaffected: search is metadata-only and never
    /// resolves into a fetch without the consumer running through
    /// the regular `MultiRegistryResolver` path that re-verifies
    /// `content_hash` per [PROP-002 §2.1].
    pub fn search(
        &self,
        query: &str,
        kind: Option<PackageKind>,
        limit: Option<usize>,
    ) -> Result<SearchResults, IndexError> {
        let url = format!("{}/v1/packages", self.server_base);
        let client = Self::build_client(Duration::from_secs(FETCH_TIMEOUT_SECS)).map_err(|e| {
            IndexError::Http {
                url: url.clone(),
                message: e.to_string(),
            }
        })?;
        let mut req = client.get(&url).query(&[("q", query)]);
        if let Some(k) = kind {
            req = req.query(&[("kind", k.as_str())]);
        }
        if let Some(lim) = limit {
            req = req.query(&[("limit", lim.to_string())]);
        }
        let resp = req.send().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(IndexError::Status {
                url,
                status: status.as_u16(),
            });
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let parsed: SearchResults =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        Ok(parsed)
    }

    fn build_client(timeout: Duration) -> Result<reqwest::blocking::Client, reqwest::Error> {
        reqwest::blocking::Client::builder()
            .user_agent(concat!("vibe-registry/", env!("CARGO_PKG_VERSION")))
            .timeout(timeout)
            .build()
    }
}

/// Decoded `by-name/<name>.json` — the candidate set for one bare name
/// (PROP-008 §2.8). Only the fields the resolver's version selector
/// needs are read; the rest of the on-disk shape is tolerated.
#[derive(Debug, Deserialize)]
struct NameEntryView {
    #[serde(default)]
    packages: Vec<PackageEntryView>,
}

#[derive(Debug, Deserialize)]
struct PackageEntryView {
    group: Group,
    #[serde(default)]
    versions: Vec<VersionEntryView>,
}

#[derive(Debug, Deserialize)]
struct VersionEntryView {
    version: Version,
}

/// Decoded body of the structured search route. Mirrors the wire
/// shape produced by `vibe_index::server::routes::packages::SearchResponse`.
/// Extra fields on the wire (today: `command`) are tolerated
/// silently — kept simple so a server-side envelope addition does
/// not force a client bump.
///
/// `Serialize` is derived alongside `Deserialize` so the CLI-side
/// `~/.vibe/search-cache/` layer can persist a decoded result and
/// load it back without a separate cache-only schema.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SearchResults {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub hit_count: usize,
    #[serde(default)]
    pub hits: Vec<SearchHit>,
}

/// One package matched by the index's search backend.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: PackageKind,
    pub name: String,
    #[serde(default)]
    pub latest_stable: Option<Version>,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub matched_tokens: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Decoded body of the structured PURL-lookup route. Mirrors
/// `vibe_index::server::routes::purls::Response`.
#[derive(Debug, Clone, Deserialize)]
pub struct PurlLookupResults {
    #[serde(default)]
    pub purl: String,
    #[serde(default)]
    pub hit_count: usize,
    #[serde(default)]
    pub hits: Vec<PurlLookupHit>,
}

/// One concrete `(kind, name, version)` whose package- or subskill-level
/// `describes` matched the queried PURL.
#[derive(Debug, Clone, Deserialize)]
pub struct PurlLookupHit {
    pub kind: PackageKind,
    pub name: String,
    pub version: Version,
    pub binding_site: BindingSite,
}

/// Where the PURL match originated on the matched entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BindingSite {
    /// PURL declared on the entry's top-level `describes` field.
    Package,
    /// PURL declared on a subskill within the entry.
    Subskill,
}

impl std::fmt::Display for BindingSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindingSite::Package => f.write_str("package"),
            BindingSite::Subskill => f.write_str("subskill"),
        }
    }
}

/// Resolve `<index_url>` for the named registry from environment.
/// Mirrors the `VIBEVM_INDEX_URL_<REGISTRY>` shape used by
/// `vibe-publish::post_hook`.
pub fn index_url_for(registry: &str) -> Option<String> {
    let suffix = registry_env_suffix(registry);
    if suffix.is_empty() {
        return None;
    }
    std::env::var(format!("VIBEVM_INDEX_URL_{suffix}"))
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn registry_env_suffix(registry: &str) -> String {
    let mut out = String::with_capacity(registry.len());
    for c in registry.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_env_suffix_uppercases() {
        assert_eq!(registry_env_suffix("vibespecs"), "VIBESPECS");
        assert_eq!(
            registry_env_suffix("vibespecs-gitverse"),
            "VIBESPECS_GITVERSE"
        );
    }

    #[test]
    fn at_strips_trailing_slash() {
        let c = IndexClient::at("https://example.com/foo/");
        assert_eq!(c.file_base(), "https://example.com/foo");
        assert_eq!(c.server_base(), "https://example.com/foo");
    }

    #[test]
    fn search_results_decode_minimal_envelope() {
        let body = serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.1.0",
                    "score": 3,
                    "matched_tokens": ["wal"],
                    "description": "Write-ahead log"
                }
            ]
        });
        let parsed: SearchResults = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.query, "wal");
        assert_eq!(parsed.hit_count, 1);
        assert_eq!(parsed.hits.len(), 1);
        assert_eq!(parsed.hits[0].kind, PackageKind::Flow);
        assert_eq!(parsed.hits[0].name, "wal");
        assert_eq!(parsed.hits[0].score, 3);
        assert_eq!(
            parsed.hits[0].latest_stable.as_ref().unwrap().to_string(),
            "0.1.0"
        );
        assert_eq!(parsed.hits[0].matched_tokens, vec!["wal".to_string()]);
        assert_eq!(
            parsed.hits[0].description.as_deref(),
            Some("Write-ahead log")
        );
    }

    #[test]
    fn search_hit_tolerates_missing_optional_fields() {
        let body = serde_json::json!({
            "kind": "feat",
            "name": "atomic-commits"
        });
        let parsed: SearchHit = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.kind, PackageKind::Feat);
        assert_eq!(parsed.name, "atomic-commits");
        assert_eq!(parsed.score, 0);
        assert!(parsed.latest_stable.is_none());
        assert!(parsed.matched_tokens.is_empty());
        assert!(parsed.description.is_none());
    }

    #[test]
    fn purl_lookup_results_decode_full_envelope() {
        let body = serde_json::json!({
            "command": "purls",
            "purl": "pkg:cargo/sqlx@0.8.0",
            "hit_count": 2,
            "hits": [
                {
                    "kind": "flow",
                    "name": "sqlx-skin",
                    "version": "0.1.0",
                    "binding_site": "package"
                },
                {
                    "kind": "stack",
                    "name": "rust",
                    "version": "0.2.0",
                    "binding_site": "subskill"
                }
            ]
        });
        let parsed: PurlLookupResults = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.purl, "pkg:cargo/sqlx@0.8.0");
        assert_eq!(parsed.hit_count, 2);
        assert_eq!(parsed.hits.len(), 2);
        assert_eq!(parsed.hits[0].kind, PackageKind::Flow);
        assert_eq!(parsed.hits[0].binding_site, BindingSite::Package);
        assert_eq!(parsed.hits[1].binding_site, BindingSite::Subskill);
    }

    #[test]
    fn binding_site_display_renders_lowercase_word() {
        assert_eq!(format!("{}", BindingSite::Package), "package");
        assert_eq!(format!("{}", BindingSite::Subskill), "subskill");
    }

    #[test]
    fn name_entry_view_extracts_candidate_groups() {
        // `name_candidates` decodes `by-name/<name>.json` into a
        // `NameEntryView` and maps each package to its `group` — the
        // candidate set short-name resolution (PROP-008 §2.6) walks.
        // Two groups under one bare name is a collision (§2.7). The
        // surrounding `name` / `indexed_at` fields are tolerated.
        let body = serde_json::json!({
            "name": "wal",
            "indexed_at": "2026-05-22T00:00:00Z",
            "packages": [
                { "group": "org.vibevm", "versions": [{ "version": "0.1.0" }] },
                { "group": "com.acme", "versions": [{ "version": "0.2.0" }] }
            ]
        });
        let parsed: NameEntryView = serde_json::from_value(body).unwrap();
        let groups: Vec<String> = parsed
            .packages
            .iter()
            .map(|p| p.group.to_string())
            .collect();
        assert_eq!(groups, vec!["org.vibevm", "com.acme"]);
    }
}
