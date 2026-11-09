//! The Linux desktops' and the environment's proxy settings, parsed
//! (`PROP-068#proxy-system`).
//!
//! Three pure readings, each over text a thin reader handed over:
//! [`parse_gnome_settings`] over `gsettings list-recursively
//! org.gnome.system.proxy`, [`parse_kioslaverc`] over the `[Proxy
//! Settings]` section of KDE's `kioslaverc`, and [`env_proxy`] over the
//! process environment's proxy variables. Nothing here reads a file, runs
//! a command or touches the environment, so every case is a fixture and
//! runs on every host.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

use super::{SystemProxyOrigin, bypass_list};

/// What a desktop's proxy settings say, in the one shape both desktops
/// reduce to: the mode, a PAC URL when the mode is `pac`, a fixed proxy
/// when the mode is `manual`, and the bypass list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DesktopProxy {
    /// The mode the settings are in.
    pub mode: DesktopMode,
    /// The PAC URL the `pac` mode names; empty means "discover it".
    pub pac_url: Option<String>,
    /// The `host:port` the `manual` mode names, `https` preferred over
    /// `http` (px tunnels HTTPS by `CONNECT`).
    pub upstream: Option<String>,
    /// The bypass list, in vibe's `no_proxy` spelling.
    pub bypass: Option<String>,
}

/// Which of the three corporate shapes a desktop's settings are in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DesktopMode {
    /// No proxy configured, or a mode vibe cannot hand to px (`socks`).
    #[default]
    None,
    /// A PAC URL, or auto-discovery when it names none.
    Pac,
    /// Auto-discovery (WPAD) outright.
    Discover,
    /// A fixed proxy server.
    Manual,
    /// KDE's `ProxyType = 4`: the desktop defers to the environment, so
    /// the environment step answers instead.
    Environment,
}

impl DesktopProxy {
    /// What the `checked` list says about these settings, so a refusal
    /// names what each desktop said rather than only that it said no.
    pub fn summary(&self) -> String {
        match self.mode {
            DesktopMode::None => "no proxy".to_owned(),
            DesktopMode::Pac => match self.pac_url.as_deref() {
                Some(url) => format!("auto, URL {url}"),
                None => "auto with no URL".to_owned(),
            },
            DesktopMode::Discover => "auto-discover (WPAD)".to_owned(),
            DesktopMode::Manual => match self.upstream.as_deref() {
                Some(upstream) => format!("manual, {upstream}"),
                None => "manual with no host".to_owned(),
            },
            DesktopMode::Environment => "defers to the environment variables".to_owned(),
        }
    }
}

/// GNOME's proxy settings out of `gsettings list-recursively
/// org.gnome.system.proxy` — one `<schema> <key> <value>` per line,
/// covering the parent schema and its `.http` / `.https` children.
///
/// `mode 'auto'` is a PAC URL (`autoconfig-url`), and an empty one means
/// auto-discovery; `mode 'manual'` is the `https` child's `host`/`port`,
/// else the `http` child's. `ignore-hosts` is the bypass list.
pub(crate) fn parse_gnome_settings(text: &str) -> DesktopProxy {
    let mut mode = None;
    let mut autoconfig = None;
    let mut hosts: [(Option<String>, Option<u16>); 2] = [(None, None), (None, None)];
    let mut ignore = String::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((schema, rest)) = line.split_once(' ') else {
            continue;
        };
        let Some((key, value)) = rest.trim().split_once(' ') else {
            continue;
        };
        let value = gvariant_scalar(value);
        match (schema, key) {
            ("org.gnome.system.proxy", "mode") => mode = Some(value.to_ascii_lowercase()),
            ("org.gnome.system.proxy", "autoconfig-url") => autoconfig = Some(value.to_owned()),
            ("org.gnome.system.proxy", "ignore-hosts") => ignore = gvariant_list(value),
            ("org.gnome.system.proxy.https", "host") => hosts[0].0 = Some(value.to_owned()),
            ("org.gnome.system.proxy.https", "port") => hosts[0].1 = value.parse().ok(),
            ("org.gnome.system.proxy.http", "host") => hosts[1].0 = Some(value.to_owned()),
            ("org.gnome.system.proxy.http", "port") => hosts[1].1 = value.parse().ok(),
            _ => {}
        }
    }
    let bypass = bypass_list(&ignore);
    match mode.as_deref() {
        Some("auto") => {
            let url = autoconfig.filter(|url| !url.trim().is_empty());
            DesktopProxy {
                mode: DesktopMode::Pac,
                pac_url: url.map(|url| url.trim().to_owned()),
                upstream: None,
                bypass,
            }
        }
        Some("manual") => DesktopProxy {
            mode: DesktopMode::Manual,
            pac_url: None,
            upstream: hosts
                .iter()
                .find_map(|(host, port)| endpoint(host.as_deref().unwrap_or_default(), *port)),
            bypass,
        },
        _ => DesktopProxy::default(),
    }
}

