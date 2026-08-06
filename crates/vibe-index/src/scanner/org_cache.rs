//! Org-image cache — `<data-dir>/state/org-cache.json`. Holds the
//! repo list enumerated from a git host plus the validator the host
//! returned with it, so the next `reindex --from-github` can ask
//! "still fresh?" with one conditional request instead of re-walking
//! every page. PROP-005 §2.8 / slice A3.
//!
//! - Р3: lives next to `checkpoint.json` under `<data-dir>/state/`,
//!   written through the same `atomic_write` discipline. The image
//!   belongs to the data directory because it belongs to the org that
//!   directory indexes — not to a global user cache.
//! - Р6: only the `--from-github` path consults it; `--from-clones`
//!   walks a local directory and has nothing to cache.
//! - ПРОВЕРЬ-4: the stored `org` + `api_base` gate reuse — an image
//!   taken for one org or one endpoint never satisfies a query for
//!   another (see [`OrgCache::matches`]).
//!
//! Field justification (УТОЧНИ-3) — every field answers a question the
//! mechanism needs; none is decorative:
//! - `schema_version` — "is this file a format I understand?" lets a
//!   future shape change reject an unreadable cache instead of
//!   mis-parsing it (same reason `checkpoint.json` carries one).
//! - `org` / `api_base` — "is this image for the org AND the endpoint
//!   I am being asked about?" cross-org / cross-host safety (ПРОВЕРЬ-4).
//! - `etag` / `last_modified` — "can I ask the host whether this is
//!   still fresh?" Without either, no conditional request is possible
//!   and the org is re-enumerated (ИЗМЕРЬ-2).
//! - `repos` — the enumerated product this cache exists to avoid
//!   recomputing.
//!
//! No timestamp is stored: the host validator IS the freshness
//! authority, and no requested behaviour keys off age (there is no
//! TTL — Р2 rejected "the org only changes via me" and made the host
//! the arbiter). A timestamp would answer no question the mechanism
//! asks, so it is omitted.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::scanner::from_github::Repo;

const FILENAME: &str = "org-cache.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgCache {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    /// The org this image was enumerated from. A cache taken for one
    /// org must never satisfy a query for another (ПРОВЕРЬ-4).
    pub org: String,
    /// The REST API base the image was enumerated from. The same org
    /// at a different endpoint (GitHub.com vs a self-hosted
    /// Enterprise instance) is a different image (ПРОВЕРЬ-4).
    pub api_base: String,
    /// `ETag` the host returned — sent back as `If-None-Match` on the
    /// next conditional request. `None` ⇒ that validator was not
    /// supplied by the host.
    #[serde(default)]
    pub etag: Option<String>,
    /// `Last-Modified` the host returned — sent back as
    /// `If-Modified-Since`. Secondary to `etag`; `None` if absent.
    #[serde(default)]
    pub last_modified: Option<String>,
    /// The enumerated repos — the expensive product this cache
    /// exists to avoid recomputing.
    pub repos: Vec<Repo>,
}

impl OrgCache {
    /// Does this image belong to the `(org, api_base)` being queried?
    /// A mismatch means the cache was written for a different org or
    /// endpoint and MUST be ignored — re-enumerate and overwrite
    /// (ПРОВЕРЬ-4). `api_base` is compared after trimming a trailing
    /// `/` so `https://api.github.com` and `https://api.github.com/`
    /// are treated as the same endpoint.
    pub fn matches(&self, org: &str, api_base: &str) -> bool {
        self.org == org && trim_slash(&self.api_base) == trim_slash(api_base)
    }

    /// The validator pair, if the host gave us one. `None` when the
    /// host returned neither `ETag` nor `Last-Modified` — meaning no
    /// conditional request is possible and the org must be
    /// re-enumerated (ИЗМЕРЬ-2: missing validator ⇒ re-enumerate,
    /// never "consider fresh").
    pub fn validator(&self) -> Option<Validator> {
        let v = Validator {
            etag: self.etag.clone(),
            last_modified: self.last_modified.clone(),
        };
        if v.has_any() { Some(v) } else { None }
    }
}

