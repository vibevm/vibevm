//! Mutable GitHub Release orchestration for independently built native bundles.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use semver::Version;
use vibe_publish::release_manifest::{
    DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME, DISTRIBUTION_BASH_INSTALLER_FILENAME,
    DISTRIBUTION_BOOTSTRAP_MAX_BYTES, DISTRIBUTION_BUNDLE_MAX_BYTES,
    DISTRIBUTION_MANIFEST_FILENAME, DISTRIBUTION_MANIFEST_MAX_BYTES,
    DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME, DISTRIBUTION_SOURCE_ARCHIVE_FILENAME,
};
use vibe_publish::{
    AggregateDistributionManifest, CreateGithubRelease, GithubGitRef, GithubMakeLatest,
    GithubRelease, GithubReleaseAsset, GithubReleaseClient, PlatformDistributionFragment,
    SUPPORTED_DISTRIBUTION_TARGETS, UpdateGithubRelease, load_token_for_host, sha256_digest,
};

use super::build::{
    absolute_out_dir, aggregate_asset_name, bootstrap_asset_name, bundle_asset_name,
    fragment_asset_name, validate_expected_version, verified_bundle_entries,
    workspace_version_at_commit,
};
use super::snapshot::{GitIdentity, committed_identity, read_commit_file};

const GITHUB_OWNER: &str = "vibevm";
const GITHUB_REPO: &str = "vibevm";

mod local;
#[cfg(test)]
use local::{LocalPlatform, read_bounded_regular_file};
use local::{read_local_platform, upload_with};

trait ReleaseHost {
    fn find_release(&self, tag: &str) -> Result<Option<GithubRelease>>;
    fn get_tag_ref(&self, tag: &str) -> Result<GithubGitRef>;
    fn create_release(&self, request: &CreateGithubRelease) -> Result<GithubRelease>;
    fn delete_release(&self, release_id: u64) -> Result<()>;
    fn force_move_or_create_tag(&self, tag: &str, source_commit: &str) -> Result<GithubGitRef>;
    fn list_assets(&self, release_id: u64) -> Result<Vec<GithubReleaseAsset>>;
    fn download_asset(&self, asset_id: u64, expected_size: u64, max_size: u64) -> Result<Vec<u8>>;
    fn cleanup_temporary_assets(&self, release_id: u64, canonical_names: &[&str]) -> Result<()>;
    fn publish_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<GithubReleaseAsset>;
}

impl ReleaseHost for GithubReleaseClient {
    fn find_release(&self, tag: &str) -> Result<Option<GithubRelease>> {
        Ok(self.find_release_authenticated(tag)?)
    }

    fn get_tag_ref(&self, tag: &str) -> Result<GithubGitRef> {
        Ok(self.get_tag_ref_authenticated(tag)?)
    }

    fn create_release(&self, request: &CreateGithubRelease) -> Result<GithubRelease> {
        Ok(self.create_release(request)?)
    }

    fn delete_release(&self, release_id: u64) -> Result<()> {
        Ok(self.delete_release(release_id)?)
    }

    fn force_move_or_create_tag(&self, tag: &str, source_commit: &str) -> Result<GithubGitRef> {
        Ok(self.force_move_or_create_tag(tag, source_commit)?)
    }

    fn list_assets(&self, release_id: u64) -> Result<Vec<GithubReleaseAsset>> {
        Ok(self.list_assets_authenticated(release_id)?)
    }

    fn download_asset(&self, asset_id: u64, expected_size: u64, max_size: u64) -> Result<Vec<u8>> {
        Ok(self.download_asset_authenticated_bounded(asset_id, expected_size, max_size)?)
    }

    fn cleanup_temporary_assets(&self, release_id: u64, canonical_names: &[&str]) -> Result<()> {
        Ok(self.cleanup_temporary_assets(release_id, canonical_names)?)
    }

