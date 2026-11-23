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
//!   - 401 / 403 / 404 / 422 mapped per PROP-002 §2.10.
//!
//! Cooperative control. Both endpoints run through the retained
//! [`ManagedHttpClient`](vibe_registry::http_admission::ManagedHttpClient),
//! exactly as the GitHub adapter does, so a
//! [`CancellationToken`](vibe_core::cancellation::CancellationToken)
//! carrying control stops the publish *before* a request is started and
//! aborts one already in flight
//! ([PROP-073 `#PROJECT-COOPERATIVE-STOP`](../../../vibevm/vibespecs/common/PROP-073.xml)).
//! The uncontrolled [`RepoCreator::repo_exists`] / [`RepoCreator::create_repo`]
//! entry points are the same code paths driven with a default (no-control)
//! token, so the two can never drift apart.
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

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#publish");

use specmark::spec;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::token::Token;
use crate::{CreateOpts, PublishError, RepoCreator, RepoInfo, ValidatedOrg};

/// Default base URL for the GitVerse REST API.
pub const DEFAULT_GITVERSE_API_BASE: &str = "https://api.gitverse.ru";

/// Default human-readable host name for error messages.
pub const DEFAULT_GITVERSE_HOST_NAME: &str = "gitverse.ru";

/// Required Accept header carrying the API version. Without the
/// `;version=1` suffix the API returns 400 Bad Request.
const GITVERSE_ACCEPT: &str = "application/vnd.gitverse.object+json;version=1";

#[specmark::cell(seam = "RepoCreator", variant = "gitverse")]
pub struct GitverseRepoCreator {
    api_base: String,
    host_name: String,
    /// Org this adapter is scoped to. Drives [`RepoCreator::expected_org`]
    /// and the default [`RepoCreator::validate_scope`] guard. Constructor
    /// requires it so adapters built in production never trust a caller
    /// not to escalate scope.
    expected_org: String,
    token: Token,
    client: vibe_registry::http_admission::ManagedHttpClient,
}

