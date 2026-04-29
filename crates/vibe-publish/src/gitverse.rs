//! GitVerse public-API [`RepoCreator`] impl.
//!
//! GitVerse exposes a Gitea-shape REST API at `https://api.gitverse.ru`
//! (no `/v1/` prefix — versioning is carried in the `Accept` header
//! per [the public API docs](https://gitverse.ru/docs/public-api/)).
//! The two endpoints we use:
//!
//! - `GET /repos/{org}/{repo}` — repo presence check.
//!   - 200 → exists.
//!   - 404 → does not exist.
//!   - 401 / 403 → auth issue.
//! - `POST /orgs/{org}/repos` — create repo in an org.
//!   - 201 → created.
//!   - 409 → already exists (race condition).
//!   - 401 / 403 / 404 mapped per PROP-002 §2.10.
//!
//! Authentication: `Authorization: Bearer <token>` per the GitVerse
//! docs. The `Accept` header MUST be
//! `application/vnd.gitverse.object+json;version=1` — without the
//! versioning suffix the API returns 400 Bad Request with an empty
//! body and the response header `gitverse-api-latest-version: 1`
//! (informing the client which version to ask for). Token loading
//! lives in [`crate::token`].
//!
//! Endpoint discovery (2026-04-26): `gitverse.ru/api/v1/...` and
//! Gitea-style `Authorization: token <value>` both 404/401 against
//! the live host; the correct shape was found by curl-probing
//! `https://api.gitverse.ru` and reading the docs' "How to authenticate"
//! section. Recorded here for posterity — if GitVerse moves to a new
//! major API version, this is the file to update first.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::token::Token;
use crate::{CreateOpts, PublishError, RepoCreator, RepoInfo};

/// Default base URL for the GitVerse REST API.
pub const DEFAULT_GITVERSE_API_BASE: &str = "https://api.gitverse.ru";

/// Default human-readable host name for error messages.
pub const DEFAULT_GITVERSE_HOST_NAME: &str = "gitverse.ru";

/// Required Accept header carrying the API version. Without the
/// `;version=1` suffix the API returns 400 Bad Request.
const GITVERSE_ACCEPT: &str = "application/vnd.gitverse.object+json;version=1";

pub struct GitVerseCreator {
    api_base: String,
    host_name: String,
    /// Org this adapter is scoped to. Drives [`RepoCreator::expected_org`]
    /// and the default [`RepoCreator::validate_scope`] guard. Constructor
    /// requires it so adapters built in production never trust a caller
    /// not to escalate scope.
    expected_org: String,
    token: Token,
    client: reqwest::blocking::Client,
}

impl GitVerseCreator {
    pub fn new(token: Token, expected_org: impl Into<String>) -> Result<Self, PublishError> {
        Self::with_endpoint(
            token,
            expected_org,
            DEFAULT_GITVERSE_API_BASE,
            DEFAULT_GITVERSE_HOST_NAME,
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
        Ok(GitVerseCreator {
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
    default_branch: Option<&'a str>,
    auto_init: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    website: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct RepoResponse {
    /// Gitea-compatible field carrying the SSH clone URL.
    #[serde(default)]
    ssh_url: Option<String>,
    /// Gitea-compatible field carrying the HTTPS clone URL.
    #[serde(default)]
    clone_url: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
}

impl RepoCreator for GitVerseCreator {
    fn host_name(&self) -> &str {
        &self.host_name
    }

    fn expected_org(&self) -> Option<&str> {
        Some(&self.expected_org)
    }

    fn push_url(&self, org: &str, name: &str) -> String {
        // GitVerse uses SSH for pushes — the user's SSH agent / key handles
        // authentication. The token is API-only, never embedded in URL.
        format!("git@{}:{}/{}.git", self.host_name, org, name)
    }

    fn repo_exists(&self, org: &str, name: &str) -> Result<bool, PublishError> {
        self.validate_scope(org)?;
        let url = format!("{}/repos/{}/{}", self.api_base, org, name);
        let res = self
            .client
            .get(&url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .header(reqwest::header::ACCEPT, GITVERSE_ACCEPT)
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
            default_branch: opts.default_branch.as_deref(),
            // We push our own initial commit; never let the host
            // pre-populate, that would conflict with our first push.
            auto_init: false,
            website: opts.homepage.as_deref(),
        };
        let res = self
            .client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .header(reqwest::header::ACCEPT, GITVERSE_ACCEPT)
            .json(&body)
            .send()
            .map_err(|e| classify_send_error(e, &self.host_name))?;
        let status = res.status();
        if status.is_success() {
            let parsed: RepoResponse = res.json().map_err(|e| PublishError::HttpFailed {
                host: self.host_name.clone(),
                message: format!("parsing create-repo response: {e}"),
            })?;
            // Prefer SSH for clone URL since contributors typically have
            // SSH keys configured against the host. Fall back to HTTPS if
            // the host omitted SSH.
            let clone_url = parsed
                .ssh_url
                .or(parsed.clone_url)
                .ok_or_else(|| PublishError::UnexpectedResponse {
                    host: self.host_name.clone(),
                    status: status.as_u16(),
                    body: "create-repo response missing both ssh_url and clone_url".to_string(),
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
            409 => {
                // Race condition: someone created the repo between our
                // exists-check and our create. Treat as OK (re-fetch
                // info) — but keep it tight: bubble UnexpectedResponse
                // so the operator notices in the (unlikely) production
                // case. They can re-run; the second invocation's
                // exists-check will pick up the now-existing repo.
                Err(PublishError::UnexpectedResponse {
                    host: self.host_name.clone(),
                    status: 409,
                    body: format!(
                        "repo `{org}/{name}` already exists (created concurrently?). \
                         Re-run `vibe registry publish` — the existing repo will be reused."
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