    fn publish_asset(
        &self,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<GithubReleaseAsset> {
        Ok(self.publish_asset_owned(release_id, name, content_type, bytes)?)
    }
}

pub(crate) fn prepare(repo_root: &Path, raw_version: &str) -> Result<()> {
    let identity = committed_identity(repo_root, true)?;
    let version = checked_version(repo_root, raw_version, &identity)?;
    let client = write_client()?;
    let release = prepare_with(&client, &version, &identity)?;
    println!(
        "dist prepare: fresh draft `{}` at {} (release id {}, prior release replaced)",
        release.tag_name, identity.commit, release.id
    );
    Ok(())
}

pub(crate) fn upload(repo_root: &Path, asset_path: &Path, fragment_path: &Path) -> Result<()> {
    let identity = committed_identity(repo_root, true)?;
    let mut local = read_local_platform(asset_path, fragment_path, &identity)?;
    let client = write_client()?;
    upload_with(&client, &mut local)?;
    println!(
        "dist upload: replaced `{}`, `{}`, and `{}` in draft `{}`",
        local.fragment.asset.name,
        local.fragment.bootstrap.name,
        local.fragment_name,
        local.fragment.tag
    );
    Ok(())
}

pub(crate) fn upload_built(repo_root: &Path, target: &str, out_dir: &Path) -> Result<()> {
    if !SUPPORTED_DISTRIBUTION_TARGETS.contains(&target) {
        bail!("unsupported distribution target `{target}`");
    }
    let identity = committed_identity(repo_root, true)?;
    let version = workspace_version_at_commit(repo_root, &identity)?;
    validate_expected_version(&version)?;
    let out_dir = absolute_out_dir(repo_root, out_dir);
    upload(
        repo_root,
        &out_dir.join(bundle_asset_name(&version, target)),
        &out_dir.join(fragment_asset_name(&version, target)),
    )
}

pub(crate) fn status(repo_root: &Path, raw_version: &str) -> Result<()> {
    let identity = committed_identity(repo_root, false)?;
    let version = checked_version(repo_root, raw_version, &identity)?;
    let client = write_client()?;
    let tag = format!("v{version}");
    let release = client
        .find_release(&tag)?
        .with_context(|| format!("GitHub release `{tag}` does not exist; run `dist prepare`"))?;
    let assets = unique_assets(client.list_assets(release.id)?)?;
    println!(
        "dist status: {} ({})",
        tag,
        if release.draft { "draft" } else { "published" }
    );
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let expected = [
            bundle_asset_name(&version, target),
            bootstrap_asset_name(target),
            fragment_asset_name(&version, target),
        ];
        let present = expected
            .iter()
            .filter(|name| assets.contains_key(*name))
            .count();
        println!("  {target}: {present}/3 assets");
    }
    for optional in [
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
        DISTRIBUTION_BASH_INSTALLER_FILENAME,
        DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
    ] {
        println!(
            "  {optional}: {}",
            if assets.contains_key(optional) {
                "present"
            } else {
                "absent"
            }
        );
    }
    Ok(())
}

pub(crate) fn finalize(repo_root: &Path, raw_version: &str, publish: bool) -> Result<()> {
    let identity = committed_identity(repo_root, true)?;
    let version = checked_version(repo_root, raw_version, &identity)?;
    let client = write_client()?;
    let mut verified = finalize_with(&client, &version, &identity)?;
    let bytes = verified.aggregate.to_json_bytes()?;
    let aggregate_asset = client.publish_asset_owned(
        verified.release_id,
        aggregate_asset_name(),
        "application/json",
        bytes,
    )?;
    let bash_installer = read_commit_file(
        repo_root,
        &identity.commit,
        "distribution/install/install.sh",
    )?;
    let powershell_installer = read_commit_file(
        repo_root,
        &identity.commit,
        "distribution/install/install.ps1",
    )?;
    if bash_installer.is_empty() || powershell_installer.is_empty() {
        bail!("committed bootstrap installer assets must be non-empty");
    }
    let bash_asset = client.publish_asset_owned(
        verified.release_id,
        DISTRIBUTION_BASH_INSTALLER_FILENAME,
        "text/x-shellscript; charset=utf-8",
        bash_installer,
    )?;
    let powershell_asset = client.publish_asset_owned(
        verified.release_id,
        DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
        "text/plain; charset=utf-8",
        powershell_installer,
    )?;
    for (asset, expected_name) in [
        (aggregate_asset, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME),
        (bash_asset, DISTRIBUTION_BASH_INSTALLER_FILENAME),
        (powershell_asset, DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME),
    ] {
        if asset.name != expected_name {
            bail!(
                "GitHub renamed final asset `{expected_name}` to `{}`",
                asset.name
            );
        }
        verified
            .asset_ids
            .insert(expected_name.to_string(), asset.id);
    }
    ensure_release_ready_for_publish(&client, &verified)?;
    if publish {
        client.update_release(
            verified.release_id,
            &UpdateGithubRelease {
                draft: Some(false),
                make_latest: Some(GithubMakeLatest::Legacy),
                ..UpdateGithubRelease::default()
            },
        )?;
        println!(
            "dist finalize: published `{}` with {} verified native bundles",
            verified.aggregate.tag,
            verified.aggregate.platforms.len()
        );
    } else {
        println!(
            "dist finalize: `{}` is complete and verified; draft retained (pass --publish to publish)",
            verified.aggregate.tag
        );
    }
    Ok(())
}

