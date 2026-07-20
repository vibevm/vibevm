//! Publish-token loading.
//!
//! Host-aware precedence pinned in
//! [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy):
//!
//! 1. Explicit value (`Token::from_explicit`) — used by tests.
//! 2. `VIBEVM_PUBLISH_TOKEN_<HOST>` environment variable — host-specific
//!    (`VIBEVM_PUBLISH_TOKEN_GITHUB` for `github.com`,
//!    `VIBEVM_PUBLISH_TOKEN_GITVERSE` for `gitverse.ru`, …). Lets CI
//!    hold tokens for several hosts in the same env without a single
//!    `VIBEVM_PUBLISH_TOKEN` clobbering them all.
//! 3. `VIBEVM_PUBLISH_TOKEN` — legacy host-agnostic env var. Kept so
//!    setups that already exported it keep working without a rename;
//!    the host-specific form should be preferred in new setups.
//! 4. `<settings-dir>/<host-prefix>.publish.token` — per-host file in the
//!    canonical settings dir (`~/.vibe`, or `$VIBE_SETTINGS`). The prefix
//!    is the **first label** of the host: `github` for `github.com`,
//!    `gitverse` for `gitverse.ru`, `gitlab` for `gitlab.com`. Lets one
//!    operator hold tokens for several hosts without juggling env vars.
//! 5. `~/.vibevm/<host-prefix>.publish.token` — the same per-host file in
//!    the pre-consolidation dir; a read-only migration fallback.
//! 6. `<settings-dir>/git.publish.token` — legacy host-agnostic file.
//!    Kept so existing GitVerse-only setups keep working without a
//!    rename. Will be retired in a future major version once the
//!    per-host pattern is universal.
//! 7. `~/.vibevm/git.publish.token` — the same host-agnostic file in the
//!    pre-consolidation dir; a read-only migration fallback.
//!
//! Tokens are surface secrets; never logged at any level. The
//! [`Token`] type wraps the string and `Display`s as `***` to make
//! accidental logging visible at code-review time. See
//! [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy)
//! for the full discipline.

specmark::scope!("spec://vibevm/common/PROP-000#token-secrecy");

use std::fmt;
use std::fs;
use std::path::PathBuf;

use crate::PublishError;

/// Where the loaded token came from. Not the value — that stays inside
/// the `Token`. Useful in CLI output ("loaded token from
/// $VIBEVM_PUBLISH_TOKEN_GITHUB") and error attribution. The variable
/// name is owned because the host-specific form is computed at runtime
/// from the host segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSource {
    Explicit,
    EnvVar(String),
    File(PathBuf),
}

/// Wraps a publish token string so it never accidentally lands in a log.
#[derive(Clone)]
pub struct Token {
    value: String,
    source: TokenSource,
}

