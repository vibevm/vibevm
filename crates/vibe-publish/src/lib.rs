//! `vibe registry publish <path>` — maintainer-side per-package publishing.
//!
//! Layered design per [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish):
//!
//! - [`RepoCreator`] — host-specific trait for "create a repo in this
//!   org, check whether one exists, produce the URL to push to". Two
//!   impls today: [`GitHubCreator`] (primary, drives the `vibespecs`
//!   org migration); [`GitVerseCreator`] (retained for any future
//!   Gitea-shape host that fully supports the org-scoped POST). New
//!   adapters land as one new `impl RepoCreator` per host.
//! - [`Publisher`] — host-agnostic orchestrator. Reads manifest,
//!   coordinates with [`RepoCreator`] for repo presence + creation,
//!   shells out to `git` for the working-tree → push → tag flow,
//!   classifies errors per the surface in PROP-002.
//! - [`Token`] — token loading per [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy).
//!   Per-host file precedence: `VIBEVM_PUBLISH_TOKEN` env →
//!   `~/.vibevm/<host-prefix>.publish.token` → legacy
//!   `~/.vibevm/git.publish.token`. Token never logged, never
//!   persisted, never leaks out of process.
//!
//! Consuming code (the CLI command) instantiates a `RepoCreator`,
//! constructs a `Publisher`, calls `Publisher::publish`, and renders
//! the [`PublishOutcome`] to the user. Tests use a mock `RepoCreator`
//! to drive every branch without hitting the network.

#![forbid(unsafe_code)]
specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#publish");

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;

mod creator;
pub mod direct_git;
pub mod git_publish;
pub mod github;
pub mod gitverse;
mod orchestrator;
pub mod post_hook;
pub mod redirect_sync;
pub mod token;

pub use creator::{CreateOpts, RepoCreator, RepoInfo};
pub use direct_git::DirectGitCreator;
pub use github::GitHubCreator;
pub use gitverse::GitVerseCreator;
pub use orchestrator::{PublishConfig, PublishOutcome, Publisher};
pub use post_hook::{HookConfig, HookError, HookReport, fire as fire_index_hook};
pub use token::{Token, TokenSource, host_env_var, load_token, load_token_for_host};