struct VerifiedRelease {
    release_id: u64,
    aggregate: AggregateDistributionManifest,
    asset_ids: BTreeMap<String, u64>,
}

fn write_client() -> Result<GithubReleaseClient> {
    let token = load_token_for_host("github.com").context("loading GitHub publish token")?;
    GithubReleaseClient::new(token, GITHUB_OWNER, GITHUB_REPO)
        .context("constructing GitHub release client")
}

fn checked_version(repo_root: &Path, raw: &str, identity: &GitIdentity) -> Result<Version> {
    let requested = Version::parse(raw)
        .with_context(|| format!("distribution version `{raw}` is not SemVer"))?;
    let workspace = workspace_version_at_commit(repo_root, identity)?;
    if requested != workspace {
        bail!(
            "requested distribution version `{requested}` differs from committed workspace version \
             `{workspace}`"
        );
    }
    Ok(requested)
}

fn prepare_with(
    host: &dyn ReleaseHost,
    version: &Version,
    identity: &GitIdentity,
) -> Result<GithubRelease> {
    let mut request = CreateGithubRelease::for_version(version, identity.commit.clone());
    request.draft = true;
    request.name = format!("vibevm {version}");
    request.body = format!(
        "Mutable vibevm {version} distribution. Verify downloads through \
         `{DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME}`."
    );
    if let Some(existing) = host.find_release(&request.tag_name)? {
        host.delete_release(existing.id)?;
    }
    let reference = host.force_move_or_create_tag(&request.tag_name, &identity.commit)?;
    validate_tag_ref(&reference, &request.tag_name, &identity.commit)?;
    host.create_release(&request)
}

