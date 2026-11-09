//! `[artifacts]` — the machine's artifact drops — and `[self]` — where
//! `vibe self` reads its releases from (`PROP-066#drops`,
//! `PROP-019#release-sources`).
//!
//! A closed network has no release host to download from. What it has is a
//! directory a person fills by hand, from a browser on another machine.
//! These two tables name such directories and the release source; the CLI
//! lowers them into remap rows, so `vibe remap explain` still answers
//! "where did these bytes come from" with one chain.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specmark::spec;

/// `[artifacts]` section of the user-level config: the two artifact drops.
///
/// ```
/// use vibe_core::user_config::ArtifactsConfig;
///
/// let drops: ArtifactsConfig = toml::from_str(
///     "vibe = \"/srv/vibe-releases\"\npackages = \"/srv/vibe-artifacts\"",
/// )
/// .unwrap();
/// assert_eq!(drops.vibe.unwrap().to_str(), Some("/srv/vibe-releases"));
/// assert!(!ArtifactsConfig { vibe: None, packages: drops.packages }.is_default());
/// assert!(ArtifactsConfig::default().is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
pub struct ArtifactsConfig {
    /// The vibe release drop: `<vibe>/v<version>/` holds the files of one
    /// published release exactly as the release page lists them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibe: Option<PathBuf>,
    /// The package payload drop, searched by the convention of
    /// [`crate::remap::drop::package_lookup`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packages: Option<PathBuf>,
}

impl ArtifactsConfig {
    /// `true` for the all-empty table, skipped on write.
    pub fn is_default(&self) -> bool {
        *self == ArtifactsConfig::default()
    }
}

/// The kinds of release source `vibe self` knows (`PROP-019#release-sources`).
/// Spellings are case-insensitive; the stored spelling is lower-case.
///
/// ```
/// use vibe_core::user_config::ReleaseSourceKind;
///
/// assert_eq!("GitVerse".parse::<ReleaseSourceKind>().unwrap(), ReleaseSourceKind::Gitverse);
/// assert_eq!("LOCALDIR".parse::<ReleaseSourceKind>().unwrap().to_string(), "localdir");
/// assert!("ftp".parse::<ReleaseSourceKind>().is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-019#release-sources")]
pub enum ReleaseSourceKind {
    /// The public GitHub release page — the built-in default.
    Github,
    /// A GitVerse instance, read through its public REST API.
    Gitverse,
    /// An HTTP(S) raw repository (Nexus, Artifactory, a web server) holding
    /// the `v<version>/<asset>` layout.
    Nexus,
    /// A local directory holding the `v<version>/<asset>` layout.
    Localdir,
}

impl ReleaseSourceKind {
    /// Every kind, in the order help text lists them.
    pub const ALL: [ReleaseSourceKind; 4] = [
        ReleaseSourceKind::Github,
        ReleaseSourceKind::Gitverse,
        ReleaseSourceKind::Nexus,
        ReleaseSourceKind::Localdir,
    ];

    /// The stored, lower-case spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            ReleaseSourceKind::Github => "github",
            ReleaseSourceKind::Gitverse => "gitverse",
            ReleaseSourceKind::Nexus => "nexus",
            ReleaseSourceKind::Localdir => "localdir",
        }
    }
}

impl fmt::Display for ReleaseSourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A release-source spelling no kind answers to.
///
/// ```
/// use vibe_core::user_config::ReleaseSourceKind;
///
/// let error = "s3".parse::<ReleaseSourceKind>().unwrap_err();
/// assert!(error.to_string().contains("PROP-019#release-sources"));
/// assert!(error.to_string().contains("github, gitverse, nexus, localdir"));
/// ```
#[derive(Debug, thiserror::Error)]
#[error(
    "unknown release source type `{0}` \
     (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
      fix: use one of github, gitverse, nexus, localdir — any letter case)"
)]
pub struct UnknownReleaseSource(pub String);

impl FromStr for ReleaseSourceKind {
    type Err = UnknownReleaseSource;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let lower = raw.trim().to_ascii_lowercase();
        ReleaseSourceKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == lower)
            .ok_or_else(|| UnknownReleaseSource(raw.to_owned()))
    }
}

