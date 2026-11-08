//! The `vibe search` report rows.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct SearchReport {
    pub(super) ok: bool,
    pub(super) command: &'static str,
    pub(super) project: String,
    pub(super) query: String,
    pub(super) registries_searched: Vec<String>,
    pub(super) registries_unconfigured: Vec<String>,
    pub(super) registries_unreachable: Vec<UnreachableRegistry>,
    pub(super) registries_policy_skipped: Vec<PolicySkippedRegistry>,
    /// Registries scanned via `--full-scan` (org-walk fallback).
    /// Always empty when `--full-scan` is off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) registries_full_scanned: Vec<String>,
    /// Registries `--full-scan` could not handle (non-GitHub host).
    /// Always empty when `--full-scan` is off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) registries_full_scan_unsupported: Vec<UnreachableRegistry>,
    pub(super) hit_count: usize,
    pub(super) hits: Vec<HitRow>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct UnreachableRegistry {
    pub(super) name: String,
    pub(super) reason: String,
}

/// A contact refusal says nothing about whether the package exists remotely.
#[derive(Debug, Clone, Serialize)]
pub(super) struct PolicySkippedRegistry {
    pub(super) name: String,
    pub(super) observation: &'static str,
    pub(super) reason: String,
}
impl PolicySkippedRegistry {
    pub(super) fn refused(name: &str, refusal: &vibe_registry::ContactRefusal) -> Self {
        Self {
            name: name.into(),
            observation: "unknown",
            reason: refusal.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub(super) struct HitRow {
    pub(super) kind: String,
    pub(super) name: String,
    pub(super) latest_stable: Option<String>,
    pub(super) score: u32,
    pub(super) matched_tokens: Vec<String>,
    pub(super) description: Option<String>,
    pub(super) registry: String,
    /// `"index"` (served from the registry's `vibe-index` server) or
    /// `"full-scan"` (built by walking the org's repos directly).
    /// Lets a CI consumer tell whether the search ran against a
    /// curated index or the rate-limited fallback.
    pub(super) source: &'static str,
}
