//! The non-Windows resolution orders of [`crate::net::system_proxy`],
//! their parsers and the WPAD candidate list, driven through the
//! [`UnixProxySource`] seam so no case depends on the desktop the test
//! runs on (`PROP-068#proxy-system`).

use super::desktop::{DesktopMode, env_proxy, parse_gnome_settings, parse_kioslaverc};
use super::macos::parse_scutil_proxies;
use super::wpad::{WPAD_CANDIDATE_LIMIT, looks_like_pac, wpad_candidates};
use super::{SystemProxyError, SystemProxyOrigin, UnixProxySource, resolve_mac_with};
use crate::net::resolve_unix_with;

/// A host whose sources answer exactly what the test puts here; every
/// unstated source answers nothing, so a case names only what it is
/// about.
#[derive(Debug, Default)]
struct Host {
    gnome: Option<String>,
    gnome_error: Option<String>,
    kde: Option<String>,
    scutil: Option<String>,
    env: Vec<(String, String)>,
    domains: Vec<String>,
    wpad: Option<String>,
}

impl Host {
    fn env(mut self, name: &str, value: &str) -> Self {
        self.env.push((name.to_owned(), value.to_owned()));
        self
    }
}

impl UnixProxySource for Host {
    fn gnome_settings(&self) -> Result<Option<String>, String> {
        match &self.gnome_error {
            Some(reason) => Err(reason.clone()),
            None => Ok(self.gnome.clone()),
        }
    }
    fn kde_settings(&self) -> Result<Option<String>, String> {
        Ok(self.kde.clone())
    }
    fn scutil_proxies(&self) -> Result<Option<String>, String> {
        Ok(self.scutil.clone())
    }
    fn proxy_environment(&self) -> Vec<(String, String)> {
        self.env.clone()
    }
    fn dns_domains(&self) -> Result<Vec<String>, String> {
        Ok(self.domains.clone())
    }
    fn fetch_wpad(&self, candidates: &[String]) -> Result<Option<String>, String> {
        Ok(self
            .wpad
            .clone()
            .filter(|url| candidates.iter().any(|candidate| candidate == url)))
    }
}

/// The GNOME settings of a desktop in `mode 'auto'`, as `gsettings
/// list-recursively org.gnome.system.proxy` prints them.
const GNOME_AUTO: &str = "org.gnome.system.proxy autoconfig-url \
     'https://pac.example.corp/proxy.pac'\n\
     org.gnome.system.proxy ignore-hosts ['localhost', '127.0.0.0/8', '*.corp.example']\n\
     org.gnome.system.proxy mode 'auto'\n\
     org.gnome.system.proxy.http host ''\n\
     org.gnome.system.proxy.http port 0\n\
     org.gnome.system.proxy.https host ''\n\
     org.gnome.system.proxy.https port 0\n";

/// The same desktop in `mode 'manual'`, with both children set so the
/// `https` child's precedence is visible.
const GNOME_MANUAL: &str = "org.gnome.system.proxy autoconfig-url ''\n\
     org.gnome.system.proxy ignore-hosts ['localhost', '.corp.example']\n\
     org.gnome.system.proxy mode 'manual'\n\
     org.gnome.system.proxy.http host 'http.example.corp'\n\
     org.gnome.system.proxy.http port 3128\n\
     org.gnome.system.proxy.https host 'https.example.corp'\n\
     org.gnome.system.proxy.https port 8080\n";

/// `scutil --proxy` on a Mac whose network service names a PAC URL.
const SCUTIL_PAC: &str = "<dictionary> {\n\
     \x20 ExceptionsList : <array> {\n\
     \x20   0 : *.local\n\
     \x20   1 : *.corp.example\n\
     \x20 }\n\
     \x20 FTPPassive : 1\n\
     \x20 ProxyAutoConfigEnable : 1\n\
     \x20 ProxyAutoConfigURLString : https://pac.example.corp/proxy.pac\n\
     \x20 ProxyAutoDiscoveryEnable : 0\n\
     }\n";

