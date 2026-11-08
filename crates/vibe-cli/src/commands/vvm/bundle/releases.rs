//! Release directories, release listings and bare-bundle drops per release
//! source (`PROP-019#release-sources`).
//!
//! The release lane asks three questions of a source: where are the files
//! of version V, which versions exist, and — for a hand-filled drop that
//! holds only a platform zip — what manifest describes it. GitHub and an
//! `$VIBEVM_RELEASE_ORIGIN` mirror answer the first through the remap row
//! and name the newest release through their `latest/download` alias; a
//! drop and a raw repository answer through the same row and list by
//! directory scan, `RELEASES.json` or Nexus search; GitVerse answers
//! through its REST API.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#release-sources");

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use vibe_core::remap::drop::{bundle_file_name, bundle_version, release_dir_version};
use vibe_core::remap::{Artifact, ArtifactKind, ByteLocation, Locator};
use vibe_publish::release_manifest::{
    BundleDistributionManifest, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
    DISTRIBUTION_MANIFEST_FILENAME, DISTRIBUTION_MANIFEST_MAX_BYTES, DistributionAsset,
    DistributionComponentName, PlatformDistributionFragment,
};
use vibe_publish::token::Token;

use super::api::{ApiClient, Fetched};
use super::release_source::{EffectiveReleaseSource, ReleaseSource};

/// The GitHub release download root every declared release URL starts with.
pub(super) const GITHUB_RELEASE_ROOT: &str = vibe_core::remap::drop::RELEASE_DOWNLOAD_ROOT;
/// The optional listing a raw repository or a drop may carry at its root.
pub(super) const RELEASES_LISTING: &str = "RELEASES.json";
const GITVERSE_ACCEPT: &str = "application/vnd.gitverse.object+json;version=1";
const GITHUB_API_RELEASES: &str = "https://api.github.com/repos/vibevm/vibevm/releases";
const PAGE_LIMIT: usize = 50;

/// Where the files of one release are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReleaseDir {
    /// `<base>/<asset>`: an `https://`, `http://` or `file://` base.
    Base(String),
    /// Each asset at its own address (GitVerse).
    Assets(BTreeMap<String, String>),
}

impl ReleaseDir {
    /// The address of one asset of the release.
    pub(super) fn asset(&self, name: &str) -> Result<String> {
        match self {
            ReleaseDir::Base(base) => Ok(format!("{}/{name}", base.trim_end_matches('/'))),
            ReleaseDir::Assets(assets) => assets.get(name).cloned().with_context(|| {
                format!(
                    "the release carries no asset `{name}` \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
                      fix: upload every file of the GitHub release, `{name}` included)"
                )
            }),
        }
    }

    /// The local directory, when the release is on this machine.
    pub(super) fn local(&self) -> Option<PathBuf> {
        match self {
            ReleaseDir::Base(base) => match ByteLocation::parse(base) {
                Ok(ByteLocation::Local(path)) => Some(path),
                _ => None,
            },
            ReleaseDir::Assets(_) => None,
        }
    }
}

/// One release-lane source with the clients it lists through, the
/// Locator that lowers its row, and the GitVerse token when one is
/// configured (never printed).
pub(super) struct Releases<'a> {
    pub(super) source: &'a EffectiveReleaseSource,
    pub(super) api: &'a ApiClient,
    pub(super) locator: &'a Locator,
    pub(super) token: Option<Token>,
}

/// How the newest release is learned.
pub(super) enum Newest {
    /// The GitHub-shaped `latest/download/DISTRIBUTIONS.json` alias.
    Alias(String),
    /// A listing named this version.
    Listed(semver::Version),
}

impl<'a> Releases<'a> {
    /// Whether the source is read through GitVerse's API.
    fn gitverse(&self) -> Option<(&str, &str, &str, &str)> {
        match &self.source.source {
            ReleaseSource::Gitverse {
                web,
                api,
                owner,
                repo,
            } => Some((web, api, owner, repo)),
            _ => None,
        }
    }