/// Pull the org segment out of an org URL.
///
/// - `git@gitverse.ru:vibespecs` → `vibespecs`
/// - `git@gitverse.ru:vibespecs/` → `vibespecs`
/// - `https://gitverse.ru/vibespecs` → `vibespecs`
/// - `https://github.com/vibespecs` → `vibespecs`
/// - `ssh://git@gitverse.ru/vibespecs` → `vibespecs`
/// - `git+https://...` → strips the `git+` first
///
/// ```
/// use vibe_publish::extract_org_segment;
///
/// assert_eq!(
///     extract_org_segment("https://github.com/vibespecs").unwrap(),
///     "vibespecs",
/// );
/// assert_eq!(
///     extract_org_segment("git@gitverse.ru:vibespecs").unwrap(),
///     "vibespecs",
/// );
/// ```
pub fn extract_org_segment(org_url: &str) -> Result<String, PublishError> {
    let url = org_url.trim().trim_end_matches('/');
    let url = url.strip_prefix("git+").unwrap_or(url);
    // ssh shorthand `user@host:path`
    if let Some((_, rest)) = url.split_once(':')
        && !url.contains("://")
    {
        return Ok(rest.trim_end_matches('/').to_string());
    }
    if let Some((_, rest)) = url.split_once("://") {
        // schemes://host/<path...>
        if let Some(slash) = rest.find('/') {
            return Ok(rest[slash + 1..].trim_end_matches('/').to_string());
        }
    }
    Err(PublishError::OrgUrlInvalid {
        url: org_url.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Errors — surface tuned for non-admin contributors per PROP-002 §2.10.
// ---------------------------------------------------------------------------

/// Publish failure surface, tuned for non-admin contributors per
/// [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish)
/// — every refusal names the violated expectation and the fix surface:
///
/// ```
/// use vibe_publish::PublishError;
///
/// let err = PublishError::TagCollision {
///     repo: "vibespecs/org.vibevm_wal".to_string(),
///     tag: "v0.1.0".to_string(),
/// };
/// let rendered = err.to_string();
/// assert!(rendered.contains("tag `v0.1.0` already exists"));
/// assert!(rendered.contains("does not force-push tags"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#publish")]
pub enum PublishError {
    #[error(
        "publish refused: source directory `{path}` does not look like a vibevm package — \
         {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: point the command at a directory whose `vibe.toml` carries a `[package]` section)"
    )]
    SourceInvalid { path: PathBuf, reason: String },

    #[error(
        "publish refused: cannot derive an organization segment from `{url}`. \
         Configure `[[registry]].url` to a value `git` accepts (e.g. `git@gitverse.ru:vibespecs`). \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: include the organization segment after the host in `[[registry]].url`)"
    )]
    OrgUrlInvalid { url: String },

    #[error(
        "publish refused: token lacks `repo:create` permission in organization `{org}` on `{host}`. \
         Contact an org owner, or use a token whose scope includes repository creation. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: mint a token with `repo:create` scope for `{org}` or have an org owner elevate it)"
    )]
    AuthForbidden { host: String, org: String },

    #[error(
        "publish refused: no token available for host `{host}`. \
         Set `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibevm/git.publish.token`. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: export `VIBEVM_PUBLISH_TOKEN` or write `~/.vibevm/<host-prefix>.publish.token`)"
    )]
    AuthMissing { host: String },

    #[error(
        "publish refused: organization `{org}` does not exist on `{host}` \
         (or the token cannot see it). Check spelling — different from \
         a permissions error. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: correct the org segment in `[[registry]].url` or use a token that can see `{org}`)"
    )]
    OrgNotFound { host: String, org: String },

    #[error(
        "publish refused: tag `{tag}` already exists on `{repo}`. \
         Pick a new version — `vibe registry publish` does not force-push tags. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: bump `[package].version` in `vibe.toml` and publish again)"
    )]
    TagCollision { repo: String, tag: String },

    #[error(
        "publish refused: no push access to `{repo}`. Ask a maintainer of \
         that repo to grant you push, or use a different registry. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: request push access on `{repo}` or point `[[registry]].url` at a registry \
         you can write to)"
    )]
    PushDenied { repo: String },

    #[error(
        "publish refused: host `{host}` is unreachable (network or DNS error). \
         Check connectivity and try again. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: restore network/DNS reachability of `{host}`, then re-run the publish)"
    )]
    HostUnreachable { host: String },

    #[error(
        "git operation failed during publish: {0} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: resolve the reported git failure, then re-run `vibe registry publish`)"
    )]
    Git(String),

    #[error(
        "HTTP request to `{host}` failed: {message} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: check connectivity to `{host}` and retry)"
    )]
    HttpFailed { host: String, message: String },

    #[error(
        "unexpected response from `{host}` (status {status}): {body} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: retry the publish — if the status persists, the host API shape changed and \
         the adapter needs updating)"
    )]
    UnexpectedResponse {
        host: String,
        status: u16,
        body: String,
    },

    #[error(
        "filesystem error during publish at `{path}`: {message} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: check permissions and free space at `{path}`, then re-run)"
    )]
    Io { path: PathBuf, message: String },

    #[error(
        "publish refused: scope violation — adapter for `{host}` is scoped to organization \
         `{expected_org}` but the request targeted `{attempted_org}`. The publish utility never \
         operates outside the organization named in `[[registry]].url`. \
         See spec://vibevm/common/PROP-000#token-secrecy."
    )]
    ScopeViolation {
        host: String,
        expected_org: String,
        attempted_org: String,
    },

    #[error(
        "publish refused: no `RepoCreator` adapter for host `{host}`. Configured registry URL \
         points at an unsupported host; add an adapter per PROP-002 §2.10 or use a supported one \
         (today: `github.com`, `gitverse.ru`). \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: point `[[registry]].url` at `github.com` or `gitverse.ru`, or add a \
         `RepoCreator` impl for `{host}`)"
    )]
    UnsupportedHost { host: String },
}

/// Pull the host segment out of an org URL.
///
/// - `git@github.com:vibespecs` → `github.com`
/// - `https://github.com/vibespecs` → `github.com`
/// - `ssh://git@github.com/vibespecs` → `github.com`
/// - `git+https://github.com/vibespecs` → `github.com` (strips `git+` first)
///
/// ```
/// use vibe_publish::extract_host_segment;
///
/// assert_eq!(
///     extract_host_segment("git@github.com:vibespecs").unwrap(),
///     "github.com",
/// );
/// assert_eq!(
///     extract_host_segment("https://gitverse.ru/vibespecs").unwrap(),
///     "gitverse.ru",
/// );
/// ```
pub fn extract_host_segment(org_url: &str) -> Result<String, PublishError> {
    let url = org_url.trim().trim_end_matches('/');
    let url = url.strip_prefix("git+").unwrap_or(url);
    if let Some((before_colon, _)) = url.split_once(':')
        && !url.contains("://")
    {
        // ssh shorthand `user@host:path`
        if let Some((_, host)) = before_colon.split_once('@') {
            if !host.is_empty() {
                return Ok(host.to_string());
            }
        } else if !before_colon.is_empty() {
            return Ok(before_colon.to_string());
        }
    }
    if let Some((_, rest)) = url.split_once("://") {
        let after_at = rest.split_once('@').map(|(_, r)| r).unwrap_or(rest);
        if let Some((host, _)) = after_at.split_once('/') {
            if !host.is_empty() {
                return Ok(host.to_string());
            }
        } else if !after_at.is_empty() {
            return Ok(after_at.to_string());
        }
    }
    Err(PublishError::OrgUrlInvalid {
        url: org_url.to_string(),
    })
}