#[test]
fn gnome_auto_is_the_pac_url_it_names() {
    let settings = parse_gnome_settings(GNOME_AUTO);
    assert_eq!(settings.mode, DesktopMode::Pac);
    assert_eq!(
        settings.pac_url.as_deref(),
        Some("https://pac.example.corp/proxy.pac")
    );
    assert_eq!(
        settings.bypass.as_deref(),
        Some("localhost,127.0.0.0/8,*.corp.example"),
        "the desktop's `ignore-hosts`, in vibe's `no_proxy` spelling"
    );
}

#[test]
fn gnome_manual_prefers_the_https_child() {
    let settings = parse_gnome_settings(GNOME_MANUAL);
    assert_eq!(settings.mode, DesktopMode::Manual);
    assert_eq!(
        settings.upstream.as_deref(),
        Some("https.example.corp:8080")
    );
    assert_eq!(settings.bypass.as_deref(), Some("localhost,.corp.example"));
}

#[test]
fn gnome_manual_falls_back_to_the_http_child_and_ignores_port_zero() {
    let text = "org.gnome.system.proxy mode 'manual'\n\
                org.gnome.system.proxy.https host 'https.example.corp'\n\
                org.gnome.system.proxy.https port 0\n\
                org.gnome.system.proxy.http host 'http.example.corp'\n\
                org.gnome.system.proxy.http port 3128\n";
    assert_eq!(
        parse_gnome_settings(text).upstream.as_deref(),
        Some("http.example.corp:3128")
    );
}

#[test]
fn gnome_mode_none_is_no_configuration() {
    let text = "org.gnome.system.proxy mode 'none'\n\
                org.gnome.system.proxy autoconfig-url 'https://stale.example.corp/p.pac'\n";
    let settings = parse_gnome_settings(text);
    assert_eq!(settings.mode, DesktopMode::None);
    assert_eq!(
        settings.pac_url, None,
        "a URL left behind by a switched-off mode is stale, as GNOME itself treats it"
    );
}

#[test]
fn gnome_auto_with_no_url_asks_for_discovery() {
    let text = "org.gnome.system.proxy mode 'auto'\n\
                org.gnome.system.proxy autoconfig-url ''\n";
    let settings = parse_gnome_settings(text);
    assert_eq!(settings.mode, DesktopMode::Pac);
    assert_eq!(settings.pac_url, None);
    assert_eq!(settings.summary(), "auto with no URL");
}

#[test]
fn kde_reads_the_proxy_settings_section_only() {
    let text = "[General]\nProxyType=2\n\n\
                [Proxy Settings]\nProxyType=1\n\
                httpsProxy=http://proxy.example.corp 8080\n\
                httpProxy=http://other.example.corp 3128\n\
                NoProxyFor=.corp.example,10.0.0.0/8\n";
    let settings = parse_kioslaverc(text);
    assert_eq!(settings.mode, DesktopMode::Manual);
    assert_eq!(
        settings.upstream.as_deref(),
        Some("proxy.example.corp:8080")
    );
    assert_eq!(settings.bypass.as_deref(), Some(".corp.example,10.0.0.0/8"));
}

#[test]
fn kde_proxy_types_two_three_and_four() {
    let script = parse_kioslaverc(
        "[Proxy Settings]\nProxyType=2\n\
         Proxy Config Script=https://pac.example.corp/proxy.pac\n",
    );
    assert_eq!(script.mode, DesktopMode::Pac);
    assert_eq!(
        script.pac_url.as_deref(),
        Some("https://pac.example.corp/proxy.pac")
    );
    let wpad = parse_kioslaverc("[Proxy Settings]\nProxyType=3\n");
    assert_eq!(wpad.mode, DesktopMode::Discover);
    let env = parse_kioslaverc("[Proxy Settings]\nProxyType=4\n");
    assert_eq!(env.mode, DesktopMode::Environment);
    assert_eq!(env.summary(), "defers to the environment variables");
}

#[test]
fn kde_accepts_a_colon_port_and_a_localised_key() {
    let settings = parse_kioslaverc(
        "[Proxy Settings]\nProxyType[$i]=1\nhttpsProxy=http://proxy.example.corp:8080\n",
    );
    assert_eq!(
        settings.upstream.as_deref(),
        Some("proxy.example.corp:8080")
    );
}

