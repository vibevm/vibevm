//! `[template]` — where a template package's produced versions and their
//! digests come from (PROP-075 §2).
//!
//! The table is data only: a closed set of source kinds, an RFC 6901
//! pointer into each fetched document, and the channel rules derived from
//! a version record's own fields. There is no expression language and no
//! scripting of a source, exactly as PROP-065 `#principle` forbids for
//! installers. Rendering lives in `vibe-template`; this module owns the
//! declaration and its refusals, each naming its TOML path.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-SPELLING");

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specmark::spec;

#[path = "template/validation.rs"]
mod validation;

/// `[template]` — the declaration that turns one manifest into a family.
///
/// ```
/// use vibe_core::manifest::{DigestSource, TemplateSection, VersionSource};
///
/// let section: TemplateSection = toml::from_str(
///     "[versions]\nkind = \"json\"\nurl = \"https://example.invalid/index.json\"\n\
///      select = \"\"\nstrip_prefix = \"v\"\n\
///      [digests]\nkind = \"sha256sums\"\n\
///      url = \"https://example.invalid/v{version}/SHASUMS256.txt\"\n",
/// )?;
/// assert!(matches!(section.versions, VersionSource::Json { .. }));
/// assert!(matches!(section.digests, DigestSource::Sha256Sums { .. }));
/// assert!(!section.has_sizes(), "sha256sum output carries no sizes");
/// # Ok::<(), toml::de::Error>(())
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-VERSIONS")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateSection {
    /// `[template.versions]` — where the produced versions come from.
    pub versions: VersionSource,
    /// `[template.digests]` — where the SHA-256 of every input comes from.
    /// A template without one is refused (PROP-075 `#TEMPLATE-VERSIONS`).
    pub digests: DigestSource,
    /// `[[template.channel]]` — named channels derived from the version
    /// records (PROP-075 `#CHANNELS`).
    #[serde(default, rename = "channel", skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<ChannelRule>,
    /// How long a cached version list is used before it is refetched. The
    /// user's `~/.vibe/config.toml` carries the default; a package may ask
    /// for a shorter or longer one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_list_ttl: Option<Ttl>,
}

impl TemplateSection {
    /// Whether the declared digest source carries byte sizes, which is what
    /// makes a `{size:FILE}` variable legal (PROP-075
    /// `#TEMPLATE-DIGEST-SOURCE`).
    ///
    /// ```
    /// use vibe_core::manifest::TemplateSection;
    ///
    /// let section: TemplateSection = toml::from_str(
    ///     "[versions]\nkind = \"list\"\n[[versions.item]]\nversion = \"1.0.0\"\n\
    ///      [digests]\nkind = \"json\"\nurl = \"https://e.invalid/{version}.json\"\n\
    ///      select = \"/files\"\nfile_field = \"name\"\ndigest_field = \"sha256\"\n\
    ///      size_field = \"bytes\"\n",
    /// )?;
    /// assert!(section.has_sizes());
    /// # Ok::<(), toml::de::Error>(())
    /// ```
    pub fn has_sizes(&self) -> bool {
        match &self.digests {
            DigestSource::Sha256Sums { .. } => false,
            DigestSource::Json { size_field, .. } => size_field.is_some(),
        }
    }

    /// The version-list URL, which never carries a template variable: the
    /// list cannot depend on a version.
    pub fn version_list_url(&self) -> Option<&str> {
        match &self.versions {
            VersionSource::Json { url, .. } => Some(url),
            VersionSource::List { .. } => None,
        }
    }

    /// The per-version digest-document URL **template**, over the version
    /// variables only.
    pub fn digest_url_template(&self) -> &str {
        match &self.digests {
            DigestSource::Sha256Sums { url, .. } | DigestSource::Json { url, .. } => url,
        }
    }
}

/// `[template.versions]` — the closed source-kind set (PROP-075
/// `#TEMPLATE-SOURCE-KINDS`). A third kind is a later amendment of that
/// fact and a new `kind` value, never an escape hatch.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-SOURCE-KINDS")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VersionSource {
    /// A JSON document with an RFC 6901 pointer at an array of records.
    Json {
        /// An HTTPS URL carrying no template variable.
        url: String,
        /// The RFC 6901 pointer at the record array; `""` is the document
        /// root (PROP-075 `#TEMPLATE-SELECTOR`).
        select: String,
        /// The key of the version string inside each selected record.
        #[serde(default = "version_field_default")]
        version_field: String,
        /// A literal prefix removed before the version is parsed as semver.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        strip_prefix: Option<String>,
        /// An inclusive lower bound; versions below it are dropped before
        /// anything else sees them.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<semver::Version>,
        /// An inclusive upper bound.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<semver::Version>,
        #[serde(default, skip_serializing_if = "PrereleasePolicy::is_default")]
        prerelease: PrereleasePolicy,
    },
    /// One `[[template.versions.item]]` row per version, for a tool whose
    /// releases are not published as a document.
    List {
        #[serde(rename = "item", default, skip_serializing_if = "Vec::is_empty")]
        items: Vec<VersionItem>,
    },
}

fn version_field_default() -> String {
    "version".to_string()
}

/// Whether a source's prereleases are produced versions.
///
/// ```
/// use vibe_core::manifest::PrereleasePolicy;
///
/// assert_eq!(PrereleasePolicy::default(), PrereleasePolicy::Exclude);
/// assert!(PrereleasePolicy::default().is_default());
/// assert!(PrereleasePolicy::Include.includes_prerelease());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrereleasePolicy {
    /// The default: a prerelease is not a produced version.
    #[default]
    Exclude,
    Include,
}

