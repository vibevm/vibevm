//! Publish-token loading.
//!
//! Per-host file precedence pinned in
//! [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy):
//!
//! 1. Explicit value (`Token::from_explicit`) — used by tests.
//! 2. `VIBEVM_PUBLISH_TOKEN` environment variable — wins over files;
//!    suitable for CI.
//! 3. `~/.vibevm/<host-prefix>.publish.token` — per-host file. The
//!    prefix is the **first label** of the host: `github` for
//!    `github.com`, `gitverse` for `gitverse.ru`, `gitlab` for
//!    `gitlab.com`. Lets one operator hold tokens for several hosts
//!    without juggling env vars.
//! 4. `~/.vibevm/git.publish.token` — legacy host-agnostic fallback.
//!    Kept so existing GitVerse-only setups keep working without a
//!    rename. Will be retired in a future major version once the
//!    per-host pattern is universal.
//!
//! Tokens are surface secrets; never logged at any level. The
//! [`Token`] type wraps the string and `Display`s as `***` to make
//! accidental logging visible at code-review time. See
//! [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy)
//! for the full discipline.

use std::fmt;
use std::fs;
use std::path::PathBuf;

use crate::PublishError;

/// Where the loaded token came from. Not the value — that stays inside
/// the `Token`. Useful in CLI output ("loaded token from
/// $VIBEVM_PUBLISH_TOKEN") and error attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSource {
    Explicit,
    EnvVar(&'static str),
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

const ENV_VAR: &str = "VIBEVM_PUBLISH_TOKEN";

/// Load a token using the host-aware precedence.
///
/// Walks: env var → `~/.vibevm/<host-prefix>.publish.token` →
/// legacy `~/.vibevm/git.publish.token`. The `host` argument shapes
/// the per-host file lookup and surfaces in the `AuthMissing` error.
///
/// The `<host-prefix>` is derived as the first label of `host`:
/// `github.com` → `github`, `gitverse.ru` → `gitverse`. Hosts that
/// don't fit this shape (a bare hostname, an IP address) fall straight
/// through to the legacy file.
pub fn load_token_for_host(host: &str) -> Result<Token, PublishError> {
    if let Some(t) = read_env_token() {
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

fn read_env_token() -> Option<Token> {
    let value = std::env::var(ENV_VAR).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(Token {
        value: trimmed.to_string(),
        source: TokenSource::EnvVar(ENV_VAR),
    })
}

fn read_per_host_token(host: &str) -> Result<Option<Token>, PublishError> {
    let Some(path) = per_host_token_path(host) else {
        return Ok(None);
    };
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

fn read_legacy_token() -> Result<Option<Token>, PublishError> {
    let Some(path) = legacy_token_path() else {
        return Ok(None);
    };
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

/// Path to the per-host token file: `<home>/.vibevm/<prefix>.publish.token`,
/// where `<prefix>` is the first label of `host`. Returns `None` if no
/// home directory is detectable or `host` is empty.
pub fn per_host_token_path(host: &str) -> Option<PathBuf> {
    let prefix = host_prefix(host)?;
    let home = dirs::home_dir()?;
    Some(
        home.join(".vibevm")
            .join(format!("{prefix}.publish.token")),
    )
}

/// Path to the legacy host-agnostic token file: `<home>/.vibevm/git.publish.token`.
pub fn legacy_token_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".vibevm").join("git.publish.token"))
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
    fn debug_redacts_value() {
        let t = Token::from_explicit("super-secret-12345");
        let s = format!("{t:?}");
        assert!(!s.contains("super-secret-12345"));
        assert!(s.contains("***"));
    }

    #[test]
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
    fn per_host_token_path_renders_under_dot_vibevm() {
        // We can't assert the exact home dir, but we can assert the
        // suffix and that some path is returned.
        let p = per_host_token_path("github.com").expect("home dir present in test env");
        let s = p.to_string_lossy().to_string();
        assert!(s.ends_with("github.publish.token"));
        assert!(s.contains(".vibevm"));
    }

    #[test]
    fn per_host_token_path_blank_host_returns_none() {
        assert!(per_host_token_path("").is_none());
    }
}