#[test]
fn the_mac_dictionary_is_a_pac_url_and_its_exceptions() {
    let settings = parse_scutil_proxies(SCUTIL_PAC);
    assert_eq!(settings.mode, DesktopMode::Pac);
    assert_eq!(
        settings.pac_url.as_deref(),
        Some("https://pac.example.corp/proxy.pac")
    );
    assert_eq!(
        settings.bypass.as_deref(),
        Some("*.local,*.corp.example"),
        "the `ExceptionsList` array, and no other array's entries"
    );
}

#[test]
fn the_mac_dictionary_prefers_https_then_http_then_nothing() {
    let both = parse_scutil_proxies(
        "<dictionary> {\n  HTTPEnable : 1\n  HTTPPort : 3128\n  HTTPProxy : http.example.corp\n\
         \x20 HTTPSEnable : 1\n  HTTPSPort : 8080\n  HTTPSProxy : https.example.corp\n}\n",
    );
    assert_eq!(both.upstream.as_deref(), Some("https.example.corp:8080"));
    let http = parse_scutil_proxies(
        "<dictionary> {\n  HTTPEnable : 1\n  HTTPPort : 3128\n  HTTPProxy : http.example.corp\n\
         \x20 HTTPSEnable : 0\n  HTTPSProxy : stale.example.corp\n}\n",
    );
    assert_eq!(http.upstream.as_deref(), Some("http.example.corp:3128"));
    let off = parse_scutil_proxies(
        "<dictionary> {\n  HTTPEnable : 0\n  HTTPProxy : stale.example.corp\n\
         \x20 ProxyAutoConfigEnable : 0\n  ProxyAutoConfigURLString : https://stale/p.pac\n}\n",
    );
    assert_eq!(off.mode, DesktopMode::None, "a switched-off service is off");
}

#[test]
fn the_mac_dictionary_asks_for_discovery() {
    let settings = parse_scutil_proxies(
        "<dictionary> {\n  ProxyAutoDiscoveryEnable : 1\n  ProxyAutoConfigEnable : 0\n}\n",
    );
    assert_eq!(settings.mode, DesktopMode::Discover);
}

#[test]
fn the_environment_prefers_https_then_http_then_all() {
    let vars = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    };
    let (found, bypass, checked) = env_proxy(&vars(&[
        ("http_proxy", "http://http.example.corp:3128"),
        ("https_proxy", "http://user:secret@https.example.corp:8080/"),
        ("no_proxy", "localhost,.corp.example"),
    ]));
    assert_eq!(
        found,
        Some((
            SystemProxyOrigin::EnvironmentHttps,
            "https.example.corp:8080".to_owned()
        )),
        "the credential is dropped with the scheme and never printed"
    );
    assert_eq!(bypass.as_deref(), Some("localhost,.corp.example"));
    assert!(
        checked.iter().all(|line| !line.contains("secret")),
        "{checked:?}"
    );

    let (http, _, _) = env_proxy(&vars(&[("HTTP_PROXY", "http.example.corp:3128")]));
    assert_eq!(http.unwrap().0, SystemProxyOrigin::EnvironmentHttp);
    let (all, _, _) = env_proxy(&vars(&[("all_proxy", "all.example.corp:3128")]));
    assert_eq!(all.unwrap().0, SystemProxyOrigin::EnvironmentAll);
}

#[test]
fn a_loopback_environment_proxy_is_vibes_own_px_and_is_skipped() {
    let vars = vec![
        ("https_proxy".to_owned(), "http://127.0.0.1:3128".to_owned()),
        (
            "http_proxy".to_owned(),
            "http://proxy.example.corp:8080".to_owned(),
        ),
    ];
    let (found, _, checked) = env_proxy(&vars);
    assert_eq!(
        found,
        Some((
            SystemProxyOrigin::EnvironmentHttp,
            "proxy.example.corp:8080".to_owned()
        ))
    );
    assert!(
        checked.iter().any(|line| line.contains("is loopback")),
        "{checked:?}"
    );
    for loopback in ["localhost:3128", "http://[::1]:3128", "http://127.9.9.9:1"] {
        let (found, _, _) = env_proxy(&[("https_proxy".to_owned(), loopback.to_owned())]);
        assert_eq!(found, None, "{loopback}");
    }
}