impl PrereleasePolicy {
    /// `true` for the default, so the serializer skips the field.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Exclude)
    }

    /// Whether a prerelease version is admitted.
    pub fn includes_prerelease(&self) -> bool {
        matches!(self, Self::Include)
    }
}

/// One `[[template.versions.item]]` row: a version and its free `meta`.
///
/// ```
/// use vibe_core::manifest::{Scalar, VersionItem};
///
/// let row: VersionItem =
///     toml::from_str("version = \"1.2.3\"\nmeta = { channel = \"old\" }\n")?;
/// assert_eq!(row.version, "1.2.3");
/// assert_eq!(row.meta["channel"], Scalar::String("old".into()));
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionItem {
    pub version: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, Scalar>,
}

/// `[template.digests]` — the closed digest-source set (PROP-075
/// `#TEMPLATE-DIGEST-SOURCE`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DIGEST-SOURCE")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DigestSource {
    /// `sha256sum(1)` output: one `<digest><space>[*]<name>` pair per line,
    /// other lines skipped, a duplicate name an error.
    #[serde(rename = "sha256sums")]
    Sha256Sums {
        url: String,
        /// Declared only to be refused when it disagrees with the source:
        /// `sha256sum` output carries no sizes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sizes: Option<bool>,
    },
    /// A JSON checksum manifest.
    Json {
        url: String,
        select: String,
        file_field: String,
        digest_field: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size_field: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sizes: Option<bool>,
    },
}

/// One `[[template.channel]]` row (PROP-075 `#CHANNEL-SYNTAX`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChannelRule {
    /// Every version whose `from` field is truthy is a member of `name`.
    Truthy { name: String, from: String },
    /// One channel per distinct `from` value, named `<prefix>/<value>`
    /// with the value lower-cased.
    Value { prefix: String, from: String },
}

impl ChannelRule {
    /// The record field this rule reads.
    pub fn from(&self) -> &str {
        match self {
            Self::Truthy { from, .. } | Self::Value { from, .. } => from,
        }
    }
}

/// A scalar of a version record: what a `{meta.PATH}` variable may resolve
/// to. A non-integral number is refused naming the path, so the declaration
/// stays comparable (`Eq`) and a rendered number is exact.
///
/// ```
/// use std::collections::BTreeMap;
/// use vibe_core::manifest::Scalar;
///
/// let meta: BTreeMap<String, Scalar> =
///     toml::from_str("lts = \"Jod\"\nsecurity = true\nmodules = 137\n")?;
/// assert_eq!(meta["lts"], Scalar::String("Jod".into()));
/// assert_eq!(meta["security"], Scalar::Bool(true));
/// assert_eq!(meta["modules"].to_string(), "137");
/// assert!(meta["lts"].is_truthy());
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    Bool(bool),
    Integer(i64),
    String(String),
}

impl fmt::Display for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => write!(f, "{value}"),
            Self::Integer(value) => write!(f, "{value}"),
            Self::String(value) => f.write_str(value),
        }
    }
}

impl Scalar {
    /// Whether this scalar counts as truthy for a `rule = "truthy"` channel:
    /// `true`, a non-zero integer, or a non-empty string that is not
    /// `"false"`.
    ///
    /// ```
    /// use vibe_core::manifest::Scalar;
    ///
    /// assert!(Scalar::String("Jod".into()).is_truthy());
    /// assert!(!Scalar::Bool(false).is_truthy());
    /// assert!(!Scalar::String(String::new()).is_truthy());
    /// ```
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Integer(value) => *value != 0,
            Self::String(value) => !value.is_empty() && value != "false",
        }
    }
}

/// A duration written as `<count><unit>` with `s`, `m` or `h`.
///
/// ```
/// use std::time::Duration;
/// use vibe_core::manifest::Ttl;
///
/// let ttl: Ttl = "15m".parse()?;
/// assert_eq!(ttl.as_duration(), Duration::from_secs(900));
/// assert_eq!(ttl.to_string(), "15m");
/// assert!("15".parse::<Ttl>().is_err());
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Ttl(Duration);

impl Ttl {
    /// The duration this spelling names.
    pub fn as_duration(self) -> Duration {
        self.0
    }
}

impl std::str::FromStr for Ttl {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let refuse = || {
            format!(
                "`{text}` is not a duration — write a count and one of the units `s`, `m` or `h`, \
                 for example `15m`"
            )
        };
        let (count, unit) = text.split_at(text.len().checked_sub(1).ok_or_else(refuse)?);
        let seconds: u64 = match unit {
            "s" => 1,
            "m" => 60,
            "h" => 3600,
            _ => return Err(refuse()),
        };
        let count: u64 = count.parse().map_err(|_| refuse())?;
        let seconds = count.checked_mul(seconds).ok_or_else(refuse)?;
        Ok(Self(Duration::from_secs(seconds)))
    }
}

impl TryFrom<String> for Ttl {
    type Error = String;

    fn try_from(value: String) -> Result<Self, String> {
        value.parse()
    }
}

impl From<Ttl> for String {
    fn from(value: Ttl) -> Self {
        value.to_string()
    }
}

impl fmt::Display for Ttl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.0.as_secs();
        if seconds != 0 && seconds.is_multiple_of(3600) {
            write!(f, "{}h", seconds / 3600)
        } else if seconds != 0 && seconds.is_multiple_of(60) {
            write!(f, "{}m", seconds / 60)
        } else {
            write!(f, "{seconds}s")
        }
    }
}

pub use validation::{CHANNEL_NAME_RULE, channel_name_is_valid};

#[cfg(test)]
#[path = "template/tests.rs"]
mod tests;
