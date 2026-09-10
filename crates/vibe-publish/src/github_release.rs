//! Blocking GitHub Releases client for assembling vibevm's independently
//! built platform bundles into one mutable release.
//!
//! Writes use the existing redacting [`crate::Token`] wrapper. Public asset
//! downloads deliberately omit authorization so the same path works for a
//! bootstrap client that has no publish credential.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::Method;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderValue, USER_AGENT};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use crate::Token;
use crate::git_publish::redact_credentials;
use crate::github::DEFAULT_GITHUB_API_BASE;

mod model;
pub use model::*;

const GITHUB_JSON: &str = "application/vnd.github+json";
const GITHUB_BINARY: &str = "application/octet-stream";
static TEMP_ASSET_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Client scoped to one GitHub repository.
pub struct GithubReleaseClient {
    owner: String,
    repo: String,
    api_base: String,
    upload_base: String,
    token: Option<Token>,
    client: Client,
}

impl GithubReleaseClient {
    pub fn new(
        token: Token,
        owner: impl Into<String>,
        repo: impl Into<String>,
    ) -> Result<Self, GithubReleaseError> {
        Self::build(
            Some(token),
            owner,
            repo,
            DEFAULT_GITHUB_API_BASE,
            DEFAULT_GITHUB_UPLOAD_BASE,
        )
    }

    /// Construct a read-only client for public release discovery and asset
    /// downloads. No Authorization header is sent on any read request.
    pub fn anonymous(
        owner: impl Into<String>,
        repo: impl Into<String>,
    ) -> Result<Self, GithubReleaseError> {
        Self::build(
            None,
            owner,
            repo,
            DEFAULT_GITHUB_API_BASE,
            DEFAULT_GITHUB_UPLOAD_BASE,
        )
    }

    pub fn with_endpoints(
        token: Token,
        owner: impl Into<String>,
        repo: impl Into<String>,
        api_base: &str,
        upload_base: &str,
    ) -> Result<Self, GithubReleaseError> {
        Self::build(Some(token), owner, repo, api_base, upload_base)
    }

    pub fn anonymous_with_endpoints(
        owner: impl Into<String>,
        repo: impl Into<String>,
        api_base: &str,
        upload_base: &str,
    ) -> Result<Self, GithubReleaseError> {
        Self::build(None, owner, repo, api_base, upload_base)
    }