    /// The lane of `source`, loading the GitVerse token when the source is
    /// GitVerse; no token is anonymous (public assets).
    pub(super) fn new(
        source: &'a EffectiveReleaseSource,
        api: &'a ApiClient,
        locator: &'a Locator,
    ) -> Result<Self> {
        let mut lane = Releases {
            source,
            api,
            locator,
            token: None,
        };
        if let Some((web, ..)) = lane.gitverse() {
            let host = reqwest::Url::parse(web)
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned))
                .unwrap_or_default();
            lane.token = match vibe_publish::token::load_token_for_host(&host) {
                Ok(token) => Some(token),
                Err(vibe_publish::PublishError::AuthMissing { .. }) => None,
                Err(error) => return Err(error).context("reading the GitVerse token"),
            };
        }
        Ok(lane)
    }

    /// Where the files of `version` are (`PROP-019#release-sources`).
    pub(super) fn dir(&self, version: &semver::Version) -> Result<ReleaseDir> {
        if let Some((_, api, owner, repo)) = self.gitverse() {
            let url = format!("{api}/repos/{owner}/{repo}/releases/tags/v{version}");
            return match self
                .api
                .get(&url, &[("Accept", GITVERSE_ACCEPT)], self.token.as_ref())?
            {
                Fetched::Body(body) => {
                    let release: GitverseRelease = serde_json::from_slice(&body)
                        .with_context(|| format!("reading the GitVerse release `v{version}`"))?;
                    Ok(ReleaseDir::Assets(release.asset_map()))
                }
                Fetched::Absent => bail!(
                    "GitVerse has no release `v{version}` \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
                      fix: `vibe self releases` lists the versions it has)"
                ),
            };
        }
        Ok(ReleaseDir::Base(located(
            self.locator,
            &format!("{GITHUB_RELEASE_ROOT}/v{version}"),
        )?))
    }

    /// How to learn the newest release: the alias for GitHub-shaped
    /// sources, a listing for the others.
    pub(super) fn newest(&self) -> Result<Newest> {
        match &self.source.source {
            ReleaseSource::Github | ReleaseSource::Origin { .. } => Ok(Newest::Alias(located(
                self.locator,
                &format!(
                    "{}/latest/download/{DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME}",
                    super::release_source::CANONICAL_RELEASE_ORIGIN
                ),
            )?)),
            _ => {
                let versions = self.list()?;
                versions
                    .into_iter()
                    .next()
                    .map(Newest::Listed)
                    .with_context(|| {
                        format!(
                            "the release source `{}` ({}) holds no release \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
                          fix: put a release's files into `<source>/v<version>/` — \
                          `vibe artifacts where` prints the exact names)",
                            self.source.source.kind(),
                            self.source.source.location()
                        )
                    })
            }
        }
    }

    /// Every version the source offers, newest first.
    pub(super) fn list(&self) -> Result<Vec<semver::Version>> {
        let mut versions = match &self.source.source {
            ReleaseSource::Gitverse { .. } => self.list_gitverse()?,
            ReleaseSource::Github => self.list_github()?,
            ReleaseSource::Origin { .. }
            | ReleaseSource::Nexus { .. }
            | ReleaseSource::Localdir { .. } => {
                let root = located(self.locator, GITHUB_RELEASE_ROOT)?;
                match ByteLocation::parse(&root)? {
                    ByteLocation::Local(path) => scan(&path, super::current_target()?)?,
                    ByteLocation::Remote(base) => self.list_remote(&base)?,
                }
            }
        };
        versions.sort_by(|a, b| b.cmp(a));
        versions.dedup();
        Ok(versions)
    }

    fn list_remote(&self, base: &str) -> Result<Vec<semver::Version>> {
        let listing = format!("{}/{RELEASES_LISTING}", base.trim_end_matches('/'));
        if let Fetched::Body(body) =
            self.api
                .get(&listing, &[("Accept", "application/json")], None)?
        {
            return parse_listing(&body).with_context(|| format!("reading `{listing}`"));
        }
        if let Some(versions) = self.list_nexus(base)? {
            return Ok(versions);
        }
        bail!(
            "`{base}` offers no release listing: no `{RELEASES_LISTING}` and no Nexus search \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
              fix: copy `{RELEASES_LISTING}` beside the releases, or name the version: \
              `vibe self install X.Y.Z`)"
        )
    }

    /// The Nexus search REST API, when the base is a Nexus repository URL
    /// (`<origin>/repository/<repo>[/<path>]`) and the server answers it.
    fn list_nexus(&self, base: &str) -> Result<Option<Vec<semver::Version>>> {
        let Some((origin, repository, prefix)) = nexus_repository(base) else {
            return Ok(None);
        };
        let mut versions = Vec::new();
        let mut continuation: Option<String> = None;
        for _ in 0..PAGE_LIMIT {
            let mut url = reqwest::Url::parse(&format!("{origin}/service/rest/v1/search/assets"))
                .context("building the Nexus search address")?;
            url.query_pairs_mut().append_pair("repository", &repository);
            if let Some(token) = &continuation {
                url.query_pairs_mut()
                    .append_pair("continuationToken", token);
            }
            let page = match self
                .api
                .get(url.as_str(), &[("Accept", "application/json")], None)
            {
                Ok(Fetched::Body(body)) => body,
                Ok(Fetched::Absent) | Err(_) if versions.is_empty() && continuation.is_none() => {
                    return Ok(None);
                }
                Ok(Fetched::Absent) => break,
                Err(error) => return Err(error),
            };
            let page: NexusPage =
                serde_json::from_slice(&page).context("reading a Nexus search page")?;
            versions.extend(
                page.items
                    .iter()
                    .filter_map(|item| nexus_version(&item.path, &prefix)),
            );
            match page.continuation_token {
                Some(token) => continuation = Some(token),
                None => break,
            }
        }
        Ok(Some(versions))
    }

    fn list_gitverse(&self) -> Result<Vec<semver::Version>> {
        let Some((_, api, owner, repo)) = self.gitverse() else {
            return Ok(Vec::new());
        };
        let mut versions = Vec::new();
        for page in 1..=PAGE_LIMIT {
            let url = format!("{api}/repos/{owner}/{repo}/releases?page={page}&per_page=30");
            let Fetched::Body(body) =
                self.api
                    .get(&url, &[("Accept", GITVERSE_ACCEPT)], self.token.as_ref())?
            else {
                break;
            };
            let releases: Vec<GitverseRelease> =
                serde_json::from_slice(&body).context("reading the GitVerse release list")?;
            let count = releases.len();
            versions.extend(
                releases
                    .iter()
                    .filter_map(GitverseRelease::published_version),
            );
            if count < 30 {
                break;
            }
        }
        Ok(versions)
    }

    fn list_github(&self) -> Result<Vec<semver::Version>> {
        let mut versions = Vec::new();
        for page in 1..=PAGE_LIMIT {
            let url = format!("{GITHUB_API_RELEASES}?per_page=100&page={page}");
            let Fetched::Body(body) =
                self.api
                    .get(&url, &[("Accept", "application/vnd.github+json")], None)?
            else {
                break;
            };
            let releases: Vec<GitverseRelease> =
                serde_json::from_slice(&body).context("reading the GitHub release list")?;
            let count = releases.len();
            versions.extend(
                releases
                    .iter()
                    .filter_map(GitverseRelease::published_version),
            );
            if count < 100 {
                break;
            }
        }
        Ok(versions)
    }
}

