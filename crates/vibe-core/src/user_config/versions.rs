//! `[versions]` — how a requirement over an application package chooses
//! among the versions that satisfy it (PROP-075 `#VERSIONS-SELECT`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT");

use serde::{Deserialize, Serialize};
use specmark::spec;

pub use crate::version_select::VersionSelect;

/// `[versions]` section — today the one knob of version selection.
///
/// The value is `Option` rather than a defaulted enum on purpose: the
/// site profile layers under the user's own file, and only an absent key
/// can fall back to the site's ([`crate::user_config::UserConfig::over_site`]).
/// A profile that sets `drop-only` for a closed network is therefore
/// overridable per developer, and a developer who wrote nothing inherits
/// the site's posture.
///
/// ```
/// use vibe_core::user_config::{VersionSelect, VersionsConfig};
///
/// assert!(VersionsConfig::default().is_default());
/// assert_eq!(VersionsConfig::default().effective(), VersionSelect::Newest);
/// let c: VersionsConfig = toml::from_str("select = \"prefer-drop\"").unwrap();
/// assert_eq!(c.effective(), VersionSelect::PreferDrop);
/// assert!(!c.is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT")]
pub struct VersionsConfig {
    /// `select = "newest" | "prefer-drop" | "drop-only"`; absent means
    /// [`VersionSelect::Newest`], which is nvm's own rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<VersionSelect>,
}

impl VersionsConfig {
    /// `true` for the all-empty table, skipped on write.
    pub fn is_default(&self) -> bool {
        *self == VersionsConfig::default()
    }

    /// The declared policy, else the built-in default.
    pub fn effective(&self) -> VersionSelect {
        self.select.unwrap_or_default()
    }
}
