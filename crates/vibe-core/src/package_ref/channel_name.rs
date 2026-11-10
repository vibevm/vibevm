//! [`ChannelName`] — the channel grammar of PROP-075 `#CHANNEL-SYNTAX`.
//!
//! One grammar in one place: `VersionSpec::parse` reads it, the
//! `[[template.channel]]` validator reads it, and the version-file reader
//! reads it, so a name that is a channel in one of them is a channel in
//! all three.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX");

use std::fmt;

use super::VersionSpec;

impl VersionSpec {
    /// The human label of this requirement, for a message that needs a
    /// word where [`fmt::Display`] renders [`VersionSpec::Latest`] as the
    /// empty string (which would leave a dangling `@`).
    ///
    /// ```
    /// use vibe_core::VersionSpec;
    ///
    /// assert_eq!(VersionSpec::Latest.requirement_label(), "latest");
    /// assert_eq!(VersionSpec::parse("^22")?.requirement_label(), "^22");
    /// assert_eq!(VersionSpec::parse("lts/jod")?.requirement_label(), "lts/jod");
    /// # Ok::<(), vibe_core::Error>(())
    /// ```
    pub fn requirement_label(&self) -> String {
        match self {
            VersionSpec::Latest => "latest".to_string(),
            VersionSpec::Req(req) => req.to_string(),
            VersionSpec::Channel(name) => name.as_str().to_string(),
        }
    }

    /// The channel this spec names, when it names one.
    pub fn channel(&self) -> Option<&ChannelName> {
        match self {
            VersionSpec::Channel(name) => Some(name),
            _ => None,
        }
    }
}

/// A channel name of PROP-075 `#CHANNEL-SYNTAX`: a lowercase segment,
/// optionally one `/` and a second such segment.
///
/// ```
/// use vibe_core::ChannelName;
///
/// assert_eq!(ChannelName::parse("lts/jod").unwrap().as_str(), "lts/jod");
/// assert!(ChannelName::parse("LTS").is_none());
/// // `22` is a version requirement, never a channel.
/// assert!(ChannelName::parse("22").is_none());
/// // The two names vibe implements for every package, template or not.
/// assert!(ChannelName::parse("latest").unwrap().is_reserved());
/// assert!(ChannelName::parse("stable").unwrap().is_reserved());
/// assert!(!ChannelName::parse("lts").unwrap().is_reserved());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChannelName(String);

impl ChannelName {
    /// The reserved channel every package has: the newest version in the
    /// domain (PROP-019 `#selectors`' own meaning of the word).
    pub const LATEST: &'static str = "latest";
    /// The reserved channel every package has: the newest non-prerelease
    /// version in the domain.
    pub const STABLE: &'static str = "stable";

    /// `Some` iff `name` is a channel name by the grammar.
    pub fn parse(name: &str) -> Option<Self> {
        crate::manifest::channel_name_is_valid(name).then(|| ChannelName(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is one of the two names vibe answers itself for every
    /// package rather than reading from a declaration.
    pub fn is_reserved(&self) -> bool {
        self.0 == Self::LATEST || self.0 == Self::STABLE
    }
}

impl fmt::Display for ChannelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
