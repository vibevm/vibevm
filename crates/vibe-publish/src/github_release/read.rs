//! Explicit public-versus-authenticated GitHub Release read surfaces.

use std::io::Read;

use reqwest::Method;
use reqwest::header::LINK;

use super::{
    GITHUB_BINARY, GITHUB_JSON, GithubGitRef, GithubRelease, GithubReleaseAsset,
    GithubReleaseClient, GithubReleaseError, validate_release_tag,
};

const MAX_ASSET_PAGES: usize = 32;

impl GithubReleaseClient {
    /// Read the exact Git tag reference with authentication. Release
    /// finalization uses this independently of Release metadata so a manual
    /// force-move between prepare and publish cannot detach verified assets
    /// from the commit named by their manifests.
    pub fn get_tag_ref_authenticated(&self, tag: &str) -> Result<GithubGitRef, GithubReleaseError> {
        const OPERATION: &str = "read release tag reference";
        validate_release_tag(tag, self.token_value())?;
        let url = self.api_url(&["git", "ref", "tags", tag])?;
        let response = self.send(
            self.authenticated_read_request(Method::GET, url, GITHUB_JSON, OPERATION)?,
            OPERATION,
        )?;
        self.json_response(response, OPERATION)
    }

    /// Find a public release without sending Authorization, even when this
    /// client also carries a token for later writes.
    pub fn find_release(&self, tag: &str) -> Result<Option<GithubRelease>, GithubReleaseError> {
        self.find_release_inner(tag, false)
    }

    /// Find a draft or otherwise non-public release with the client's token.
    pub fn find_release_authenticated(
        &self,
        tag: &str,
    ) -> Result<Option<GithubRelease>, GithubReleaseError> {
        self.find_release_inner(tag, true)
    }

    fn find_release_inner(
        &self,
        tag: &str,
        authenticated: bool,
    ) -> Result<Option<GithubRelease>, GithubReleaseError> {
        validate_release_tag(tag, self.token_value())?;
        let url = self.api_url(&["releases", "tags", tag])?;
        let operation = if authenticated {
            "find draft release"
        } else {
            "find release"
        };
        let request = if authenticated {
            self.authenticated_read_request(Method::GET, url, GITHUB_JSON, operation)?
        } else {
            self.read_request(Method::GET, url, GITHUB_JSON)
        };
        let response = self.send(request, operation)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        self.json_response(response, operation).map(Some)
    }

    /// List public release assets without sending Authorization.
    pub fn list_assets(
        &self,
        release_id: u64,
    ) -> Result<Vec<GithubReleaseAsset>, GithubReleaseError> {
        self.list_assets_inner(release_id, false)
    }

    /// List assets attached to a draft/non-public release with the client's token.
    pub fn list_assets_authenticated(
        &self,
        release_id: u64,
    ) -> Result<Vec<GithubReleaseAsset>, GithubReleaseError> {
        self.list_assets_inner(release_id, true)
    }

    fn list_assets_inner(
        &self,
        release_id: u64,
        authenticated: bool,
    ) -> Result<Vec<GithubReleaseAsset>, GithubReleaseError> {
        let id = release_id.to_string();
        let mut url = self.api_url(&["releases", &id, "assets"])?;
        url.query_pairs_mut().append_pair("per_page", "100");
        let origin = (
            url.scheme().to_string(),
            url.host_str().map(str::to_string),
            url.port_or_known_default(),
            url.path().to_string(),
        );
        let operation = if authenticated {
            "list draft release assets"
        } else {
            "list release assets"
        };
        let mut assets = Vec::new();
        for _ in 0..MAX_ASSET_PAGES {
            let request = if authenticated {
                self.authenticated_read_request(Method::GET, url.clone(), GITHUB_JSON, operation)?
            } else {
                self.read_request(Method::GET, url.clone(), GITHUB_JSON)
            };
            let response = self.send(request, operation)?;
            let next = response
                .headers()
                .get(LINK)
                .map(|value| value.to_str())
                .transpose()
                .map_err(|error| GithubReleaseError::InvalidResponse {
                    operation,
                    message: self.redact(format!("invalid Link header: {error}")),
                })?
                .and_then(next_link)
                .map(str::to_string);
            let mut page: Vec<GithubReleaseAsset> = self.json_response(response, operation)?;
            assets.append(&mut page);
            let Some(next) = next else {
                return Ok(assets);
            };
            let candidate = reqwest::Url::parse(&next).map_err(|error| {
                GithubReleaseError::InvalidResponse {
                    operation,
                    message: self.redact(format!("invalid next-page URL: {error}")),
                }
            })?;
            if candidate.scheme() != origin.0
                || candidate.host_str() != origin.1.as_deref()
                || candidate.port_or_known_default() != origin.2
                || candidate.path() != origin.3
            {
                return Err(GithubReleaseError::InvalidResponse {
                    operation,
                    message: "next-page URL escaped the release-assets endpoint".to_string(),
                });
            }
            url = candidate;
        }
        Err(GithubReleaseError::InvalidResponse {
            operation,
            message: format!("asset pagination exceeded {MAX_ASSET_PAGES} pages"),
        })
    }

