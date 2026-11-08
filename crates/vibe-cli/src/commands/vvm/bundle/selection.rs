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

/// The release directory a bootstrap reads its assets from.
///
/// Two regimes, and the difference is who chose the base. A base **typed
/// on the command line** keeps the strict check it always had: exactly the
/// canonical HTTPS GitHub directory for this tag, no userinfo, no port, no
/// query, no fragment — a bootstrap is the one moment a vibe has no vibe
/// to check it, so an operator's typo must not become an origin.
///
/// A base a **remap row produced** (`from_remap`, `PROP-066#locator`) is
/// accepted for any location the one location rule reads
/// (`PROP-066#locations`): `https://`, `http://` or `file://` (a `path`
/// target arrives as a `file://` URL). The row is data the user wrote or
/// accepted on this machine, it is listed by `vibe remap list` and
/// explained by `vibe remap explain`, and every asset read from it is still
/// verified against the aggregate manifest's own digests and sizes — which
/// is the check that was ever load-bearing. Refusing a non-GitHub base
/// there would refuse the whole point of PROP-066: a closed network has no
/// github.com to be canonical about. `update` and `install` read the same
/// spellings, so the three verbs share one rule.
pub(super) fn validated_release_base(
    raw: &str,
    expected_tag: &str,
    from_remap: bool,
) -> Result<String> {
    if from_remap {
        vibe_core::remap::ByteLocation::parse(raw)?;
        return Ok(raw.trim_end_matches('/').to_string());
    }
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

pub(in crate::commands::vvm) fn current_target() -> Result<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "windows") => Ok("x86_64-pc-windows-msvc"),
        ("x86_64", "linux") if cfg!(target_env = "musl") => Ok("x86_64-unknown-linux-musl"),
        ("x86_64", "linux") => Ok("x86_64-unknown-linux-gnu"),
        ("x86_64", "macos") => Ok("x86_64-apple-darwin"),
        ("aarch64", "macos") => Ok("aarch64-apple-darwin"),
        (arch, os) => bail!("no vibevm binary distribution target for {arch}-{os}"),
    }
}
