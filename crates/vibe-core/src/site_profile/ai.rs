//! `[ai]` of a site profile (`PROP-019#site-profile`): where a developer
//! gets an API key and which endpoint `vibe ai setup` proposes. Never a key.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use serde::{Deserialize, Serialize};

use super::SiteProfileError;

/// `[ai]` — an optional API-key acquisition hint and default endpoint.
///
/// ```
/// use vibe_core::site_profile::AiHints;
/// let ai: AiHints = toml::from_str(
///     "key_hint_label = 'Example AI Hub'\ndefault_endpoint = 'https://ai.example.com/v1'",
/// )
/// .unwrap();
/// assert_eq!(ai.key_hint_label.as_deref(), Some("Example AI Hub"));
/// assert_eq!(ai.key_hint_url, None);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiHints {
    /// Where to get a key, as plain text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_hint_label: Option<String>,
    /// An optional page for that hint; never opened automatically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_hint_url: Option<String>,
    /// The endpoint `vibe ai setup` proposes for a new connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_endpoint: Option<String>,
}

impl AiHints {
    pub(super) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub(super) fn validate(&self, origin: &str) -> Result<(), SiteProfileError> {
        let invalid = |key: &str, reason: &str, fix: &str| SiteProfileError::Invalid {
            origin: origin.to_owned(),
            key: key.to_owned(),
            reason: reason.to_owned(),
            fix: fix.to_owned(),
        };
        if let Some(label) = &self.key_hint_label
            && (label.trim().is_empty() || label.len() > 256 || label.chars().any(char::is_control))
        {
            return Err(invalid(
                "ai.key_hint_label",
                "it is empty, longer than 256 bytes or has control characters",
                "write a short plain-text hint",
            ));
        }
        if let Some(url) = &self.key_hint_url {
            if self.key_hint_label.is_none() {
                return Err(invalid(
                    "ai.key_hint_url",
                    "a hint URL needs `key_hint_label`",
                    "add `key_hint_label` or drop the URL",
                ));
            }
            if !web_url(url, false) {
                return Err(invalid(
                    "ai.key_hint_url",
                    "it is not an http or https URL without credentials",
                    "give a plain https page address",
                ));
            }
        }
        if let Some(endpoint) = &self.default_endpoint
            && !web_url(endpoint, true)
        {
            return Err(invalid(
                "ai.default_endpoint",
                "it is not an https URL (or http on a loopback address) without credentials, query or fragment",
                "give the endpoint as `vibe ai connections add --endpoint` accepts it",
            ));
        }
        Ok(())
    }
}

/// An `http(s)` URL without credentials; an `endpoint` also refuses a query,
/// a fragment and plain `http` off the loopback address.
fn web_url(text: &str, endpoint: bool) -> bool {
    let Ok(url) = url::Url::parse(text) else {
        return false;
    };
    if !url.username().is_empty() || url.password().is_some() || url.host_str().is_none() {
        return false;
    }
    if !endpoint {
        return matches!(url.scheme(), "https" | "http");
    }
    let local = match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    url.query().is_none()
        && url.fragment().is_none()
        && (url.scheme() == "https" || (url.scheme() == "http" && local))
}
