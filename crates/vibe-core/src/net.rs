//! The network route of one installation: which proxy a URL goes through,
//! which hosts bypass it and which extra certificate authorities are
//! trusted (`PROP-068#proxy`).
//!
//! This module only decides. The HTTP clients, the managed px process and
//! the child `git` configuration that act on the decision live in
//! `vibe-net`; the transfer record reads the same decision so that what a
//! report says about a connection is what the client did.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy");

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::user_config::{PacSource, ProxyConfig, ProxyMode, RevocationCheck};

mod bypass;
pub use bypass::Bypass;

mod layers;
pub use layers::{
    MODE_ENV, ModeOrigin, PAC_ENV, REVOCATION_ENV, effective_mode, effective_pac,
    effective_revocation,
};

pub mod system_proxy;
pub use system_proxy::{
    CONNECTION_AUTO_CONFIG_URL, CONNECTION_AUTO_DETECT, CONNECTION_DIRECT, CONNECTION_PROXY,
    ConnectionSettings, IeProxyConfig, SystemProxy, SystemProxyError, SystemProxyOrigin,
    UnixProxySource, WindowsProxySource, bypass_list, installed_mac_source, installed_source,
    installed_unix_source, looks_like_pac, parse_connection_settings, pick_upstream,
    resolve_mac_with, resolve_unix_with, resolve_with,
};

/// Why a `[proxy]` table cannot become a route. Cites the spec and the fix.
///
/// ```
/// use vibe_core::net::ProxyRoute;
/// use vibe_core::user_config::ProxyConfig;
/// let cfg: ProxyConfig = toml::from_str("mode = 'px'").unwrap();
/// let error = ProxyRoute::from_config(&cfg).unwrap_err().to_string();
/// assert!(error.contains("proxy.pac") && error.contains("PROP-068#proxy") && error.contains("fix:"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#proxy")]
pub enum ProxyRouteError {
    #[error(
        "`[proxy]` key `{key}` is invalid: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; fix: {fix})"
    )]
    Invalid {
        key: &'static str,
        reason: String,
        fix: &'static str,
    },
}

/// What vibe-managed px is started with (`mode = "px"`).
///
/// ```
/// use vibe_core::net::{ModeOrigin, ProxyRoute};
/// use vibe_core::user_config::{PacSource, ProxyConfig};
/// let cfg: ProxyConfig =
///     toml::from_str("mode='px'\npac='https://pac.example.corp/proxy.pac'\npx_port=3129").unwrap();
/// let route = ProxyRoute::from_config(&cfg).unwrap();
/// let px = route.px().unwrap();
/// assert!(matches!(px.pac, Some(PacSource::Url(_))));
/// assert_eq!((px.port, px.pac_origin), (Some(3129), ModeOrigin::User));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PxSettings {
    /// `--pac=<url or path>`, or the system settings (no `--pac`).
    pub pac: Option<PacSource>,
    /// Which layer chose the PAC location.
    pub pac_origin: ModeOrigin,
    /// `--proxy=<host:port>`.
    pub upstream: Option<String>,
    /// Extra arguments, already checked against the ones vibe owns.
    pub args: Vec<String>,
    /// An explicit px executable, used instead of the installation's.
    pub path: Option<PathBuf>,
    /// A fixed loopback port (`[proxy] px_port`); `None` is a free one.
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Direct,
    Explicit {
        https: Option<url::Url>,
        http: Option<url::Url>,
    },
    Px(PxSettings),
}

/// Where one request goes (`PROP-068#proxy`).
///
/// ```
/// use vibe_core::net::{ProxyChoice, ProxyRoute};
/// use vibe_core::user_config::ProxyConfig;
/// let cfg: ProxyConfig = toml::from_str("mode='px'\nupstream='proxy.example.corp:8080'").unwrap();
/// let url = url::Url::parse("https://github.com/").unwrap();
/// assert_eq!(ProxyRoute::from_config(&cfg).unwrap().choice_for(&url), Some(ProxyChoice::ManagedPx));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyChoice<'a> {
    /// Through this proxy URL (it may carry credentials; never print it —
    /// [`ProxyRoute::label_for`] is the printable form).
    Explicit(&'a url::Url),
    /// Through the operation's vibe-managed px.
    ManagedPx,
}

