use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Method;
use reqwest::blocking::Body;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderValue, USER_AGENT};
use serde::Serialize;
use vibe_safefs::{StreamCompletion, StreamOutcome, VerifiedStreamReader};

use super::{
    GITHUB_API_VERSION, GITHUB_JSON, GithubReleaseAsset, GithubReleaseClient, GithubReleaseError,
    sha256_digest, user_agent, validate_asset_name,
};

const STREAMED_UPLOAD_RESPONSE_CAP: u64 = 64 * 1024;

/// A POST response only. The publication owner still has to reobserve the
/// remote release and asset independently before it can write Landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamedUploadResponse {
    pub asset_id: u64,
    pub temporary_name: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum StreamedUploadError {
    #[error("streamed asset upload refused before dispatch: {0}")]
    Refused(&'static str),
    #[error(
        "streamed asset upload may have taken effect; inspect the owner journal and remote state: {0}"
    )]
    Uncertain(&'static str),
}

static TEMP_ASSET_SEQUENCE: AtomicU64 = AtomicU64::new(1);

impl GithubReleaseClient {
    /// One explicit temporary-asset POST using a held nofollow reader. This
    /// does not clean up, rename, retry, or turn a response into Landed.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn upload_temporary_asset_streamed(
        &self,
        release_id: u64,
        temporary_name: &str,
        content_type: &str,
        expected_size: u64,
        expected_sha256: &str,
        reader: VerifiedStreamReader,
        completion: StreamCompletion,
    ) -> Result<StreamedUploadResponse, StreamedUploadError> {
        if release_id == 0 || expected_size == 0 {
            return Err(StreamedUploadError::Refused("invalid release ID or size"));
        }
        if validate_asset_name(temporary_name, self.token_value()).is_err() {
            return Err(StreamedUploadError::Refused("invalid temporary asset name"));
        }
        let Some(hex) = expected_sha256.strip_prefix("sha256:") else {
            return Err(StreamedUploadError::Refused("invalid expected SHA-256"));
        };
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(StreamedUploadError::Refused("invalid expected SHA-256"));
        }
        let content_type = HeaderValue::from_str(content_type)
            .map_err(|_| StreamedUploadError::Refused("invalid media type header"))?;
        let token = self
            .token
            .as_ref()
            .ok_or(StreamedUploadError::Refused("publish token absent"))?;
        if completion.outcome() != StreamOutcome::Incomplete {
            return Err(StreamedUploadError::Refused(
                "stream was already consumed or uncertain",
            ));
        }
        let id = release_id.to_string();
        let mut url = self
            .upload_url(&["releases", &id, "assets"])
            .map_err(|_| StreamedUploadError::Refused("invalid upload endpoint"))?;
        url.query_pairs_mut().append_pair("name", temporary_name);
        // This client has redirect policy None; reqwest cannot forward the
        // Authorization header or replay the move-only body to a 3xx target.
        let request = self
            .no_redirect_write_client
            .request(Method::POST, url)
            .header(ACCEPT, GITHUB_JSON)
            .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
            .header(USER_AGENT, user_agent())
            .header(AUTHORIZATION, format!("Bearer {}", token.value()))
            .header(CONTENT_TYPE, content_type)
            .body(Body::sized(reader, expected_size));
        let response = request
            .send()
            .map_err(|_| StreamedUploadError::Uncertain("POST transport failed"))?;
        if !matches!(completion.outcome(), StreamOutcome::Complete { bytes, sha256 }
            if bytes == expected_size && sha256 == hex)
        {
            return Err(StreamedUploadError::Uncertain(
                "local stream did not complete exact bytes",
            ));
        }
        if !response.status().is_success() {
            return Err(StreamedUploadError::Uncertain(
                "POST returned a non-success status or redirect",
            ));
        }
        let mut body = Vec::new();
        response
            .take(STREAMED_UPLOAD_RESPONSE_CAP + 1)
            .read_to_end(&mut body)
            .map_err(|_| StreamedUploadError::Uncertain("POST response read failed"))?;
        if body.len() as u64 > STREAMED_UPLOAD_RESPONSE_CAP {
            return Err(StreamedUploadError::Uncertain(
                "POST response exceeded bound",
            ));
        }
        let asset: GithubReleaseAsset = serde_json::from_slice(&body)
            .map_err(|_| StreamedUploadError::Uncertain("POST response malformed"))?;
        let response_digest = asset.digest.ok_or(StreamedUploadError::Uncertain(
            "POST response asset digest missing",
        ))?;
        if asset.id == 0
            || asset.name != temporary_name
            || asset.size != expected_size
            || response_digest != expected_sha256
        {
            return Err(StreamedUploadError::Uncertain(
                "POST response asset identity differs or digest missing",
            ));
        }
        Ok(StreamedUploadResponse {
            asset_id: asset.id,
            temporary_name: asset.name,
            size_bytes: asset.size,
            sha256: response_digest,
        })
    }

    /// Remove interrupted temporary uploads for the supplied canonical names.
    /// Other temporary namespaces and canonical assets remain untouched.
    pub fn cleanup_temporary_assets(
        &self,
        release_id: u64,
        canonical_names: &[&str],
    ) -> Result<(), GithubReleaseError> {
        for name in canonical_names {
            validate_asset_name(name, self.token_value())?;
        }
        let assets = self.list_assets_authenticated(release_id)?;
        for asset in assets {
            if canonical_names
                .iter()
                .any(|name| temporary_asset_belongs_to(&asset.name, name))
            {
                self.delete_asset(asset.id)?;
            }
        }
        Ok(())
    }

    pub fn upload_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<GithubReleaseAsset, GithubReleaseError> {
        self.upload_asset_owned(release_id, name, content_type, bytes.to_vec())
    }

    /// Ownership-taking path for large distribution assets; reqwest receives
    /// the existing Vec rather than cloning the complete body.
    pub fn upload_asset_owned(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: Vec<u8>,
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
                .body(bytes),
            "upload release asset",
        )?;
        self.json_response(response, "upload release asset")
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

    pub fn publish_asset_owned(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<GithubReleaseAsset, GithubReleaseError> {
        self.replace_asset_owned(release_id, name, content_type, bytes)
    }

    /// Replace all existing assets named `name` without a delete-first gap.
    /// The verified temporary upload is cleaned on every later failure while
    /// any cleanup error remains secondary to the original failure.
    pub fn replace_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<GithubReleaseAsset, GithubReleaseError> {
        self.replace_asset_owned(release_id, name, content_type, bytes.to_vec())
    }

    pub fn replace_asset_owned(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<GithubReleaseAsset, GithubReleaseError> {
        self.require_write("replace release asset")?;
        validate_asset_name(name, self.token_value())?;
        let assets = self.list_assets_authenticated(release_id)?;
        let expected_size = bytes.len() as u64;
        let expected_digest = sha256_digest(&bytes);
        let stale_temporary = assets
            .iter()
            .filter(|asset| temporary_asset_belongs_to(&asset.name, name))
            .map(|asset| asset.id)
            .collect::<Vec<_>>();
        for stale_id in stale_temporary {
            self.delete_asset(stale_id)?;
        }
        let old_assets = assets
            .into_iter()
            .filter(|asset| asset.name == name)
            .collect::<Vec<_>>();
        let temporary_name = temporary_asset_name(name, &expected_digest);
        let uploaded = self.upload_asset_owned(release_id, &temporary_name, content_type, bytes)?;
        if uploaded.size != expected_size
            || uploaded.digest.as_deref() != Some(expected_digest.as_str())
        {
            let primary = GithubReleaseError::AssetVerification {
                name: self.redact(uploaded.name),
                expected_size,
                actual_size: uploaded.size,
                expected_digest,
                actual_digest: uploaded.digest.map(|digest| self.redact(digest)),
            };
            let _ = self.delete_asset(uploaded.id);
            return Err(primary);
        }
        for old in old_assets {
            if let Err(primary) = self.delete_asset(old.id) {
                let _ = self.delete_asset(uploaded.id);
                return Err(primary);
            }
        }
        match self.rename_asset(uploaded.id, name) {
            Ok(asset) => Ok(asset),
            Err(primary) => {
                let _ = self.delete_asset(uploaded.id);
                Err(primary)
            }
        }
    }
}

#[cfg(test)]
#[path = "streamed_upload_tests.rs"]
mod streamed_upload_tests;

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

fn temporary_asset_belongs_to(candidate: &str, name: &str) -> bool {
    let Some(namespace) = candidate
        .strip_prefix(".vibe-upload-")
        .and_then(|value| value.strip_suffix(&format!("-{name}")))
    else {
        return false;
    };
    let Some((identity, digest_prefix)) = namespace.rsplit_once('-') else {
        return false;
    };
    !identity.is_empty()
        && digest_prefix.len() == 12
        && digest_prefix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
