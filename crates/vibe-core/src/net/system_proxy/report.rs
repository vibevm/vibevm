//! What `pac = "system"` resolved to, where it came from, and why it
//! could not be resolved (`PROP-068#proxy-system`).
//!
//! The report shape is one for every host: the Windows resolution
//! ([`super::resolve_with`]), the Unix desktop/environment order
//! ([`super::resolve_unix_with`]) and the macOS network settings
//! ([`super::resolve_mac_with`]) all answer with the same [`SystemProxy`]
//! and refuse with the same [`SystemProxyError`], so every report prints
//! one phrase and `vibe-net` hands px one kind of answer.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

/// Which setting answered (`PROP-068#proxy-system`). Reported beside the
/// resolved location, so a user who disagrees with the answer knows which
/// setting to change.
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
/// // Off Windows the sources are the desktop's own settings, the macOS
/// // network settings, and the environment.
/// assert_eq!(SystemProxyOrigin::GnomeSettings.as_str(), "GNOME settings");
/// assert_eq!(SystemProxyOrigin::MacNetworkSettings.as_str(), "macOS network settings");
/// assert_eq!(SystemProxyOrigin::EnvironmentHttps.as_str(), "environment https_proxy");
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
    /// `WinHttpDetectAutoProxyConfigUrl` (DHCP then DNS A) — the Windows
    /// configuration asks for auto-detection and names no URL.
    Wpad,
    /// `org.gnome.system.proxy` and its child schemas, as `gsettings`
    /// reports them — the GNOME / GTK desktop's own proxy settings.
    GnomeSettings,
    /// `[Proxy Settings]` of the user's `kioslaverc` — the KDE desktop's
    /// own proxy settings.
    KdeSettings,
    /// The macOS proxy configuration of the current network service, as
    /// `SCDynamicStoreCopyProxies` (`scutil --proxy`) reports it.
    MacNetworkSettings,
    /// `https_proxy` / `HTTPS_PROXY` of the process environment.
    EnvironmentHttps,
    /// `http_proxy` / `HTTP_PROXY` of the process environment.
    EnvironmentHttp,
    /// `all_proxy` / `ALL_PROXY` of the process environment.
    EnvironmentAll,
    /// WPAD discovery done by vibe itself: the desktop or the macOS
    /// settings ask to auto-discover and name no URL, so vibe walks the
    /// host's DNS domains for a `wpad.<domain>/wpad.dat` that answers.
    WpadDiscovery,
}

impl SystemProxyOrigin {
    /// The phrase a report prints after `from`.
    ///
    /// ```
    /// use vibe_core::net::SystemProxyOrigin;
    /// assert_eq!(SystemProxyOrigin::PolicyMachine.as_str(), "policy HKLM");
    /// assert_eq!(SystemProxyOrigin::InternetOptions.as_str(), "user Internet Options");
    /// assert_eq!(SystemProxyOrigin::Wpad.as_str(), "WPAD");
    /// assert_eq!(SystemProxyOrigin::KdeSettings.as_str(), "KDE settings");
    /// assert_eq!(SystemProxyOrigin::WpadDiscovery.as_str(), "WPAD discovery");
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
            SystemProxyOrigin::GnomeSettings => "GNOME settings",
            SystemProxyOrigin::KdeSettings => "KDE settings",
            SystemProxyOrigin::MacNetworkSettings => "macOS network settings",
            SystemProxyOrigin::EnvironmentHttps => "environment https_proxy",
            SystemProxyOrigin::EnvironmentHttp => "environment http_proxy",
            SystemProxyOrigin::EnvironmentAll => "environment all_proxy",
            SystemProxyOrigin::WpadDiscovery => "WPAD discovery",
        }
    }
}

/// The route this host's own proxy configuration describes: exactly one of
/// a PAC location or a fixed upstream proxy, the bypass list the same
/// setting carried, and which setting answered.
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
    /// ([`bypass_list`](super::bypass_list)); `None` when it named none.
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
    /// // The same phrase off Windows, naming the desktop that answered.
    /// let gnome = SystemProxy {
    ///     pac_url: Some("http://wpad.example.corp/wpad.dat".into()),
    ///     upstream: None,
    ///     bypass: None,
    ///     origin: SystemProxyOrigin::GnomeSettings,
    /// };
    /// assert_eq!(
    ///     gnome.describe(),
    ///     "system → http://wpad.example.corp/wpad.dat (from GNOME settings)"
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
///     looked_in: "the Windows configuration of the logged-in user",
///     configure_hint: "configure the proxy in Windows Internet Options and rerun",
///     checked: vec!["HKCU\\…\\Internet Settings AutoConfigURL: not set".into()],
/// };
/// let text = error.to_string();
/// assert!(text.contains("AutoConfigURL: not set"), "{text}");
/// assert!(text.contains("PROP-068#proxy-system") && text.contains("fix:"), "{text}");
/// assert!(SystemProxyError::Unsupported.to_string().contains("neither Windows"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system")]
pub enum SystemProxyError {
    #[error(
        "`[proxy] pac = \"system\"` reads the proxy configuration this operating system is \
         configured with; this host is neither Windows, macOS nor another Unix \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system; \
          fix: set `[proxy] pac = \"<your PAC URL>\"` or an absolute PAC file path, or \
          `[proxy] upstream = \"host:port\"`)"
    )]
    Unsupported,
    #[error(
        "`[proxy] pac = \"system\"` found no proxy in {looked_in}: {} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system; \
          fix: set `[proxy] pac = \"<your PAC URL>\"` or an absolute PAC file path, or \
          `[proxy] upstream = \"host:port\"`; or {configure_hint})",
        checked.join("; ")
    )]
    NotConfigured {
        /// What was read, named as the message's subject: the Windows
        /// configuration, this host's desktop and environment, or the
        /// macOS network settings.
        looked_in: &'static str,
        /// How to configure a proxy on this kind of host, as the fix
        /// clause's last way out.
        configure_hint: &'static str,
        /// Every setting that was read, in order, with what it said.
        checked: Vec<String>,
    },
}

/// What the Windows resolution says it looked in, and how to configure a
/// proxy there. The pair [`SystemProxyError::NotConfigured`] carries when
/// the Windows order found nothing.
pub(crate) const WINDOWS_SCOPE: (&str, &str) = (
    "the Windows configuration of the logged-in user",
    "configure the proxy in Windows Internet Options and rerun",
);

/// The same pair for the Unix desktop/environment order.
pub(crate) const UNIX_SCOPE: (&str, &str) = (
    "this host's desktop and environment proxy configuration",
    "set the proxy in your desktop settings (GNOME, KDE) or export `https_proxy`, and rerun",
);

/// The same pair for the macOS network settings.
pub(crate) const MAC_SCOPE: (&str, &str) = (
    "the macOS proxy configuration of the current network service",
    "set the proxy in System Settings \u{2192} Network \u{2192} Proxies, or export \
     `https_proxy`, and rerun",
);
