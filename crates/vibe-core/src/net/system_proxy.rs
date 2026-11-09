//! `[proxy] pac = "system"` resolved by vibe, not by px
//! (`PROP-068#proxy-system`).
//!
//! px given no `--pac` auto-detects the Windows proxy configuration
//! itself, and on a managed desktop that detection fails: the PAC URL
//! lives inside the `DefaultConnectionSettings` blob of the user's
//! `Connections` key and the plain `AutoConfigURL` value is often absent,
//! so px finds nothing and every request goes nowhere. vibe therefore
//! reads the configuration itself and hands px an explicit `--pac=<url>`
//! (or `--proxy=<host:port>` when the configuration names a fixed proxy
//! instead), so px never has to detect anything.
//!
//! The resolution order is [`resolve_with`], a pure function over the
//! [`WindowsProxySource`] seam: the order, the `DefaultConnectionSettings`
//! parser ([`parse_connection_settings`]) and the proxy-string reduction
//! ([`pick_upstream`], [`bypass_list`]) are platform-independent and
//! tested on every host. Only the reader behind the seam is Windows.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

use std::sync::{Arc, RwLock};

/// The registry path of the Internet Settings policy, under `HKCU` and
/// then `HKLM`, whose `AutoConfigURL` outranks the user's own choice.
pub const POLICY_SUBKEY: &str =
    r"Software\Policies\Microsoft\Windows\CurrentVersion\Internet Settings";

/// The registry path of the user's own Internet Settings: its
/// `AutoConfigURL` value and its `Connections\DefaultConnectionSettings`
/// blob are the fallbacks when the WinHTTP API answers nothing.
pub const USER_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

/// Which Windows setting answered (`PROP-068#proxy-system`). Reported beside
/// the resolved location, so a user who disagrees with the answer knows
/// which setting to change.
///
/// ```
/// use vibe_core::net::SystemProxyOrigin;
/// // The policy outranks the user's own choice, so it is the first the
/// // resolution reports and the first it reads.
/// assert_eq!(SystemProxyOrigin::PolicyUser.as_str(), "policy HKCU");
/// assert_eq!(
///     SystemProxyOrigin::ConnectionSettings.as_str(),
///     "user Internet Options DefaultConnectionSettings"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemProxyOrigin {
    /// `HKCU\…\Policies\…\Internet Settings` `AutoConfigURL`.
    PolicyUser,
    /// `HKLM\…\Policies\…\Internet Settings` `AutoConfigURL`.
    PolicyMachine,
    /// `WinHttpGetIEProxyConfigForCurrentUser` — the user's configuration
    /// as Windows itself exposes it, the `Connections` blob included.
    InternetOptions,
    /// The `AutoConfigURL` value of the user's Internet Settings, read
    /// directly because the API answered nothing.
    AutoConfigUrlValue,
    /// The `DefaultConnectionSettings` blob, parsed directly because the
    /// API answered nothing.
    ConnectionSettings,
    /// `WinHttpDetectAutoProxyConfigUrl` (DHCP then DNS A) — the
    /// configuration asks for auto-detection and names no URL.
    Wpad,
}

impl SystemProxyOrigin {
    /// The phrase a report prints after `from`.
    ///
    /// ```
    /// use vibe_core::net::SystemProxyOrigin;
    /// assert_eq!(SystemProxyOrigin::PolicyMachine.as_str(), "policy HKLM");
    /// assert_eq!(SystemProxyOrigin::InternetOptions.as_str(), "user Internet Options");
    /// assert_eq!(SystemProxyOrigin::Wpad.as_str(), "WPAD");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            SystemProxyOrigin::PolicyUser => "policy HKCU",
            SystemProxyOrigin::PolicyMachine => "policy HKLM",
            SystemProxyOrigin::InternetOptions => "user Internet Options",
            SystemProxyOrigin::AutoConfigUrlValue => "user Internet Options AutoConfigURL",
            SystemProxyOrigin::ConnectionSettings => {
                "user Internet Options DefaultConnectionSettings"
            }
            SystemProxyOrigin::Wpad => "WPAD",
        }
    }
}

