//! GitVerse public-API release requests: pure request building plus one
//! blocking HTTP client that sends them.
//!
//! Endpoint shapes follow <https://gitverse.ru/docs/developers/public-api>
//! (fetched 2026-11-08): releases by tag / list / create, and a release's
//! asset list / multipart upload (`?name=`, form field `attachment`) /
//! delete. Every request carries the versioned `Accept` header; every
//! request except an asset download to a foreign origin carries
//! `Authorization: Bearer <token>`. No request description, error message
//! or log line ever contains the token value.

use std::io::Read;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use vibe_publish::Token;

/// Required by the GitVerse API; without `;version=1` it answers 400.
pub(crate) const GITVERSE_ACCEPT: &str = "application/vnd.gitverse.object+json;version=1";
pub(crate) const DEFAULT_GITVERSE_API_BASE: &str = "https://api.gitverse.ru";
/// The documented maxima: 30 releases and 50 assets per page.
pub(crate) const RELEASES_PER_PAGE: u32 = 30;
pub(crate) const ASSETS_PER_PAGE: u32 = 50;
pub(crate) const MAX_PAGES: u32 = 100;
pub(crate) const ATTACHMENT_FIELD: &str = "attachment";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlannedBody {
    None,
    Json(serde_json::Value),
    /// The bytes travel separately; the plan names them by size and digest.
    Multipart {
        field: &'static str,
        file_name: String,
        content_type: String,
        size: u64,
        digest: String,
    },
}

/// One request exactly as it will be sent, minus credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedRequest {
    pub(crate) method: &'static str,
    pub(crate) url: Url,
    pub(crate) body: PlannedBody,
}

