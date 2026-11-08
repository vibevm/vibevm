//! Artifact drops: directories a person fills by hand, and the naming
//! conventions vibe searches them by (`PROP-066#drops`).
//!
//! A drop is not a second routing mechanism. It is configuration that
//! lowers into ordinary remap `path` rows — the vibe release drop into a
//! `distribution` row, the package payload drop into an `app-input` row —
//! plus the lookup convention below, so `vibe remap explain` shows why a
//! fetch went to the drop and the digest check above the Locator is the
//! same check every other source passes.
//!
//! Everything here is pure: names and paths in, names and paths out.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::path::{Path, PathBuf};

use specmark::spec;

use super::artifact::ArtifactKind;
use super::layers::Layer;
use super::locator::Locator;
use super::row::{PathLookup, Provenance, RemapKey, RemapRow, RemapTarget};

/// The GitHub release download root every `vibe self` URL is built from;
/// the release drop and every mirror replace exactly this prefix.
pub const RELEASE_DOWNLOAD_ROOT: &str = "https://github.com/vibevm/vibevm/releases/download";

/// The provenance label of the package-drop row.
pub const PACKAGES_DROP_SOURCE: &str = "[artifacts] packages";

/// The directory of one release inside a release drop: `<drop>/v<version>`.
///
/// ```
/// use std::path::Path;
/// use vibe_core::remap::drop::release_dir;
///
/// assert_eq!(release_dir(Path::new("/srv/rel"), "1.0.13"), Path::new("/srv/rel/v1.0.13"));
/// ```
pub fn release_dir(drop: &Path, version: &str) -> PathBuf {
    drop.join(format!("v{version}"))
}

/// The platform bundle's published file name, `vibevm-<version>-<target>.zip`.
///
/// ```
/// use vibe_core::remap::drop::bundle_file_name;
///
/// assert_eq!(
///     bundle_file_name("1.0.13", "x86_64-unknown-linux-gnu"),
///     "vibevm-1.0.13-x86_64-unknown-linux-gnu.zip"
/// );
/// ```
pub fn bundle_file_name(version: &str, target: &str) -> String {
    format!("vibevm-{version}-{target}.zip")
}

/// The version a bare bundle file name carries for `target`, when it is one.
///
/// ```
/// use vibe_core::remap::drop::bundle_version;
///
/// let target = "x86_64-unknown-linux-gnu";
/// assert_eq!(
///     bundle_version("vibevm-1.0.13-x86_64-unknown-linux-gnu.zip", target),
///     Some(semver::Version::new(1, 0, 13))
/// );
/// assert_eq!(bundle_version("vibevm-1.0.13-aarch64-apple-darwin.zip", target), None);
/// ```
pub fn bundle_version(file_name: &str, target: &str) -> Option<semver::Version> {
    let middle = file_name
        .strip_prefix("vibevm-")?
        .strip_suffix(".zip")?
        .strip_suffix(target)?
        .strip_suffix('-')?;
    semver::Version::parse(middle).ok()
}

/// The version a `v<version>` directory name carries, when it is one.
///
/// ```
/// use vibe_core::remap::drop::release_dir_version;
///
/// assert_eq!(release_dir_version("v1.0.13"), Some(semver::Version::new(1, 0, 13)));
/// assert_eq!(release_dir_version("latest"), None);
/// ```
pub fn release_dir_version(name: &str) -> Option<semver::Version> {
    semver::Version::parse(name.strip_prefix('v')?).ok()
}

/// The last path segment of a declared URL, without query or fragment —
/// the name a payload is looked for under in a drop.
///
/// ```
/// use vibe_core::remap::drop::url_file_name;
///
/// assert_eq!(url_file_name("https://h.example/d/tool-1.0.tgz?x=1#f"), Some("tool-1.0.tgz".into()));
/// assert_eq!(url_file_name("https://h.example/"), None);
/// ```
pub fn url_file_name(url: &str) -> Option<String> {
    let without = url.split(['?', '#']).next().unwrap_or(url);
    let path = without.split_once("://").map_or(without, |(_, rest)| rest);
    let (_, last) = path.rsplit_once('/')?;
    (!last.is_empty() && last != "." && last != "..").then(|| last.to_owned())
}

/// The ordered places a package payload is looked for in a package drop
/// (`PROP-066#drops`): for an input of package `G/N@V` with file name `F`
/// and digest `D`,
///
/// 1. `<drop>/<G>/<N>/<V>/<F>`
/// 2. `<drop>/<G>/<N>/<F>`
/// 3. `<drop>/<D>/<F>`
/// 4. `<drop>/<F>`
///
/// A rung whose part is unknown (no package, no file name) is skipped. The
/// digest is verified on whichever file is found, so a wrong file is a
/// refusal, never a silent install.
///
/// ```
/// use std::path::{Path, PathBuf};
/// use vibe_core::remap::drop::package_lookup;
///
/// let rungs = package_lookup(Path::new("/d"), Some("org.example/tool@1.2.0"), "ab12", Some("t.tgz"));
/// assert_eq!(rungs, vec![
///     PathBuf::from("/d/org.example/tool/1.2.0/t.tgz"),
///     PathBuf::from("/d/org.example/tool/t.tgz"),
///     PathBuf::from("/d/ab12/t.tgz"),
///     PathBuf::from("/d/t.tgz"),
/// ]);
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
pub fn package_lookup(
    drop: &Path,
    package: Option<&str>,
    sha256: &str,
    file_name: Option<&str>,
) -> Vec<PathBuf> {
    let Some(file) = file_name.filter(|name| portable_name(name)) else {
        return Vec::new();
    };
    let mut rungs = Vec::with_capacity(4);
    if let Some((group, name, version)) = package.and_then(split_package) {
        if let Some(version) = version {
            rungs.push(drop.join(group).join(name).join(version).join(file));
        }
        rungs.push(drop.join(group).join(name).join(file));
    }
    rungs.push(drop.join(sha256.to_ascii_lowercase()).join(file));
    rungs.push(drop.join(file));
    rungs
}

