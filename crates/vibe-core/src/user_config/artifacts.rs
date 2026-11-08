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
use std::path::PathBuf;
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
}

impl SelfConfig {
    /// `true` for the all-empty table, skipped on write.
    pub fn is_default(&self) -> bool {
        *self == SelfConfig::default()
    }
}