impl GitverseRepoCreator {
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#PUB-GITVERSE-CREATOR"
    )]
    pub fn new(token: Token, expected_org: impl Into<String>) -> Result<Self, PublishError> {
        Self::with_endpoint(
            token,
            expected_org,
            DEFAULT_GITVERSE_API_BASE,
            DEFAULT_GITVERSE_HOST_NAME,
        )
    }

    #[spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#PUB-GITVERSE-CREATOR"
    )]
    pub fn with_endpoint(
        token: Token,
        expected_org: impl Into<String>,
        api_base: &str,
        host_name: &str,
    ) -> Result<Self, PublishError> {
        let client =
            vibe_registry::http_admission::ManagedHttpClient::legacy(Duration::from_secs(30))
                .map_err(|e| PublishError::HttpFailed {
                    host: host_name.to_string(),
                    message: format!("constructing HTTP client: {e}"),
                })?;
        Ok(GitverseRepoCreator {
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

impl RepoCreator for GitverseRepoCreator {
    fn host_name(&self) -> &str {
        &self.host_name
    }

    fn expected_org(&self) -> Option<&str> {
        Some(&self.expected_org)
    }

    fn push_url(&self, org: &ValidatedOrg, name: &str) -> String {
        // GitVerse uses SSH for pushes — the user's SSH agent / key handles
        // authentication. The token is API-only, never embedded in URL.
        format!("git@{}:{}/{}.git", self.host_name, org.as_str(), name)
    }

    fn repo_exists(&self, org: &ValidatedOrg, name: &str) -> Result<bool, PublishError> {
        self.repo_exists_with_control(
            org,
            name,
            &vibe_core::cancellation::CancellationToken::default(),
        )
    }

    fn create_repo(
        &self,
        org: &ValidatedOrg,
        name: &str,
        opts: &CreateOpts,
    ) -> Result<RepoInfo, PublishError> {
        self.create_repo_with_control(
            org,
            name,
            opts,
            &vibe_core::cancellation::CancellationToken::default(),
        )
    }

    fn repo_exists_controlled(
        &self,
        org: &ValidatedOrg,
        name: &str,
        token: &vibe_core::cancellation::CancellationToken,
    ) -> Result<bool, PublishError> {
        self.repo_exists_with_control(org, name, token)
    }

    fn create_repo_controlled(
        &self,
        org: &ValidatedOrg,
        name: &str,
        opts: &CreateOpts,
        token: &vibe_core::cancellation::CancellationToken,
    ) -> Result<RepoInfo, PublishError> {
        self.create_repo_with_control(org, name, opts, token)
    }
}

impl GitverseRepoCreator {
    /// One cancellable GitVerse request: the adapter's two required
    /// headers, an optional JSON body, and the owner's stop control.
    /// Mirrors the GitHub adapter's helper so both hosts refuse, abort
    /// and classify transport failures identically.
    fn request_controlled(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<Vec<u8>>,
        token: &vibe_core::cancellation::CancellationToken,
    ) -> Result<vibe_registry::http_admission::ManagedResponse, PublishError> {
        let mut headers = reqwest::header::HeaderMap::new();
        let invalid = |_: reqwest::header::InvalidHeaderValue| PublishError::HttpFailed {
            host: self.host_name.clone(),
            message: "invalid publisher request header".into(),
        };
        headers.insert(
            reqwest::header::AUTHORIZATION,
            self.auth_header().parse().map_err(invalid)?,
        );
        headers.insert(
            reqwest::header::ACCEPT,
            GITVERSE_ACCEPT.parse().map_err(invalid)?,
        );
        if body.is_some() {
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            );
        }
        self.client
            .execute_legacy(method, url, headers, body, token)
            .map_err(|error| match error {
                vibe_registry::http_admission::HttpGetError::Transport(error) => {
                    classify_send_error(error, &self.host_name)
                }
                error => PublishError::HttpFailed {
                    host: self.host_name.clone(),
                    message: error.to_string(),
                },
            })
    }

    fn repo_exists_with_control(
        &self,
        org: &ValidatedOrg,
        name: &str,
        cancellation: &vibe_core::cancellation::CancellationToken,
    ) -> Result<bool, PublishError> {
        let url = format!("{}/repos/{}/{}", self.api_base, org.as_str(), name);
        let res = self.request_controlled(reqwest::Method::GET, &url, None, cancellation)?;
        let status = res.status();
        if status.is_success() {
            return Ok(true);
        }
        match status.as_u16() {
            404 => Ok(false),
            401 | 403 => Err(PublishError::AuthForbidden {
                host: self.host_name.clone(),
                org: org.as_str().to_string(),
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

    fn create_repo_with_control(
        &self,
        org: &ValidatedOrg,
        name: &str,
        opts: &CreateOpts,
        cancellation: &vibe_core::cancellation::CancellationToken,
    ) -> Result<RepoInfo, PublishError> {
        let url = format!("{}/orgs/{}/repos", self.api_base, org.as_str());
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
        let bytes = serde_json::to_vec(&body).map_err(|error| PublishError::HttpFailed {
            host: self.host_name.clone(),
            message: error.to_string(),
        })?;
        let res =
            self.request_controlled(reqwest::Method::POST, &url, Some(bytes), cancellation)?;
        let status = res.status();
        if status.is_success() {
            let parsed: RepoResponse = res.json().map_err(|error| {
                if cancellation.acknowledged() {
                    PublishError::Cancelled {
                        repository_creation: "completed".into(),
                        release_push: "not-started".into(),
                        reason: "create-repo accepted; response metadata interrupted".into(),
                    }
                } else {
                    PublishError::HttpFailed {
                        host: self.host_name.clone(),
                        message: format!("parsing create-repo response: {error}"),
                    }
                }
            })?;
            // Prefer SSH for clone URL since contributors typically have
            // SSH keys configured against the host. Fall back to HTTPS if
            // the host omitted SSH.
            let clone_url = parsed.ssh_url.or(parsed.clone_url).ok_or_else(|| {
                PublishError::UnexpectedResponse {
                    host: self.host_name.clone(),
                    status: status.as_u16(),
                    body: "create-repo response missing both ssh_url and clone_url".to_string(),
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
                org: org.as_str().to_string(),
            }),
            404 => Err(PublishError::OrgNotFound {
                host: self.host_name.clone(),
                org: org.as_str().to_string(),
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
                        "repo `{}/{name}` already exists (created concurrently?). \
                         Re-run `vibe registry publish` — the existing repo will be reused.",
                        org.as_str()
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
#[path = "gitverse/tests.rs"]
mod tests;