/// KDE's proxy settings out of the `[Proxy Settings]` section of
/// `kioslaverc`: `ProxyType` 1 is manual (`httpsProxy`, else
/// `httpProxy`), 2 is a PAC URL (`Proxy Config Script`), 3 is
/// auto-discovery, 4 defers to the environment, and anything else is no
/// proxy. `NoProxyFor` is the bypass list.
pub(crate) fn parse_kioslaverc(text: &str) -> DesktopProxy {
    let mut in_section = false;
    let mut kind = None;
    let mut script = None;
    let mut https = None;
    let mut http = None;
    let mut ignore = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line.eq_ignore_ascii_case("[Proxy Settings]");
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        // A localised or immutable key carries `[$i]` / `[de]` suffixes.
        let key = key.trim();
        let key = key.split_once('[').map(|(name, _)| name).unwrap_or(key);
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "proxytype" => kind = value.parse::<i64>().ok(),
            "proxy config script" => script = Some(value.to_owned()),
            "httpsproxy" => https = Some(value.to_owned()),
            "httpproxy" => http = Some(value.to_owned()),
            "noproxyfor" => ignore = value.to_owned(),
            _ => {}
        }
    }
    let bypass = bypass_list(&ignore);
    match kind {
        Some(1) => DesktopProxy {
            mode: DesktopMode::Manual,
            pac_url: None,
            upstream: [https, http]
                .into_iter()
                .flatten()
                .find_map(|raw| kde_endpoint(&raw)),
            bypass,
        },
        Some(2) => DesktopProxy {
            mode: DesktopMode::Pac,
            pac_url: script
                .map(|url| url.trim().to_owned())
                .filter(|url| !url.is_empty()),
            upstream: None,
            bypass,
        },
        Some(3) => DesktopProxy {
            mode: DesktopMode::Discover,
            bypass,
            ..DesktopProxy::default()
        },
        Some(4) => DesktopProxy {
            mode: DesktopMode::Environment,
            bypass,
            ..DesktopProxy::default()
        },
        _ => DesktopProxy::default(),
    }
}