/// The route the current user's Windows proxy configuration describes:
/// exactly one of a PAC location or a fixed upstream proxy, the bypass
/// list the same setting carried, and which setting answered.
///
/// ```
/// use vibe_core::net::{SystemProxy, SystemProxyOrigin};
/// let resolved = SystemProxy {
///     pac_url: Some("https://pac.example.corp/proxy.pac".into()),
///     upstream: None,
///     bypass: None,
///     origin: SystemProxyOrigin::InternetOptions,
/// };
/// // What px is given, and the one phrase every report prints.
/// assert_eq!(resolved.pac_url.as_deref(), Some("https://pac.example.corp/proxy.pac"));
/// assert!(resolved.describe().starts_with("system \u{2192} https://pac.example.corp"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemProxy {
    /// The PAC URL px is given as `--pac=<url>`.
    pub pac_url: Option<String>,
    /// The fixed upstream proxy px is given as `--proxy=<host:port>`,
    /// when the configuration names one instead of a PAC file.
    pub upstream: Option<String>,
    /// The configuration's bypass list, in vibe's `no_proxy` spelling
    /// ([`bypass_list`]); `None` when it named none.
    pub bypass: Option<String>,
    /// Which setting answered.
    pub origin: SystemProxyOrigin,
}

impl SystemProxy {
    /// `system → <what px is given> (from <setting>)` — the one phrase
    /// every report prints.
    ///
    /// ```
    /// use vibe_core::net::{SystemProxy, SystemProxyOrigin};
    /// let pac = SystemProxy {
    ///     pac_url: Some("https://pac.example.corp/proxy.pac".into()),
    ///     upstream: None,
    ///     bypass: None,
    ///     origin: SystemProxyOrigin::InternetOptions,
    /// };
    /// assert_eq!(
    ///     pac.describe(),
    ///     "system → https://pac.example.corp/proxy.pac (from user Internet Options)"
    /// );
    /// let fixed = SystemProxy {
    ///     pac_url: None,
    ///     upstream: Some("proxy.example.corp:8080".into()),
    ///     bypass: Some(".corp.example".into()),
    ///     origin: SystemProxyOrigin::ConnectionSettings,
    /// };
    /// assert_eq!(
    ///     fixed.describe(),
    ///     "system → proxy proxy.example.corp:8080 · bypass .corp.example \
    ///      (from user Internet Options DefaultConnectionSettings)"
    /// );
    /// ```
    pub fn describe(&self) -> String {
        let what = match (&self.pac_url, &self.upstream) {
            (Some(url), _) => url.clone(),
            (None, Some(upstream)) => format!("proxy {upstream}"),
            (None, None) => "nothing".to_owned(),
        };
        let bypass = self
            .bypass
            .as_deref()
            .map(|list| format!(" · bypass {list}"))
            .unwrap_or_default();
        format!("system → {what}{bypass} (from {})", self.origin.as_str())
    }
}

/// Why `pac = "system"` could not be resolved. Each message names what
/// was checked and the fix, and carries no credential.
///
/// ```
/// use vibe_core::net::SystemProxyError;
/// let error = SystemProxyError::NotConfigured {
///     checked: vec!["HKCU\\…\\Internet Settings AutoConfigURL: not set".into()],
/// };
/// let text = error.to_string();
/// assert!(text.contains("AutoConfigURL: not set"), "{text}");
/// assert!(text.contains("PROP-068#proxy-system") && text.contains("fix:"), "{text}");
/// assert!(SystemProxyError::NotWindows.to_string().contains("not Windows"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system")]
pub enum SystemProxyError {
    #[error(
        "`[proxy] pac = \"system\"` reads the Windows proxy configuration of the logged-in \
         user; this host is not Windows \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system; \
          fix: set `[proxy] pac = \"<your PAC URL>\"` or an absolute PAC file path, or \
          `[proxy] upstream = \"host:port\"`)"
    )]
    NotWindows,
    #[error(
        "`[proxy] pac = \"system\"` found no proxy in the Windows configuration of the \
         logged-in user: {} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system; \
          fix: set `[proxy] pac = \"<your PAC URL>\"` or an absolute PAC file path, or \
          `[proxy] upstream = \"host:port\"`; or configure the proxy in Windows \
          Internet Options and rerun)",
        checked.join("; ")
    )]
    NotConfigured {
        /// Every setting that was read, in order, with what it said.
        checked: Vec<String>,
    },
}

