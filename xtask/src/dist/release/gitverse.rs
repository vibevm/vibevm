//! `cargo xtask dist gitverse-publish` — mirror one complete, verified
//! release onto GitVerse.
//!
//! A separate, explicit step: no other `dist` command calls it, so the owner
//! decides when (and whether) a release appears on GitVerse. It is
//! idempotent: an existing release for the tag is reused, an asset whose
//! bytes are already there is skipped, and a same-named asset with other
//! bytes is refused unless `--replace` is given. `--dry-run` loads no token,
//! contacts no GitVerse endpoint and prints the requests it would send.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use semver::Version;
use vibe_publish::release_manifest::DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME;
use vibe_publish::{GithubReleaseClient, Token, TokenSource, load_token_for_host, sha256_digest};

use super::{GITHUB_OWNER, GITHUB_REPO};
use crate::dist::build::absolute_out_dir;

mod client;
mod source;

pub(crate) use client::DEFAULT_GITVERSE_API_BASE;
use client::{
    CreateRelease, Endpoint, GitverseAsset, GitverseClient, GitverseRelease,
    asset_name_has_extension, validate_asset_name,
};
use source::{SourceAsset, SourceRelease};

/// Token lookup host: `VIBEVM_PUBLISH_TOKEN_GITVERSE`, then the legacy
/// `VIBEVM_PUBLISH_TOKEN`, then `~/.vibe/gitverse.publish.token`, then the
/// legacy `~/.vibe/git.publish.token` — the GitHub convention, per host.
const GITVERSE_TOKEN_HOST: &str = "gitverse.ru";
pub(crate) const DEFAULT_GITVERSE_OWNER: &str = "vibevm";
pub(crate) const DEFAULT_GITVERSE_REPO: &str = "vibevm";

pub(crate) struct GitverseOptions {
    pub(crate) version: String,
    pub(crate) from_dir: Option<PathBuf>,
    pub(crate) dry_run: bool,
    pub(crate) replace: bool,
    pub(crate) api_base: String,
    pub(crate) owner: String,
    pub(crate) repo: String,
}

pub(crate) fn publish(repo_root: &Path, options: &GitverseOptions) -> Result<()> {
    let version = Version::parse(&options.version)
        .with_context(|| format!("distribution version `{}` is not SemVer", options.version))?;
    let endpoint = Endpoint::new(&options.api_base, &options.owner, &options.repo)?;
    let source = match &options.from_dir {
        Some(dir) => source::from_dir(&absolute_out_dir(repo_root, dir), &version)?,
        None => source::from_github(
            &GithubReleaseClient::anonymous(GITHUB_OWNER, GITHUB_REPO)
                .context("constructing anonymous GitHub release reader")?,
            &version,
        )?,
    };
    let mut out = std::io::stdout().lock();
    if options.dry_run {
        return write_dry_run(&mut out, &endpoint, &source, options.replace);
    }
    let token =
        load_token_for_host(GITVERSE_TOKEN_HOST).context("loading GitVerse publish token")?;
    writeln!(
        out,
        "dist gitverse-publish: token from {}",
        token_origin(&token)
    )?;
    let client = GitverseClient::new(endpoint, token)?;
    let report = mirror_with(&client, &source, options.replace, &mut out)?;
    writeln!(
        out,
        "dist gitverse-publish: `{}` on {} (release id {}{}): {} uploaded, {} replaced, {} identical",
        source.tag,
        client.endpoint().repo_label(),
        report.release_id,
        if report.created {
            ", created"
        } else {
            ", reused"
        },
        report.uploaded.len(),
        report.replaced.len(),
        report.skipped.len()
    )?;
    Ok(())
}

