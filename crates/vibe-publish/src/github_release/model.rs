use serde::{Deserialize, Serialize};
use specmark::spec;
use thiserror::Error;

pub const DEFAULT_GITHUB_UPLOAD_BASE: &str = "https://uploads.github.com";
pub const GITHUB_API_VERSION: &str = "2022-11-28";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateGithubRelease {
    pub tag_name: String,
    pub target_commitish: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub generate_release_notes: bool,
}

impl CreateGithubRelease {
    /// The canonical mutable release request for one semantic version.
    pub fn for_version(version: &semver::Version, source_commit: impl Into<String>) -> Self {
        let tag = format!("v{version}");
        Self {
            tag_name: tag.clone(),
            target_commitish: source_commit.into(),
            name: tag,
            body: String::new(),
            draft: false,
            prerelease: !version.pre.is_empty(),
            generate_release_notes: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct UpdateGithubRelease {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_commitish: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerelease: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GithubRelease {
    pub id: u64,
    pub tag_name: String,
    #[serde(default)]
    pub target_commitish: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub upload_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GithubReleaseAsset {
    pub id: u64,
    pub name: String,
    pub size: u64,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub browser_download_url: String,
    #[serde(default)]
    pub content_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GithubGitObject {
    pub sha: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GithubGitRef {
    #[serde(rename = "ref")]
    pub reference: String,
    pub object: GithubGitObject,
}

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#instances")]
pub enum GithubReleaseError {
    #[error(
        "invalid GitHub endpoint `{endpoint}`: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: configure an http(s) GitHub API base without credentials, query, or fragment)"
    )]
    InvalidEndpoint { endpoint: String, detail: String },
    #[error(
        "GitHub {operation} request failed: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: restore GitHub connectivity and retry the bounded operation)"
    )]
    Transport {
        operation: &'static str,
        message: String,
    },
    #[error(
        "GitHub {resource} was not found \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: refresh release discovery and select an extant release or asset ID)"
    )]
    NotFound { resource: String },
    #[error(
        "GitHub rejected {operation} as unprocessable (422): {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: inspect the host validation message, refresh state, and retry)"
    )]
    Unprocessable {
        operation: &'static str,
        message: String,
    },
    #[error(
        "GitHub {operation} returned status {status}: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: resolve the reported host or authorization failure and retry)"
    )]
    UnexpectedStatus {
        operation: &'static str,
        status: u16,
        message: String,
    },
    #[error(
        "GitHub {operation} returned invalid JSON: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: retry against the supported GitHub API version or update the adapter)"
    )]
    InvalidResponse {
        operation: &'static str,
        message: String,
    },
    #[error(
        "uploaded asset `{name}` failed verification: expected size {expected_size} and digest `{expected_digest}`, got size {actual_size} and digest {actual_digest:?} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: retain the old asset, remove the invalid temporary upload, and retry)"
    )]
    AssetVerification {
        name: String,
        expected_size: u64,
        actual_size: u64,
        expected_digest: String,
        actual_digest: Option<String>,
    },
    #[error(
        "invalid HTTP content type `{value}`: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: pass a valid release-asset media type)"
    )]
    InvalidContentType { value: String, detail: String },
    #[error(
        "invalid vibevm release tag `{tag}`; expected `v<semver>` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: derive the single release tag from its semantic version)"
    )]
    InvalidReleaseTag { tag: String },
    #[error(
        "invalid GitHub release asset name `{name}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: use a non-empty plain file name without path separators)"
    )]
    InvalidAssetName { name: String },
    #[error(
        "GitHub {operation} requires a publish token; load one with `load_token_for_host(\"github.com\")` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: construct a write client with the loaded Token, or use only anonymous read methods)"
    )]
    AuthenticationRequired { operation: &'static str },
}
