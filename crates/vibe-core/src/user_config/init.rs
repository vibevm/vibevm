//! `[init]` — the defaults `vibe init` remembers between runs.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#materialise-diff");

use serde::{Deserialize, Serialize};

/// `[init]` section — defaults persisted from `vibe init` prompts.
///
/// ```
/// use vibe_core::user_config::InitConfig;
///
/// let c = InitConfig::default();
/// assert!(c.last_author.is_none());
/// assert!(c.is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InitConfig {
    /// The author name entered in the last interactive `vibe init`.
    /// Saved on first use, updated when the user enters a different
    /// value. Reused as the default for subsequent inits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_author: Option<String>,
}

impl InitConfig {
    pub fn is_default(&self) -> bool {
        *self == InitConfig::default()
    }
}
