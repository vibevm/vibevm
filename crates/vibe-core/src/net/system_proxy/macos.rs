//! The macOS proxy configuration of the current network service, parsed
//! (`PROP-068#proxy-system`).
//!
//! `SCDynamicStoreCopyProxies` is the one place macOS keeps the answer —
//! the dictionary `scutil --proxy` prints, which is how the reader behind
//! the seam obtains it. This module is the pure reading of that
//! dictionary: no framework, no process, no file, so every case is a
//! fixture and runs on every host.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

use super::{
    bypass_list,
    desktop::{DesktopMode, DesktopProxy},
};

/// The macOS proxy dictionary, reduced to the one shape every host's
/// settings reduce to.
///
/// `ProxyAutoConfigEnable` with a `ProxyAutoConfigURLString` is a PAC
/// URL; `ProxyAutoDiscoveryEnable` is WPAD; `HTTPSEnable` with
/// `HTTPSProxy`/`HTTPSPort` (else the `HTTP*` triple) is a fixed proxy;
/// `ExceptionsList` is the bypass list. An `…Enable` that is off is
/// ignored exactly as macOS ignores it, so a stale URL left behind by a
/// switched-off profile never becomes the route.
pub(crate) fn parse_scutil_proxies(text: &str) -> DesktopProxy {
    let mut values: Vec<(String, String)> = Vec::new();
    let mut exceptions: Vec<String> = Vec::new();
    let mut in_array = false;
    for line in text.lines() {
        let line = line.trim();
        if in_array {
            if line.starts_with('}') {
                in_array = false;
            } else if let Some((_, value)) = line.split_once(':') {
                let value = value.trim();
                if !value.is_empty() {
                    exceptions.push(value.to_owned());
                }
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() {
            continue;
        }
        if value.starts_with("<array>") {
            // The one array the resolution reads; any other is skipped
            // whole, so its entries never look like top-level keys.
            in_array = true;
            if !key.eq_ignore_ascii_case("ExceptionsList") {
                continue;
            }
            exceptions.clear();
            continue;
        }
        values.push((key.to_owned(), value.to_owned()));
    }
    let get = |name: &str| {
        values
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .filter(|value| !value.is_empty())
    };
    let on = |name: &str| get(name).is_some_and(|value| value == "1" || value == "true");
    let bypass = bypass_list(&exceptions.join(","));

    if on("ProxyAutoConfigEnable") {
        return DesktopProxy {
            mode: DesktopMode::Pac,
            pac_url: get("ProxyAutoConfigURLString").map(str::to_owned),
            upstream: None,
            bypass,
        };
    }
    if on("ProxyAutoDiscoveryEnable") {
        return DesktopProxy {
            mode: DesktopMode::Discover,
            bypass,
            ..DesktopProxy::default()
        };
    }
    for (enable, host, port) in [
        ("HTTPSEnable", "HTTPSProxy", "HTTPSPort"),
        ("HTTPEnable", "HTTPProxy", "HTTPPort"),
    ] {
        if !on(enable) {
            continue;
        }
        let Some(host) = get(host) else { continue };
        let upstream = match get(port).and_then(|port| port.parse::<u16>().ok()) {
            Some(port) if port != 0 => format!("{host}:{port}"),
            // A service with no port is the proxy's default; px wants an
            // explicit one, and 80 is what macOS itself shows.
            _ => format!("{host}:80"),
        };
        return DesktopProxy {
            mode: DesktopMode::Manual,
            pac_url: None,
            upstream: super::pick_upstream(&upstream),
            bypass,
        };
    }
    DesktopProxy {
        bypass,
        ..DesktopProxy::default()
    }
}