/// The route every client and child `git` of this installation follows.
///
/// ```
/// use vibe_core::net::{ProxyChoice, ProxyRoute};
/// use vibe_core::user_config::ProxyConfig;
///
/// let cfg: ProxyConfig = toml::from_str(
///     "https = 'http://user:secret@proxy.example.corp:8080'\nno_proxy = '.corp.example'",
/// ).unwrap();
/// let route = ProxyRoute::from_config(&cfg).unwrap();
/// let github = url::Url::parse("https://github.com/org/repo.git").unwrap();
/// assert!(matches!(route.choice_for(&github), Some(ProxyChoice::Explicit(_))));
/// // The printable form never carries the credential.
/// assert_eq!(route.label_for("https://github.com/a").as_deref(), Some("http://proxy.example.corp:8080"));
/// // Bypassed and loopback hosts connect directly.
/// assert_eq!(route.label_for("https://nexus.corp.example/a"), None);
/// assert_eq!(route.label_for("http://127.0.0.1:8080/a"), None);
/// // `http://` has no proxy of its own here.
/// assert_eq!(route.label_for("http://example.test/a"), None);
/// // No `[proxy]`: no route, nothing proxied.
/// assert!(!ProxyRoute::from_config(&ProxyConfig::default()).unwrap().is_proxied());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyRoute {
    mode: Mode,
    origin: ModeOrigin,
    explicit: Option<(Option<url::Url>, Option<url::Url>)>,
    bypass: Bypass,
    no_proxy: Option<String>,
    ca_file: Option<PathBuf>,
    revocation: RevocationCheck,
    revocation_origin: ModeOrigin,
}

impl Default for ProxyRoute {
    fn default() -> Self {
        Self::direct()
    }
}

impl ProxyRoute {
    /// No proxy and no extra trust: the behaviour without `[proxy]`.
    pub fn direct() -> Self {
        Self {
            mode: Mode::Direct,
            origin: ModeOrigin::Default,
            explicit: None,
            bypass: Bypass::default(),
            no_proxy: None,
            ca_file: None,
            revocation: RevocationCheck::Check,
            revocation_origin: ModeOrigin::Default,
        }
    }