/// The remap row an `[artifacts] packages` drop lowers to: every
/// application input with a declared URL may be served from the drop, as
/// the weakest non-built-in row, before the declared origin.
///
/// ```
/// use std::path::Path;
/// use vibe_core::remap::{ArtifactKind, Layer, PathLookup};
/// use vibe_core::remap::drop::package_drop_row;
///
/// let row = package_drop_row(Path::new("/srv/pay"), Layer::Local, "config.toml");
/// assert_eq!(row.only, Some(ArtifactKind::AppInput));
/// assert_eq!(row.lookup, PathLookup::PackageDrop);
/// ```
pub fn package_drop_row(drop: &Path, layer: Layer, source: &str) -> RemapRow {
    derived(
        RemapKey::Prefix(String::new()),
        RemapTarget::Path(drop.to_path_buf()),
        format!("`[artifacts] packages` drop declared in `{source}`"),
        Provenance::new(layer, PACKAGES_DROP_SOURCE, 0),
        Some(ArtifactKind::AppInput),
        PathLookup::PackageDrop,
    )
}

/// The remap row a release source lowers to: the GitHub download root,
/// for distributions only, redirected to `target` — a local release drop
/// (`<drop>/v<version>/<asset>`) or an HTTP(S) base with the same layout.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::remap::{Artifact, ArtifactKind, Layer, LoadedSource, Locator, RemapTarget};
/// use vibe_core::remap::drop::{RELEASE_DOWNLOAD_ROOT, release_source_row};
///
/// let row = release_source_row(
///     RemapTarget::Path(PathBuf::from("/srv/rel")),
///     Layer::Local,
///     "[self] source",
///     "localdir release source",
/// );
/// let locator = Locator::from_sources(
///     vec![LoadedSource { layer: Layer::Local, source: "[self] source".into(), rows: vec![row], policy: Default::default() }],
///     Vec::new(),
/// );
/// let asset = Artifact::new(ArtifactKind::Distribution)
///     .with_url(format!("{RELEASE_DOWNLOAD_ROOT}/v1.0.13/DISTRIBUTIONS.json"));
/// assert_eq!(
///     locator.choose_candidate(&asset).unwrap().target,
///     RemapTarget::Path(PathBuf::from("/srv/rel/v1.0.13/DISTRIBUTIONS.json"))
/// );
/// ```
pub fn release_source_row(
    target: RemapTarget,
    layer: Layer,
    source: &str,
    reason: &str,
) -> RemapRow {
    derived(
        RemapKey::Prefix(RELEASE_DOWNLOAD_ROOT.to_owned()),
        target,
        reason.to_owned(),
        Provenance::new(layer, source, 0),
        Some(ArtifactKind::Distribution),
        PathLookup::Mirror,
    )
}

fn derived(
    key: RemapKey,
    target: RemapTarget,
    reason: String,
    from: Provenance,
    only: Option<ArtifactKind>,
    lookup: PathLookup,
) -> RemapRow {
    RemapRow {
        key,
        target,
        scope: None,
        reason: Some(reason),
        from,
        from_package: None,
        target_ref: None,
        only,
        lookup,
    }
}

impl Locator {
    /// The roots of every `[artifacts] packages` drop the rows declare, in
    /// winning order (highest layer first) — what an input declared with
    /// `drop = "packages"` is resolved through (`PROP-066#drops`).
    pub fn package_drops(&self) -> Vec<&std::path::Path> {
        let mut drops: Vec<(u8, usize, &std::path::Path)> = self
            .rows()
            .iter()
            .zip(&self.source_ordinals)
            .filter(|(row, _)| row.lookup == PathLookup::PackageDrop)
            .filter_map(|(row, ordinal)| match &row.target {
                RemapTarget::Path(path) => Some((row.from.layer.rank(), *ordinal, path.as_path())),
                _ => None,
            })
            .collect();
        drops.sort_by_key(|(rank, ordinal, _)| std::cmp::Reverse((*rank, *ordinal)));
        drops.into_iter().map(|(_, _, path)| path).collect()
    }
}

/// Join a URL remainder (`/v1.0.0/a.zip?q`) onto a directory, one
/// component per `/`-separated segment, dropping the query and fragment
/// and every segment that could climb out of the directory.
pub(super) fn join_remainder(root: &std::path::Path, remainder: &str) -> std::path::PathBuf {
    let path = remainder.split(['?', '#']).next().unwrap_or_default();
    path.split('/')
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .fold(root.to_path_buf(), |joined, segment| joined.join(segment))
}

/// `group/name[@version]` split into its parts.
fn split_package(spelling: &str) -> Option<(&str, &str, Option<&str>)> {
    let (coordinate, version) = match spelling.split_once('@') {
        Some((coordinate, version)) => (coordinate, Some(version)),
        None => (spelling, None),
    };
    let (group, name) = coordinate.split_once('/')?;
    [group, name]
        .iter()
        .chain(version.iter())
        .all(|part| portable_name(part))
        .then_some((group, name, version))
}

/// One path component that cannot climb out of the drop.
fn portable_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':'])
        && !name.chars().any(char::is_control)
}
