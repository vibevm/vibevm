//! The resolution order, the blob parser and the string reductions of
//! [`crate::net::system_proxy`], driven through the
//! [`WindowsProxySource`] seam so no case depends on the machine the test
//! runs on.

use std::sync::Arc;

use crate::net::system_proxy::*;

/// Every setting answers what the test puts here; the default answers
/// nothing, so a test states only the settings it is about.
#[derive(Debug)]
struct Stub {
    policy_user: Result<Option<String>, String>,
    policy_machine: Result<Option<String>, String>,
    ie: Result<Option<IeProxyConfig>, String>,
    auto_config_url: Result<Option<String>, String>,
    blob: Result<Option<Vec<u8>>, String>,
    wpad: Result<Option<String>, String>,
}

/// Nothing configured anywhere: `Result` has no `Default`, and the
/// absent setting is `Ok(None)`, not an error.
impl Default for Stub {
    fn default() -> Self {
        Self {
            policy_user: Ok(None),
            policy_machine: Ok(None),
            ie: Ok(None),
            auto_config_url: Ok(None),
            blob: Ok(None),
            wpad: Ok(None),
        }
    }
}

impl WindowsProxySource for Stub {
    fn policy_auto_config_url(&self, machine: bool) -> Result<Option<String>, String> {
        if machine {
            self.policy_machine.clone()
        } else {
            self.policy_user.clone()
        }
    }
    fn ie_proxy_config(&self) -> Result<Option<IeProxyConfig>, String> {
        self.ie.clone()
    }
    fn auto_config_url_value(&self) -> Result<Option<String>, String> {
        self.auto_config_url.clone()
    }
    fn default_connection_settings(&self) -> Result<Option<Vec<u8>>, String> {
        self.blob.clone()
    }
    fn detect_auto_proxy_url(&self) -> Result<Option<String>, String> {
        self.wpad.clone()
    }
}

/// A blob in the layout Windows writes, with the owner's reported
/// shape: flags, a proxy server, a bypass list and a PAC URL.
fn blob(flags: u32, proxy: &str, bypass: &str, url: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0x46u32.to_le_bytes());
    out.extend_from_slice(&11u32.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    for field in [proxy, bypass, url] {
        out.extend_from_slice(&u32::try_from(field.len()).unwrap().to_le_bytes());
        out.extend_from_slice(field.as_bytes());
    }
    // The per-connection GUID and reserved words Windows appends.
    out.extend_from_slice(&[0; 32]);
    out
}