    /// The route an effective `[proxy]` table describes.
    pub fn from_config(cfg: &ProxyConfig) -> Result<Self, ProxyRouteError> {
        if let Some((key, reason, fix)) = cfg.problem() {
            return Err(ProxyRouteError::Invalid { key, reason, fix });
        }
        let parse = |key: &'static str, value: &Option<String>| {
            value
                .as_deref()
                .map(|raw| {
                    url::Url::parse(raw).map_err(|_| ProxyRouteError::Invalid {
                        key,
                        reason: "it is not a URL".into(),
                        fix: "write the proxy as `http://host:port`",
                    })
                })
                .transpose()
        };
        let https = parse("proxy.https", &cfg.https)?;
        let http = parse("proxy.http", &cfg.http)?;
        let explicit = (https.is_some() || http.is_some()).then(|| (https.clone(), http.clone()));
        let mode = match cfg.effective_mode() {
            None | Some(ProxyMode::None) => Mode::Direct,
            Some(ProxyMode::Explicit) => Mode::Explicit { https, http },
            Some(ProxyMode::Px) => Mode::Px(PxSettings {
                pac: cfg
                    .pac
                    .as_deref()
                    .map(|raw| {
                        PacSource::parse(raw).map_err(|fix| ProxyRouteError::Invalid {
                            key: "proxy.pac",
                            reason: format!("`{raw}` is not a PAC location"),
                            fix,
                        })
                    })
                    .transpose()?,
                pac_origin: if cfg.pac.is_some() {
                    ModeOrigin::User
                } else {
                    ModeOrigin::Default
                },
                upstream: cfg.upstream.clone(),
                args: cfg.px_args.clone(),
                path: cfg.px_path.clone(),
                port: cfg.px_port,
            }),
        };
        Ok(Self {
            mode,
            origin: if cfg.mode.is_some() {
                ModeOrigin::User
            } else {
                ModeOrigin::Default
            },
            explicit,
            bypass: Bypass::parse(cfg.no_proxy.as_deref().unwrap_or_default()),
            no_proxy: cfg.no_proxy.clone(),
            ca_file: cfg.ca_file.clone(),
            revocation: cfg.revocation.unwrap_or_default(),
            revocation_origin: if cfg.revocation.is_some() {
                ModeOrigin::User
            } else {
                ModeOrigin::Default
            },
        })
    }

    /// This route with the layer that chose its PAC location recorded.
    pub fn with_pac_origin(mut self, origin: ModeOrigin) -> Self {
        if let Mode::Px(settings) = &mut self.mode {
            settings.pac_origin = origin;
        }
        self
    }

    /// This route with the layer that chose its mode recorded.
    pub fn with_origin(mut self, origin: ModeOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Which layer chose the mode.
    pub fn mode_origin(&self) -> ModeOrigin {
        self.origin
    }

    /// The same route with px switched off (`vibe dl --no-px`): the
    /// table's proxy URLs when it has them, else a direct connection.
    ///
    /// ```
    /// use vibe_core::net::ProxyRoute;
    /// use vibe_core::user_config::ProxyConfig;
    /// let cfg: ProxyConfig = toml::from_str("mode='px'\nupstream='proxy.example.corp:8080'").unwrap();
    /// let off = ProxyRoute::from_config(&cfg).unwrap().without_px();
    /// assert_eq!(off.mode_name(), "direct");
    /// assert!(off.px().is_none());
    /// ```
    pub fn without_px(mut self) -> Self {
        if matches!(self.mode, Mode::Px(_)) {
            self.mode = match self.explicit.clone() {
                Some((https, http)) => Mode::Explicit { https, http },
                None => Mode::Direct,
            };
            self.origin = ModeOrigin::Flag;
        }
        self
    }

    /// Whether any request can go through a proxy.
    pub fn is_proxied(&self) -> bool {
        !matches!(self.mode, Mode::Direct)
    }

    /// Whether the route needs a vibe-managed px.
    pub fn px(&self) -> Option<&PxSettings> {
        match &self.mode {
            Mode::Px(settings) => Some(settings),
            _ => None,
        }
    }

    /// The configured mode name (`direct`, `explicit`, `px`).
    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            Mode::Direct => "direct",
            Mode::Explicit { .. } => "explicit",
            Mode::Px(_) => "px",
        }
    }

    /// The extra certificate authorities (`[proxy] ca_file`).
    pub fn ca_file(&self) -> Option<&Path> {
        self.ca_file.as_deref()
    }

    /// Whether a child `git` on Windows checks certificate revocation
    /// (`[proxy] revocation`, `PROP-068#proxy-revocation`).
    pub fn revocation(&self) -> RevocationCheck {
        self.revocation
    }

    /// Which layer chose the revocation setting.
    pub fn revocation_origin(&self) -> ModeOrigin {
        self.revocation_origin
    }

    /// This route with the revocation setting and the layer that chose it
    /// recorded (the environment override over the user's file over the
    /// site profile).
    pub fn with_revocation(mut self, revocation: RevocationCheck, origin: ModeOrigin) -> Self {
        self.revocation = revocation;
        self.revocation_origin = origin;
        self
    }

    /// The `https` / `http` proxy URLs of an explicit route, as written
    /// (they may carry a credential — [`ProxyRoute::label_for`] is the
    /// printable form). `None` unless the mode is `explicit`.
    ///
    /// ```
    /// use vibe_core::net::ProxyRoute;
    /// use vibe_core::user_config::ProxyConfig;
    /// let cfg: ProxyConfig = toml::from_str("https = 'http://proxy.example.corp:3128'").unwrap();
    /// let route = ProxyRoute::from_config(&cfg).unwrap();
    /// let (https, http) = route.explicit_urls().unwrap();
    /// assert_eq!(https.map(url::Url::to_string).as_deref(), Some("http://proxy.example.corp:3128/"));
    /// assert!(http.is_none());
    /// assert!(ProxyRoute::direct().explicit_urls().is_none());
    /// ```
    pub fn explicit_urls(&self) -> Option<(Option<&url::Url>, Option<&url::Url>)> {
        match &self.mode {
            Mode::Explicit { https, http } => Some((https.as_ref(), http.as_ref())),
            _ => None,
        }
    }

    /// This route with px bound to `port` instead of `[proxy] px_port` or
    /// a free one (`vibe tool px --port`); nothing changes off `px` mode.
    ///
    /// ```
    /// use vibe_core::net::ProxyRoute;
    /// use vibe_core::user_config::ProxyConfig;
    /// let cfg: ProxyConfig = toml::from_str("mode='px'\nupstream='proxy.example.corp:8080'").unwrap();
    /// let route = ProxyRoute::from_config(&cfg).unwrap().with_px_port(3129);
    /// assert_eq!(route.px().unwrap().port, Some(3129));
    /// ```
    pub fn with_px_port(mut self, port: u16) -> Self {
        if let Mode::Px(settings) = &mut self.mode {
            settings.port = Some(port);
        }
        self
    }

    /// The bypass list.
    pub fn bypass(&self) -> &Bypass {
        &self.bypass
    }

    /// The bypass list as written, for a child's `NO_PROXY`.
    pub fn no_proxy(&self) -> Option<&str> {
        self.no_proxy.as_deref()
    }

    /// Where a request to `url` goes; `None` is a direct connection.
    pub fn choice_for(&self, url: &url::Url) -> Option<ProxyChoice<'_>> {
        let host = url.host_str()?;
        if self.bypass.matches(host) {
            return None;
        }
        match (&self.mode, url.scheme()) {
            (Mode::Direct, _) => None,
            (Mode::Explicit { https, .. }, "https") => https.as_ref().map(ProxyChoice::Explicit),
            (Mode::Explicit { http, .. }, "http") => http.as_ref().map(ProxyChoice::Explicit),
            (Mode::Px(_), "https" | "http") => Some(ProxyChoice::ManagedPx),
            _ => None,
        }
    }

    /// The printable proxy a request to `raw` goes through — scheme, host
    /// and port, never a credential — or `px` for the managed px; `None`
    /// for a direct connection or a URL that is not HTTP(S).
    pub fn label_for(&self, raw: &str) -> Option<String> {
        let url = url::Url::parse(raw).ok()?;
        match self.choice_for(&url)? {
            ProxyChoice::Explicit(proxy) => Some(printable(proxy)),
            ProxyChoice::ManagedPx => Some("px".to_owned()),
        }
    }
}