/// The proxy variable of the environment vibe can hand px, out of the
/// variables a reader collected: `https_proxy` first (px tunnels HTTPS by
/// `CONNECT`), then `http_proxy`, then `all_proxy`, each in the lower-case
/// spelling before the upper-case one. The value is reduced to
/// `host:port`, so any credential in it is dropped rather than printed.
///
/// A loopback address is never taken: it is vibe's own px (or another
/// local proxy vibe cannot describe to px), and px pointed at itself is a
/// loop. `no_proxy` becomes the bypass list.
pub(crate) fn env_proxy(
    vars: &[(String, String)],
) -> (
    Option<(SystemProxyOrigin, String)>,
    Option<String>,
    Vec<String>,
) {
    let look = |name: &str| {
        vars.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.trim())
            .filter(|value| !value.is_empty())
    };
    let bypass = ["no_proxy", "NO_PROXY"]
        .into_iter()
        .find_map(look)
        .and_then(bypass_list);
    let mut checked = Vec::new();
    let mut found = None;
    for (origin, names) in [
        (
            SystemProxyOrigin::EnvironmentHttps,
            ["https_proxy", "HTTPS_PROXY"],
        ),
        (
            SystemProxyOrigin::EnvironmentHttp,
            ["http_proxy", "HTTP_PROXY"],
        ),
        (
            SystemProxyOrigin::EnvironmentAll,
            ["all_proxy", "ALL_PROXY"],
        ),
    ] {
        let Some(raw) = names.into_iter().find_map(look) else {
            checked.push(format!("environment {}: not set", names[0]));
            continue;
        };
        match super::pick_upstream(&without_credentials(raw)) {
            Some(upstream) if is_loopback(&upstream) => checked.push(format!(
                "environment {}: {upstream} is loopback — that is vibe's own px, not an \
                 upstream it can use",
                names[0]
            )),
            Some(upstream) => {
                checked.push(format!("environment {}: {upstream}", names[0]));
                if found.is_none() {
                    found = Some((origin, upstream));
                }
            }
            None => checked.push(format!(
                "environment {}: names no `host:port` vibe can hand px",
                names[0]
            )),
        }
    }
    (found, bypass, checked)
}

/// A proxy value without its `user:password@` part, if it carried one: px
/// authenticates to the upstream itself (that is why vibe starts it), and
/// a credential must never travel in px's argument list, a report or a
/// refusal. The scheme is kept, so [`super::pick_upstream`] still reads
/// the value.
fn without_credentials(raw: &str) -> String {
    let (scheme, rest) = match raw.split_once("://") {
        Some((scheme, rest)) => (format!("{scheme}://"), rest),
        None => (String::new(), raw),
    };
    // The authority ends at the first `/`, and the credential's `@` is the
    // last one inside it — a password may itself contain one.
    let (authority, path) = match rest.find('/') {
        Some(at) => rest.split_at(at),
        None => (rest, ""),
    };
    let authority = match authority.rsplit_once('@') {
        Some((_, after)) => after,
        None => authority,
    };
    format!("{scheme}{authority}{path}")
}

/// Whether an upstream names the loopback interface, whatever the port.
fn is_loopback(upstream: &str) -> bool {
    let host = match upstream.rsplit_once(':') {
        // `[::1]:3128`, and a bare `::1` whose last colon is not a port.
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => host,
        _ => upstream,
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host == "::1"
        || host
            .strip_prefix("127.")
            .is_some_and(|rest| rest.split('.').count() == 3)
}

/// `host:port` out of a host and an optional port, or `None` when the
/// host is empty or the port is absent or zero (GNOME's "unset").
fn endpoint(host: &str, port: Option<u16>) -> Option<String> {
    let host = host.trim();
    let port = port.filter(|port| *port != 0)?;
    (!host.is_empty()).then(|| format!("{host}:{port}"))
}

/// KDE writes a manual proxy as a URL whose port is either in the usual
/// place or separated by a space (`http://proxy.example.corp 8080`, the
/// spelling the KDE settings dialog has always written).
fn kde_endpoint(raw: &str) -> Option<String> {
    let raw = raw.trim();
    match raw.rsplit_once(' ') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) && !port.is_empty() => {
            super::pick_upstream(&format!("{}:{port}", without_credentials(host.trim())))
        }
        _ => super::pick_upstream(&without_credentials(raw)),
    }
}

/// A GVariant scalar as `gsettings` prints it: a quoted string loses its
/// quotes, everything else travels as written.
fn gvariant_scalar(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
        .or_else(|| {
            value
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
        })
        .unwrap_or(value)
}

/// A GVariant string array as `gsettings` prints it (`['localhost',
/// '::1']`) flattened to a comma-separated list; `@as []` and `[]` are
/// empty.
fn gvariant_list(value: &str) -> String {
    let value = value.trim().trim_start_matches("@as").trim();
    let inner = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(value);
    inner
        .split(',')
        .map(|entry| gvariant_scalar(entry).trim())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}
