//! Optional post-publish hook — POSTs a freshly-built `VersionEntry`
//! to a vibevm-index server immediately after a successful push.
//!
//! Activation is opt-in per-registry: the hook fires when *both*
//! `VIBEVM_INDEX_URL_<REGISTRY>` and `VIBEVM_INDEX_TOKEN_<REGISTRY>`
//! environment variables resolve for the registry the publish is
//! targeting. If either is missing, the hook stays silent. Hook
//! failures are warnings — they NEVER fail the publish itself
//! (PROP-005 §2.14: "Failure of the index POST does NOT fail the
//! publish — it logs a warning and the operator's next
//! `vibe-index reindex` covers the gap.").
//!
//! Token discipline per [PROP-000 §20]: the token bytes never
//! appear in stdout / stderr / log output. The Authorization header
//! carries the bearer token to the index server only.

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;
use vibe_core::manifest::Manifest;
use vibe_registry::compute_content_hash;
use walkdir::WalkDir;

use crate::PublishOutcome;

/// Sanitise a registry name into the env-var suffix shape (uppercase
/// ASCII alphanumeric + underscore). Mirrors the `<HOST>` munging
/// that `host_env_var` does for `VIBEVM_PUBLISH_TOKEN_<HOST>`.
pub fn registry_env_suffix(registry: &str) -> String {
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

pub fn index_url_for(registry: &str) -> Option<String> {
    let suffix = registry_env_suffix(registry);
    if suffix.is_empty() {
        return None;
    }
    std::env::var(format!("VIBEVM_INDEX_URL_{suffix}")).ok()
}

pub fn index_token_for(registry: &str) -> Option<String> {
    let suffix = registry_env_suffix(registry);
    if suffix.is_empty() {
        return None;
    }
    std::env::var(format!("VIBEVM_INDEX_TOKEN_{suffix}")).ok()
}

#[derive(Debug, Error)]
pub enum HookError {
    #[error("could not read manifest at `{path}`: {source}")]
    Manifest {
        path: std::path::PathBuf,
        #[source]
        source: Box<vibe_core::Error>,
    },
    #[error("could not compute content_hash on `{path}`: {source}")]
    ContentHash {
        path: std::path::PathBuf,
        #[source]
        source: Box<vibe_registry::RegistryError>,
    },
    #[error("HTTP POST to index failed: {0}")]
    Http(#[from] Box<reqwest::Error>),
    #[error("index server returned status {status}: {body}")]
    UnexpectedStatus { status: u16, body: String },
    #[error("invalid header: {0}")]
    Header(String),
}

impl From<reqwest::Error> for HookError {
    fn from(e: reqwest::Error) -> Self {
        HookError::Http(Box::new(e))
    }
}

#[derive(Debug, Clone)]
pub struct HookConfig {
    pub index_url: String,
    pub token: String,
    pub timeout: Duration,
}

impl HookConfig {
    /// Resolve hook config from env vars for `registry`. Returns
    /// `None` when either URL or token is missing — the hook stays
    /// dormant in that case.
    pub fn from_env(registry: &str) -> Option<Self> {
        let url = index_url_for(registry)?;
        let token = index_token_for(registry)?;
        if url.trim().is_empty() || token.trim().is_empty() {
            return None;
        }
        Some(HookConfig {
            index_url: url,
            token,
            timeout: Duration::from_secs(15),
        })
    }
}

/// Outcome of a hook fire — what was sent and whether the index
/// accepted it.
#[derive(Debug, Clone, Serialize)]
pub struct HookReport {
    pub command: &'static str,
    pub fired: bool,
    pub url_endpoint: Option<String>,
    pub status: Option<u16>,
    pub error: Option<String>,
}

impl HookReport {
    pub fn dormant() -> Self {
        HookReport {
            command: "registry:publish:index-hook",
            fired: false,
            url_endpoint: None,
            status: None,
            error: None,
        }
    }
}

/// Build the JSON payload the index server expects (matches
/// `vibe-index::types::VersionEntry`'s serde shape).
pub fn build_payload(
    outcome: &PublishOutcome,
    manifest: &Manifest,
    source_dir: &Path,
    registry: &str,
    indexed_at: DateTime<Utc>,
) -> Result<serde_json::Value, HookError> {
    // The index hook only ever describes a publishable `[package]`
    // manifest; reject a `[project]`/`[workspace]`-only `vibe.toml`.
    let meta = manifest
        .require_package()
        .map_err(|e| HookError::Manifest {
            path: source_dir.join(Manifest::FILENAME),
            source: Box::new(e),
        })?;
    let content_hash =
        compute_content_hash(source_dir).map_err(|source| HookError::ContentHash {
            path: source_dir.to_path_buf(),
            source: Box::new(source),
        })?;
    let files_count = WalkDir::new(source_dir)
        .into_iter()
        .filter_map(|e: walkdir::Result<walkdir::DirEntry>| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u32;

    let provides_caps: Vec<String> = manifest
        .provides
        .capabilities
        .iter()
        .map(|c| c.to_string())
        .collect();
    let requires_packages: Vec<String> = manifest
        .requires
        .packages
        .iter()
        .map(|p| p.to_string())
        .collect();
    let requires_caps: Vec<String> = manifest
        .requires
        .capabilities
        .iter()
        .map(|c| c.to_string())
        .collect();
    let requires_any: Vec<serde_json::Value> = manifest
        .requires_any
        .iter()
        .map(|ra| {
            serde_json::json!({
                "one_of": ra.one_of.iter().map(|p| p.to_string()).collect::<Vec<_>>()
            })
        })
        .collect();
    let obsoletes_packages: Vec<String> = manifest
        .obsoletes
        .packages
        .iter()
        .map(|p| p.to_string())
        .collect();
    let conflicts_packages: Vec<String> = manifest
        .conflicts
        .packages
        .iter()
        .map(|p| p.to_string())
        .collect();
    let i18n_available: Vec<String> = manifest.i18n.available.clone();

    let mut payload = serde_json::json!({
        "schema_version": 1u32,
        "kind": meta.kind,
        "name": meta.name,
        "version": meta.version,
        "content_hash": content_hash,
        "source_url": outcome.repo_url,
        "source_ref": outcome.tag,
        "registry": registry,
        "license": meta.license,
        "authors": meta.authors,
        "description": meta.description,
        "homepage": meta.homepage,
        "keywords": meta.keywords,
        "describes": meta.describes.as_ref().map(|p| p.to_string()),
        "compatibility": serde_json::json!({
            "min_vibe_version": manifest.compatibility.min_vibe_version,
            "requires_kinds": manifest.compatibility.requires_kinds,
        }),
        "provides": serde_json::json!({ "capabilities": provides_caps }),
        "requires": serde_json::json!({
            "packages": requires_packages,
            "capabilities": requires_caps,
        }),
        "requires_any": requires_any,
        "obsoletes": serde_json::json!({ "packages": obsoletes_packages }),
        "conflicts": serde_json::json!({ "packages": conflicts_packages }),
        "i18n": serde_json::json!({
            "available": i18n_available,
            "default": manifest.i18n.canonical,
        }),
        "files_count": files_count,
        "indexed_at": indexed_at,
        "indexed_by": format!("vibe-publish {}", env!("CARGO_PKG_VERSION")),
    });

    if let Some(boot) = &manifest.boot_snippet {
        payload["boot_snippet"] = serde_json::json!({
            "source": boot.source,
            "category": boot.category,
        });
    }

    Ok(payload)
}

/// POST `payload` to `<config.index_url>/v1/packages` with bearer
/// auth. Returns the index server's status code on success.
pub fn post_to_index(config: &HookConfig, payload: &serde_json::Value) -> Result<u16, HookError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(config.timeout)
        .build()?;
    let url = format!("{}/v1/packages", config.index_url.trim_end_matches('/'));
    let auth = HeaderValue::from_str(&format!("Bearer {}", config.token))
        .map_err(|e| HookError::Header(e.to_string()))?;
    let resp = client
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header(CONTENT_TYPE, "application/json")
        .json(payload)
        .send()?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(HookError::UnexpectedStatus {
            status: status.as_u16(),
            body,
        });
    }
    Ok(status.as_u16())
}

/// One-shot fire helper — resolves config from env, builds payload,
/// posts. Errors are converted to warnings + a `HookReport` with
/// `error: Some(...)`. Caller decides whether to surface the report
/// to the operator (e.g. via JSON envelope).
pub fn fire(
    outcome: &PublishOutcome,
    source_dir: &Path,
    registry: &str,
) -> HookReport {
    let Some(config) = HookConfig::from_env(registry) else {
        return HookReport::dormant();
    };
    let endpoint = format!("{}/v1/packages", config.index_url.trim_end_matches('/'));
    let manifest = match Manifest::read(source_dir.join(Manifest::FILENAME)) {
        Ok(m) => m,
        Err(e) => {
            warn!(target: "vibe_publish::post_hook", error = %e, "skipping index hook: manifest unreadable");
            return HookReport {
                command: "registry:publish:index-hook",
                fired: false,
                url_endpoint: Some(endpoint),
                status: None,
                error: Some(format!("manifest read: {e}")),
            };
        }
    };
    let payload = match build_payload(outcome, &manifest, source_dir, registry, Utc::now()) {
        Ok(p) => p,
        Err(e) => {
            warn!(target: "vibe_publish::post_hook", error = %e, "skipping index hook: payload build failed");
            return HookReport {
                command: "registry:publish:index-hook",
                fired: false,
                url_endpoint: Some(endpoint),
                status: None,
                error: Some(format!("payload: {e}")),
            };
        }
    };
    match post_to_index(&config, &payload) {
        Ok(status) => HookReport {
            command: "registry:publish:index-hook",
            fired: true,
            url_endpoint: Some(endpoint),
            status: Some(status),
            error: None,
        },
        Err(e) => {
            warn!(target: "vibe_publish::post_hook", error = %e, "index hook POST failed (publish itself succeeded)");
            HookReport {
                command: "registry:publish:index-hook",
                fired: false,
                url_endpoint: Some(endpoint),
                status: None,
                error: Some(format!("post: {e}")),
            }
        }
    }
}

/// Re-exported for downstream consumers (vibe-index server) that
/// want to deserialise the hook payload via a typed wrapper. Slice
/// 9 ships only the JSON producer; the typed shape lives in
/// vibe-index, not here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEnvelope {
    pub kind: String,
    pub name: String,
    pub version: String,
    pub registry: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_env_suffix_uppercases_and_substitutes() {
        assert_eq!(registry_env_suffix("vibespecs"), "VIBESPECS");
        assert_eq!(
            registry_env_suffix("vibespecs-gitverse"),
            "VIBESPECS_GITVERSE"
        );
        assert_eq!(registry_env_suffix("foo.bar"), "FOO_BAR");
    }

    #[test]
    fn fire_returns_dormant_when_unconfigured() {
        // No env vars touched — relying on the test environment not
        // having `VIBEVM_INDEX_URL_NOENVCONFIGURED` set. The
        // unsafe-block-needing env-mutation tests live in the
        // integration test crate (tests/post_hook.rs), where the
        // forbid(unsafe_code) lib invariant does not apply.
        let outcome = PublishOutcome {
            kind: vibe_core::PackageKind::Flow,
            name: "test".into(),
            version: "0.1.0".parse().unwrap(),
            repo_name: "flow-test".into(),
            repo_url: "https://example/foo.git".into(),
            tag: "v0.1.0".into(),
            created_repo: true,
            host: "example.invalid".into(),
            dry_run: false,
        };
        let report = fire(
            &outcome,
            std::path::Path::new("/nonexistent"),
            "noenvconfigured",
        );
        assert!(!report.fired);
        assert_eq!(report.error, None);
    }
}
