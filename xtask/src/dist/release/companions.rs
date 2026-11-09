//! Companion tools in release assembly (PROP-019 `#companion-tools`): px is
//! uploaded per target as `px-<ver>-<target>.zip` with its listing
//! `px-<ver>-<target>.tool.json`, and `dist finalize` verifies every pair
//! and publishes `TOOLS.json` beside `DISTRIBUTIONS.json`. Both are optional:
//! a release without px is complete, and its `DISTRIBUTIONS.json` is the
//! unchanged shape vibe 1.0.11 and older read.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_publish::SUPPORTED_DISTRIBUTION_TARGETS;
use vibe_publish::release_manifest::{
    CompanionTool, DISTRIBUTION_TOOL_ASSET_MAX_BYTES, DISTRIBUTION_TOOLS_MANIFEST_FILENAME,
    DistributionToolsManifest,
};
use vibe_publish::{AggregateDistributionManifest, GithubReleaseAsset, sha256_digest};

use super::{ReleaseHost, VerifiedRelease};

const LISTING_MAX_BYTES: u64 = 64 * 1024;
const LISTING_SUFFIX: &str = ".tool.json";

/// Whether `name` is a companion asset or listing of a supported target.
pub(super) fn is_companion_asset(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("px-") else {
        return false;
    };
    let stem = rest
        .strip_suffix(LISTING_SUFFIX)
        .or_else(|| rest.strip_suffix(".zip"))
        .or_else(|| rest.strip_suffix(".tar.gz"));
    stem.is_some_and(|stem| {
        SUPPORTED_DISTRIBUTION_TARGETS
            .iter()
            .any(|target| stem.ends_with(&format!("-{target}")))
    })
}

/// `dist upload`'s px step: the pair `cargo xtask dist px` staged into the
/// same output directory for `target`, when it is there.
pub(super) fn upload_built(
    repo_root: &Path,
    client: &vibe_publish::GithubReleaseClient,
    version: &semver::Version,
    out_dir: &Path,
    target: &str,
) -> Result<()> {
    let px = crate::dist::px_tool::read_manifest(repo_root)?;
    let tag = format!("v{version}");
    let release = client
        .find_release(&tag)?
        .with_context(|| format!("prepared draft `{tag}` is absent"))?;
    if upload(client, release.id, out_dir, target, &px.version)? {
        println!(
            "dist upload: added px {} for `{target}` to draft `{tag}`",
            px.version
        );
    }
    Ok(())
}

/// `dist finalize`'s px step: publish `TOOLS.json` when the draft carries
/// companion tools.
pub(super) fn publish_listing(
    client: &vibe_publish::GithubReleaseClient,
    verified: &mut VerifiedRelease,
) -> Result<()> {
    let Some(tools) = &verified.tools else {
        return Ok(());
    };
    let asset = client.publish_asset_owned(
        verified.release_id,
        DISTRIBUTION_TOOLS_MANIFEST_FILENAME,
        "application/json",
        serde_json::to_vec_pretty(tools)?,
    )?;
    if asset.name != DISTRIBUTION_TOOLS_MANIFEST_FILENAME {
        bail!(
            "GitHub renamed final asset `{DISTRIBUTION_TOOLS_MANIFEST_FILENAME}` to `{}`",
            asset.name
        );
    }
    verified
        .asset_ids
        .insert(DISTRIBUTION_TOOLS_MANIFEST_FILENAME.to_string(), asset.id);
    Ok(())
}