fn finalize_with(
    host: &dyn ReleaseHost,
    version: &Version,
    identity: &GitIdentity,
) -> Result<VerifiedRelease> {
    let tag = format!("v{version}");
    let release = host
        .find_release(&tag)?
        .with_context(|| format!("prepared draft `{tag}` is absent"))?;
    if !release.draft {
        bail!("release `{tag}` is already published; run `dist prepare` before rebuilding it");
    }
    validate_tag_ref(&host.get_tag_ref(&tag)?, &tag, &identity.commit)?;
    host.cleanup_temporary_assets(
        release.id,
        &[
            DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
            DISTRIBUTION_BASH_INSTALLER_FILENAME,
            DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
        ],
    )?;
    let assets = unique_assets(host.list_assets(release.id)?)?;
    validate_remote_asset_set(&assets, version)?;
    let mut fragments = Vec::new();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let fragment_name = fragment_asset_name(version, target);
        let fragment_asset = assets
            .get(&fragment_name)
            .with_context(|| format!("draft lacks fragment `{fragment_name}`"))?;
        if fragment_asset.size > DISTRIBUTION_MANIFEST_MAX_BYTES {
            bail!(
                "fragment `{fragment_name}` metadata declares {} bytes, exceeding the {}-byte limit",
                fragment_asset.size,
                DISTRIBUTION_MANIFEST_MAX_BYTES
            );
        }
        let fragment_bytes = host.download_asset(
            fragment_asset.id,
            fragment_asset.size,
            DISTRIBUTION_MANIFEST_MAX_BYTES,
        )?;
        let fragment = PlatformDistributionFragment::from_json_slice(&fragment_bytes)?;
        validate_fragment_asset_names(&fragment)?;
        if fragment.target != target {
            bail!(
                "fragment asset `{fragment_name}` declares target `{}` instead of `{target}`",
                fragment.target
            );
        }
        if fragment.version != version.to_string()
            || fragment.source_commit != identity.commit
            || fragment.bundle.source_archive.tree_oid != identity.tree
        {
            bail!(
                "fragment `{fragment_name}` does not belong to version {version} at commit/tree \
                 {}/{}",
                identity.commit,
                identity.tree
            );
        }
        let bundle_asset = assets
            .get(&fragment.asset.name)
            .with_context(|| format!("draft lacks bundle `{}`", fragment.asset.name))?;
        let bootstrap_asset = assets
            .get(&fragment.bootstrap.name)
            .with_context(|| format!("draft lacks bootstrap `{}`", fragment.bootstrap.name))?;
        validate_remote_metadata(bundle_asset, &fragment.asset, DISTRIBUTION_BUNDLE_MAX_BYTES)?;
        validate_remote_metadata(
            bootstrap_asset,
            &fragment.bootstrap,
            DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        )?;
        let bundle_bytes = host.download_asset(
            bundle_asset.id,
            fragment.asset.size,
            DISTRIBUTION_BUNDLE_MAX_BYTES,
        )?;
        let bootstrap_bytes = host.download_asset(
            bootstrap_asset.id,
            fragment.bootstrap.size,
            DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        )?;
        verify_platform_bytes(&fragment, &bundle_bytes, &bootstrap_bytes)?;
        fragments.push(fragment);
    }
    let aggregate = AggregateDistributionManifest::from_platforms(fragments)
        .context("assembling aggregate distribution manifest")?;
    Ok(VerifiedRelease {
        release_id: release.id,
        aggregate,
        asset_ids: assets
            .into_iter()
            .map(|(name, asset)| (name, asset.id))
            .collect(),
    })
}

fn ensure_release_ready_for_publish(
    host: &dyn ReleaseHost,
    verified: &VerifiedRelease,
) -> Result<()> {
    let current = host
        .find_release(&verified.aggregate.tag)?
        .context("verified draft disappeared before publication")?;
    if current.id != verified.release_id
        || !current.draft
        || current.tag_name != verified.aggregate.tag
    {
        bail!("verified draft identity changed before publication; refusing to publish");
    }
    validate_tag_ref(
        &host.get_tag_ref(&verified.aggregate.tag)?,
        &verified.aggregate.tag,
        &verified.aggregate.source_commit,
    )?;
    let current_ids = unique_assets(host.list_assets(verified.release_id)?)?
        .into_iter()
        .map(|(name, asset)| (name, asset.id))
        .collect::<BTreeMap<_, _>>();
    if current_ids != verified.asset_ids {
        bail!("verified draft asset set changed before publication; refusing to publish");
    }
    Ok(())
}

fn validate_tag_ref(reference: &GithubGitRef, tag: &str, source_commit: &str) -> Result<()> {
    let expected_ref = format!("refs/tags/{tag}");
    if reference.reference != expected_ref
        || reference.object.kind != "commit"
        || reference.object.sha != source_commit
    {
        bail!(
            "release tag provenance mismatch: expected `{expected_ref}` to point directly to \
             commit `{source_commit}`, got `{}` / `{}` / `{}`",
            reference.reference,
            reference.object.kind,
            reference.object.sha
        );
    }
    Ok(())
}

fn validate_remote_metadata(
    remote: &GithubReleaseAsset,
    declared: &vibe_publish::DistributionAsset,
    max_size: u64,
) -> Result<()> {
    if remote.size != declared.size || remote.size > max_size {
        bail!(
            "GitHub asset `{}` metadata declares {} bytes; fragment requires {} within the independent {}-byte limit",
            remote.name,
            remote.size,
            declared.size,
            max_size
        );
    }
    if let Some(remote_digest) = remote.digest.as_deref()
        && remote_digest != declared.digest
    {
        bail!(
            "GitHub asset `{}` metadata digest differs from the platform fragment",
            remote.name
        );
    }
    Ok(())
}