/// A conditional-request validator pair — the in-memory handle the
/// `from-github` cell builds from a response's headers (or from a
/// cached image) and sends back on the next request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Validator {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl Validator {
    /// Did the host supply at least one validator? If not, no
    /// conditional request is possible.
    pub fn has_any(&self) -> bool {
        self.etag.is_some() || self.last_modified.is_some()
    }
}

fn default_schema() -> u32 {
    1
}

fn trim_slash(s: &str) -> &str {
    s.trim_end_matches('/')
}

/// `<data-dir>/state/org-cache.json` — next to `checkpoint.json`. The
/// composition root computes this once and hands the path to the
/// from-github cell via `FromGithubOptions::org_cache_path`.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("state").join(FILENAME)
}

/// Load the cache at `cache_path`. `Ok(None)` when the file is absent
/// (first run — ПРОВЕРЬ-6). A malformed file is an error, not a silent
/// miss — the operator learns the on-disk image is corrupt.
pub fn load(cache_path: &Path) -> Result<Option<OrgCache>> {
    let bytes = match std::fs::read(cache_path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(Error::Io {
                path: cache_path.to_path_buf(),
                message: e.to_string(),
            });
        }
    };
    let cache: OrgCache = serde_json::from_slice(&bytes)
        .map_err(|e| Error::Malformed(format!("org-cache.json: {e}")))?;
    Ok(Some(cache))
}

/// Persist the image atomically (tmp + fsync + rename), the same
/// discipline `checkpoint.json` uses (Р3). Creates the parent
/// `state/` directory when missing.
pub fn save(cache_path: &Path, cache: &OrgCache) -> Result<()> {
    if let Some(state_dir) = cache_path.parent() {
        std::fs::create_dir_all(state_dir).map_err(|e| Error::Io {
            path: state_dir.to_path_buf(),
            message: e.to_string(),
        })?;
    }
    let mut bytes = serde_json::to_vec_pretty(cache)
        .map_err(|e| Error::Malformed(format!("could not serialise org cache: {e}")))?;
    bytes.push(b'\n');
    crate::index::persistence::atomic_write(cache_path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_cache(org: &str, api_base: &str) -> OrgCache {
        OrgCache {
            schema_version: 1,
            org: org.into(),
            api_base: api_base.into(),
            etag: Some("\"v1\"".into()),
            last_modified: None,
            repos: vec![],
        }
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempdir().unwrap();
        assert!(load(&path(dir.path())).unwrap().is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let cache = sample_cache("vibespecs", "https://api.github.com");
        save(&path(dir.path()), &cache).unwrap();
        let back = load(&path(dir.path())).unwrap().unwrap();
        assert_eq!(cache, back);
    }

    /// ПРОВЕРЬ-4 — an image for one org does not satisfy another.
    #[test]
    fn matches_rejects_different_org() {
        let cache = sample_cache("vibespecs", "https://api.github.com");
        assert!(cache.matches("vibespecs", "https://api.github.com"));
        assert!(!cache.matches("other-org", "https://api.github.com"));
    }

    /// ПРОВЕРЬ-4 — same org, different endpoint, still a mismatch.
    #[test]
    fn matches_rejects_different_api_base() {
        let cache = sample_cache("vibespecs", "https://api.github.com");
        assert!(!cache.matches("vibespecs", "https://ghe.example.invalid"));
    }

    /// A trailing slash on the endpoint must not defeat the match.
    #[test]
    fn matches_ignores_trailing_slash() {
        let cache = sample_cache("vibespecs", "https://api.github.com/");
        assert!(cache.matches("vibespecs", "https://api.github.com"));
    }

    /// ИЗМЕРЬ-2 — an image with no validator yields `None`, signalling
    /// "cannot ask the host ⇒ re-enumerate".
    #[test]
    fn validator_none_when_host_gave_neither() {
        let cache = OrgCache {
            schema_version: 1,
            org: "vibespecs".into(),
            api_base: "https://api.github.com".into(),
            etag: None,
            last_modified: None,
            repos: vec![],
        };
        assert!(cache.validator().is_none());
    }

    #[test]
    fn validator_some_when_etag_present() {
        let cache = sample_cache("vibespecs", "https://api.github.com");
        assert_eq!(cache.validator().unwrap().etag.as_deref(), Some("\"v1\""));
    }
}