    fn build(
        token: Option<Token>,
        owner: impl Into<String>,
        repo: impl Into<String>,
        api_base: &str,
        upload_base: &str,
    ) -> Result<Self, GithubReleaseError> {
        let secret = token.as_ref().map(Token::value);
        validate_base(api_base, secret)?;
        validate_base(upload_base, secret)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| GithubReleaseError::Transport {
                operation: "client construction",
                message: redact_optional_secret(error.to_string(), secret),
            })?;
        Ok(Self {
            owner: owner.into(),
            repo: repo.into(),
            api_base: api_base.trim_end_matches('/').to_string(),
            upload_base: upload_base.trim_end_matches('/').to_string(),
            token,
            client,
        })
    }

    pub fn find_release(&self, tag: &str) -> Result<Option<GithubRelease>, GithubReleaseError> {
        validate_release_tag(tag, self.token_value())?;
        let url = self.api_url(&["releases", "tags", tag])?;
        let response = self.send(
            self.read_request(Method::GET, url, GITHUB_JSON),
            "find release",
        )?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        self.json_response(response, "find release").map(Some)
    }

    pub fn create_release(
        &self,
        request: &CreateGithubRelease,
    ) -> Result<GithubRelease, GithubReleaseError> {
        validate_release_tag(&request.tag_name, self.token_value())?;
        let url = self.api_url(&["releases"])?;
        let builder = self
            .write_request(Method::POST, url, "create release")?
            .json(request);
        let response = self.send(builder, "create release")?;
        self.json_response(response, "create release")
    }

    /// Create a release or update its mutable metadata when the tag already
    /// has one. Moving an existing Git tag is deliberately separate and
    /// explicit via [`Self::force_move_tag`].
    pub fn upsert_release(
        &self,
        request: &CreateGithubRelease,
    ) -> Result<GithubRelease, GithubReleaseError> {
        self.require_write("upsert release")?;
        let Some(existing) = self.find_release(&request.tag_name)? else {
            return self.create_release(request);
        };
        self.update_release(
            existing.id,
            &UpdateGithubRelease {
                tag_name: Some(request.tag_name.clone()),
                target_commitish: Some(request.target_commitish.clone()),
                name: Some(request.name.clone()),
                body: Some(request.body.clone()),
                draft: Some(request.draft),
                prerelease: Some(request.prerelease),
            },
        )
    }

    pub fn update_release(
        &self,
        release_id: u64,
        update: &UpdateGithubRelease,
    ) -> Result<GithubRelease, GithubReleaseError> {
        if let Some(tag) = update.tag_name.as_deref() {
            validate_release_tag(tag, self.token_value())?;
        }
        let id = release_id.to_string();
        let url = self.api_url(&["releases", &id])?;
        let response = self.send(
            self.write_request(Method::PATCH, url, "update release")?
                .json(update),
            "update release",
        )?;
        self.json_response(response, "update release")
    }

    pub fn list_assets(
        &self,
        release_id: u64,
    ) -> Result<Vec<GithubReleaseAsset>, GithubReleaseError> {
        let id = release_id.to_string();
        let mut url = self.api_url(&["releases", &id, "assets"])?;
        url.query_pairs_mut().append_pair("per_page", "100");
        let response = self.send(
            self.read_request(Method::GET, url, GITHUB_JSON),
            "list release assets",
        )?;
        self.json_response(response, "list release assets")
    }

    pub fn upload_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<GithubReleaseAsset, GithubReleaseError> {
        validate_asset_name(name, self.token_value())?;
        let id = release_id.to_string();
        let mut url = self.upload_url(&["releases", &id, "assets"])?;
        url.query_pairs_mut().append_pair("name", name);
        let content_type = HeaderValue::from_str(content_type).map_err(|error| {
            GithubReleaseError::InvalidContentType {
                value: self.redact(content_type),
                detail: self.redact(error.to_string()),
            }
        })?;
        let response = self.send(
            self.write_request(Method::POST, url, "upload release asset")?
                .header(CONTENT_TYPE, content_type)
                .body(bytes.to_vec()),
            "upload release asset",
        )?;
        self.json_response(response, "upload release asset")
    }

    /// Download the current bytes for an exact GitHub release-asset ID.
    /// Reads never send the optional publish token.
    pub fn download_asset(&self, asset_id: u64) -> Result<Vec<u8>, GithubReleaseError> {
        let id = asset_id.to_string();
        let url = self.api_url(&["releases", "assets", &id])?;
        let response = self.send(
            self.read_request(Method::GET, url, GITHUB_BINARY),
            "download public release asset",
        )?;
        let status = response.status();
        if !status.is_success() {
            return Err(self.status_error(response, "download public release asset"));
        }
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| GithubReleaseError::Transport {
                operation: "download public release asset",
                message: self.redact(error.to_string()),
            })
    }

    pub fn delete_asset(&self, asset_id: u64) -> Result<(), GithubReleaseError> {
        let id = asset_id.to_string();
        let url = self.api_url(&["releases", "assets", &id])?;
        let response = self.send(
            self.write_request(Method::DELETE, url, "delete release asset")?,
            "delete release asset",
        )?;
        if response.status().is_success() {
            return Ok(());
        }
        Err(self.status_error(response, "delete release asset"))
    }

    pub fn rename_asset(
        &self,
        asset_id: u64,
        name: &str,
    ) -> Result<GithubReleaseAsset, GithubReleaseError> {
        validate_asset_name(name, self.token_value())?;
        #[derive(Serialize)]
        struct RenameAsset<'a> {
            name: &'a str,
        }
        let id = asset_id.to_string();
        let url = self.api_url(&["releases", "assets", &id])?;
        let response = self.send(
            self.write_request(Method::PATCH, url, "rename release asset")?
                .json(&RenameAsset { name }),
            "rename release asset",
        )?;
        self.json_response(response, "rename release asset")
    }

    /// Publish one logical release asset. Existing same-name assets are
    /// replaced by default through the verified temporary-upload protocol.
    pub fn publish_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<GithubReleaseAsset, GithubReleaseError> {
        self.replace_asset(release_id, name, content_type, bytes)
    }

    /// Replace all existing assets named `name` without a delete-first gap.
    ///
    /// The new bytes are uploaded under a process-unique temporary name. The
    /// returned size and SHA-256 digest must match locally computed values
    /// before any old asset is deleted; only then is the verified asset renamed.
    pub fn replace_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<GithubReleaseAsset, GithubReleaseError> {
        self.require_write("replace release asset")?;
        validate_asset_name(name, self.token_value())?;
        let old_assets = self
            .list_assets(release_id)?
            .into_iter()
            .filter(|asset| asset.name == name)
            .collect::<Vec<_>>();
        let expected_digest = sha256_digest(bytes);
        let temporary_name = temporary_asset_name(name, &expected_digest);
        let uploaded = self.upload_asset(release_id, &temporary_name, content_type, bytes)?;
        if uploaded.size != bytes.len() as u64
            || uploaded.digest.as_deref() != Some(expected_digest.as_str())
        {
            return Err(GithubReleaseError::AssetVerification {
                name: self.redact(uploaded.name),
                expected_size: bytes.len() as u64,
                actual_size: uploaded.size,
                expected_digest,
                actual_digest: uploaded.digest.map(|digest| self.redact(digest)),
            });
        }
        for old in old_assets {
            self.delete_asset(old.id)?;
        }
        self.rename_asset(uploaded.id, name)
    }

    /// Force-move the Git tag backing a mutable release to `source_commit`.
    pub fn force_move_tag(
        &self,
        tag: &str,
        source_commit: &str,
    ) -> Result<GithubGitRef, GithubReleaseError> {
        validate_release_tag(tag, self.token_value())?;
        #[derive(Serialize)]
        struct MoveRef<'a> {
            sha: &'a str,
            force: bool,
        }
        let url = self.api_url(&["git", "refs", "tags", tag])?;
        let response = self.send(
            self.write_request(Method::PATCH, url, "force-move release tag")?
                .json(&MoveRef {
                    sha: source_commit,
                    force: true,
                }),
            "force-move release tag",
        )?;
        self.json_response(response, "force-move release tag")
    }

    fn api_url(&self, suffix: &[&str]) -> Result<reqwest::Url, GithubReleaseError> {
        self.repo_url(&self.api_base, suffix)
    }

    fn upload_url(&self, suffix: &[&str]) -> Result<reqwest::Url, GithubReleaseError> {
        self.repo_url(&self.upload_base, suffix)
    }

    fn repo_url(&self, base: &str, suffix: &[&str]) -> Result<reqwest::Url, GithubReleaseError> {
        let endpoint = format!("{}/", base.trim_end_matches('/'));
        let mut url = reqwest::Url::parse(&endpoint).map_err(|error| {
            GithubReleaseError::InvalidEndpoint {
                endpoint: self.redact(&endpoint),
                detail: self.redact(error.to_string()),
            }
        })?;
        let safe_endpoint = self.redact(&endpoint);
        let mut segments =
            url.path_segments_mut()
                .map_err(|_| GithubReleaseError::InvalidEndpoint {
                    endpoint: safe_endpoint,
                    detail: "URL cannot be a base".to_string(),
                })?;
        segments.pop_if_empty();
        segments.push("repos");
        segments.push(&self.owner);
        segments.push(&self.repo);
        for segment in suffix {
            segments.push(segment);
        }
        drop(segments);
        Ok(url)
    }

    fn read_request(
        &self,
        method: Method,
        url: reqwest::Url,
        accept: &'static str,
    ) -> RequestBuilder {
        self.client
            .request(method, url)
            .header(ACCEPT, accept)
            .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
            .header(USER_AGENT, user_agent())
    }

    fn write_request(
        &self,
        method: Method,
        url: reqwest::Url,
        operation: &'static str,
    ) -> Result<RequestBuilder, GithubReleaseError> {
        let token = self
            .token
            .as_ref()
            .ok_or(GithubReleaseError::AuthenticationRequired { operation })?;
        Ok(self
            .read_request(method, url, GITHUB_JSON)
            .header(AUTHORIZATION, format!("Bearer {}", token.value())))
    }

    fn require_write(&self, operation: &'static str) -> Result<(), GithubReleaseError> {
        if self.token.is_none() {
            return Err(GithubReleaseError::AuthenticationRequired { operation });
        }
        Ok(())
    }

    fn send(
        &self,
        request: RequestBuilder,
        operation: &'static str,
    ) -> Result<Response, GithubReleaseError> {
        request
            .send()
            .map_err(|error| GithubReleaseError::Transport {
                operation,
                message: self.redact(error.to_string()),
            })
    }

    fn json_response<T: DeserializeOwned>(
        &self,
        response: Response,
        operation: &'static str,
    ) -> Result<T, GithubReleaseError> {
        let status = response.status();
        if !status.is_success() {
            return Err(self.status_error(response, operation));
        }
        let text = response
            .text()
            .map_err(|error| GithubReleaseError::Transport {
                operation,
                message: self.redact(error.to_string()),
            })?;
        serde_json::from_str(&text).map_err(|error| GithubReleaseError::InvalidResponse {
            operation,
            message: self.redact(format!("{error}; response body: {text}")),
        })
    }

    fn status_error(&self, response: Response, operation: &'static str) -> GithubReleaseError {
        let status = response.status();
        let message = response
            .text()
            .map(|body| self.redact(body))
            .unwrap_or_else(|error| self.redact(error.to_string()));
        match status.as_u16() {
            404 => GithubReleaseError::NotFound {
                resource: operation.to_string(),
            },
            422 => GithubReleaseError::Unprocessable { operation, message },
            value => GithubReleaseError::UnexpectedStatus {
                operation,
                status: value,
                message,
            },
        }
    }

    fn redact(&self, value: impl AsRef<str>) -> String {
        redact_optional_secret(value, self.token_value())
    }

    fn token_value(&self) -> Option<&str> {
        self.token.as_ref().map(Token::value)
    }
}

