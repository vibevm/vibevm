//! Shared on-disk substrate for the git-backed registries — where
//! registry clones live (`~/.vibe/registries/`), the implicit-pull
//! freshness TTL, and the URL canonicalisation that keys the cache.
//!
//! Lives outside any `Registry`-seam cell file on purpose: both git
//! cells (`GitRegistry`, `GitPackageRegistry`) and the multi-registry
//! resolver consume these helpers, and a cell must not import a
//! sibling cell's module to reach shared cache mechanics (R-002).
//! `git_registry` re-exports the public items, so the historical
//! paths (`vibe_registry::git_registry::default_cache_root`, …)
//! remain valid.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-001#cache-layout");

use std::path::PathBuf;

use crate::RegistryError;

/// Default freshness TTL for an implicit pull: 1 hour.
pub const DEFAULT_FRESHNESS_SECS: u64 = 3600;

/// Return the default cache root for registry clones.
///
/// Honours `VIBE_REGISTRY_CACHE` if set (used by tests and by users
/// who want an explicit out-of-home location), otherwise returns
/// `<settings-dir>/registries/` per `VIBEVM-SPEC.md` §8.3 — resolved
/// through the one `vibe_core::settings` chokepoint so `$VIBE_SETTINGS`
/// moves the cache with the rest of the settings tree.
pub fn default_cache_root() -> Result<PathBuf, RegistryError> {
    if let Some(custom) = std::env::var_os("VIBE_REGISTRY_CACHE") {
        return Ok(PathBuf::from(custom));
    }
    vibe_core::settings::registries_cache_dir().ok_or(RegistryError::NoHomeDir)
}

/// Strip a `git+` transport-wrapper prefix (`git+ssh://`, `git+https://`,
/// `git+file://`, `git+http://`) before handing the URL to git.
///
/// `git+` is a pip / Cargo convention that labels a URL as "this is a
/// git source" in a lockfile or manifest. Native git does not
/// understand the prefix itself, so we peel it off at the backend
/// boundary. Used by `GitRegistry`, `GitPackageRegistry`, and the
/// override path in `MultiRegistryResolver`.
pub(crate) fn strip_git_plus_prefix(url: &str) -> &str {
    url.strip_prefix("git+").unwrap_or(url)
}

/// Normalize a registry URL for hashing and comparison.
///
/// Lowercases the whole string and strips a trailing `.git` plus
/// trailing slashes. This is intentionally lossy but consistent: we
/// only need it to key the on-disk cache directory, not to reconstruct
/// the URL. The full hash is recorded in `meta.toml` for audit.
pub fn normalize_url(url: &str) -> String {
    let t = url.trim().trim_end_matches('/');
    let t = t.strip_suffix(".git").unwrap_or(t);
    t.to_lowercase()
}
