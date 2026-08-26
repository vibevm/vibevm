use std::net::{Ipv4Addr, Ipv6Addr};

use reqwest::Url;
use specmark::spec;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy");

/// A parsed Chat Completions endpoint that passed the credential-aware policy.
#[derive(Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES")]
pub struct Endpoint {
    url: Url,
    literal_loopback: bool,
}

impl std::fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("scheme", &self.url.scheme())
            .field("host", &self.url.host_str())
            .field("port", &self.url.port())
            .field("path", &self.url.path())
            .field("query", &self.url.query().map(|_| "[REDACTED]"))
            .field("literal_loopback", &self.literal_loopback)
            .finish()
    }
}

impl Endpoint {
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES")]
    pub fn parse(raw: &str, has_api_key: bool) -> Result<Self, EndpointError> {
        let url = Url::parse(raw).map_err(|_| EndpointError::Malformed)?;
        if authority_contains_userinfo(raw)
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(EndpointError::UserInfo);
        }
        if url.fragment().is_some() {
            return Err(EndpointError::Fragment);
        }

        let literal_loopback = is_literal_loopback(&url);
        match (has_api_key, url.scheme()) {
            (true, "https") | (false, "https") => Ok(Self {
                url,
                literal_loopback,
            }),
            (true, _) => Err(EndpointError::KeyRequiresHttps),
            (false, "http") if literal_loopback => Ok(Self {
                url,
                literal_loopback,
            }),
            (false, "http") => Err(EndpointError::HttpRequiresLiteralLoopback),
            (false, _) => Err(EndpointError::UnsupportedScheme),
        }
    }

    pub fn as_url(&self) -> &Url {
        &self.url
    }

    /// The complete request URL, including a configured query string.
    ///
    /// # Security
    ///
    /// This is request material, not a diagnostic surface. Do not log or
    /// format it; [`Debug`](std::fmt::Debug) deliberately redacts the query.
    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }

    /// Whether the configured host itself is an exact loopback literal/name.
    /// The transport uses this retained parse-time identity to bypass proxies
    /// for both HTTP and HTTPS loopback endpoints.
    pub fn is_literal_loopback(&self) -> bool {
        self.literal_loopback
    }

    pub(crate) fn accepts_api_key(&self) -> bool {
        self.url.scheme() == "https"
    }
}

fn authority_contains_userinfo(raw: &str) -> bool {
    let Some((_, after_scheme)) = raw.split_once("://") else {
        return false;
    };
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    after_scheme[..authority_end].contains('@')
}

fn is_literal_loopback(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<Ipv4Addr>()
            .is_ok_and(|address| address.is_loopback())
        || host
            .parse::<Ipv6Addr>()
            .is_ok_and(|address| address == Ipv6Addr::LOCALHOST)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES")]
pub enum EndpointError {
    #[error(
        "the configured LLM endpoint is not a valid absolute URL \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: configure a full https URL, or loopback http without a key)"
    )]
    Malformed,
    #[error(
        "the configured LLM endpoint must not contain URL userinfo \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-NEVER-PERSISTED; \
         fix: remove credentials from the URL and use a configured credential source)"
    )]
    UserInfo,
    #[error(
        "the configured LLM endpoint must not contain a fragment \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: remove the URL fragment)"
    )]
    Fragment,
    #[error(
        "an LLM endpoint carrying a credential must use scheme `https` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: configure https)"
    )]
    KeyRequiresHttps,
    #[error(
        "a keyless `http` LLM endpoint must use literal localhost or a loopback IP \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: use https or an exact loopback literal)"
    )]
    HttpRequiresLiteralLoopback,
    #[error(
        "a keyless LLM endpoint must use `https`, or `http` on literal loopback \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: use an allowed URL scheme and host)"
    )]
    UnsupportedScheme,
}