impl Serialize for ReleaseSourceKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ReleaseSourceKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// `[self]` section of the user-level config: the release source
/// `vibe self update|install|releases` read from (`PROP-019#release-sources`).
///
/// ```
/// use vibe_core::user_config::{ReleaseSourceKind, SelfConfig};
///
/// let s: SelfConfig = toml::from_str(
///     "source = \"Nexus\"\nurl = \"https://nexus.corp.example/repository/vibe\"",
/// )
/// .unwrap();
/// assert_eq!(s.source, Some(ReleaseSourceKind::Nexus));
/// assert!(SelfConfig::default().is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-019#release-sources")]
pub struct SelfConfig {
    /// The release source type; absent means the `[artifacts] vibe` drop
    /// when one is declared, else GitHub.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ReleaseSourceKind>,
    /// `nexus`: the base URL holding `v<version>/<asset>`. `gitverse`: the
    /// repository web URL (default `https://gitverse.ru/vibevm/vibevm`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// `gitverse`: the REST API base (default `https://api.<web host>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    /// `localdir`: the release drop (default `[artifacts] vibe`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// The directory vibe installs ITSELF under — the durable
    /// `$VIBEVM_INSTALL_ROOT` (PROP-019 `#install-root`). vibe keeps its
    /// shims at `<install_root>/opt/bin` and its data at
    /// `<install_root>/opt/vibevm`, so a network that only permits
    /// executables under `C:\Work` declares `install_root = "C:/Work"`.
    /// Absent means `~/.vibe`.
    ///
    /// It is NOT part of the release-source decision: a user who chooses
    /// their own `[self] source` still inherits the site's install root
    /// ([`SelfConfig::release_is_default`] is the predicate the release
    /// pair layers on).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_root: Option<PathBuf>,
}

/// Why a configured per-user path — `[self] install_root`, `[self] path`,
/// an `[artifacts]` drop — names no directory on this machine
/// (`PROP-019#install-root`).
///
/// Each message names the value as the operator wrote it and the fix, so
/// the refusal points at the key in their own file rather than at a
/// resolved path they never typed.
///
/// ```
/// use std::path::Path;
/// use vibe_core::user_config::{InstallRootError, install_root_at};
///
/// let home = Path::new("/home/me");
/// let error = install_root_at(Path::new("~bob/opt"), Some(home)).unwrap_err();
/// assert!(matches!(error, InstallRootError::OtherUser { .. }));
/// assert!(error.to_string().contains("another account's home"));
/// assert!(error.to_string().contains("PROP-019#install-root"));
///
/// let relative = install_root_at(Path::new("opt"), Some(home)).unwrap_err();
/// assert!(matches!(relative, InstallRootError::Relative { .. }));
/// assert!(relative.to_string().contains("fix:"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#install-root")]
pub enum InstallRootError {
    #[error(
        "`[self] install_root = \"{written}\"` names another account's home, which vibe does not \
         resolve (violates spec://org.vibevm.core/vibevm/common/PROP-019#install-root; \
          fix: write `~` for your own home, or the directory in full)"
    )]
    OtherUser { written: String },
    #[error(
        "`[self] install_root = \"{written}\"` starts at the home directory, and this run has no \
         home to expand it to (violates spec://org.vibevm.core/vibevm/common/PROP-019#install-root; \
          fix: set {home_var}, or write the directory in full)"
    )]
    NoHome {
        written: String,
        home_var: &'static str,
    },
    #[error(
        "`[self] install_root = \"{written}\"` is relative, and an install root is read from \
         wherever a command happens to run (violates \
         spec://org.vibevm.core/vibevm/common/PROP-019#install-root; \
          fix: give an absolute directory, or `~`/`~/<dir>` for one under your home)"
    )]
    Relative { written: String },
}

/// The variable a `~` is expanded from, named in the refusal of a `~` with
/// no home to expand.
pub(crate) const HOME_VAR: &str = if cfg!(windows) { "USERPROFILE" } else { "HOME" };

/// A configured path that may start at the operator's home, resolved
/// against `home` (`PROP-019#install-root`).
///
/// A site profile is read on every machine, so the one directory that
/// cannot be written out in full is the reader's own home: a profile for
/// Macs and Linux boxes declares `install_root = "~"` and vibe lives in
/// `~/opt`, with its shims at `~/opt/bin`, for every developer who applies
/// it. `~` and `~/<rest>` — and `~\<rest>` on Windows, where that is the
/// separator a person types — expand; `~user` is refused, because vibe
/// resolves nobody else's home; every other path is returned exactly as
/// written, absolute or not.
///
/// `home` is the home the caller already resolved — the same one `~/.vibe`
/// is read from — never an ambient read of this function's own. The public
/// surface over it is [`install_root_at`], whose doctest shows the shapes;
/// `the_home_prefix_is_expanded_and_only_that` pins every case.
pub(crate) fn expand_home(path: &Path, home: Option<&Path>) -> Result<PathBuf, InstallRootError> {
    let mut components = path.components();
    let Some(std::path::Component::Normal(first)) = components.next() else {
        // Rooted, or a Windows prefix: nothing to expand.
        return Ok(path.to_path_buf());
    };
    if !first.to_string_lossy().starts_with('~') {
        return Ok(path.to_path_buf());
    }
    let written = path.display().to_string();
    if first != std::ffi::OsStr::new("~") {
        return Err(InstallRootError::OtherUser { written });
    }
    let home = home.ok_or(InstallRootError::NoHome {
        written,
        home_var: HOME_VAR,
    })?;
    let mut out = home.to_path_buf();
    out.extend(components);
    Ok(out)
}

