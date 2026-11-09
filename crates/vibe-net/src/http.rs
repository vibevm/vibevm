//! Every `reqwest` client vibe builds is configured here (`PROP-068#proxy`).

use crate::{NetError, NetworkSession};
use std::sync::{Arc, Weak};

/// What a client without a `[proxy]` route does with the ambient
/// `HTTPS_PROXY` / `HTTP_PROXY` environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ambient {
    /// Keep reqwest's reading of the environment (the incumbent posture).
    Inherit,
    /// Connect directly: a controlled client never inherits an ambient
    /// proxy that source policy has not been told about.
    Refuse,
}

/// The proxy address a request is sent to when px could not serve it: a
/// name that never resolves, so the request fails instead of silently
/// going direct (a failed custom proxy is a direct connection in reqwest).
pub(crate) const UNROUTABLE: &str = "http://vibe-px-unavailable.invalid:9";

/// The builder operations the route needs, for both reqwest flavours.
///
/// ```
/// use std::sync::Arc;
/// use vibe_core::net::ProxyRoute;
/// use vibe_net::{NetworkSession, http::{Ambient, route_in}};
/// let session = Arc::new(NetworkSession::new(ProxyRoute::direct()));
/// // Either flavour of reqwest builder is a `RouteBuilder`.
/// let blocking = route_in(&session, reqwest::blocking::Client::builder(), Ambient::Refuse).unwrap();
/// let asynchronous = route_in(&session, reqwest::Client::builder(), Ambient::Inherit).unwrap();
/// drop((blocking, asynchronous));
/// ```
pub trait RouteBuilder: Sized {
    fn with_proxy(self, proxy: reqwest::Proxy) -> Self;
    fn without_proxy(self) -> Self;
    fn with_root(self, root: reqwest::Certificate) -> Self;
}

impl RouteBuilder for reqwest::blocking::ClientBuilder {
    fn with_proxy(self, proxy: reqwest::Proxy) -> Self {
        self.proxy(proxy)
    }
    fn without_proxy(self) -> Self {
        self.no_proxy()
    }
    fn with_root(self, root: reqwest::Certificate) -> Self {
        self.add_root_certificate(root)
    }
}

impl RouteBuilder for reqwest::ClientBuilder {
    fn with_proxy(self, proxy: reqwest::Proxy) -> Self {
        self.proxy(proxy)
    }
    fn without_proxy(self) -> Self {
        self.no_proxy()
    }
    fn with_root(self, root: reqwest::Certificate) -> Self {
        self.add_root_certificate(root)
    }
}

/// Apply the current operation's route ([`crate::current`]) to `builder`.
///
/// ```
/// use vibe_net::http::{Ambient, route};
/// let client = route(reqwest::blocking::Client::builder(), Ambient::Inherit)
///     .unwrap()
///     .build()
///     .unwrap();
/// drop(client);
/// ```
pub fn route<B: RouteBuilder>(builder: B, ambient: Ambient) -> Result<B, NetError> {
    route_in(&crate::current(), builder, ambient)
}

/// Apply `session`'s route to `builder`: with `[proxy]`, every request
/// whose host is not bypassed goes through the proxy (HTTP `CONNECT` for
/// `https://`), the ambient proxy environment is ignored, and `ca_file` is
/// trusted together with the operating system's roots. Without `[proxy]`,
/// `ambient` decides.
pub fn route_in<B: RouteBuilder>(
    session: &Arc<NetworkSession>,
    builder: B,
    ambient: Ambient,
) -> Result<B, NetError> {
    let mut builder = builder;
    for root in session.extra_roots()? {
        builder = builder.with_root(root);
    }
    if !session.route().is_proxied() {
        return Ok(match ambient {
            Ambient::Inherit => builder,
            Ambient::Refuse => builder.without_proxy(),
        });
    }
    let weak: Weak<NetworkSession> = Arc::downgrade(session);
    let proxy = reqwest::Proxy::custom(move |url: &url::Url| -> Option<String> {
        let Some(session) = weak.upgrade() else {
            return Some(UNROUTABLE.to_owned());
        };
        match session.proxy_for(url) {
            Ok(choice) => choice.map(String::from),
            Err(error) => {
                tracing::warn!("{error}");
                Some(UNROUTABLE.to_owned())
            }
        }
    });
    Ok(builder.with_proxy(proxy))
}
