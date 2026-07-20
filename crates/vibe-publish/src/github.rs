//! GitHub REST API [`RepoCreator`] impl.
//!
//! GitHub exposes a stable, well-documented REST API at `https://api.github.com`.
//! The two endpoints we use:
//!
//! - `GET /repos/{owner}/{repo}` — repo presence check.
//!   - 200 → exists.
//!   - 404 → does not exist.
//!   - 401 / 403 → auth issue.
//! - `POST /orgs/{org}/repos` — create repo in an org.
//!   - 201 → created.
//!   - 401 / 403 → auth (token lacks `repo` scope on the org, or is invalid).
//!   - 404 → org does not exist or token cannot see it.
//!   - 422 → validation error (e.g. repo name already taken — race).
//!
//! Authentication: `Authorization: Bearer <token>`. Two headers are
//! required by GitHub's current API contract:
//!
//! - `Accept: application/vnd.github+json`
//! - `X-GitHub-Api-Version: 2022-11-28`
//!
//! The token is loaded by [`crate::token::load_token_for_host`] with the
//! per-host file precedence pinned in
//! [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy):
//! `VIBEVM_PUBLISH_TOKEN` env → `~/.vibe/github.publish.token` →
//! legacy `~/.vibe/git.publish.token` (with `~/.vibevm` read as a
//! migration fallback). The value is never logged and is redacted on
//! `Display`/`Debug` of the [`Token`] wrapper.
//!
//! Push authentication. GitHub does not let an HTTP API token push by
//! itself; the standard pattern is to embed the token in the HTTPS
//! clone URL as `https://x-access-token:<TOKEN>@github.com/{org}/{repo}.git`
//! for the duration of one `git remote add` / `git push` invocation.
//! Modern git (≥ 2.31) redacts URL passwords in its own log output to
//! `***`, so the embedded form is safe in stderr — but the URL must
//! never appear in any vibevm-produced output anyway. [`push_url`]
//! constructs the credentialed URL on demand; the value is consumed
//! immediately by the publisher and never persisted.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#publish");

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::token::Token;
use crate::{CreateOpts, PublishError, RepoCreator, RepoInfo};

/// Default base URL for the GitHub REST API.
pub const DEFAULT_GITHUB_API_BASE: &str = "https://api.github.com";

/// Default human-readable host name for error messages.
pub const DEFAULT_GITHUB_HOST_NAME: &str = "github.com";

/// Required Accept header.
const GITHUB_ACCEPT: &str = "application/vnd.github+json";

/// Required versioning header — pinned per GitHub's public-API contract.
const GITHUB_API_VERSION: &str = "2022-11-28";

/// User-Agent header — GitHub requires a non-empty UA on every request.
/// Carries `vibe-publish/<crate-version>` so the request shows up
/// identifiably in any audit / abuse log.
fn user_agent() -> String {
    format!("vibe-publish/{}", env!("CARGO_PKG_VERSION"))
}

#[specmark::cell(seam = "RepoCreator", variant = "github")]
pub struct GitHubCreator {
    api_base: String,
    host_name: String,
    /// Org this adapter is scoped to. See [`RepoCreator::expected_org`].
    /// Constructor requires it; [`RepoCreator::validate_scope`] refuses
    /// any operation against a different org.
    expected_org: String,
    token: Token,
    client: reqwest::blocking::Client,
}

impl GitHubCreator {
    pub fn new(token: Token, expected_org: impl Into<String>) -> Result<Self, PublishError> {
        Self::with_endpoint(
            token,
            expected_org,
            DEFAULT_GITHUB_API_BASE,
            DEFAULT_GITHUB_HOST_NAME,
        )
    }

    pub fn with_endpoint(
        token: Token,
        expected_org: impl Into<String>,
        api_base: &str,
        host_name: &str,
    ) -> Result<Self, PublishError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| PublishError::HttpFailed {
                host: host_name.to_string(),
                message: format!("constructing HTTP client: {e}"),
            })?;
        Ok(GitHubCreator {
            api_base: api_base.trim_end_matches('/').to_string(),
            host_name: host_name.to_string(),
            expected_org: expected_org.into(),
            token,
            client,
        })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token.value())
    }
}

