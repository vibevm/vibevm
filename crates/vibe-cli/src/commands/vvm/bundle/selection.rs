//! Aggregate-manifest reading and platform selection.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#provenance");

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_publish::release_manifest::{
    AggregateDistributionManifest, DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    PlatformDistributionFragment,
};

use super::super::store::open_regular_no_follow;
use super::download::copy_download;

pub(super) fn read_aggregate(path: &Path) -> Result<AggregateDistributionManifest> {
    let (mut file, metadata) = open_regular_no_follow(path)
        .with_context(|| format!("reading aggregate manifest `{}`", path.display()))?;
    if metadata.len() > DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES {
        bail!(
            "aggregate manifest `{}` is not a bounded regular file",
            path.display()
        );
    }
    let mut bytes = Vec::with_capacity(metadata.len().min(64 * 1024) as usize);
    let copied = copy_download(
        &mut file,
        &mut bytes,
        DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    )?;
    if copied != metadata.len() {
        bail!("aggregate manifest changed size while reading");
    }
    AggregateDistributionManifest::from_json_slice(&bytes).map_err(Into::into)
}

pub(super) fn select_platform<'a>(
    aggregate: &'a AggregateDistributionManifest,
    expected_version: &str,
    target: &str,
) -> Result<&'a PlatformDistributionFragment> {
    aggregate.validate()?;
    let expected = semver::Version::parse(expected_version)
        .with_context(|| format!("invalid bootstrap version `{expected_version}`"))?
        .to_string();
    if aggregate.version != expected || aggregate.tag != format!("v{expected}") {
        bail!(
            "aggregate manifest version/tag `{}`/`{}` does not match requested `{expected}`",
            aggregate.version,
            aggregate.tag
        );
    }
    aggregate
        .platforms
        .iter()
        .find(|platform| platform.target == target)
        .with_context(|| format!("aggregate manifest has no platform `{target}`"))
}

pub(super) fn validated_release_base(raw: &str, expected_tag: &str) -> Result<String> {
    let url = reqwest::Url::parse(raw).context("parsing --release-base URL")?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != format!("/vibevm/vibevm/releases/download/{expected_tag}")
    {
        bail!("release base must be the canonical HTTPS GitHub directory for `{expected_tag}`");
    }
    Ok(raw.trim_end_matches('/').to_string())
}

pub(super) fn current_target() -> Result<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "windows") => Ok("x86_64-pc-windows-msvc"),
        ("x86_64", "linux") if cfg!(target_env = "musl") => Ok("x86_64-unknown-linux-musl"),
        ("x86_64", "linux") => Ok("x86_64-unknown-linux-gnu"),
        ("x86_64", "macos") => Ok("x86_64-apple-darwin"),
        ("aarch64", "macos") => Ok("aarch64-apple-darwin"),
        (arch, os) => bail!("no vibevm binary distribution target for {arch}-{os}"),
    }
}