/// A configured install root, expanded and checked: `~`-rooted against
/// `home`, and refused when it names no one directory on this machine
/// (`PROP-019#install-root`).
///
/// ```
/// use std::path::{Path, PathBuf};
/// use vibe_core::user_config::install_root_at;
///
/// let home = PathBuf::from("/home/me");
/// assert_eq!(
///     install_root_at(Path::new("~"), Some(&home)).unwrap(),
///     home
/// );
/// // A Windows drive path is absolute on every host, as in a profile.
/// assert!(install_root_at(Path::new(r"C:\Work"), None).is_ok());
/// assert!(install_root_at(Path::new("opt"), Some(&home)).is_err());
/// ```
pub fn install_root_at(path: &Path, home: Option<&Path>) -> Result<PathBuf, InstallRootError> {
    let expanded = expand_home(path, home)?;
    if !crate::site_profile::is_absolute_anywhere(&expanded) {
        return Err(InstallRootError::Relative {
            written: path.display().to_string(),
        });
    }
    Ok(expanded)
}

/// The per-user paths a configured layer may root at the reader's own
/// home, expanded against `home` (`PROP-019#install-root`, `PROP-066#drops`).
///
/// A site profile is read on every machine, so the directories that cannot
/// be written out in full are the ones under the reader's home: a profile
/// for Macs and Linux boxes says `install_root = "~"`, `packages =
/// "~/opt/drop/packages"` and gets each developer's own. The file keeps the
/// `~` it was written with; only what is read into memory is resolved.
/// A path that cannot be resolved — `~user` — is left exactly as written,
/// so the reader that needs it refuses by name instead of this helper
/// guessing (`SiteProfile::validate`, `ConfigLayers::install_base`,
/// `packages_drop`, the release source).
pub(crate) fn home_expanded(path: &Path, home: Option<&Path>) -> PathBuf {
    expand_home(path, home).unwrap_or_else(|_| path.to_path_buf())
}

impl ArtifactsConfig {
    /// This table with its drops resolved against `home`.
    #[must_use]
    pub fn with_home(&self, home: Option<&Path>) -> Self {
        ArtifactsConfig {
            vibe: self.vibe.as_deref().map(|path| home_expanded(path, home)),
            packages: self
                .packages
                .as_deref()
                .map(|path| home_expanded(path, home)),
        }
    }
}

impl SelfConfig {
    /// This table with its per-user paths resolved against `home`.
    #[must_use]
    pub fn with_home(&self, home: Option<&Path>) -> Self {
        SelfConfig {
            path: self.path.as_deref().map(|path| home_expanded(path, home)),
            install_root: self
                .install_root
                .as_deref()
                .map(|path| home_expanded(path, home)),
            ..self.clone()
        }
    }
}

use crate::user_config::UserConfig;

impl UserConfig {
    /// This configuration with its per-user paths — the artifact drops,
    /// `[self] path` and `[self] install_root` — resolved against `home`
    /// (`PROP-019#install-root`, `PROP-066#drops`). The file on disk keeps
    /// the `~` it was written with; only what is read into memory is
    /// resolved, so this value is never saved back.
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    /// use vibe_core::user_config::UserConfig;
    ///
    /// let mut config = UserConfig::default();
    /// config.artifacts.packages = Some(PathBuf::from("~/opt/drop/packages"));
    /// let home = PathBuf::from("/home/me");
    /// assert_eq!(
    ///     config.with_home(Some(&home)).artifacts.packages.as_deref(),
    ///     Some(home.join("opt/drop/packages").as_path())
    /// );
    /// ```
    #[must_use]
    pub fn with_home(&self, home: Option<&Path>) -> Self {
        UserConfig {
            artifacts: self.artifacts.with_home(home),
            self_source: self.self_source.with_home(home),
            ..self.clone()
        }
    }
}

impl SelfConfig {
    /// `true` for the all-empty table, skipped on write.
    pub fn is_default(&self) -> bool {
        *self == SelfConfig::default()
    }

    /// The declared install root as a directory on this machine
    /// (`PROP-019#install-root`): `~`-rooted values expanded against
    /// `home`, anything that names no directory refused.
    pub fn install_root_at(
        &self,
        home: Option<&Path>,
    ) -> Option<Result<PathBuf, InstallRootError>> {
        self.install_root
            .as_deref()
            .map(|root| install_root_at(root, home))
    }

    /// `true` when the table says nothing about WHERE RELEASES COME FROM.
    ///
    /// `install_root` lives in the same table but answers a different
    /// question — where vibe installs itself — and the two layer
    /// independently, so the release pair (`[self]` + `[artifacts] vibe`,
    /// taken whole from the higher declaring layer) must not be decided by
    /// it. A table that carries only an `install_root` declares no release
    /// source.
    pub fn release_is_default(&self) -> bool {
        let SelfConfig {
            source,
            url,
            api,
            path,
            install_root: _,
        } = self;
        source.is_none() && url.is_none() && api.is_none() && path.is_none()
    }
}