/// The address the Locator chooses for a declared release URL, as one URL
/// string: a `path` target becomes a `file://` URL.
pub(super) fn located(locator: &Locator, declared: &str) -> Result<String> {
    let artifact = Artifact::new(ArtifactKind::Distribution).with_url(declared);
    let candidate = locator.choose_candidate(&artifact)?;
    Ok(ByteLocation::of_target(&candidate.target)?.to_url())
}

/// The versions a local release drop holds for `target`: every
/// `v<version>/` directory holding a manifest or this target's zip, and
/// every bare `vibevm-<version>-<target>.zip` at its root.
pub(super) fn scan(root: &Path, target: &str) -> Result<Vec<semver::Version>> {
    let entries = std::fs::read_dir(root).with_context(|| {
        format!(
            "reading the release drop `{}` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
              fix: create the directory or point `vibe self source set localdir --path` at it)",
            root.display()
        )
    })?;
    let mut versions = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir()
            && let Some(version) = release_dir_version(&name)
            && (path
                .join(DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME)
                .is_file()
                || path
                    .join(bundle_file_name(&version.to_string(), target))
                    .is_file())
        {
            versions.push(version);
        } else if path.is_file()
            && let Some(version) = bundle_version(&name, target)
        {
            versions.push(version);
        }
    }
    Ok(versions)
}

/// The bare platform zip of `version` near a local release directory:
/// inside it, or beside it at the drop root.
pub(super) fn bare_bundle(dir: &Path, version: &str, target: &str) -> Option<PathBuf> {
    let name = bundle_file_name(version, target);
    [
        Some(dir.join(&name)),
        dir.parent().map(|root| root.join(&name)),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.is_file())
}

