//! `[proxy]` — the one proxy table of the user-level config
//! (`PROP-066#locator`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locator");

use serde::{Deserialize, Serialize};

/// `[proxy]` section — the one place a proxy is configured
/// (`PROP-066#locator`).
///
/// A corporate network that allows one artifact store usually allows it
/// through one proxy, and before this table every client in the workspace
/// had to be told separately (or not at all). The three keys are the
/// familiar ones, so an operator who already knows `HTTPS_PROXY` knows
/// this file: [`ProxyConfig::https`] and [`ProxyConfig::http`] are proxy
/// URLs, and [`ProxyConfig::no_proxy`] is the usual comma-separated
/// bypass list. Both halves of a fetch read the same three names: the
/// CLI's composition root promotes the table into the process environment
/// as `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY`, which is what `reqwest`
/// consults for every client vibe builds and what a child `git`
/// inherits — so the two cannot disagree, and no client is told
/// separately.
///
/// A variable already set in the environment wins: this table is a
/// default for a machine, never an override of what an operator exported
/// for one shell.
///
/// ```
/// use vibe_core::user_config::ProxyConfig;
///
/// let p: ProxyConfig = toml::from_str(
///     "https = \"http://proxy.corp.example:3128\"\nno_proxy = \"localhost,.corp.example\"",
/// )
/// .unwrap();
/// assert_eq!(p.https.as_deref(), Some("http://proxy.corp.example:3128"));
/// assert!(!p.is_default());
/// // The all-empty table is the default, and is skipped on write.
/// assert!(ProxyConfig::default().is_default());
/// // The promoted environment is exactly the keys that are set.
/// assert_eq!(p.child_env(), [
///     ("HTTPS_PROXY", "http://proxy.corp.example:3128".to_owned()),
///     ("NO_PROXY", "localhost,.corp.example".to_owned()),
/// ]);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    /// Proxy for `https://` requests, exported as `HTTPS_PROXY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub https: Option<String>,
    /// Proxy for `http://` requests, exported as `HTTP_PROXY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<String>,
    /// Hosts that bypass the proxy, exported as `NO_PROXY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_proxy: Option<String>,
}

impl ProxyConfig {
    /// `true` for the all-empty table — lets the serializer skip
    /// `[proxy]` entirely on a config that never set it.
    pub fn is_default(&self) -> bool {
        *self == ProxyConfig::default()
    }

    /// The `(name, value)` pairs the table promotes, for the keys that are
    /// set. The conventional uppercase spellings: `reqwest`, `git` and
    /// `curl` read both cases, and the uppercase one is what a reader
    /// expects to see in a process listing.
    pub fn child_env(&self) -> Vec<(&'static str, String)> {
        [
            ("HTTPS_PROXY", &self.https),
            ("HTTP_PROXY", &self.http),
            ("NO_PROXY", &self.no_proxy),
        ]
        .into_iter()
        .filter_map(|(name, value)| value.clone().map(|value| (name, value)))
        .collect()
    }
}
