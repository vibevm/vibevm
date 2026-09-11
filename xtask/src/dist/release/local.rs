use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, bail};
use semver::Version;
use vibe_publish::{
    DISTRIBUTION_BOOTSTRAP_MAX_BYTES, DISTRIBUTION_BUNDLE_MAX_BYTES,
    DISTRIBUTION_MANIFEST_MAX_BYTES, PlatformDistributionFragment,
};

use super::{ReleaseHost, validate_fragment_asset_names, verify_platform_bytes};
use crate::dist::build::{fragment_asset_name, validate_expected_version};
use crate::dist::snapshot::GitIdentity;

pub(super) struct LocalPlatform {
    pub(super) fragment: PlatformDistributionFragment,
    pub(super) fragment_name: String,
    pub(super) asset_bytes: Vec<u8>,
    pub(super) bootstrap_bytes: Vec<u8>,
    pub(super) fragment_bytes: Vec<u8>,
}

pub(super) fn read_local_platform(
    asset_path: &Path,
    fragment_path: &Path,
    identity: &GitIdentity,
) -> Result<LocalPlatform> {
    let fragment_bytes = read_bounded_regular_file(
        fragment_path,
        None,
        DISTRIBUTION_MANIFEST_MAX_BYTES,
        "distribution fragment",
    )?;
    let fragment = PlatformDistributionFragment::from_json_slice(&fragment_bytes)?;
    validate_expected_version(
        &Version::parse(&fragment.version).context("fragment version is not SemVer")?,
    )?;
    validate_fragment_asset_names(&fragment)?;
    if fragment.source_commit != identity.commit {
        bail!(
            "fragment source commit `{}` differs from pinned release commit `{}`",
            fragment.source_commit,
            identity.commit
        );
    }
    if fragment.bundle.source_archive.tree_oid != identity.tree {
        bail!(
            "fragment source tree `{}` differs from pinned release tree `{}`",
            fragment.bundle.source_archive.tree_oid,
            identity.tree
        );
    }
    let asset_name = file_name(asset_path)?;
    if asset_name != fragment.asset.name {
        bail!(
            "asset path names `{asset_name}`, but the fragment binds `{}`",
            fragment.asset.name
        );
    }
    let fragment_name = file_name(fragment_path)?;
    let expected_fragment = fragment_asset_name(
        &Version::parse(&fragment.version).context("fragment version is not SemVer")?,
        &fragment.target,
    );
    if fragment_name != expected_fragment {
        bail!("fragment path names `{fragment_name}`; expected `{expected_fragment}`");
    }
    let asset_bytes = read_bounded_regular_file(
        asset_path,
        Some(fragment.asset.size),
        DISTRIBUTION_BUNDLE_MAX_BYTES,
        "distribution bundle",
    )?;
    let bootstrap_path = fragment_path
        .parent()
        .context("fragment path has no parent")?
        .join(&fragment.bootstrap.name);
    let bootstrap_bytes = read_bounded_regular_file(
        &bootstrap_path,
        Some(fragment.bootstrap.size),
        DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        "raw bootstrap",
    )?;
    verify_platform_bytes(&fragment, &asset_bytes, &bootstrap_bytes)?;
    Ok(LocalPlatform {
        fragment,
        fragment_name,
        asset_bytes,
        bootstrap_bytes,
        fragment_bytes,
    })
}

pub(super) fn read_bounded_regular_file(
    path: &Path,
    expected_size: Option<u64>,
    max_size: u64,
    label: &str,
) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting {label} `{}`", path.display()))?;
    if !metadata.file_type().is_file() {
        bail!(
            "{label} `{}` must be a regular non-symlink file",
            path.display()
        );
    }
    let declared = metadata.len();
    if declared > max_size || expected_size.is_some_and(|expected| expected != declared) {
        bail!(
            "{label} `{}` declares {declared} bytes; expected {:?} within the independent {max_size}-byte limit",
            path.display(),
            expected_size
        );
    }
    let limit = expected_size
        .unwrap_or(declared)
        .min(max_size)
        .saturating_add(1);
    let mut bytes = Vec::with_capacity(usize::try_from(declared.min(64 * 1024)).unwrap_or(0));
    File::open(path)
        .with_context(|| format!("opening {label} `{}`", path.display()))?
        .take(limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading bounded {label} `{}`", path.display()))?;
    if bytes.len() as u64 != declared {
        bail!(
            "{label} `{}` changed size while it was read",
            path.display()
        );
    }
    Ok(bytes)
}

pub(super) fn upload_with(host: &dyn ReleaseHost, local: &mut LocalPlatform) -> Result<()> {
    let release = host
        .find_release(&local.fragment.tag)?
        .with_context(|| format!("prepared draft `{}` is absent", local.fragment.tag))?;
    if !release.draft {
        bail!(
            "release `{}` is already published; run `dist prepare --version {}` before replacing assets",
            local.fragment.tag,
            local.fragment.version
        );
    }
    host.publish_asset(
        release.id,
        &local.fragment.asset.name,
        "application/zip",
        std::mem::take(&mut local.asset_bytes),
    )?;
    host.publish_asset(
        release.id,
        &local.fragment.bootstrap.name,
        "application/octet-stream",
        std::mem::take(&mut local.bootstrap_bytes),
    )?;
    host.publish_asset(
        release.id,
        &local.fragment_name,
        "application/json",
        std::mem::take(&mut local.fragment_bytes),
    )?;
    Ok(())
}

fn file_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .with_context(|| format!("path `{}` has no UTF-8 file name", path.display()))
}