impl PlannedRequest {
    /// A one-line human description. It names the auth scheme but never
    /// carries a token, because this type never holds one.
    pub(crate) fn describe(&self) -> String {
        let body = match &self.body {
            PlannedBody::None => String::new(),
            PlannedBody::Json(value) => format!(" json {value}"),
            PlannedBody::Multipart {
                field,
                file_name,
                content_type,
                size,
                digest,
            } => {
                format!(" multipart {field}=@{file_name} ({content_type}, {size} bytes, {digest})")
            }
        };
        format!("{:<6} {}{body}", self.method, self.url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct CreateRelease {
    pub(crate) tag_name: String,
    pub(crate) name: String,
    pub(crate) target_commitish: String,
    pub(crate) body: String,
    pub(crate) draft: bool,
    pub(crate) prerelease: bool,
    /// `false` keeps the attached files downloadable anonymously, which the
    /// `gitverse` release source of `vibe self` relies on.
    pub(crate) is_authorized_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct GitverseRelease {
    pub(crate) id: u64,
    pub(crate) tag_name: String,
    #[serde(default)]
    pub(crate) target_commitish: Option<String>,
    #[serde(default)]
    pub(crate) draft: bool,
    #[serde(default)]
    pub(crate) is_authorized_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct GitverseAsset {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) browser_download_url: Option<String>,
}

/// `{api_base}/repos/{owner}/{repo}` with validated components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Endpoint {
    api_base: Url,
    owner: String,
    repo: String,
}

impl Endpoint {
    pub(crate) fn new(api_base: &str, owner: &str, repo: &str) -> Result<Self> {
        let api_base = Url::parse(&format!("{}/", api_base.trim_end_matches('/')))
            .with_context(|| format!("GitVerse API base `{api_base}` is not a URL"))?;
        if !matches!(api_base.scheme(), "https" | "http") || api_base.cannot_be_a_base() {
            bail!("GitVerse API base `{api_base}` must be an http(s) base URL");
        }
        if !api_base.username().is_empty() || api_base.password().is_some() {
            bail!("GitVerse API base must not embed credentials");
        }
        if api_base.query().is_some() || api_base.fragment().is_some() {
            bail!("GitVerse API base `{api_base}` must not carry a query or fragment");
        }
        for (label, value) in [("owner", owner), ("repo", repo)] {
            if !is_safe_component(value) {
                bail!("GitVerse {label} `{value}` must be letters, digits, `.`, `_` or `-`");
            }
        }
        Ok(Self {
            api_base,
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }

    pub(crate) fn repo_label(&self) -> String {
        self.url(&[]).to_string()
    }

    fn url(&self, suffix: &[&str]) -> Url {
        let mut url = self.api_base.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .expect("validated as a base URL in Endpoint::new");
            segments.pop_if_empty();
            segments.push("repos").push(&self.owner).push(&self.repo);
            segments.extend(suffix);
        }
        url
    }

    fn get(&self, suffix: &[&str], query: &[(&str, String)]) -> PlannedRequest {
        let mut url = self.url(suffix);
        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }
        PlannedRequest {
            method: "GET",
            url,
            body: PlannedBody::None,
        }
    }

    pub(crate) fn release_by_tag(&self, tag: &str) -> PlannedRequest {
        self.get(&["releases", "tags", tag], &[])
    }

    pub(crate) fn list_releases(&self, page: u32) -> PlannedRequest {
        self.get(
            &["releases"],
            &[
                ("page", page.to_string()),
                ("per_page", RELEASES_PER_PAGE.to_string()),
            ],
        )
    }

    pub(crate) fn create_release(&self, request: &CreateRelease) -> Result<PlannedRequest> {
        Ok(PlannedRequest {
            method: "POST",
            url: self.url(&["releases"]),
            body: PlannedBody::Json(serde_json::to_value(request)?),
        })
    }

    pub(crate) fn list_assets(&self, release_id: u64, page: u32) -> PlannedRequest {
        self.get(
            &["releases", &release_id.to_string(), "assets"],
            &[
                ("page", page.to_string()),
                ("per_page", ASSETS_PER_PAGE.to_string()),
            ],
        )
    }

    pub(crate) fn upload_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        size: u64,
        digest: &str,
    ) -> PlannedRequest {
        let mut url = self.url(&["releases", &release_id.to_string(), "assets"]);
        url.query_pairs_mut().append_pair("name", name);
        PlannedRequest {
            method: "POST",
            url,
            body: PlannedBody::Multipart {
                field: ATTACHMENT_FIELD,
                file_name: name.to_string(),
                content_type: content_type.to_string(),
                size,
                digest: digest.to_string(),
            },
        }
    }

    pub(crate) fn delete_asset(&self, release_id: u64, asset_id: u64) -> PlannedRequest {
        PlannedRequest {
            method: "DELETE",
            url: self.url(&[
                "releases",
                &release_id.to_string(),
                "assets",
                &asset_id.to_string(),
            ]),
            body: PlannedBody::None,
        }
    }

    /// Authorization goes only to the API's own origin.
    pub(crate) fn same_origin(&self, url: &Url) -> bool {
        url.origin() == self.api_base.origin()
    }
}

fn is_safe_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// Asset names GitVerse accepts: 1–255 of `[A-Za-z0-9._-]`. The docs also
/// demand a file extension; names without one are reported, not renamed,
/// because `vibe self` looks assets up by their canonical names.
pub(crate) fn asset_name_has_extension(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
}

pub(crate) fn validate_asset_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 255 || !is_safe_component(name) {
        bail!("asset name `{name}` is not a valid GitVerse asset name");
    }
    Ok(())
}

/// A `multipart/form-data` body with one file part. The boundary derives
/// from the content digest, so the same bytes always produce the same body.
pub(crate) fn multipart_body(
    file_name: &str,
    content_type: &str,
    digest: &str,
    bytes: &[u8],
) -> (String, Vec<u8>) {
    let tail = digest.rsplit(':').next().unwrap_or(digest);
    let boundary = format!(
        "vibevm-xtask-{}",
        tail.get(..32).unwrap_or(tail).to_ascii_lowercase()
    );
    let mut body = Vec::with_capacity(bytes.len() + 256);
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{ATTACHMENT_FIELD}\"; \
             filename=\"{file_name}\"\r\nContent-Type: {content_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

/// Sends planned requests with the token. Holds the only copy of the token.
pub(crate) struct GitverseClient {
    endpoint: Endpoint,
    token: Token,
    http: reqwest::blocking::Client,
}

impl GitverseClient {
    pub(crate) fn new(endpoint: Endpoint, token: Token) -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(30 * 60))
            .user_agent(concat!("vibevm-xtask/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("constructing GitVerse HTTP client")?;
        Ok(Self {
            endpoint,
            token,
            http,
        })
    }

    pub(crate) fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    fn redact(&self, text: &str) -> String {
        let value = self.token.value();
        if value.is_empty() {
            text.to_string()
        } else {
            text.replace(value, "***")
        }
    }

    fn send(
        &self,
        planned: &PlannedRequest,
        payload: Option<&[u8]>,
    ) -> Result<reqwest::blocking::Response> {
        let method = reqwest::Method::from_bytes(planned.method.as_bytes())?;
        let mut request = self
            .http
            .request(method, planned.url.clone())
            .header(reqwest::header::ACCEPT, GITVERSE_ACCEPT)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.token.value()),
            );
        request = match &planned.body {
            PlannedBody::None => request,
            PlannedBody::Json(value) => request.json(value),
            PlannedBody::Multipart {
                file_name,
                content_type,
                digest,
                ..
            } => {
                let bytes = payload.context("multipart request lacks its payload")?;
                let (header, body) = multipart_body(file_name, content_type, digest, bytes);
                request
                    .header(reqwest::header::CONTENT_TYPE, header)
                    .body(body)
            }
        };
        request.send().map_err(|error| {
            anyhow::anyhow!(
                "GitVerse request `{} {}` failed: {}",
                planned.method,
                planned.url,
                self.redact(&error.to_string())
            )
        })
    }