#[test]
fn the_unix_order_reads_gnome_then_kde_then_the_environment() {
    // GNOME wins over everything below it.
    let resolved = resolve_unix_with(&Host {
        gnome: Some(GNOME_AUTO.to_owned()),
        kde: Some("[Proxy Settings]\nProxyType=1\nhttpsProxy=kde.example.corp:1\n".to_owned()),
        ..Host::default()
    })
    .unwrap()
    .clone();
    assert_eq!(resolved.origin, SystemProxyOrigin::GnomeSettings);
    assert_eq!(
        resolved.pac_url.as_deref(),
        Some("https://pac.example.corp/proxy.pac")
    );

    // GNOME says nothing: KDE answers.
    let resolved = resolve_unix_with(&Host {
        gnome: Some("org.gnome.system.proxy mode 'none'\n".to_owned()),
        kde: Some(
            "[Proxy Settings]\nProxyType=2\nProxy Config Script=http://kde.example.corp/p.pac\n"
                .to_owned(),
        ),
        ..Host::default()
    })
    .unwrap();
    assert_eq!(resolved.origin, SystemProxyOrigin::KdeSettings);

    // Neither desktop: the environment is the last source.
    let resolved =
        resolve_unix_with(&Host::default().env("https_proxy", "http://proxy.example.corp:8080"))
            .unwrap();
    assert_eq!(resolved.origin, SystemProxyOrigin::EnvironmentHttps);
    assert_eq!(
        resolved.describe(),
        "system → proxy proxy.example.corp:8080 (from environment https_proxy)"
    );
}

#[test]
fn a_desktop_asking_for_discovery_is_answered_by_wpad() {
    let resolved = resolve_unix_with(&Host {
        gnome: Some("org.gnome.system.proxy mode 'auto'\n".to_owned()),
        domains: vec!["eng.corp.example.com".to_owned()],
        wpad: Some("http://wpad.eng.corp.example.com/wpad.dat".to_owned()),
        ..Host::default()
    })
    .unwrap();
    assert_eq!(resolved.origin, SystemProxyOrigin::WpadDiscovery);
    assert_eq!(
        resolved.describe(),
        "system → http://wpad.eng.corp.example.com/wpad.dat (from WPAD discovery)"
    );
}

#[test]
fn a_discovery_that_finds_nothing_lets_the_order_continue() {
    let resolved = resolve_unix_with(
        &Host {
            kde: Some("[Proxy Settings]\nProxyType=3\n".to_owned()),
            domains: vec!["corp.example".to_owned()],
            ..Host::default()
        }
        .env("https_proxy", "http://proxy.example.corp:8080"),
    )
    .unwrap();
    assert_eq!(resolved.origin, SystemProxyOrigin::EnvironmentHttps);
}

#[test]
fn the_unix_refusal_names_every_source_and_both_ways_out() {
    let error = resolve_unix_with(&Host::default()).unwrap_err();
    assert!(matches!(error, SystemProxyError::NotConfigured { .. },),);
    let text = error.to_string();
    for part in [
        "GNOME settings: not configured on this host",
        "KDE settings: not configured on this host",
        "environment https_proxy: not set",
        "environment http_proxy: not set",
        "environment all_proxy: not set",
        "`[proxy] pac = \"<your PAC URL>\"`",
        "`[proxy] upstream = \"host:port\"`",
        "GNOME, KDE",
        "spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system",
    ] {
        assert!(text.contains(part), "{part} missing from: {text}");
    }
}

#[test]
fn a_reader_that_fails_is_reported_not_swallowed() {
    let error = resolve_unix_with(&Host {
        gnome_error: Some("`gsettings` exited 1 (No such schema)".to_owned()),
        ..Host::default()
    })
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("GNOME settings: `gsettings` exited 1 (No such schema)"),
        "{error}"
    );
}

