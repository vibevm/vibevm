//! `pac = "system"` off Windows: the resolution order on Linux and the
//! other Unixes, and on macOS (`PROP-068#proxy-system`).
//!
//! Corporate profiles come in three variants per operating system —
//! manual (an explicit `upstream`), pac (an explicit PAC URL) and auto
//! (`pac = "system"`: whatever the operating system is configured with).
//! Auto has to mean something on every host vibe runs on, so the Windows
//! resolution of [`super::resolve_with`] has two siblings here, built the
//! same way: an ordered set of sources, the first that yields a
//! configuration wins, each reported by name, and every step a pure
//! function over the [`UnixProxySource`] seam so the cases run on every
//! host and no test depends on the desktop it runs on.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

use std::sync::{Arc, RwLock};

use super::{
    SystemProxy, SystemProxyError, SystemProxyOrigin,
    desktop::{DesktopMode, DesktopProxy, env_proxy, parse_gnome_settings, parse_kioslaverc},
    macos::parse_scutil_proxies,
    report::{MAC_SCOPE, UNIX_SCOPE},
    wpad::wpad_candidates,
};

/// The settings a non-Windows host keeps its proxy configuration in. The
/// seam that keeps the order logic pure: production installs the readers
/// of `vibe-net` (a `gsettings` run, two file reads, the environment, a
/// bounded WPAD fetch), a test installs a stub, and neither depends on
/// the desktop the process happens to run under.
///
/// Every method answers the absent setting — `Ok(None)`, or an empty list
/// — and `Err(reason)` for a setting that could not be read; a reader
/// failure is reported among the sources that were checked, never
/// swallowed. Every method has a default that answers nothing, so a stub
/// states only the sources its case is about.
///
/// ```
/// use vibe_core::net::{SystemProxyOrigin, UnixProxySource, resolve_unix_with};
///
/// /// A GNOME desktop set to "automatic proxy configuration".
/// #[derive(Debug)]
/// struct Gnome;
/// impl UnixProxySource for Gnome {
///     fn gnome_settings(&self) -> Result<Option<String>, String> {
///         Ok(Some(
///             "org.gnome.system.proxy mode 'auto'\n\
///              org.gnome.system.proxy autoconfig-url 'https://pac.example.corp/proxy.pac'\n"
///                 .to_owned(),
///         ))
///     }
/// }
///
/// let resolved = resolve_unix_with(&Gnome).unwrap();
/// assert_eq!(resolved.pac_url.as_deref(), Some("https://pac.example.corp/proxy.pac"));
/// assert_eq!(resolved.origin, SystemProxyOrigin::GnomeSettings);
/// assert_eq!(
///     resolved.describe(),
///     "system → https://pac.example.corp/proxy.pac (from GNOME settings)"
/// );
/// ```
pub trait UnixProxySource: std::fmt::Debug + Send + Sync {
    /// `gsettings list-recursively org.gnome.system.proxy` — the GNOME /
    /// GTK desktop's proxy settings, parent schema and children.
    /// `Ok(None)` when there is no `gsettings` on this host.
    fn gnome_settings(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
    /// The user's `kioslaverc` (KDE's proxy settings), as read from
    /// `$XDG_CONFIG_HOME`. `Ok(None)` when the file is not there.
    fn kde_settings(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
    /// `scutil --proxy` — the macOS `SCDynamicStoreCopyProxies`
    /// dictionary of the current network service. `Ok(None)` off macOS.
    fn scutil_proxies(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
    /// The proxy variables of the process environment, each under the
    /// name it is spelled with (`https_proxy`, `HTTPS_PROXY`, …).
    fn proxy_environment(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    /// The host's DNS search domains and its own FQDN, for the WPAD
    /// candidate list.
    fn dns_domains(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
    /// The first of these `http://wpad.<domain>/wpad.dat` candidates that
    /// answers `200` with a PAC body, fetched directly and with a bounded
    /// timeout each.
    fn fetch_wpad(&self, _candidates: &[String]) -> Result<Option<String>, String> {
        Ok(None)
    }
}

static UNIX_OVERRIDE: RwLock<Option<Arc<dyn UnixProxySource>>> = RwLock::new(None);
static MAC_OVERRIDE: RwLock<Option<Arc<dyn UnixProxySource>>> = RwLock::new(None);

fn put(slot: &RwLock<Option<Arc<dyn UnixProxySource>>>, source: Option<Arc<dyn UnixProxySource>>) {
    match slot.write() {
        Ok(mut held) => *held = source,
        Err(poisoned) => *poisoned.into_inner() = source,
    }
}

fn get(slot: &RwLock<Option<Arc<dyn UnixProxySource>>>) -> Option<Arc<dyn UnixProxySource>> {
    match slot.read() {
        Ok(held) => held.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// Install a [`UnixProxySource`] for `vibe_net::system_proxy::resolve` to
/// read instead of this host's desktop, with the Unix order
/// ([`resolve_unix_with`]). The test seam; `None` restores the host.
pub fn install_unix_source(source: Option<Arc<dyn UnixProxySource>>) {
    put(&UNIX_OVERRIDE, source);
}

/// The source [`install_unix_source`] put in place, if any.
pub fn installed_unix_source() -> Option<Arc<dyn UnixProxySource>> {
    get(&UNIX_OVERRIDE)
}

/// Install a [`UnixProxySource`] to be read with the macOS order
/// ([`resolve_mac_with`]), so that order is testable on any host. The
/// test seam; `None` restores the host.
pub fn install_mac_source(source: Option<Arc<dyn UnixProxySource>>) {
    put(&MAC_OVERRIDE, source);
}

/// The source [`install_mac_source`] put in place, if any.
pub fn installed_mac_source() -> Option<Arc<dyn UnixProxySource>> {
    get(&MAC_OVERRIDE)
}

/// One source's verdict: a resolved route, or nothing with a line for the
/// `checked` list.
type Step = Option<SystemProxy>;

/// The Linux (and other Unix) resolution order, first hit wins
/// (`PROP-068#proxy-system`):
///
/// 1. the GNOME / GTK desktop — `org.gnome.system.proxy`: `mode 'auto'`
///    is `autoconfig-url` (empty: WPAD), `mode 'manual'` is the `https`
///    child's `host`/`port`, else the `http` child's, with `ignore-hosts`
///    as the bypass list;
/// 2. the KDE desktop — `[Proxy Settings]` of `kioslaverc`: `ProxyType`
///    2 is `Proxy Config Script`, 3 is WPAD, 1 is `httpsProxy` (else
///    `httpProxy`), 4 defers to the environment, with `NoProxyFor` as
///    the bypass list;
/// 3. the environment — `https_proxy` / `HTTPS_PROXY`, then
///    `http_proxy` / `HTTP_PROXY`, then `all_proxy` / `ALL_PROXY`,
///    reduced to `host:port`, with `no_proxy` as the bypass list. A
///    loopback value is skipped: that is vibe's own px;
/// 4. nothing — [`SystemProxyError::NotConfigured`], naming every source
///    that was looked at, what each said, and the two ways out.
///
/// A source that asks to auto-discover is answered by WPAD *there*,
/// before the next source is read; a discovery that finds nothing is
/// reported and the order continues.
///
/// ```
/// use vibe_core::net::{SystemProxyOrigin, UnixProxySource, resolve_unix_with};
///
/// /// A KDE desktop with a fixed proxy, and `https_proxy` exported too.
/// #[derive(Debug)]
/// struct Desktop;
/// impl UnixProxySource for Desktop {
///     fn kde_settings(&self) -> Result<Option<String>, String> {
///         Ok(Some(
///             "[Proxy Settings]\nProxyType=1\nhttpsProxy=http://proxy.example.corp 8080\n\
///              NoProxyFor=.corp.example\n"
///                 .to_owned(),
///         ))
///     }
///     fn proxy_environment(&self) -> Vec<(String, String)> {
///         vec![("https_proxy".to_owned(), "http://other.example.corp:3128".to_owned())]
///     }
/// }
///
/// // The desktop is read before the environment, so it wins.
/// let resolved = resolve_unix_with(&Desktop).unwrap();
/// assert_eq!(resolved.upstream.as_deref(), Some("proxy.example.corp:8080"));
/// assert_eq!(resolved.bypass.as_deref(), Some(".corp.example"));
/// assert_eq!(resolved.origin, SystemProxyOrigin::KdeSettings);
///
/// // Nothing anywhere names every source that was looked at.
/// #[derive(Debug)]
/// struct Bare;
/// impl UnixProxySource for Bare {}
/// let error = resolve_unix_with(&Bare).unwrap_err().to_string();
/// assert!(error.contains("GNOME") && error.contains("KDE"), "{error}");
/// assert!(error.contains("environment https_proxy") && error.contains("fix:"), "{error}");
/// ```
pub fn resolve_unix_with(source: &dyn UnixProxySource) -> Result<SystemProxy, SystemProxyError> {
    let mut checked: Vec<String> = Vec::new();
    // Each desktop is read only if the one before it answered nothing:
    // the arguments of a call that is never made are never evaluated, so
    // a GNOME answer means `kioslaverc` is not even opened.
    if let Some(resolved) = desktop(
        source,
        ("GNOME settings", SystemProxyOrigin::GnomeSettings),
        source.gnome_settings(),
        parse_gnome_settings,
        &mut checked,
    ) {
        return Ok(resolved);
    }
    if let Some(resolved) = desktop(
        source,
        ("KDE settings", SystemProxyOrigin::KdeSettings),
        source.kde_settings(),
        parse_kioslaverc,
        &mut checked,
    ) {
        return Ok(resolved);
    }
    match environment(source, &mut checked) {
        Some(resolved) => Ok(resolved),
        None => Err(not_configured(UNIX_SCOPE, checked)),
    }
}

/// The macOS resolution order, first hit wins (`PROP-068#proxy-system`):
///
/// 1. the system proxy configuration of the current network service, as
///    `SCDynamicStoreCopyProxies` reports it — the data `scutil --proxy`
///    prints: `ProxyAutoConfigEnable` + `ProxyAutoConfigURLString` is a
///    PAC URL, `ProxyAutoDiscoveryEnable` is WPAD, `HTTPSEnable` +
///    `HTTPSProxy` + `HTTPSPort` (else the `HTTP*` triple) is a fixed
///    proxy, and `ExceptionsList` is the bypass list;
/// 2. the environment, exactly as the Unix order reads it;
/// 3. nothing — [`SystemProxyError::NotConfigured`].
///
/// ```
/// use vibe_core::net::{SystemProxyOrigin, UnixProxySource, resolve_mac_with};
///
/// #[derive(Debug)]
/// struct Mac;
/// impl UnixProxySource for Mac {
///     fn scutil_proxies(&self) -> Result<Option<String>, String> {
///         Ok(Some(
///             "<dictionary> {\n  ExceptionsList : <array> {\n    0 : *.corp.example\n  }\n\
///                HTTPSEnable : 1\n  HTTPSPort : 8080\n  HTTPSProxy : proxy.example.corp\n}\n"
///                 .to_owned(),
///         ))
///     }
/// }
///
/// let resolved = resolve_mac_with(&Mac).unwrap();
/// assert_eq!(resolved.upstream.as_deref(), Some("proxy.example.corp:8080"));
/// assert_eq!(resolved.bypass.as_deref(), Some("*.corp.example"));
/// assert_eq!(resolved.origin, SystemProxyOrigin::MacNetworkSettings);
/// ```
pub fn resolve_mac_with(source: &dyn UnixProxySource) -> Result<SystemProxy, SystemProxyError> {
    let mut checked: Vec<String> = Vec::new();
    match source.scutil_proxies() {
        Ok(Some(text)) => {
            let settings = parse_scutil_proxies(&text);
            if let Some(resolved) = step(
                source,
                "macOS network settings",
                SystemProxyOrigin::MacNetworkSettings,
                settings,
                &mut checked,
            ) {
                return Ok(resolved);
            }
        }
        Ok(None) => checked.push("macOS network settings: not readable on this host".to_owned()),
        Err(reason) => checked.push(format!("macOS network settings: {reason}")),
    }
    match environment(source, &mut checked) {
        Some(resolved) => Ok(resolved),
        None => Err(not_configured(MAC_SCOPE, checked)),
    }
}

/// One desktop's reading: what it said goes into `checked`, a reader
/// failure included, and a configuration it named becomes the route.
fn desktop(
    source: &dyn UnixProxySource,
    (name, origin): (&str, SystemProxyOrigin),
    read: Result<Option<String>, String>,
    parse: fn(&str) -> DesktopProxy,
    checked: &mut Vec<String>,
) -> Step {
    match read {
        Ok(Some(text)) => step(source, name, origin, parse(&text), checked),
        Ok(None) => {
            checked.push(format!("{name}: not configured on this host"));
            None
        }
        Err(reason) => {
            checked.push(format!("{name}: {reason}"));
            None
        }
    }
}

/// One source's settings turned into a route, with WPAD run on the spot
/// when they ask to auto-discover. Appends what the source said to
/// `checked` either way.
fn step(
    source: &dyn UnixProxySource,
    name: &str,
    origin: SystemProxyOrigin,
    settings: DesktopProxy,
    checked: &mut Vec<String>,
) -> Step {
    checked.push(format!("{name}: {}", settings.summary()));
    match settings.mode {
        DesktopMode::Pac => match settings.pac_url {
            Some(url) => Some(SystemProxy {
                pac_url: Some(url),
                upstream: None,
                bypass: settings.bypass,
                origin,
            }),
            // "Automatic" with no URL is auto-discovery, as every
            // desktop and Windows itself treat it.
            None => discover(source, settings.bypass, checked),
        },
        DesktopMode::Discover => discover(source, settings.bypass, checked),
        DesktopMode::Manual => settings.upstream.map(|upstream| SystemProxy {
            pac_url: None,
            upstream: Some(upstream),
            bypass: settings.bypass,
            origin,
        }),
        DesktopMode::None | DesktopMode::Environment => None,
    }
}

/// The WPAD half: the candidate list for this host's DNS domains, and the
/// first candidate that answered with a PAC body.
fn discover(
    source: &dyn UnixProxySource,
    bypass: Option<String>,
    checked: &mut Vec<String>,
) -> Step {
    let domains = match source.dns_domains() {
        Ok(domains) => domains,
        Err(reason) => {
            checked.push(format!("WPAD discovery: no DNS domain to walk ({reason})"));
            return None;
        }
    };
    let candidates = wpad_candidates(&domains);
    if candidates.is_empty() {
        checked.push(
            "WPAD discovery: this host has no DNS search domain above a public suffix to walk"
                .to_owned(),
        );
        return None;
    }
    match source.fetch_wpad(&candidates) {
        Ok(Some(url)) => Some(SystemProxy {
            pac_url: Some(url),
            upstream: None,
            bypass,
            origin: SystemProxyOrigin::WpadDiscovery,
        }),
        Ok(None) => {
            checked.push(format!(
                "WPAD discovery: none of {} answered with a PAC file ({})",
                candidates.len(),
                candidates.join(", ")
            ));
            None
        }
        Err(reason) => {
            checked.push(format!("WPAD discovery: {reason}"));
            None
        }
    }
}

/// The environment step, shared by both orders.
fn environment(source: &dyn UnixProxySource, checked: &mut Vec<String>) -> Step {
    let (found, bypass, mut lines) = env_proxy(&source.proxy_environment());
    checked.append(&mut lines);
    found.map(|(origin, upstream)| SystemProxy {
        pac_url: None,
        upstream: Some(upstream),
        bypass,
        origin,
    })
}

/// The refusal, with the subject and the configure hint of this kind of
/// host.
fn not_configured(scope: (&'static str, &'static str), checked: Vec<String>) -> SystemProxyError {
    SystemProxyError::NotConfigured {
        looked_in: scope.0,
        configure_hint: scope.1,
        checked,
    }
}