/// Construct the right [`RepoCreator`] for a given registry URL.
/// Selects the adapter from the URL's host segment per PROP-002 §2.10.
/// Tokens are loaded by the caller and passed in — this function does
/// not touch token storage.
///
/// `expected_org` is the organization segment the adapter will be
/// scoped to (extracted from the same registry URL by the caller via
/// [`extract_org_segment`]). Adapters refuse operations against any
/// other org per [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy).
///
/// ```
/// use vibe_publish::{RepoCreator, Token, creator_for_url, extract_org_segment};
///
/// let org_url = "https://github.com/vibespecs";
/// let org = extract_org_segment(org_url).unwrap();
/// let token = Token::from_explicit("test-token-please-redact");
/// let creator = creator_for_url(org_url, org, token).unwrap();
/// assert_eq!(creator.host_name(), "github.com");
/// assert_eq!(creator.expected_org(), Some("vibespecs"));
/// ```
pub fn creator_for_url(
    org_url: &str,
    expected_org: impl Into<String>,
    token: Token,
) -> Result<Box<dyn RepoCreator>, PublishError> {
    let host = extract_host_segment(org_url)?;
    let expected_org = expected_org.into();
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "github.com" || host_lower.ends_with(".github.com") {
        let creator = GitHubCreator::new(token, expected_org)?;
        return Ok(Box::new(creator));
    }
    if host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru") {
        let creator = GitVerseCreator::new(token, expected_org)?;
        return Ok(Box::new(creator));
    }
    Err(PublishError::UnsupportedHost { host })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_org_segment_ssh_shorthand() {
        assert_eq!(
            extract_org_segment("git@gitverse.ru:vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("git@gitverse.ru:vibespecs/").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_org_segment_https() {
        assert_eq!(
            extract_org_segment("https://gitverse.ru/vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("https://gitverse.ru/vibespecs/").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_org_segment_ssh_scheme() {
        assert_eq!(
            extract_org_segment("ssh://git@gitverse.ru/vibespecs").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_org_segment_strips_git_plus() {
        assert_eq!(
            extract_org_segment("git+https://gitverse.ru/vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("git+ssh://git@gitverse.ru/vibespecs").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_org_segment_rejects_bare_host() {
        assert!(extract_org_segment("git@gitverse.ru").is_err());
        assert!(extract_org_segment("https://gitverse.ru").is_err());
    }

    #[test]
    fn extract_org_segment_github_shapes() {
        assert_eq!(
            extract_org_segment("https://github.com/vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("git@github.com:vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("ssh://git@github.com/vibespecs").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_host_segment_ssh_shorthand() {
        assert_eq!(
            extract_host_segment("git@github.com:vibespecs").unwrap(),
            "github.com"
        );
        assert_eq!(
            extract_host_segment("git@gitverse.ru:vibespecs").unwrap(),
            "gitverse.ru"
        );
    }

    #[test]
    fn extract_host_segment_https() {
        assert_eq!(
            extract_host_segment("https://github.com/vibespecs").unwrap(),
            "github.com"
        );
        assert_eq!(
            extract_host_segment("https://gitverse.ru/vibespecs").unwrap(),
            "gitverse.ru"
        );
    }

    #[test]
    fn extract_host_segment_ssh_scheme() {
        assert_eq!(
            extract_host_segment("ssh://git@github.com/vibespecs").unwrap(),
            "github.com"
        );
    }

    #[test]
    fn extract_host_segment_strips_git_plus() {
        assert_eq!(
            extract_host_segment("git+https://github.com/vibespecs").unwrap(),
            "github.com"
        );
    }

    #[test]
    fn creator_for_url_picks_github() {
        let token = Token::from_explicit("test-token-please-redact");
        let creator = creator_for_url("https://github.com/vibespecs", "vibespecs", token).unwrap();
        assert_eq!(creator.host_name(), "github.com");
        assert_eq!(creator.expected_org(), Some("vibespecs"));
    }

    #[test]
    fn creator_for_url_picks_gitverse() {
        let token = Token::from_explicit("test-token-please-redact");
        let creator = creator_for_url("git@gitverse.ru:vibespecs", "vibespecs", token).unwrap();
        assert_eq!(creator.host_name(), "gitverse.ru");
        assert_eq!(creator.expected_org(), Some("vibespecs"));
    }

    #[test]
    fn creator_for_url_rejects_unknown_host() {
        let token = Token::from_explicit("test-token-please-redact");
        match creator_for_url("https://example.invalid/whatever", "whatever", token) {
            Ok(_) => panic!("expected unsupported-host error"),
            Err(PublishError::UnsupportedHost { host }) => assert_eq!(host, "example.invalid"),
            Err(other) => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn validate_scope_refuses_other_org() {
        let token = Token::from_explicit("test-token-please-redact");
        let creator = GitHubCreator::new(token, "vibespecs").unwrap();
        let err = creator
            .validate_scope("not-vibespecs")
            .expect_err("scope guard should fire");
        match err {
            PublishError::ScopeViolation {
                expected_org,
                attempted_org,
                ..
            } => {
                assert_eq!(expected_org, "vibespecs");
                assert_eq!(attempted_org, "not-vibespecs");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