pub fn sha256_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity("sha256:".len() + digest.len() * 2);
    output.push_str("sha256:");
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn temporary_asset_name(name: &str, digest: &str) -> String {
    let sequence = TEMP_ASSET_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let digest_prefix = digest
        .strip_prefix("sha256:")
        .unwrap_or(digest)
        .chars()
        .take(12)
        .collect::<String>();
    format!(
        ".vibe-upload-{}-{nanos}-{sequence}-{digest_prefix}-{name}",
        std::process::id()
    )
}

fn validate_base(base: &str, token: Option<&str>) -> Result<(), GithubReleaseError> {
    let safe = redact_optional_secret(base, token);
    let url = reqwest::Url::parse(base).map_err(|error| GithubReleaseError::InvalidEndpoint {
        endpoint: safe.clone(),
        detail: redact_optional_secret(error.to_string(), token),
    })?;
    if url.cannot_be_a_base() {
        return Err(GithubReleaseError::InvalidEndpoint {
            endpoint: safe,
            detail: "URL cannot be a base".to_string(),
        });
    }
    if !matches!(url.scheme(), "http" | "https") {
        return Err(GithubReleaseError::InvalidEndpoint {
            endpoint: safe,
            detail: "endpoint must use http or https".to_string(),
        });
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(GithubReleaseError::InvalidEndpoint {
            endpoint: safe,
            detail: "API base URL must not contain user-info".to_string(),
        });
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(GithubReleaseError::InvalidEndpoint {
            endpoint: safe,
            detail: "API base URL must not contain a query or fragment".to_string(),
        });
    }
    Ok(())
}

fn validate_release_tag(tag: &str, token: Option<&str>) -> Result<(), GithubReleaseError> {
    let valid = tag
        .strip_prefix('v')
        .is_some_and(|version| !version.is_empty() && semver::Version::parse(version).is_ok());
    if !valid {
        return Err(GithubReleaseError::InvalidReleaseTag {
            tag: redact_optional_secret(tag, token),
        });
    }
    Ok(())
}

fn validate_asset_name(name: &str, token: Option<&str>) -> Result<(), GithubReleaseError> {
    if name.trim().is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.chars().any(char::is_control)
    {
        return Err(GithubReleaseError::InvalidAssetName {
            name: redact_optional_secret(name, token),
        });
    }
    Ok(())
}

fn redact_optional_secret(value: impl AsRef<str>, secret: Option<&str>) -> String {
    let redacted = redact_credentials(value);
    match secret {
        Some(secret) if !secret.is_empty() => redacted.replace(secret, "***"),
        _ => redacted,
    }
}

fn user_agent() -> String {
    format!("vibe-publish/{}", env!("CARGO_PKG_VERSION"))
}