fn token_origin(token: &Token) -> String {
    match token.source() {
        TokenSource::Explicit => "an explicit value".to_string(),
        TokenSource::EnvVar(name) => format!("${name}"),
        TokenSource::File(path) => format!("`{}`", path.display()),
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MirrorReport {
    pub(crate) release_id: u64,
    pub(crate) created: bool,
    pub(crate) uploaded: Vec<String>,
    pub(crate) replaced: Vec<String>,
    pub(crate) skipped: Vec<String>,
}

/// What a same-named remote asset means before any bytes are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Presence {
    Absent,
    /// Same size: identity is decided by downloading and hashing.
    SameSize {
        asset_id: u64,
    },
    DifferentSize {
        asset_id: u64,
        remote_size: u64,
    },
}

pub(crate) fn presence(source: &SourceAsset, remote: Option<&GitverseAsset>) -> Presence {
    match remote {
        None => Presence::Absent,
        Some(asset) if asset.size == source.bytes.len() as u64 => {
            Presence::SameSize { asset_id: asset.id }
        }
        Some(asset) => Presence::DifferentSize {
            asset_id: asset.id,
            remote_size: asset.size,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Upload,
    Skip,
    Differs { asset_id: u64 },
}

pub(crate) fn release_request(source: &SourceRelease) -> CreateRelease {
    CreateRelease {
        tag_name: source.tag.clone(),
        name: format!("vibevm {}", source.version),
        target_commitish: source.source_commit.clone(),
        body: format!(
            "vibevm {} distribution, mirrored from the GitHub release. Verify downloads through \
             `{DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME}`.",
            source.version
        ),
        draft: false,
        prerelease: !source.version.pre.is_empty(),
        is_authorized_only: false,
    }
}

pub(crate) fn mirror_with(
    client: &GitverseClient,
    source: &SourceRelease,
    replace: bool,
    out: &mut dyn Write,
) -> Result<MirrorReport> {
    for asset in &source.assets {
        validate_asset_name(&asset.name)?;
    }
    let (release, created) = match client.find_release(&source.tag)? {
        Some(release) => (release, false),
        None => (client.create_release(&release_request(source))?, true),
    };
    warn_about_release(&release, out)?;
    let remote = unique_by_name(client.list_assets(release.id)?)?;
    let mut actions = Vec::new();
    for asset in &source.assets {
        let action = match presence(asset, remote.get(&asset.name)) {
            Presence::Absent => Action::Upload,
            Presence::SameSize { asset_id } => {
                let bytes = client.download_asset(&remote[&asset.name])?;
                if sha256_digest(&bytes) == asset.digest {
                    Action::Skip
                } else {
                    Action::Differs { asset_id }
                }
            }
            Presence::DifferentSize { asset_id, .. } => Action::Differs { asset_id },
        };
        actions.push(action);
    }
    let differing = source
        .assets
        .iter()
        .zip(&actions)
        .filter(|(_, action)| matches!(action, Action::Differs { .. }))
        .map(|(asset, _)| asset.name.clone())
        .collect::<Vec<_>>();
    if !differing.is_empty() && !replace {
        bail!(
            "GitVerse release `{}` already holds different bytes for {differing:?}; nothing was \
             uploaded. Pass --replace to replace them.",
            source.tag
        );
    }
    let mut report = MirrorReport {
        release_id: release.id,
        created,
        ..MirrorReport::default()
    };
    // Remove replaced assets manifest-first, so no reader sees the old
    // DISTRIBUTIONS.json beside a new bundle.
    let mut deletions = source
        .assets
        .iter()
        .zip(&actions)
        .filter_map(|(asset, action)| match action {
            Action::Differs { asset_id } => Some((asset.name.as_str(), *asset_id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    deletions.sort_by_key(|(name, _)| *name != DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME);
    for (name, asset_id) in deletions {
        writeln!(out, "  delete  {name} (differs)")?;
        client.delete_asset(release.id, asset_id)?;
    }
    for (asset, action) in source.assets.iter().zip(&actions) {
        match action {
            Action::Skip => {
                writeln!(out, "  same    {}", asset.name)?;
                report.skipped.push(asset.name.clone());
                continue;
            }
            Action::Upload => report.uploaded.push(asset.name.clone()),
            Action::Differs { .. } => report.replaced.push(asset.name.clone()),
        }
        writeln!(
            out,
            "  upload  {} ({} bytes)",
            asset.name,
            asset.bytes.len()
        )?;
        client.upload_asset(
            release.id,
            &asset.name,
            asset.content_type,
            &asset.digest,
            &asset.bytes,
        )?;
    }
    verify_remote(client, release.id, source, out)?;
    Ok(report)
}

fn warn_about_release(release: &GitverseRelease, out: &mut dyn Write) -> Result<()> {
    if release.draft {
        writeln!(
            out,
            "  warning: GitVerse release `{}` is a draft; publish it on GitVerse",
            release.tag_name
        )?;
    }
    if release.is_authorized_only {
        writeln!(
            out,
            "  warning: GitVerse release `{}` allows downloads only to signed-in users",
            release.tag_name
        )?;
    }
    Ok(())
}

fn unique_by_name(assets: Vec<GitverseAsset>) -> Result<BTreeMap<String, GitverseAsset>> {
    let mut by_name = BTreeMap::new();
    for asset in assets {
        let name = asset.name.clone();
        if by_name.insert(name.clone(), asset).is_some() {
            bail!("GitVerse release holds asset `{name}` more than once; remove the duplicate");
        }
    }
    Ok(by_name)
}

fn verify_remote(
    client: &GitverseClient,
    release_id: u64,
    source: &SourceRelease,
    out: &mut dyn Write,
) -> Result<()> {
    let remote = unique_by_name(client.list_assets(release_id)?)?;
    for asset in &source.assets {
        let stored = remote
            .get(&asset.name)
            .with_context(|| format!("GitVerse lost `{}` after upload", asset.name))?;
        if stored.size != asset.bytes.len() as u64 {
            bail!(
                "GitVerse lists `{}` with {} bytes after upload; expected {}",
                asset.name,
                stored.size,
                asset.bytes.len()
            );
        }
    }
    let expected = source
        .assets
        .iter()
        .map(|asset| asset.name.as_str())
        .collect::<BTreeSet<_>>();
    for name in remote
        .keys()
        .filter(|name| !expected.contains(name.as_str()))
    {
        writeln!(
            out,
            "  warning: GitVerse release also holds `{name}` (left untouched)"
        )?;
    }
    Ok(())
}

/// The requests a real run sends, for a release and asset list that do not
/// exist yet; conditional steps say when they apply. No token is loaded.
fn placeholders(line: String) -> String {
    line.replace(
        "/releases/0/assets/0",
        "/releases/{release_id}/assets/{asset_id}",
    )
    .replace("/releases/0/", "/releases/{release_id}/")
}

pub(crate) fn write_dry_run(
    out: &mut dyn Write,
    endpoint: &Endpoint,
    source: &SourceRelease,
    replace: bool,
) -> Result<()> {
    for asset in &source.assets {
        validate_asset_name(&asset.name)?;
    }
    writeln!(
        out,
        "dist gitverse-publish --dry-run: `{}` -> {} ({} verified assets); token not loaded, \
         nothing sent",
        source.tag,
        endpoint.repo_label(),
        source.assets.len()
    )?;
    writeln!(
        out,
        "every request: Accept: {}; Authorization: Bearer <token from \
         $VIBEVM_PUBLISH_TOKEN_GITVERSE or ~/.vibe/gitverse.publish.token>",
        client::GITVERSE_ACCEPT
    )?;
    writeln!(
        out,
        "  {}",
        placeholders(endpoint.release_by_tag(&source.tag).describe())
    )?;
    writeln!(
        out,
        "  {}  [on 404: scan pages until <{} entries]",
        placeholders(endpoint.list_releases(1).describe()),
        client::RELEASES_PER_PAGE
    )?;
    writeln!(
        out,
        "  {}  [only when no release has the tag]",
        placeholders(
            endpoint
                .create_release(&release_request(source))?
                .describe()
        )
    )?;
    writeln!(
        out,
        "  {}  [{{release_id}} from the release]",
        placeholders(endpoint.list_assets(0, 1).describe())
    )?;
    writeln!(
        out,
        "  GET    <browser_download_url>  [per same-name same-size asset: compare SHA-256, \
         identical => skip]"
    )?;
    if replace {
        writeln!(
            out,
            "  {}  [--replace: per differing asset, DISTRIBUTIONS.json first]",
            placeholders(endpoint.delete_asset(0, 0).describe())
        )?;
    } else {
        writeln!(
            out,
            "  [a differing same-name asset stops the run before any upload]"
        )?;
    }
    for asset in &source.assets {
        let planned = endpoint.upload_asset(
            0,
            &asset.name,
            asset.content_type,
            asset.bytes.len() as u64,
            &asset.digest,
        );
        writeln!(out, "  {}", placeholders(planned.describe()))?;
    }
    writeln!(
        out,
        "  {}  [final check: every asset present with its size]",
        placeholders(endpoint.list_assets(0, 1).describe())
    )?;
    for asset in source
        .assets
        .iter()
        .filter(|asset| !asset_name_has_extension(&asset.name))
    {
        writeln!(
            out,
            "  note: `{}` has no file extension; the GitVerse docs ask for one",
            asset.name
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