impl Token {
    pub fn from_explicit(value: impl Into<String>) -> Self {
        Token {
            value: value.into(),
            source: TokenSource::Explicit,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn source(&self) -> &TokenSource {
        &self.source
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("value", &"***")
            .field("source", &self.source)
            .finish()
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

/// Legacy host-agnostic env var. Kept as a fallback for setups that
/// already exported it; the host-specific form via [`host_env_var`]
/// is preferred for new configuration.
pub const LEGACY_ENV_VAR: &str = "VIBEVM_PUBLISH_TOKEN";

/// Backwards-compatible alias. Prefer [`LEGACY_ENV_VAR`] in new code.
pub const ENV_VAR: &str = LEGACY_ENV_VAR;

/// Compute the host-specific env var name for `host` (e.g.
/// `github.com` → `VIBEVM_PUBLISH_TOKEN_GITHUB`,
/// `gitverse.ru` → `VIBEVM_PUBLISH_TOKEN_GITVERSE`). Returns `None`
/// for a host whose first-label prefix can't be derived.
pub fn host_env_var(host: &str) -> Option<String> {
    let prefix = host_prefix(host)?;
    let upper: String = prefix
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    if upper.is_empty() {
        return None;
    }
    Some(format!("{LEGACY_ENV_VAR}_{upper}"))
}

/// Load a token using the host-aware precedence.
///
/// Walks (in order):
/// 1. `VIBEVM_PUBLISH_TOKEN_<HOST>` env var (host-specific).
/// 2. `VIBEVM_PUBLISH_TOKEN` env var (legacy host-agnostic).
/// 3. `<settings-dir>/<host-prefix>.publish.token` file (host-specific),
///    then the same file under the legacy `~/.vibevm` dir.
/// 4. `<settings-dir>/git.publish.token` file (legacy host-agnostic),
///    then the same file under the legacy `~/.vibevm` dir.
///
/// The `host` argument shapes the per-host lookup and surfaces in the
/// `AuthMissing` error. The `<host-prefix>` is derived as the first
/// label of `host`: `github.com` → `github`, `gitverse.ru` →
/// `gitverse`. Hosts that don't fit this shape (a bare hostname, an
/// IP address) skip the host-specific layers and fall straight through
/// to the legacy ones.
pub fn load_token_for_host(host: &str) -> Result<Token, PublishError> {
    if let Some(t) = read_host_env_token(host) {
        return Ok(t);
    }

    if let Some(t) = read_legacy_env_token() {
        return Ok(t);
    }

    if let Some(t) = read_per_host_token(host)? {
        return Ok(t);
    }

    if let Some(t) = read_legacy_token()? {
        return Ok(t);
    }

    Err(PublishError::AuthMissing {
        host: host.to_string(),
    })
}

/// Backward-compatible wrapper. Behaves identically to
/// [`load_token_for_host`] — kept so call sites that already exist
/// don't need to know about the per-host plumbing immediately.
pub fn load_token(host: &str) -> Result<Token, PublishError> {
    load_token_for_host(host)
}

fn read_host_env_token(host: &str) -> Option<Token> {
    let name = host_env_var(host)?;
    let value = std::env::var(&name).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(Token {
        value: trimmed.to_string(),
        source: TokenSource::EnvVar(name),
    })
}

fn read_legacy_env_token() -> Option<Token> {
    let value = std::env::var(LEGACY_ENV_VAR).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(Token {
        value: trimmed.to_string(),
        source: TokenSource::EnvVar(LEGACY_ENV_VAR.to_string()),
    })
}

/// Read a token from `path` if it exists and is non-empty. Missing file
/// or blank content is `Ok(None)` — the caller falls through to the next
/// candidate path; an I/O error surfaces.
fn read_token_file(path: PathBuf) -> Result<Option<Token>, PublishError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|e| PublishError::Io {
        path: path.clone(),
        message: format!("reading token: {e}"),
    })?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(Token {
        value: trimmed.to_string(),
        source: TokenSource::File(path),
    }))
}

/// Per-host token file: the canonical settings dir first, then the legacy
/// `~/.vibevm` dir as a read-only migration fallback.
fn read_per_host_token(host: &str) -> Result<Option<Token>, PublishError> {
    for path in [
        per_host_token_path(host),
        dot_vibevm_per_host_token_path(host),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(t) = read_token_file(path)? {
            return Ok(Some(t));
        }
    }
    Ok(None)
}

/// Host-agnostic legacy-format token file: the canonical settings dir
/// first, then the legacy `~/.vibevm` dir as a read-only migration
/// fallback.
fn read_legacy_token() -> Result<Option<Token>, PublishError> {
    for path in [legacy_token_path(), dot_vibevm_token_path()]
        .into_iter()
        .flatten()
    {
        if let Some(t) = read_token_file(path)? {
            return Ok(Some(t));
        }
    }
    Ok(None)
}

/// Path to the per-host token file `<settings-dir>/<prefix>.publish.token`
/// (canonical `~/.vibe`, or `$VIBE_SETTINGS`), where `<prefix>` is the
/// first label of `host`. Returns `None` if no home directory is
/// detectable or `host` is empty.
pub fn per_host_token_path(host: &str) -> Option<PathBuf> {
    let prefix = host_prefix(host)?;
    Some(vibe_core::settings::settings_dir()?.join(format!("{prefix}.publish.token")))
}

/// The per-host token file under the pre-consolidation `~/.vibevm` dir —
/// a read-only migration fallback for [`per_host_token_path`].
fn dot_vibevm_per_host_token_path(host: &str) -> Option<PathBuf> {
    let prefix = host_prefix(host)?;
    Some(vibe_core::settings::legacy_settings_dir()?.join(format!("{prefix}.publish.token")))
}

/// Path to the legacy host-agnostic token file
/// `<settings-dir>/git.publish.token` (canonical `~/.vibe`).
pub fn legacy_token_path() -> Option<PathBuf> {
    Some(vibe_core::settings::settings_dir()?.join("git.publish.token"))
}

/// The host-agnostic token file under the pre-consolidation `~/.vibevm`
/// dir — a read-only migration fallback for [`legacy_token_path`].
fn dot_vibevm_token_path() -> Option<PathBuf> {
    Some(vibe_core::settings::legacy_settings_dir()?.join("git.publish.token"))
}

/// Backwards-compatible alias. Prefer [`legacy_token_path`] in new code.
pub fn default_token_path() -> Option<PathBuf> {
    legacy_token_path()
}

fn host_prefix(host: &str) -> Option<String> {
    let host = host.trim().trim_end_matches('.');
    if host.is_empty() {
        return None;
    }
    let first = host.split('.').next()?.trim();
    if first.is_empty() {
        return None;
    }
    Some(first.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_value_lands() {
        let t = Token::from_explicit("abc123");
        assert_eq!(t.value(), "abc123");
        assert!(matches!(t.source(), TokenSource::Explicit));
    }

    #[test]
    #[specmark::verifies("spec://vibevm/common/PROP-000#token-secrecy")]
    fn debug_redacts_value() {
        let t = Token::from_explicit("super-secret-12345");
        let s = format!("{t:?}");
        assert!(!s.contains("super-secret-12345"));
        assert!(s.contains("***"));
    }

    #[test]
    #[specmark::verifies("spec://vibevm/common/PROP-000#token-secrecy")]
    fn display_redacts_value() {
        let t = Token::from_explicit("super-secret-12345");
        let s = format!("{t}");
        assert!(!s.contains("super-secret-12345"));
        assert_eq!(s, "***");
    }

    // Tests that mutate process-wide environment variables would need
    // `unsafe` (Rust 2024 marks `std::env::set_var` / `remove_var` as
    // unsafe due to global-state cross-thread hazards), and this crate
    // has `#![forbid(unsafe_code)]`. Skip env-mutation tests; the
    // construction path is exercised by the explicit-value test plus
    // the redaction tests above. Live env / file behaviour gets a
    // smoke-test pass during real publish runs.

    #[test]
    fn host_env_var_renders_per_host() {
        assert_eq!(
            host_env_var("github.com").as_deref(),
            Some("VIBEVM_PUBLISH_TOKEN_GITHUB")
        );
        assert_eq!(
            host_env_var("gitverse.ru").as_deref(),
            Some("VIBEVM_PUBLISH_TOKEN_GITVERSE")
        );
        assert_eq!(
            host_env_var("gitlab.com").as_deref(),
            Some("VIBEVM_PUBLISH_TOKEN_GITLAB")
        );
    }

    #[test]
    fn host_env_var_uppercases_and_sanitises_prefix() {
        // Mixed case input → uppercase env var.
        assert_eq!(
            host_env_var("GitHub.COM").as_deref(),
            Some("VIBEVM_PUBLISH_TOKEN_GITHUB")
        );
        // Hyphens / dots / underscores in the first label fold to `_`
        // so the env-var name stays a valid POSIX identifier — the
        // shell barfs on `VIBEVM_PUBLISH_TOKEN_some-host` otherwise.
        assert_eq!(
            host_env_var("some-host").as_deref(),
            Some("VIBEVM_PUBLISH_TOKEN_SOME_HOST")
        );
    }

    #[test]
    fn host_env_var_handles_blank_input() {
        assert_eq!(host_env_var(""), None);
        assert_eq!(host_env_var("   "), None);
    }

    #[test]
    fn host_prefix_strips_to_first_label() {
        assert_eq!(host_prefix("github.com"), Some("github".to_string()));
        assert_eq!(host_prefix("gitverse.ru"), Some("gitverse".to_string()));
        assert_eq!(host_prefix("gitlab.com"), Some("gitlab".to_string()));
        assert_eq!(host_prefix("api.github.com"), Some("api".to_string()));
        assert_eq!(host_prefix("bare-host"), Some("bare-host".to_string()));
    }

    #[test]
    fn host_prefix_handles_blank_input() {
        assert_eq!(host_prefix(""), None);
        assert_eq!(host_prefix("   "), None);
    }

    #[test]
    fn host_prefix_lowercases_input() {
        assert_eq!(host_prefix("GitHub.COM"), Some("github".to_string()));
    }

    #[test]
    fn per_host_token_path_renders_under_canonical_settings_dir() {
        // We can't assert the exact home dir, but the file name is fixed
        // and the path now lands in the canonical settings dir (`~/.vibe`),
        // not the legacy `~/.vibevm` — whose reads are a fallback only.
        let p = per_host_token_path("github.com").expect("home dir present in test env");
        let s = p.to_string_lossy().to_string();
        assert!(s.ends_with("github.publish.token"));
        assert!(!s.contains(".vibevm"));
    }

    #[test]
    fn per_host_token_path_blank_host_returns_none() {
        assert!(per_host_token_path("").is_none());
    }
}