/// The current user's Windows proxy configuration as
/// `WinHttpGetIEProxyConfigForCurrentUser` returns it.
///
/// ```
/// use vibe_core::net::IeProxyConfig;
/// // The shape a managed desktop has: a PAC URL and auto-detection on,
/// // with no fixed proxy server.
/// let config = IeProxyConfig {
///     auto_detect: true,
///     auto_config_url: Some("https://pac.example.corp/proxy.pac".into()),
///     ..Default::default()
/// };
/// assert_eq!(config.proxy, None);
/// // Nothing configured at all is the default.
/// assert_eq!(IeProxyConfig::default(), IeProxyConfig {
///     auto_detect: false,
///     auto_config_url: None,
///     proxy: None,
///     proxy_bypass: None,
/// });
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IeProxyConfig {
    /// `fAutoDetect`: the configuration asks for WPAD.
    pub auto_detect: bool,
    /// `lpszAutoConfigUrl`.
    pub auto_config_url: Option<String>,
    /// `lpszProxy`, as Windows spells it (`host:port`, or
    /// `http=host:port;https=host:port`).
    pub proxy: Option<String>,
    /// `lpszProxyBypass`, as Windows spells it (`;`-separated, `<local>`).
    pub proxy_bypass: Option<String>,
}

impl IeProxyConfig {
    /// Whether the API said anything usable at all. When it did not, the
    /// registry fallbacks run.
    fn answered(&self) -> bool {
        self.auto_detect
            || self.auto_config_url.is_some()
            || self.proxy.as_deref().is_some_and(|p| !p.trim().is_empty())
    }
}

/// The Windows settings the resolution reads. The seam that keeps the
/// order logic pure: production installs the Windows reader, a test
/// installs a stub, and neither depends on the host's own settings.
///
/// Every method answers `Ok(None)` for "the setting is not there" and
/// `Err(reason)` for "it could not be read"; a reader failure is reported
/// among the settings that were checked, never swallowed.
///
/// ```
/// use std::sync::Arc;
/// use vibe_core::net::{IeProxyConfig, SystemProxyOrigin, WindowsProxySource, resolve_with};
///
/// /// A machine whose Internet Options name one PAC URL.
/// #[derive(Debug)]
/// struct Desktop;
/// impl WindowsProxySource for Desktop {
///     fn policy_auto_config_url(&self, _machine: bool) -> Result<Option<String>, String> {
///         Ok(None)
///     }
///     fn ie_proxy_config(&self) -> Result<Option<IeProxyConfig>, String> {
///         Ok(Some(IeProxyConfig {
///             auto_config_url: Some("https://pac.example.corp/proxy.pac".into()),
///             ..Default::default()
///         }))
///     }
///     fn auto_config_url_value(&self) -> Result<Option<String>, String> { Ok(None) }
///     fn default_connection_settings(&self) -> Result<Option<Vec<u8>>, String> { Ok(None) }
///     fn detect_auto_proxy_url(&self) -> Result<Option<String>, String> { Ok(None) }
/// }
///
/// // Driven directly…
/// let resolved = resolve_with(&Desktop).unwrap();
/// assert_eq!(resolved.origin, SystemProxyOrigin::InternetOptions);
/// // …or installed, so a whole operation reads it instead of this host.
/// vibe_core::net::system_proxy::install_source(Some(Arc::new(Desktop)));
/// assert!(vibe_core::net::system_proxy::installed_source().is_some());
/// vibe_core::net::system_proxy::install_source(None);
/// ```
pub trait WindowsProxySource: std::fmt::Debug + Send + Sync {
    /// `AutoConfigURL` under [`POLICY_SUBKEY`], in `HKLM` when `machine`
    /// and `HKCU` otherwise.
    fn policy_auto_config_url(&self, machine: bool) -> Result<Option<String>, String>;
    /// `WinHttpGetIEProxyConfigForCurrentUser`.
    fn ie_proxy_config(&self) -> Result<Option<IeProxyConfig>, String>;
    /// The `AutoConfigURL` value under [`USER_SUBKEY`].
    fn auto_config_url_value(&self) -> Result<Option<String>, String>;
    /// The raw `Connections\DefaultConnectionSettings` blob under
    /// [`USER_SUBKEY`].
    fn default_connection_settings(&self) -> Result<Option<Vec<u8>>, String>;
    /// `WinHttpDetectAutoProxyConfigUrl(DHCP | DNS_A)`.
    fn detect_auto_proxy_url(&self) -> Result<Option<String>, String>;
}

