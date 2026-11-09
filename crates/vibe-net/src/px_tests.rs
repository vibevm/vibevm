//! The route arguments of a px run (`PROP-068#proxy-system`): what
//! `pac = "system"` becomes, and that a configured route still travels
//! exactly as it did. The Windows settings come from the
//! [`WindowsProxySource`] seam, so these cases run on every host.

use std::sync::Arc;

use vibe_core::net::{IeProxyConfig, ModeOrigin, PxSettings, WindowsProxySource, system_proxy};
use vibe_core::user_config::PacSource;

use crate::PxError;
use crate::px::route_args;

/// A Windows machine whose settings say exactly this.
#[derive(Debug)]
struct Stub(IeProxyConfig);

impl WindowsProxySource for Stub {
    fn policy_auto_config_url(&self, _machine: bool) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn ie_proxy_config(&self) -> Result<Option<IeProxyConfig>, String> {
        Ok(Some(self.0.clone()))
    }
    fn auto_config_url_value(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn default_connection_settings(&self) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
    fn detect_auto_proxy_url(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
}

fn settings(pac: Option<PacSource>, upstream: Option<&str>) -> PxSettings {
    PxSettings {
        pac,
        pac_origin: ModeOrigin::User,
        upstream: upstream.map(str::to_owned),
        args: Vec::new(),
        path: None,
        port: None,
    }
}

/// Installed for the body of one test and removed after it. The
/// installed source is process-global, so these tests take turns.
fn with_source<T>(source: IeProxyConfig, body: impl FnOnce() -> T) -> T {
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    system_proxy::install_source(Some(Arc::new(Stub(source))));
    let out = body();
    system_proxy::install_source(None);
    out
}

/// The whole point: px is handed an explicit `--pac`, never left to
/// detect the settings itself (`PROP-068#proxy-px`).
#[test]
fn a_system_pac_becomes_an_explicit_pac_argument() {
    let args = with_source(
        IeProxyConfig {
            auto_config_url: Some("https://pac.example.corp/proxy.pac".into()),
            proxy_bypass: Some("<local>;*.corp.example".into()),
            ..Default::default()
        },
        || route_args(&settings(Some(PacSource::System), None), None).unwrap(),
    );
    assert_eq!(
        args,
        vec![
            "--pac=https://pac.example.corp/proxy.pac".to_owned(),
            "--noproxy=*.corp.example".to_owned(),
        ]
    );
}

/// Settings that name a fixed proxy and no PAC become `--proxy`.
#[test]
fn a_system_proxy_server_becomes_the_upstream() {
    let args = with_source(
        IeProxyConfig {
            proxy: Some("proxy.example.corp:8080".into()),
            ..Default::default()
        },
        || route_args(&settings(Some(PacSource::System), None), None).unwrap(),
    );
    assert_eq!(args, vec!["--proxy=proxy.example.corp:8080".to_owned()]);
}

/// A configured `no_proxy` is the user's own word and is not replaced
/// by the bypass list Windows carried; a configured `upstream` likewise.
#[test]
fn the_configuration_outranks_the_windows_settings() {
    let args = with_source(
        IeProxyConfig {
            proxy: Some("ignored.example.corp:1".into()),
            proxy_bypass: Some("ignored.example.corp".into()),
            ..Default::default()
        },
        || {
            route_args(
                &settings(Some(PacSource::System), Some("mine.example.corp:8080")),
                Some(".mine.example"),
            )
            .unwrap()
        },
    );
    assert_eq!(
        args,
        vec![
            "--proxy=mine.example.corp:8080".to_owned(),
            "--noproxy=.mine.example".to_owned(),
        ]
    );
}

/// Nothing configured in Windows: px is not started at all, and the
/// error names what was checked and the fix.
#[test]
fn unresolvable_system_settings_stop_px_with_the_cause() {
    let error = with_source(IeProxyConfig::default(), || {
        route_args(&settings(Some(PacSource::System), None), None).unwrap_err()
    });
    assert!(matches!(error, PxError::SystemProxy(_)), "{error}");
    let text = error.to_string();
    assert!(text.contains("DefaultConnectionSettings"), "{text}");
    assert!(text.contains("fix:"), "{text}");
}

/// A PAC URL or file is still passed exactly as before.
#[test]
fn a_configured_pac_is_unchanged() {
    let url = PacSource::parse("https://pac.example.corp/p.pac").unwrap();
    assert_eq!(
        route_args(&settings(Some(url), None), None).unwrap(),
        vec!["--pac=https://pac.example.corp/p.pac".to_owned()]
    );
    let file = PacSource::parse("file:///etc/corp/proxy.pac").unwrap();
    assert_eq!(
        route_args(&settings(Some(file), None), None).unwrap(),
        vec!["--pac=/etc/corp/proxy.pac".to_owned()]
    );
}
