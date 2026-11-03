//! `[transfer]` — the machine's download settings (`PROP-068#limit`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#limit");

use serde::{Deserialize, Serialize};

/// `[transfer]` section of the user-level config.
///
/// The limit is a machine setting, not a run's: there is no flag, and the
/// desktop shell's Settings writes this key.
///
/// ```
/// use vibe_core::user_config::TransferConfig;
///
/// let config: TransferConfig = toml::from_str("limit = \"5 MB/s\"").unwrap();
/// assert_eq!(config.limit.as_deref(), Some("5 MB/s"));
/// assert!(TransferConfig::default().is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransferConfig {
    /// The sum of the process's HTTP downloads stays under this rate
    /// (`"5 MB/s"`); `"0"` or no key is no limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
}

impl TransferConfig {
    pub fn is_default(&self) -> bool {
        self.limit.is_none()
    }
}
