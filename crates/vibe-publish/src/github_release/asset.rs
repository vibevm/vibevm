use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Method;
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use serde::Serialize;

use super::{
    GithubReleaseAsset, GithubReleaseClient, GithubReleaseError, sha256_digest, validate_asset_name,
};

static TEMP_ASSET_SEQUENCE: AtomicU64 = AtomicU64::new(1);

impl GithubReleaseClient {
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