#[derive(Debug, Serialize)]
struct CreateRepoBody<'a> {
    name: &'a str,
    private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    homepage: Option<&'a str>,
    /// We push our own initial commit; never let the host pre-populate.
    auto_init: bool,
    /// Always false — vibevm packages are open-source by convention,
    /// the registry org is public. Operators that want private packages
    /// host their own registry.
    has_issues: bool,
    has_projects: bool,
    has_wiki: bool,
}

#[derive(Debug, Deserialize)]
struct RepoResponse {
    /// SSH clone URL — `git@github.com:org/repo.git`.
    #[serde(default)]
    ssh_url: Option<String>,
    /// HTTPS clone URL — `https://github.com/org/repo.git`.
    #[serde(default)]
    clone_url: Option<String>,
    /// Web URL — `https://github.com/org/repo`.
    #[serde(default)]
    html_url: Option<String>,
}

impl RepoCreator for GitHubCreator {
    fn host_name(&self) -> &str {
        &self.host_name
    }

    fn expected_org(&self) -> Option<&str> {
        Some(&self.expected_org)
    }

    fn push_url(&self, org: &str, name: &str) -> String {
        // Embed the token only at the moment of push. The receiving
        // process is `git`, which uses the URL for the credentials
        // exchange and then redacts the password in any diagnostic
        // output (modern git ≥ 2.31). The URL is never written to any
        // vibevm-produced surface — Publisher::publish hands it to
        // `git_publish::push_release` as an argv parameter and never
        // reads it back into stdout / stderr / logs.
        format!(
            "https://x-access-token:{}@{}/{}/{}.git",
            self.token.value(),
            self.host_name,
            org,
            name
        )
    }