/// Describe a bare platform zip by its own embedded `DISTRIBUTION.json`
/// (`PROP-019#release-sources`). The outer digest is the file's own, so
/// such a release is verified for internal consistency only — every
/// component against the embedded manifest — and the caller says so.
pub(super) fn synthesize_platform(
    zip: &Path,
    version: &str,
    target: &str,
) -> Result<PlatformDistributionFragment> {
    let shown = zip.display().to_string();
    let file = std::fs::File::open(zip).with_context(|| format!("opening `{shown}`"))?;
    let mut archive = zip::ZipArchive::new(file).with_context(|| format!("reading `{shown}`"))?;
    let mut bytes = Vec::new();
    archive
        .by_name(DISTRIBUTION_MANIFEST_FILENAME)
        .with_context(|| format!("`{shown}` carries no `{DISTRIBUTION_MANIFEST_FILENAME}`"))?
        .take(DISTRIBUTION_MANIFEST_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading `{DISTRIBUTION_MANIFEST_FILENAME}` in `{shown}`"))?;
    let bundle = BundleDistributionManifest::from_json_slice(&bytes)
        .with_context(|| format!("the manifest inside `{shown}`"))?;
    if bundle.version != version || bundle.target != target {
        bail!(
            "`{shown}` holds vibevm {} for {}, not {version} for {target} \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
              fix: name the file after the release it holds)",
            bundle.version,
            bundle.target
        );
    }
    let digest = file_digest(zip)?;
    let size = std::fs::metadata(zip)
        .with_context(|| format!("reading `{shown}`"))?
        .len();
    let vibe = bundle
        .components
        .iter()
        .find(|component| component.name == DistributionComponentName::Vibe)
        .with_context(|| format!("`{shown}` declares no `vibe` component"))?;
    let windows = target.contains("windows");
    let bootstrap = DistributionAsset {
        name: format!(
            "vibe-bootstrap-{target}{}",
            if windows { ".exe" } else { "" }
        ),
        size: vibe.size,
        digest: vibe.digest.clone(),
    };
    let asset = DistributionAsset {
        name: bundle_file_name(version, target),
        size,
        digest,
    };
    PlatformDistributionFragment::new(asset, bootstrap, bundle).map_err(Into::into)
}

/// `sha256:<hex>` of a file, streamed.
fn file_digest(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file =
        std::fs::File::open(path).with_context(|| format!("opening `{}`", path.display()))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .with_context(|| format!("reading `{}`", path.display()))?;
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// `RELEASES.json`: `{"releases": [{"version": "1.0.13"}, …]}`, the same
/// with bare version strings, or a bare array of either.
fn parse_listing(bytes: &[u8]) -> Result<Vec<semver::Version>> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Entry {
        Version(String),
        Table { version: String },
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Listing {
        Wrapped { releases: Vec<Entry> },
        Bare(Vec<Entry>),
    }
    let listing: Listing = serde_json::from_slice(bytes)?;
    let entries = match listing {
        Listing::Wrapped { releases } | Listing::Bare(releases) => releases,
    };
    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let raw = match entry {
                Entry::Version(version) | Entry::Table { version } => version,
            };
            semver::Version::parse(raw.trim_start_matches('v')).ok()
        })
        .collect())
}

/// `<origin>`, repository name and in-repository prefix of a Nexus
/// repository URL.
fn nexus_repository(base: &str) -> Option<(String, String, String)> {
    let url = reqwest::Url::parse(base).ok()?;
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let at = segments
        .iter()
        .position(|segment| *segment == "repository")?;
    let repository = (*segments.get(at + 1)?).to_owned();
    let mut origin = format!("{}://{}", url.scheme(), url.host_str()?);
    if let Some(port) = url.port() {
        origin.push_str(&format!(":{port}"));
    }
    // A Nexus mounted under a context path keeps it.
    for segment in &segments[..at] {
        origin.push('/');
        origin.push_str(segment);
    }
    Some((origin, repository, segments[at + 2..].join("/")))
}

/// The version of a Nexus asset path `<prefix>/v<version>/DISTRIBUTIONS.json`.
fn nexus_version(path: &str, prefix: &str) -> Option<semver::Version> {
    let path = path.trim_start_matches('/');
    let rest = if prefix.is_empty() {
        path
    } else {
        path.strip_prefix(prefix)?.strip_prefix('/')?
    };
    let (dir, file) = rest.split_once('/')?;
    (file == DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME).then(|| release_dir_version(dir))?
}

#[derive(Deserialize)]
struct NexusPage {
    #[serde(default)]
    items: Vec<NexusItem>,
    #[serde(rename = "continuationToken", default)]
    continuation_token: Option<String>,
}

#[derive(Deserialize)]
struct NexusItem {
    path: String,
}

/// The release object GitHub and GitVerse share the fields of.
#[derive(Deserialize)]
struct GitverseRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GitverseAsset>,
}

#[derive(Deserialize)]
struct GitverseAsset {
    name: String,
    browser_download_url: String,
}

impl GitverseRelease {
    fn published_version(&self) -> Option<semver::Version> {
        if self.draft {
            return None;
        }
        semver::Version::parse(self.tag_name.strip_prefix('v')?).ok()
    }

    fn asset_map(&self) -> BTreeMap<String, String> {
        self.assets
            .iter()
            .map(|asset| (asset.name.clone(), asset.browser_download_url.clone()))
            .collect()
    }
}

#[cfg(test)]
#[path = "releases_tests.rs"]
mod tests;