#[test]
fn policy_wins_over_everything_the_user_can_set() {
    let resolved = resolve_with(&Stub {
        policy_user: Ok(Some("https://pac.example.corp/policy-user.pac".into())),
        policy_machine: Ok(Some("https://pac.example.corp/policy-machine.pac".into())),
        ie: Ok(Some(IeProxyConfig {
            auto_config_url: Some("https://pac.example.corp/user.pac".into()),
            ..Default::default()
        })),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        resolved.pac_url.as_deref(),
        Some("https://pac.example.corp/policy-user.pac")
    );
    assert_eq!(resolved.origin, SystemProxyOrigin::PolicyUser);
}

#[test]
fn the_machine_policy_answers_when_the_user_policy_is_empty() {
    let resolved = resolve_with(&Stub {
        policy_user: Ok(Some("   ".into())),
        policy_machine: Ok(Some("https://pac.example.corp/policy-machine.pac".into())),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(resolved.origin, SystemProxyOrigin::PolicyMachine);
}

/// The owner's shape: the WinHTTP API reads the `Connections` blob, so
/// the URL comes back although the `AutoConfigURL` value is absent.
#[test]
fn the_api_answers_the_connections_blob_url() {
    let resolved = resolve_with(&Stub {
        ie: Ok(Some(IeProxyConfig {
            auto_detect: true,
            auto_config_url: Some("https://pac.example.corp/proxy.pac".into()),
            proxy_bypass: Some("<local>;*.corp.example".into()),
            ..Default::default()
        })),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        resolved.pac_url.as_deref(),
        Some("https://pac.example.corp/proxy.pac")
    );
    assert_eq!(resolved.bypass.as_deref(), Some("*.corp.example"));
    assert_eq!(resolved.origin, SystemProxyOrigin::InternetOptions);
}

/// The same shape with no WinHTTP API at all: the blob is parsed and
/// its URL is the answer, flags 0x0D exactly as the owner reported.
#[test]
fn the_blob_answers_when_the_api_is_unavailable() {
    let resolved = resolve_with(&Stub {
        ie: Err("WinHttpGetIEProxyConfigForCurrentUser failed: 87".into()),
        blob: Ok(Some(blob(
            0x0d,
            "",
            "<local>;*.corp.example",
            "https://pac.example.corp/proxy.pac",
        ))),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        resolved.pac_url.as_deref(),
        Some("https://pac.example.corp/proxy.pac")
    );
    assert_eq!(resolved.bypass.as_deref(), Some("*.corp.example"));
    assert_eq!(resolved.origin, SystemProxyOrigin::ConnectionSettings);
}

#[test]
fn the_auto_config_url_value_is_read_before_the_blob() {
    let resolved = resolve_with(&Stub {
        ie: Ok(None),
        auto_config_url: Ok(Some("https://pac.example.corp/value.pac".into())),
        blob: Ok(Some(blob(
            0x0d,
            "",
            "",
            "https://pac.example.corp/blob.pac",
        ))),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        resolved.pac_url.as_deref(),
        Some("https://pac.example.corp/value.pac")
    );
    assert_eq!(resolved.origin, SystemProxyOrigin::AutoConfigUrlValue);
}

/// A blob whose auto-config-URL flag is off carries a stale URL
/// Windows ignores; vibe ignores it too and falls through.
#[test]
fn a_blob_url_without_its_flag_is_not_used() {
    let resolved = resolve_with(&Stub {
        ie: Ok(None),
        blob: Ok(Some(blob(
            0x03,
            "proxy.example.corp:8080",
            "<local>",
            "https://pac.example.corp/stale.pac",
        ))),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(resolved.pac_url, None);
    assert_eq!(
        resolved.upstream.as_deref(),
        Some("proxy.example.corp:8080")
    );
    assert_eq!(resolved.origin, SystemProxyOrigin::ConnectionSettings);
}

#[test]
fn auto_detect_alone_asks_wpad() {
    let resolved = resolve_with(&Stub {
        ie: Ok(Some(IeProxyConfig {
            auto_detect: true,
            ..Default::default()
        })),
        wpad: Ok(Some("http://wpad.corp.example/wpad.dat".into())),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        resolved.pac_url.as_deref(),
        Some("http://wpad.corp.example/wpad.dat")
    );
    assert_eq!(resolved.origin, SystemProxyOrigin::Wpad);
}

/// Auto-detection that finds nothing still leaves the fixed proxy.
#[test]
fn a_fixed_proxy_is_the_upstream_when_wpad_finds_nothing() {
    let resolved = resolve_with(&Stub {
        ie: Ok(Some(IeProxyConfig {
            auto_detect: true,
            proxy: Some("http=a.example.corp:3128;https=b.example.corp:3129".into()),
            proxy_bypass: Some("<local>;*.corp.example;10.0.0.0/8".into()),
            ..Default::default()
        })),
        wpad: Ok(None),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(resolved.pac_url, None);
    assert_eq!(resolved.upstream.as_deref(), Some("b.example.corp:3129"));
    assert_eq!(
        resolved.bypass.as_deref(),
        Some("*.corp.example,10.0.0.0/8")
    );
    assert_eq!(resolved.origin, SystemProxyOrigin::InternetOptions);
}

#[test]
fn nothing_configured_names_every_setting_that_was_read() {
    let error = resolve_with(&Stub {
        ie: Ok(Some(IeProxyConfig::default())),
        auto_config_url: Ok(None),
        blob: Ok(None),
        ..Default::default()
    })
    .unwrap_err();
    let SystemProxyError::NotConfigured { checked, .. } = &error else {
        panic!("{error}");
    };
    assert_eq!(checked.len(), 5, "{checked:?}");
    let text = error.to_string();
    for expected in [
        "Policies",
        "WinHttpGetIEProxyConfigForCurrentUser",
        "AutoConfigURL",
        "DefaultConnectionSettings",
        "fix:",
        "PROP-068#proxy-system",
    ] {
        assert!(text.contains(expected), "{expected} missing from {text}");
    }
}

/// A reader that fails is reported among the settings checked, not
/// swallowed: the user sees which read did not work.
#[test]
fn a_reader_failure_is_reported_not_swallowed() {
    let error = resolve_with(&Stub {
        policy_machine: Err("access denied".into()),
        ie: Err("the winhttp export is missing".into()),
        auto_config_url: Err("access denied".into()),
        blob: Err("value is not binary".into()),
        ..Default::default()
    })
    .unwrap_err()
    .to_string();
    for expected in [
        "access denied",
        "the winhttp export is missing",
        "value is not binary",
    ] {
        assert!(error.contains(expected), "{expected} missing from {error}");
    }
}

#[test]
fn a_truncated_blob_is_reported_and_the_rest_still_runs() {
    // Header, two empty strings and the URL's length word, with the
    // URL itself cut off.
    let mut short = blob(0x0d, "", "", "https://pac.example.corp/proxy.pac");
    short.truncate(24);
    let error = resolve_with(&Stub {
        ie: Ok(None),
        blob: Ok(Some(short)),
        ..Default::default()
    })
    .unwrap_err()
    .to_string();
    assert!(error.contains("remain"), "{error}");
}

#[test]
fn the_header_alone_is_a_valid_blob_with_no_strings() {
    let mut header = Vec::new();
    header.extend_from_slice(&0x46u32.to_le_bytes());
    header.extend_from_slice(&1u32.to_le_bytes());
    header.extend_from_slice(&CONNECTION_DIRECT.to_le_bytes());
    let settings = parse_connection_settings(&header).unwrap();
    assert_eq!(settings.version, 0x46);
    assert_eq!(settings.flags, CONNECTION_DIRECT);
    assert_eq!(settings.proxy, None);
}

/// The shape a machine with a fixed proxy and no PAC writes: flags
/// `0x03`, a `host:port`, a long bypass list, an empty URL field, and
/// the reserved tail. (A managed desktop writes `0x0D` with a URL
/// instead; both are parsed by the same code.)
#[test]
fn a_fixed_proxy_blob_parses_field_by_field() {
    let bytes = blob(
        CONNECTION_DIRECT | CONNECTION_PROXY,
        "proxy.example.corp:8080",
        "<local>;localhost;127.*;10.*;192.168.*",
        "",
    );
    let settings = parse_connection_settings(&bytes).unwrap();
    assert_eq!(settings.version, 0x46);
    assert_eq!(settings.flags, 0x03);
    assert!(settings.proxy_enabled() && !settings.auto_config_url_enabled());
    assert!(!settings.auto_detect());
    assert_eq!(settings.proxy.as_deref(), Some("proxy.example.corp:8080"));
    assert_eq!(
        settings.bypass.as_deref(),
        Some("<local>;localhost;127.*;10.*;192.168.*")
    );
    assert_eq!(settings.auto_config_url, None);
    assert_eq!(
        bypass_list(settings.bypass.as_deref().unwrap()).as_deref(),
        Some("localhost,127.*,10.*,192.168.*")
    );
}

/// The installed source is what a cross-platform test drives, and
/// what `vibe_net::system_proxy::resolve` reads before the host.
#[test]
fn the_installed_source_is_what_resolution_reads() {
    install_source(Some(Arc::new(Stub {
        policy_user: Ok(Some("https://pac.example.corp/installed.pac".into())),
        ..Default::default()
    })));
    let source = installed_source().expect("installed");
    let resolved = resolve_with(source.as_ref()).unwrap();
    install_source(None);
    assert_eq!(
        resolved.pac_url.as_deref(),
        Some("https://pac.example.corp/installed.pac")
    );
    assert_eq!(
        resolved.describe(),
        "system → https://pac.example.corp/installed.pac (from policy HKCU)"
    );
}