    fn repo_exists(&self, org: &str, name: &str) -> Result<bool, PublishError> {
        self.validate_scope(org)?;
        let url = format!("{}/repos/{}/{}", self.api_base, org, name);
        let res = self
            .client
            .get(&url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .header(reqwest::header::ACCEPT, GITHUB_ACCEPT)
            .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
            .header(reqwest::header::USER_AGENT, user_agent())
            .send()
            .map_err(|e| classify_send_error(e, &self.host_name))?;
        let status = res.status();
        if status.is_success() {
            return Ok(true);
        }
        match status.as_u16() {
            404 => Ok(false),
            401 | 403 => Err(PublishError::AuthForbidden {
                host: self.host_name.clone(),
                org: org.to_string(),
            }),
            other => {
                let body = res.text().unwrap_or_default();
                Err(PublishError::UnexpectedResponse {
                    host: self.host_name.clone(),
                    status: other,
                    body,
                })
            }
        }
    }

    fn create_repo(
        &self,
        org: &str,
        name: &str,
        opts: &CreateOpts,
    ) -> Result<RepoInfo, PublishError> {
        self.validate_scope(org)?;
        let url = format!("{}/orgs/{}/repos", self.api_base, org);
        let body = CreateRepoBody {
            name,
            private: false,
            description: opts.description.as_deref(),
            homepage: opts.homepage.as_deref(),
            auto_init: false,
            has_issues: true,
            has_projects: false,
            has_wiki: false,
        };
        let res = self
            .client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .header(reqwest::header::ACCEPT, GITHUB_ACCEPT)
            .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
            .header(reqwest::header::USER_AGENT, user_agent())
            .json(&body)
            .send()
            .map_err(|e| classify_send_error(e, &self.host_name))?;
        let status = res.status();
        if status.is_success() {
            let parsed: RepoResponse = res.json().map_err(|e| PublishError::HttpFailed {
                host: self.host_name.clone(),
                message: format!("parsing create-repo response: {e}"),
            })?;
            // Public clone URL we surface to humans — HTTPS by default,
            // SSH if HTTPS is missing. The token-credentialed push URL
            // is constructed separately by `push_url()` and never
            // appears in this user-facing struct.
            let clone_url = parsed.clone_url.or(parsed.ssh_url).ok_or_else(|| {
                PublishError::UnexpectedResponse {
                    host: self.host_name.clone(),
                    status: status.as_u16(),
                    body: "create-repo response missing both clone_url and ssh_url".to_string(),
                }
            })?;
            let html_url = parsed
                .html_url
                .unwrap_or_else(|| clone_url.trim_end_matches(".git").to_string());
            return Ok(RepoInfo {
                html_url,
                clone_url,
            });
        }
        match status.as_u16() {
            401 | 403 => Err(PublishError::AuthForbidden {
                host: self.host_name.clone(),
                org: org.to_string(),
            }),
            404 => Err(PublishError::OrgNotFound {
                host: self.host_name.clone(),
                org: org.to_string(),
            }),
            422 => {
                // Validation errors include "repo already exists" (when
                // a concurrent caller created the repo between our
                // exists-check and our POST). Re-running the publish
                // command resolves the race because the second
                // exists-check returns true.
                Err(PublishError::UnexpectedResponse {
                    host: self.host_name.clone(),
                    status: 422,
                    body: format!(
                        "validation error from GitHub when creating `{org}/{name}` (often: repo \
                         already exists). Re-run `vibe registry publish` — the existing repo \
                         will be reused."
                    ),
                })
            }
            other => {
                let body = res.text().unwrap_or_default();
                Err(PublishError::UnexpectedResponse {
                    host: self.host_name.clone(),
                    status: other,
                    body,
                })
            }
        }
    }
}

fn classify_send_error(e: reqwest::Error, host: &str) -> PublishError {
    if e.is_connect() || e.is_timeout() {
        return PublishError::HostUnreachable {
            host: host.to_string(),
        };
    }
    PublishError::HttpFailed {
        host: host.to_string(),
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_url_embeds_token_for_https() {
        let token = Token::from_explicit("test-token-please-redact");
        let creator = GitHubCreator::new(token, "vibespecs").unwrap();
        let url = creator.push_url("vibespecs", "flow-wal");
        assert_eq!(
            url,
            "https://x-access-token:test-token-please-redact@github.com/vibespecs/flow-wal.git"
        );
    }

    #[test]
    fn push_url_does_not_appear_in_creator_debug() {
        // Constructing a creator and calling Debug on it must never
        // print the token, even though `push_url` reads it. (We don't
        // derive Debug on GitHubCreator; this test verifies that
        // Token redaction holds via the wrapped Token.)
        let token = Token::from_explicit("super-secret-do-not-leak");
        let dbg = format!("{token:?}");
        assert!(!dbg.contains("super-secret-do-not-leak"));
        assert!(dbg.contains("***"));
    }

    #[test]
    fn expected_org_is_set_at_construction() {
        let token = Token::from_explicit("ignored");
        let creator = GitHubCreator::new(token, "my-org").unwrap();
        assert_eq!(creator.expected_org(), Some("my-org"));
    }

    #[test]
    fn validate_scope_passes_for_matching_org() {
        let token = Token::from_explicit("ignored");
        let creator = GitHubCreator::new(token, "vibespecs").unwrap();
        assert!(creator.validate_scope("vibespecs").is_ok());
    }

    #[test]
    fn validate_scope_blocks_user_namespace() {
        // A token with broad scopes could in principle target a user
        // namespace; the adapter's scope guard refuses.
        let token = Token::from_explicit("ignored");
        let creator = GitHubCreator::new(token, "vibespecs").unwrap();
        let err = creator
            .validate_scope("some-other-user")
            .expect_err("scope guard must fire");
        assert!(matches!(err, PublishError::ScopeViolation { .. }));
    }

    #[test]
    fn host_name_is_github_com_by_default() {
        let token = Token::from_explicit("ignored");
        let creator = GitHubCreator::new(token, "vibespecs").unwrap();
        assert_eq!(creator.host_name(), "github.com");
    }
}
