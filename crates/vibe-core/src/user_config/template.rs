//! `[template]` — the one knob of the template version-list cache
//! (PROP-075 `#TEMPLATE-CACHE`, AV-FMT-005).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE");

use serde::{Deserialize, Serialize};

/// `[template]` section — the one knob of the template version-list cache
/// (PROP-075 `#TEMPLATE-CACHE`, AV-FMT-005).
///
/// A **digest** document is never revalidated: a released version's
/// checksums are immutable, and a change upstream is precisely the alarm
/// of `#TEMPLATE-LOCK-RECORD`, so caching it forever is what makes that
/// alarm meaningful rather than a race. The TTL therefore applies to the
/// version list alone.
///
/// ```
/// use std::time::Duration;
/// use vibe_core::user_config::TemplateConfig;
///
/// assert_eq!(
///     TemplateConfig::default().version_list_ttl(),
///     Duration::from_secs(15 * 60),
///     "the owner's default is fifteen minutes"
/// );
/// let c: TemplateConfig = toml::from_str("version_list_ttl = \"1h\"").unwrap();
/// assert_eq!(c.version_list_ttl(), Duration::from_secs(3600));
/// assert!(!c.is_default());
/// // Zero means «always refetch», which is a choice, not a disabled cache.
/// let eager: TemplateConfig = toml::from_str("version_list_ttl = \"0s\"").unwrap();
/// assert_eq!(eager.version_list_ttl(), Duration::ZERO);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateConfig {
    /// How long a cached version list is used before it is refetched,
    /// written as `<count><s|m|h>`. Absent means the built-in default of
    /// fifteen minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_list_ttl: Option<crate::manifest::Ttl>,
}

impl TemplateConfig {
    /// The built-in default the owner chose: fifteen minutes.
    pub const DEFAULT_VERSION_LIST_TTL: std::time::Duration =
        std::time::Duration::from_secs(15 * 60);

    /// `true` for the all-defaults section.
    pub fn is_default(&self) -> bool {
        *self == TemplateConfig::default()
    }

    /// The effective TTL: the declared one, else the built-in default.
    pub fn version_list_ttl(&self) -> std::time::Duration {
        self.version_list_ttl
            .map_or(Self::DEFAULT_VERSION_LIST_TTL, |ttl| ttl.as_duration())
    }
}