/// Upload the px pair `cargo xtask dist px` left in `out_dir` for `target`,
/// after checking the archive against its listing. Nothing when absent.
pub(super) fn upload(
    host: &dyn ReleaseHost,
    release_id: u64,
    out_dir: &Path,
    target: &str,
    px_version: &str,
) -> Result<bool> {
    let listing_name = crate::dist::px_tool::fragment_name(px_version, target);
    let listing_path = out_dir.join(&listing_name);
    if !listing_path.is_file() {
        return Ok(false);
    }
    let listing = std::fs::read(&listing_path)
        .with_context(|| format!("reading `{}`", listing_path.display()))?;
    let tool: CompanionTool = serde_json::from_slice(&listing)
        .with_context(|| format!("parsing `{}`", listing_path.display()))?;
    if tool.target != target {
        bail!("`{listing_name}` lists target `{}`", tool.target);
    }
    let archive = std::fs::read(out_dir.join(&tool.asset.name))
        .with_context(|| format!("reading `{}`", tool.asset.name))?;
    verify(&tool, &archive)?;
    host.publish_asset(release_id, &tool.asset.name, "application/zip", archive)?;
    host.publish_asset(release_id, &listing_name, "application/json", listing)?;
    Ok(true)
}

fn verify(tool: &CompanionTool, bytes: &[u8]) -> Result<()> {
    if bytes.len() as u64 != tool.asset.size || sha256_digest(bytes) != tool.asset.digest {
        bail!(
            "companion asset `{}` does not match its listing ({} bytes, {})",
            tool.asset.name,
            tool.asset.size,
            tool.asset.digest
        );
    }
    Ok(())
}

/// Verify every uploaded companion pair of the draft and assemble
/// `TOOLS.json`; `None` when the draft carries none.
pub(super) fn assemble(
    host: &dyn ReleaseHost,
    assets: &BTreeMap<String, GithubReleaseAsset>,
    aggregate: &AggregateDistributionManifest,
) -> Result<Option<DistributionToolsManifest>> {
    let mut tools = Vec::new();
    for (name, asset) in assets {
        if !(is_companion_asset(name) && name.ends_with(LISTING_SUFFIX)) {
            continue;
        }
        if asset.size > LISTING_MAX_BYTES {
            bail!("companion listing `{name}` exceeds {LISTING_MAX_BYTES} bytes");
        }
        let listing = host.download_asset(asset.id, asset.size, LISTING_MAX_BYTES)?;
        let tool: CompanionTool = serde_json::from_slice(&listing)
            .with_context(|| format!("parsing companion listing `{name}`"))?;
        if !name.ends_with(&format!("-{}{LISTING_SUFFIX}", tool.target))
            || !SUPPORTED_DISTRIBUTION_TARGETS.contains(&tool.target.as_str())
        {
            bail!("companion listing `{name}` lists target `{}`", tool.target);
        }
        let archive = assets
            .get(&tool.asset.name)
            .with_context(|| format!("draft lacks companion asset `{}`", tool.asset.name))?;
        if archive.size != tool.asset.size || archive.size > DISTRIBUTION_TOOL_ASSET_MAX_BYTES {
            bail!(
                "companion asset `{}` is {} bytes, its listing says {}",
                tool.asset.name,
                archive.size,
                tool.asset.size
            );
        }
        let bytes =
            host.download_asset(archive.id, archive.size, DISTRIBUTION_TOOL_ASSET_MAX_BYTES)?;
        verify(&tool, &bytes)?;
        tools.push(tool);
    }
    if tools.is_empty() {
        return Ok(None);
    }
    tools.sort_by(|a, b| (&a.name, &a.target).cmp(&(&b.name, &b.target)));
    Ok(Some(DistributionToolsManifest {
        schema_version: 1,
        product: aggregate.product.clone(),
        version: aggregate.version.clone(),
        tag: aggregate.tag.clone(),
        source_commit: aggregate.source_commit.clone(),
        tools,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companion_names_are_px_pairs_of_supported_targets() {
        for name in [
            "px-0.11.0-x86_64-pc-windows-msvc.zip",
            "px-0.11.0-x86_64-pc-windows-msvc.tool.json",
            "px-0.11.0-x86_64-unknown-linux-gnu.tar.gz",
        ] {
            assert!(is_companion_asset(name), "{name}");
        }
        for name in [
            "px-0.11.0-riscv64gc-unknown-linux-gnu.zip",
            "evil-x86_64-pc-windows-msvc.zip",
            "px-0.11.0-x86_64-pc-windows-msvc.exe",
            "TOOLS.json",
        ] {
            assert!(!is_companion_asset(name), "{name}");
        }
    }
}