/// `scheme://host:port` of a proxy URL: no user, password, path or query.
///
/// ```
/// let url = url::Url::parse("http://u:p@proxy.example.corp:8080/x?y").unwrap();
/// assert_eq!(vibe_core::net::printable(&url), "http://proxy.example.corp:8080");
/// ```
pub fn printable(proxy: &url::Url) -> String {
    format!(
        "{}://{}{}",
        proxy.scheme(),
        proxy.host_str().unwrap_or("proxy"),
        proxy
            .port_or_known_default()
            .map(|port| format!(":{port}"))
            .unwrap_or_default()
    )
}

static INSTALLED: OnceLock<ProxyRoute> = OnceLock::new();

/// Install this process's route, once, at the composition root. A second
/// call keeps the first route and returns `false`.
pub fn install_route(route: ProxyRoute) -> bool {
    INSTALLED.set(route).is_ok()
}

/// The installed route; a process that installed none routes directly.
pub fn route() -> &'static ProxyRoute {
    static DIRECT: OnceLock<ProxyRoute> = OnceLock::new();
    INSTALLED
        .get()
        .unwrap_or_else(|| DIRECT.get_or_init(ProxyRoute::direct))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_route_covers_both_schemes_and_respects_bypass() {
        let cfg: ProxyConfig = toml::from_str(
            "mode='px'\nupstream='proxy.example.corp:8080'\nno_proxy='git.corp.example'",
        )
        .unwrap();
        let route = ProxyRoute::from_config(&cfg).unwrap();
        assert_eq!(route.mode_name(), "px");
        assert_eq!(
            route.label_for("http://example.test/").as_deref(),
            Some("px")
        );
        assert_eq!(
            route.label_for("https://example.test/").as_deref(),
            Some("px")
        );
        assert_eq!(route.label_for("https://git.corp.example/r.git"), None);
        assert_eq!(route.label_for("ssh://example.test/r.git"), None);
        assert_eq!(route.label_for("not a url"), None);
    }

    #[test]
    fn ca_file_alone_is_not_a_proxy() {
        let cfg: ProxyConfig = toml::from_str("ca_file='/etc/corp-ca.pem'").unwrap();
        let route = ProxyRoute::from_config(&cfg).unwrap();
        assert!(!route.is_proxied());
        assert_eq!(route.ca_file(), Some(Path::new("/etc/corp-ca.pem")));
    }
}