#[test]
fn the_mac_order_reads_the_network_settings_then_the_environment() {
    let resolved = resolve_mac_with(&Host {
        scutil: Some(SCUTIL_PAC.to_owned()),
        ..Host::default()
    })
    .unwrap();
    assert_eq!(resolved.origin, SystemProxyOrigin::MacNetworkSettings);
    assert_eq!(
        resolved.describe(),
        "system → https://pac.example.corp/proxy.pac · bypass *.local,*.corp.example \
         (from macOS network settings)"
    );

    let resolved = resolve_mac_with(
        &Host {
            scutil: Some("<dictionary> {\n  HTTPEnable : 0\n}\n".to_owned()),
            ..Host::default()
        }
        .env("https_proxy", "http://proxy.example.corp:8080"),
    )
    .unwrap();
    assert_eq!(resolved.origin, SystemProxyOrigin::EnvironmentHttps);

    let error = resolve_mac_with(&Host::default()).unwrap_err().to_string();
    assert!(
        error.contains("the macOS proxy configuration of the current network service"),
        "{error}"
    );
    assert!(
        error.contains("System Settings") && error.contains("fix:"),
        "{error}"
    );
}

#[test]
fn the_mac_discovery_walks_wpad_too() {
    let resolved = resolve_mac_with(&Host {
        scutil: Some("<dictionary> {\n  ProxyAutoDiscoveryEnable : 1\n}\n".to_owned()),
        domains: vec!["corp.example".to_owned()],
        wpad: Some("http://wpad.corp.example/wpad.dat".to_owned()),
        ..Host::default()
    })
    .unwrap();
    assert_eq!(resolved.origin, SystemProxyOrigin::WpadDiscovery);
}

#[test]
fn wpad_candidates_walk_up_the_labels_and_stop_above_a_public_suffix() {
    assert_eq!(
        wpad_candidates(&["eng.corp.example.com".to_owned()]),
        vec![
            "http://wpad.eng.corp.example.com/wpad.dat".to_owned(),
            "http://wpad.corp.example.com/wpad.dat".to_owned(),
            "http://wpad.example.com/wpad.dat".to_owned(),
        ],
        "never `http://wpad.com/wpad.dat` — that host is a stranger's"
    );
    assert_eq!(
        wpad_candidates(&["corp.example".to_owned(), "CORP.example.".to_owned()]),
        vec!["http://wpad.corp.example/wpad.dat".to_owned()],
        "a repeated domain is asked once, case and trailing dot included"
    );
    assert!(
        wpad_candidates(&["localhost".to_owned(), String::new()]).is_empty(),
        "a single label is not a search domain"
    );
    let deep = vec!["a.b.c.d.e.f.g.h.example.com".to_owned()];
    assert_eq!(wpad_candidates(&deep).len(), WPAD_CANDIDATE_LIMIT);
    assert!(
        wpad_candidates(&["bad domain/with:junk".to_owned()]).is_empty(),
        "a value that is not a domain is not turned into a URL"
    );
}

#[test]
fn a_pac_body_is_the_one_function_every_pac_declares() {
    assert!(looks_like_pac(
        "function FindProxyForURL(url, host) { return \"DIRECT\"; }"
    ));
    assert!(!looks_like_pac("<html>captive portal</html>"));
}

/// A credential in a proxy value is dropped, and nothing else about the
/// value is: px authenticates to the upstream itself, and a password must
/// never reach px's argument list, a report or a refusal.
#[test]
fn a_credential_is_dropped_and_the_rest_of_the_value_is_not() {
    for (raw, want) in [
        (
            "http://user:p@ss@proxy.example.corp:8080",
            "proxy.example.corp:8080",
        ),
        (
            "http://user@proxy.example.corp:8080/",
            "proxy.example.corp:8080",
        ),
        ("proxy.example.corp:8080", "proxy.example.corp:8080"),
        // A `@` outside the authority is not a credential marker.
        (
            "http://proxy.example.corp:8080/a@b",
            "proxy.example.corp:8080/a@b",
        ),
    ] {
        let (found, _, _) = env_proxy(&[("https_proxy".to_owned(), raw.to_owned())]);
        assert_eq!(found.unwrap().1, want, "{raw}");
    }
}
