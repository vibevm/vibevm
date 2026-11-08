//! The complete, verified asset set of one release, read from a local
//! directory or from the published GitHub release.
//!
//! Whatever the origin, nothing reaches GitVerse until `DISTRIBUTIONS.json`
//! parses, names the requested version, and every bundle, bootstrap and
//! fragment matches it byte for byte — the same verification `dist finalize`
//! performs before publishing on GitHub.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use semver::Version;
use vibe_publish::release_manifest::{
    DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME, DISTRIBUTION_BASH_INSTALLER_FILENAME,
    DISTRIBUTION_CMD_INSTALLER_FILENAME, DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
};
use vibe_publish::{
    AggregateDistributionManifest, DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    DISTRIBUTION_BOOTSTRAP_MAX_BYTES, DISTRIBUTION_BUNDLE_MAX_BYTES,
    DISTRIBUTION_MANIFEST_MAX_BYTES, GithubReleaseClient, PlatformDistributionFragment,
    SUPPORTED_DISTRIBUTION_TARGETS, sha256_digest,
};

use super::super::local::read_bounded_regular_file;
use super::super::{unique_assets, validate_fragment_asset_names, verify_platform_bytes};
use crate::dist::build::{bootstrap_asset_name, bundle_asset_name, fragment_asset_name};

/// Installer scripts are small text files; this only bounds the read.
const INSTALLER_MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceAsset {
    pub(crate) name: String,
    pub(crate) content_type: &'static str,
    pub(crate) digest: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceRelease {
    pub(crate) version: Version,
    pub(crate) tag: String,
    pub(crate) source_commit: String,
    /// Upload order; `DISTRIBUTIONS.json` is always last, so a reader that
    /// sees the manifest sees every asset it names.
    pub(crate) assets: Vec<SourceAsset>,
}

/// `(name, content type, read limit)` in upload order.
pub(crate) fn expected_assets(version: &Version) -> Vec<(String, &'static str, u64)> {
    let mut expected = Vec::new();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        expected.push((
            bundle_asset_name(version, target),
            "application/zip",
            DISTRIBUTION_BUNDLE_MAX_BYTES,
        ));
        expected.push((
            bootstrap_asset_name(target),
            "application/octet-stream",
            DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        ));
        expected.push((
            fragment_asset_name(version, target),
            "application/json",
            DISTRIBUTION_MANIFEST_MAX_BYTES,
        ));
    }
    for (name, content_type) in [
        (
            DISTRIBUTION_BASH_INSTALLER_FILENAME,
            "text/x-shellscript; charset=utf-8",
        ),
        (
            DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
            "text/plain; charset=utf-8",
        ),
        (
            DISTRIBUTION_CMD_INSTALLER_FILENAME,
            "text/plain; charset=utf-8",
        ),
    ] {
        expected.push((name.to_string(), content_type, INSTALLER_MAX_BYTES));
    }
    expected.push((
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME.to_string(),
        "application/json",
        DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    ));
    expected
}

pub(crate) fn from_dir(dir: &Path, version: &Version) -> Result<SourceRelease> {
    let mut files = BTreeMap::new();
    for (name, _, max) in expected_assets(version) {
        let bytes = read_bounded_regular_file(&dir.join(&name), None, max, "release asset")?;
        files.insert(name, bytes);
    }
    verify(version, files)
}

/// Anonymous reads only: the source must be the published GitHub release.
pub(crate) fn from_github(
    client: &GithubReleaseClient,
    version: &Version,
) -> Result<SourceRelease> {
    let tag = format!("v{version}");
    let release = client
        .find_release(&tag)?
        .with_context(|| format!("GitHub has no published release `{tag}`; pass --from-dir"))?;
    if release.draft {
        bail!("GitHub release `{tag}` is still a draft; run `dist finalize --publish` first");
    }
    let remote = unique_assets(client.list_assets(release.id)?)?;
    let mut files = BTreeMap::new();
    for (name, _, max) in expected_assets(version) {
        let asset = remote
            .get(&name)
            .with_context(|| format!("GitHub release `{tag}` lacks `{name}`"))?;
        if asset.size > max {
            bail!(
                "GitHub asset `{name}` declares {} bytes, over {max}",
                asset.size
            );
        }
        let bytes = client.download_asset(asset.id)?;
        if bytes.len() as u64 != asset.size {
            bail!("GitHub asset `{name}` downloaded a different size than listed");
        }
        if let Some(digest) = asset.digest.as_deref()
            && digest != sha256_digest(&bytes)
        {
            bail!("GitHub asset `{name}` differs from its listed digest");
        }
        files.insert(name, bytes);
    }
    verify(version, files)
}

pub(crate) fn verify(
    version: &Version,
    mut files: BTreeMap<String, Vec<u8>>,
) -> Result<SourceRelease> {
    let tag = format!("v{version}");
    let aggregate_bytes = files
        .get(DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME)
        .context("release asset set lacks DISTRIBUTIONS.json")?;
    let aggregate = AggregateDistributionManifest::from_json_slice(aggregate_bytes)
        .context("parsing DISTRIBUTIONS.json")?;
    if aggregate.version != version.to_string() || aggregate.tag != tag {
        bail!(
            "DISTRIBUTIONS.json describes `{}` ({}), not `{tag}`",
            aggregate.tag,
            aggregate.version
        );
    }
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let declared = aggregate
            .platforms
            .iter()
            .find(|platform| platform.target == target)
            .with_context(|| format!("DISTRIBUTIONS.json lacks target `{target}`"))?;
        let fragment_name = fragment_asset_name(version, target);
        let fragment_bytes = files
            .get(&fragment_name)
            .with_context(|| format!("release asset set lacks `{fragment_name}`"))?;
        let fragment = PlatformDistributionFragment::from_json_slice(fragment_bytes)
            .with_context(|| format!("parsing `{fragment_name}`"))?;
        if &fragment != declared {
            bail!("`{fragment_name}` differs from its DISTRIBUTIONS.json entry");
        }
        validate_fragment_asset_names(&fragment)?;
        let bundle = files
            .get(&fragment.asset.name)
            .with_context(|| format!("release asset set lacks `{}`", fragment.asset.name))?;
        let bootstrap = files
            .get(&fragment.bootstrap.name)
            .with_context(|| format!("release asset set lacks `{}`", fragment.bootstrap.name))?;
        verify_platform_bytes(&fragment, bundle, bootstrap)?;
    }
    let mut assets = Vec::new();
    for (name, content_type, _) in expected_assets(version) {
        let bytes = files
            .remove(&name)
            .with_context(|| format!("release asset set lacks `{name}`"))?;
        if bytes.is_empty() {
            bail!("release asset `{name}` is empty");
        }
        assets.push(SourceAsset {
            digest: sha256_digest(&bytes),
            name,
            content_type,
            bytes,
        });
    }
    Ok(SourceRelease {
        version: version.clone(),
        tag,
        source_commit: aggregate.source_commit,
        assets,
    })
}
