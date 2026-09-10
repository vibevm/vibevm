//! Explicit public-versus-authenticated GitHub Release read surfaces.

use reqwest::Method;

use super::{
    GITHUB_BINARY, GITHUB_JSON, GithubRelease, GithubReleaseAsset, GithubReleaseClient,
    GithubReleaseError, validate_release_tag,
};

impl GithubReleaseClient {
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
        let operation = if authenticated {
            "list draft release assets"
        } else {
            "list release assets"
        };
        let request = if authenticated {
            self.authenticated_read_request(Method::GET, url, GITHUB_JSON, operation)?
        } else {
            self.read_request(Method::GET, url, GITHUB_JSON)
        };
        let response = self.send(request, operation)?;
        self.json_response(response, operation)
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

    fn download_asset_inner(
        &self,
        asset_id: u64,
        authenticated: bool,
    ) -> Result<Vec<u8>, GithubReleaseError> {
        let id = asset_id.to_string();
        let url = self.api_url(&["releases", "assets", &id])?;
        let operation = if authenticated {
            "download draft release asset"
        } else {
            "download public release asset"
        };
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
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| GithubReleaseError::Transport {
                operation,
                message: self.redact(error.to_string()),
            })
    }
}