fn verify_platform_bytes(
    fragment: &PlatformDistributionFragment,
    bundle_bytes: &[u8],
    bootstrap_bytes: &[u8],
) -> Result<()> {
    if bundle_bytes.len() as u64 != fragment.asset.size
        || sha256_digest(bundle_bytes) != fragment.asset.digest
    {
        bail!(
            "bundle `{}` failed size/SHA-256 verification",
            fragment.asset.name
        );
    }
    if bootstrap_bytes.len() as u64 != fragment.bootstrap.size
        || sha256_digest(bootstrap_bytes) != fragment.bootstrap.digest
    {
        bail!(
            "bootstrap `{}` failed size/SHA-256 verification",
            fragment.bootstrap.name
        );
    }
    let entries = verified_bundle_entries(bundle_bytes, &fragment.bundle)?;
    let vibe_path = fragment
        .bundle
        .components
        .iter()
        .find(|component| component.name == vibe_publish::DistributionComponentName::Vibe)
        .map(|component| component.path.as_str())
        .context("validated fragment lacks vibe component")?;
    let embedded_vibe = entries
        .iter()
        .find(|entry| entry.name == vibe_path)
        .context("bundle lacks its vibe component")?;
    if embedded_vibe.bytes != bootstrap_bytes {
        bail!("raw bootstrap differs byte-for-byte from the bundle's vibe component");
    }
    let source = entries
        .iter()
        .find(|entry| entry.name == DISTRIBUTION_SOURCE_ARCHIVE_FILENAME)
        .context("bundle lacks source archive")?;
    if sha256_digest(&source.bytes) != fragment.bundle.source_archive.digest {
        bail!("source archive digest differs from DISTRIBUTION.json");
    }
    let manifest = entries
        .iter()
        .find(|entry| entry.name == DISTRIBUTION_MANIFEST_FILENAME)
        .context("bundle lacks DISTRIBUTION.json")?;
    if vibe_publish::BundleDistributionManifest::from_json_slice(&manifest.bytes)?
        != fragment.bundle
    {
        bail!("fragment bundle and embedded DISTRIBUTION.json differ");
    }
    Ok(())
}

fn validate_fragment_asset_names(fragment: &PlatformDistributionFragment) -> Result<()> {
    let version = Version::parse(&fragment.version).context("fragment version is not SemVer")?;
    let expected_bundle = bundle_asset_name(&version, &fragment.target);
    let expected_bootstrap = bootstrap_asset_name(&fragment.target);
    if fragment.asset.name != expected_bundle || fragment.bootstrap.name != expected_bootstrap {
        bail!(
            "fragment `{}` asset names do not match target `{}` (bundle `{}`, bootstrap `{}`)",
            fragment.version,
            fragment.target,
            expected_bundle,
            expected_bootstrap
        );
    }
    Ok(())
}

fn validate_remote_asset_set(
    assets: &BTreeMap<String, GithubReleaseAsset>,
    version: &Version,
) -> Result<()> {
    let mut required = BTreeSet::new();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        required.insert(bundle_asset_name(version, target));
        required.insert(bootstrap_asset_name(target));
        required.insert(fragment_asset_name(version, target));
    }
    let optional = BTreeSet::from([
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME.to_string(),
        DISTRIBUTION_BASH_INSTALLER_FILENAME.to_string(),
        DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME.to_string(),
    ]);
    let actual = assets.keys().cloned().collect::<BTreeSet<_>>();
    let missing = required.difference(&actual).cloned().collect::<Vec<_>>();
    let unexpected = actual
        .difference(&required)
        .filter(|name| !optional.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        bail!(
            "draft release asset set is incomplete or ambiguous (missing: {missing:?}; \
             unexpected: {unexpected:?})"
        );
    }
    Ok(())
}

fn unique_assets(assets: Vec<GithubReleaseAsset>) -> Result<BTreeMap<String, GithubReleaseAsset>> {
    let mut by_name = BTreeMap::new();
    for asset in assets {
        if by_name.insert(asset.name.clone(), asset).is_some() {
            bail!("draft release contains duplicate asset name");
        }
    }
    Ok(by_name)
}

#[cfg(test)]
#[path = "release/tests.rs"]
mod tests;