    /// Download the current public bytes for an exact release-asset ID.
    pub fn download_asset(&self, asset_id: u64) -> Result<Vec<u8>, GithubReleaseError> {
        self.download_asset_inner(asset_id, false)
    }

    /// Download a draft/non-public asset by exact ID with the client's token.
    pub fn download_asset_authenticated(
        &self,
        asset_id: u64,
    ) -> Result<Vec<u8>, GithubReleaseError> {
        self.download_asset_inner(asset_id, true)
    }

    /// Download a draft asset without ever buffering more than
    /// `expected_size + 1` bytes. Both the HTTP Content-Length (when present)
    /// and the streamed body must match the independently supplied size.
    pub fn download_asset_authenticated_bounded(
        &self,
        asset_id: u64,
        expected_size: u64,
        max_size: u64,
    ) -> Result<Vec<u8>, GithubReleaseError> {
        const OPERATION: &str = "download bounded draft release asset";
        if expected_size > max_size {
            return Err(GithubReleaseError::DownloadSize {
                operation: OPERATION,
                expected_size,
                actual_size: expected_size,
                max_size,
            });
        }
        let response = self.asset_download_response(asset_id, true, OPERATION)?;
        if let Some(content_length) = response.content_length()
            && (content_length != expected_size || content_length > max_size)
        {
            return Err(GithubReleaseError::DownloadSize {
                operation: OPERATION,
                expected_size,
                actual_size: content_length,
                max_size,
            });
        }
        let read_limit = expected_size
            .saturating_add(1)
            .min(max_size.saturating_add(1));
        let mut bytes =
            Vec::with_capacity(usize::try_from(expected_size.min(64 * 1024)).unwrap_or(64 * 1024));
        response
            .take(read_limit)
            .read_to_end(&mut bytes)
            .map_err(|error| GithubReleaseError::Transport {
                operation: OPERATION,
                message: self.redact(error.to_string()),
            })?;
        let actual_size = bytes.len() as u64;
        if actual_size != expected_size {
            return Err(GithubReleaseError::DownloadSize {
                operation: OPERATION,
                expected_size,
                actual_size,
                max_size,
            });
        }
        Ok(bytes)
    }

    fn download_asset_inner(
        &self,
        asset_id: u64,
        authenticated: bool,
    ) -> Result<Vec<u8>, GithubReleaseError> {
        let operation = if authenticated {
            "download draft release asset"
        } else {
            "download public release asset"
        };
        let response = self.asset_download_response(asset_id, authenticated, operation)?;
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| GithubReleaseError::Transport {
                operation,
                message: self.redact(error.to_string()),
            })
    }

    fn asset_download_response(
        &self,
        asset_id: u64,
        authenticated: bool,
        operation: &'static str,
    ) -> Result<reqwest::blocking::Response, GithubReleaseError> {
        let id = asset_id.to_string();
        let url = self.api_url(&["releases", "assets", &id])?;
        let request = if authenticated {
            self.authenticated_read_request(Method::GET, url, GITHUB_BINARY, operation)?
        } else {
            self.read_request(Method::GET, url, GITHUB_BINARY)
        };
        let response = self.send(request, operation)?;
        let status = response.status();
        if !status.is_success() {
            return Err(self.status_error(response, operation));
        }
        Ok(response)
    }
}

fn next_link(header: &str) -> Option<&str> {
    header.split(',').find_map(|part| {
        let (url, parameters) = part.trim().split_once('>')?;
        if !parameters
            .split(';')
            .any(|parameter| parameter.trim() == "rel=\"next\"")
        {
            return None;
        }
        url.strip_prefix('<')
    })
}