static OVERRIDE: RwLock<Option<Arc<dyn WindowsProxySource>>> = RwLock::new(None);

/// Install a [`WindowsProxySource`] for [`resolve`] to read instead of the
/// host's own settings, or `None` to restore the host. The test seam: no
/// production path calls it, and a test that uses it is not at the mercy
/// of the machine it runs on.
pub fn install_source(source: Option<Arc<dyn WindowsProxySource>>) {
    match OVERRIDE.write() {
        Ok(mut slot) => *slot = source,
        Err(poisoned) => *poisoned.into_inner() = source,
    }
}

/// The source [`install_source`] put in place, if any. `vibe-net` owns
/// the resolution itself (`vibe_net::system_proxy::resolve`): the Windows
/// reader needs `unsafe` for the WinHTTP and registry calls, which this
/// crate forbids, so the order logic lives here and the reader there.
pub fn installed_source() -> Option<Arc<dyn WindowsProxySource>> {
    match OVERRIDE.read() {
        Ok(slot) => slot.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// The resolution order, first hit wins (`PROP-068#proxy-system`):
///
/// 1. the group-policy `AutoConfigURL`, `HKCU` then `HKLM`;
/// 2. the user's configuration as Windows exposes it
///    (`WinHttpGetIEProxyConfigForCurrentUser`, which covers the
///    `Connections` blob) — and, only when that answers nothing, the
///    `AutoConfigURL` value and then the `DefaultConnectionSettings` blob
///    read directly;
/// 3. auto-detection alone (`fAutoDetect` with no URL) →
///    `WinHttpDetectAutoProxyConfigUrl`;
/// 4. no PAC but a fixed proxy server → that proxy as px's upstream, with
///    the configuration's bypass list;
/// 5. nothing → [`SystemProxyError::NotConfigured`], naming every setting
///    that was read.
///
/// ```
/// use std::collections::BTreeMap;
/// use vibe_core::net::{IeProxyConfig, SystemProxyOrigin, WindowsProxySource, resolve_with};
///
/// #[derive(Debug, Default)]
/// struct Stub(Option<IeProxyConfig>);
/// impl WindowsProxySource for Stub {
///     fn policy_auto_config_url(&self, _machine: bool) -> Result<Option<String>, String> {
///         Ok(None)
///     }
///     fn ie_proxy_config(&self) -> Result<Option<IeProxyConfig>, String> {
///         Ok(self.0.clone())
///     }
///     fn auto_config_url_value(&self) -> Result<Option<String>, String> { Ok(None) }
///     fn default_connection_settings(&self) -> Result<Option<Vec<u8>>, String> { Ok(None) }
///     fn detect_auto_proxy_url(&self) -> Result<Option<String>, String> {
///         Ok(Some("http://wpad.example.corp/wpad.dat".into()))
///     }
/// }
///
/// let url = resolve_with(&Stub(Some(IeProxyConfig {
///     auto_config_url: Some("https://pac.example.corp/proxy.pac".into()),
///     ..Default::default()
/// })))
/// .unwrap();
/// assert_eq!(url.pac_url.as_deref(), Some("https://pac.example.corp/proxy.pac"));
/// assert_eq!(url.origin, SystemProxyOrigin::InternetOptions);
///
/// // Auto-detection alone asks WPAD.
/// let wpad = resolve_with(&Stub(Some(IeProxyConfig {
///     auto_detect: true,
///     ..Default::default()
/// })))
/// .unwrap();
/// assert_eq!(wpad.origin, SystemProxyOrigin::Wpad);
///
/// // Nothing configured names what was checked.
/// let error = resolve_with(&Stub(None)).unwrap_err().to_string();
/// assert!(error.contains("AutoConfigURL") && error.contains("fix:"), "{error}");
/// ```
pub fn resolve_with(source: &dyn WindowsProxySource) -> Result<SystemProxy, SystemProxyError> {
    let mut checked: Vec<String> = Vec::new();

    // 1. Group policy outranks everything the user can set.
    for (machine, origin) in [
        (false, SystemProxyOrigin::PolicyUser),
        (true, SystemProxyOrigin::PolicyMachine),
    ] {
        let root = if machine { "HKLM" } else { "HKCU" };
        match source.policy_auto_config_url(machine) {
            Ok(Some(url)) if !url.trim().is_empty() => {
                return Ok(SystemProxy {
                    pac_url: Some(url.trim().to_owned()),
                    upstream: None,
                    bypass: None,
                    origin,
                });
            }
            Ok(_) => checked.push(format!("{root}\\{POLICY_SUBKEY} AutoConfigURL: not set")),
            Err(reason) => checked.push(format!("{root}\\{POLICY_SUBKEY} AutoConfigURL: {reason}")),
        }
    }

    // 2. The user's configuration as Windows itself exposes it.
    let api = match source.ie_proxy_config() {
        Ok(Some(config)) => {
            checked.push(format!(
                "WinHttpGetIEProxyConfigForCurrentUser: auto-detect {}, auto-config URL {}, \
                 proxy {}",
                config.auto_detect,
                if config.auto_config_url.is_some() {
                    "set"
                } else {
                    "not set"
                },
                if config.proxy.is_some() {
                    "set"
                } else {
                    "not set"
                },
            ));
            Some(config)
        }
        Ok(None) => {
            checked.push("WinHttpGetIEProxyConfigForCurrentUser: no configuration".to_owned());
            None
        }
        Err(reason) => {
            checked.push(format!("WinHttpGetIEProxyConfigForCurrentUser: {reason}"));
            None
        }
    };
    if let Some(url) = api
        .as_ref()
        .and_then(|config| config.auto_config_url.as_deref())
        .map(str::trim)
        .filter(|url| !url.is_empty())
    {
        return Ok(SystemProxy {
            pac_url: Some(url.to_owned()),
            upstream: None,
            bypass: api
                .as_ref()
                .and_then(|c| bypass_list(c.proxy_bypass.as_deref()?)),
            origin: SystemProxyOrigin::InternetOptions,
        });
    }

    // The registry fallbacks, only when the API said nothing usable: the
    // API reads the same settings, so asking twice is noise when it did.
    let mut auto_detect = api.as_ref().is_some_and(|config| config.auto_detect);
    let mut proxy = api.as_ref().and_then(|config| config.proxy.clone());
    let mut bypass = api
        .as_ref()
        .and_then(|config| config.proxy_bypass.as_deref())
        .and_then(bypass_list);
    if !api.as_ref().is_some_and(IeProxyConfig::answered) {
        match source.auto_config_url_value() {
            Ok(Some(url)) if !url.trim().is_empty() => {
                return Ok(SystemProxy {
                    pac_url: Some(url.trim().to_owned()),
                    upstream: None,
                    bypass,
                    origin: SystemProxyOrigin::AutoConfigUrlValue,
                });
            }
            Ok(_) => checked.push(format!("HKCU\\{USER_SUBKEY} AutoConfigURL: not set")),
            Err(reason) => checked.push(format!("HKCU\\{USER_SUBKEY} AutoConfigURL: {reason}")),
        }
        match source.default_connection_settings() {
            Ok(Some(bytes)) => match parse_connection_settings(&bytes) {
                Ok(settings) => {
                    checked.push(format!(
                        "HKCU\\{USER_SUBKEY}\\Connections DefaultConnectionSettings: flags {:#04x}",
                        settings.flags
                    ));
                    if settings.auto_detect() {
                        auto_detect = true;
                    }
                    if settings.proxy_enabled() && proxy.is_none() {
                        proxy = settings.proxy.clone();
                    }
                    if bypass.is_none() {
                        bypass = settings.bypass.as_deref().and_then(bypass_list);
                    }
                    if let Some(url) = settings
                        .auto_config_url
                        .as_deref()
                        .map(str::trim)
                        .filter(|url| settings.auto_config_url_enabled() && !url.is_empty())
                    {
                        return Ok(SystemProxy {
                            pac_url: Some(url.to_owned()),
                            upstream: None,
                            bypass,
                            origin: SystemProxyOrigin::ConnectionSettings,
                        });
                    }
                }
                Err(reason) => checked.push(format!(
                    "HKCU\\{USER_SUBKEY}\\Connections DefaultConnectionSettings: {reason}"
                )),
            },
            Ok(None) => checked.push(format!(
                "HKCU\\{USER_SUBKEY}\\Connections DefaultConnectionSettings: not set"
            )),
            Err(reason) => checked.push(format!(
                "HKCU\\{USER_SUBKEY}\\Connections DefaultConnectionSettings: {reason}"
            )),
        }
    }

    // 3. Auto-detection with no URL: ask WPAD for one.
    if auto_detect {
        match source.detect_auto_proxy_url() {
            Ok(Some(url)) if !url.trim().is_empty() => {
                return Ok(SystemProxy {
                    pac_url: Some(url.trim().to_owned()),
                    upstream: None,
                    bypass,
                    origin: SystemProxyOrigin::Wpad,
                });
            }
            Ok(_) => checked
                .push("WinHttpDetectAutoProxyConfigUrl (DHCP, DNS A): no URL found".to_owned()),
            Err(reason) => checked.push(format!(
                "WinHttpDetectAutoProxyConfigUrl (DHCP, DNS A): {reason}"
            )),
        }
    }

    // 4. No PAC, but a fixed proxy server: px authenticates to it.
    if let Some(raw) = proxy.as_deref() {
        if let Some(upstream) = pick_upstream(raw) {
            return Ok(SystemProxy {
                pac_url: None,
                upstream: Some(upstream),
                bypass,
                origin: api
                    .as_ref()
                    .filter(|config| config.proxy.is_some())
                    .map(|_| SystemProxyOrigin::InternetOptions)
                    .unwrap_or(SystemProxyOrigin::ConnectionSettings),
            });
        }
        checked.push(format!(
            "the configured proxy server names no `host:port` vibe can hand px ({} entries)",
            raw.split(';').filter(|e| !e.trim().is_empty()).count()
        ));
    }

    // 5. Nothing.
    Err(SystemProxyError::NotConfigured { checked })
}

mod connection_settings;
pub use connection_settings::{
    CONNECTION_AUTO_CONFIG_URL, CONNECTION_AUTO_DETECT, CONNECTION_DIRECT, CONNECTION_PROXY,
    ConnectionSettings, bypass_list, parse_connection_settings, pick_upstream,
};

#[cfg(test)]
#[path = "system_proxy/tests.rs"]
mod tests;