    fn fail(
        &self,
        planned: &PlannedRequest,
        response: reqwest::blocking::Response,
    ) -> anyhow::Error {
        let status = response.status();
        let mut body = String::new();
        let _ = response.take(4096).read_to_string(&mut body);
        anyhow::anyhow!(
            "GitVerse `{} {}` answered HTTP {status}: {}",
            planned.method,
            planned.url,
            self.redact(body.trim())
        )
    }

    fn json<T: serde::de::DeserializeOwned>(
        &self,
        planned: &PlannedRequest,
        payload: Option<&[u8]>,
    ) -> Result<T> {
        let response = self.send(planned, payload)?;
        if !response.status().is_success() {
            return Err(self.fail(planned, response));
        }
        response
            .json()
            .with_context(|| format!("parsing GitVerse `{} {}`", planned.method, planned.url))
    }

    /// By tag first; a 404 there falls back to scanning the release list,
    /// which also sees a draft the by-tag route may hide.
    pub(crate) fn find_release(&self, tag: &str) -> Result<Option<GitverseRelease>> {
        let planned = self.endpoint.release_by_tag(tag);
        let response = self.send(&planned, None)?;
        if response.status().is_success() {
            let release: GitverseRelease = response
                .json()
                .context("parsing GitVerse release-by-tag response")?;
            if release.tag_name != tag {
                bail!(
                    "GitVerse answered tag `{}` for requested tag `{tag}`",
                    release.tag_name
                );
            }
            return Ok(Some(release));
        }
        if response.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(self.fail(&planned, response));
        }
        let mut found = None;
        for page in 1..=MAX_PAGES {
            let releases: Vec<GitverseRelease> =
                self.json(&self.endpoint.list_releases(page), None)?;
            let count = releases.len();
            for release in releases
                .into_iter()
                .filter(|release| release.tag_name == tag)
            {
                if found.replace(release).is_some() {
                    bail!("GitVerse lists more than one release for tag `{tag}`");
                }
            }
            if count < RELEASES_PER_PAGE as usize {
                return Ok(found);
            }
        }
        bail!("GitVerse release list exceeded {MAX_PAGES} pages")
    }

    pub(crate) fn create_release(&self, request: &CreateRelease) -> Result<GitverseRelease> {
        let planned = self.endpoint.create_release(request)?;
        let release: GitverseRelease = self.json(&planned, None).with_context(|| {
            format!(
                "creating GitVerse release `{}` (the tag must exist on GitVerse or be creatable \
                 from `{}`)",
                request.tag_name, request.target_commitish
            )
        })?;
        if release.tag_name != request.tag_name {
            bail!(
                "GitVerse created tag `{}` instead of `{}`",
                release.tag_name,
                request.tag_name
            );
        }
        Ok(release)
    }

    pub(crate) fn list_assets(&self, release_id: u64) -> Result<Vec<GitverseAsset>> {
        let mut assets = Vec::new();
        for page in 1..=MAX_PAGES {
            let mut batch: Vec<GitverseAsset> =
                self.json(&self.endpoint.list_assets(release_id, page), None)?;
            let count = batch.len();
            assets.append(&mut batch);
            if count < ASSETS_PER_PAGE as usize {
                return Ok(assets);
            }
        }
        bail!("GitVerse asset list exceeded {MAX_PAGES} pages")
    }

    /// Download an existing asset, reading at most `expected_size + 1` bytes.
    pub(crate) fn download_asset(&self, asset: &GitverseAsset) -> Result<Vec<u8>> {
        let raw = asset
            .browser_download_url
            .as_deref()
            .with_context(|| format!("GitVerse asset `{}` has no download URL", asset.name))?;
        let url = Url::parse(raw)
            .with_context(|| format!("GitVerse asset `{}` download URL is invalid", asset.name))?;
        if !matches!(url.scheme(), "https" | "http") {
            bail!(
                "GitVerse asset `{}` download URL is not http(s)",
                asset.name
            );
        }
        let mut request = self
            .http
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/octet-stream");
        if self.endpoint.same_origin(&url) {
            request = request.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.token.value()),
            );
        }
        let response = request.send().map_err(|error| {
            anyhow::anyhow!(
                "downloading GitVerse asset `{}`: {}",
                asset.name,
                self.redact(&error.to_string())
            )
        })?;
        if !response.status().is_success() {
            bail!(
                "downloading GitVerse asset `{}` answered HTTP {}",
                asset.name,
                response.status()
            );
        }
        let mut bytes = Vec::new();
        response
            .take(asset.size.saturating_add(1))
            .read_to_end(&mut bytes)
            .with_context(|| format!("reading GitVerse asset `{}`", asset.name))?;
        if bytes.len() as u64 != asset.size {
            bail!(
                "GitVerse asset `{}` downloaded {} bytes but lists {}",
                asset.name,
                bytes.len(),
                asset.size
            );
        }
        Ok(bytes)
    }

    pub(crate) fn upload_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        digest: &str,
        bytes: &[u8],
    ) -> Result<GitverseAsset> {
        let planned =
            self.endpoint
                .upload_asset(release_id, name, content_type, bytes.len() as u64, digest);
        let asset: GitverseAsset = self.json(&planned, Some(bytes))?;
        if asset.name != name || asset.size != bytes.len() as u64 {
            bail!(
                "GitVerse stored `{name}` as `{}` with {} bytes (expected {})",
                asset.name,
                asset.size,
                bytes.len()
            );
        }
        Ok(asset)
    }

    pub(crate) fn delete_asset(&self, release_id: u64, asset_id: u64) -> Result<()> {
        let planned = self.endpoint.delete_asset(release_id, asset_id);
        let response = self.send(&planned, None)?;
        if !response.status().is_success() {
            return Err(self.fail(&planned, response));
        }
        Ok(())
    }
}
