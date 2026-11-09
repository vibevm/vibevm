//! The controlled HTTP transport through an explicit `[proxy]`
//! (`PROP-068#proxy`): HTTPS tunnels by `CONNECT` under source policy,
//! the policy judges the requested host name before the proxy is asked,
//! `no_proxy` and loopback bypass, and `[proxy] ca_file` is the only way a
//! private CA is trusted — an untrusted one fails clearly.
#![cfg(unix)]

#[path = "../../vibe-net/tests/support/mod.rs"]
mod support;

use std::{path::Path, sync::Arc, time::Duration};
use support::{FIXTURE_HOST, https_server, proxy, requests, test_ca};
use vibe_core::{cancellation::CancellationToken, net::ProxyRoute, user_config::ProxyConfig};
use vibe_net::NetworkSession;
use vibe_registry::http_admission::{
    ContactRefusal, HttpAdmission, HttpGetError, ManagedHttpClient,
};

fn session(proxy_port: u16, extra: &str) -> Arc<NetworkSession> {
    let cfg: ProxyConfig = toml::from_str(&format!(
        "https = 'http://127.0.0.1:{proxy_port}'\nhttp = 'http://127.0.0.1:{proxy_port}'\n{extra}"
    ))
    .unwrap();
    Arc::new(NetworkSession::new(ProxyRoute::from_config(&cfg).unwrap()))
}

fn bound(session: &Arc<NetworkSession>, excluded: &[&str]) -> ManagedHttpClient {
    ManagedHttpClient::captured_in(
        session,
        Duration::from_secs(20),
        None,
        reqwest::header::HeaderMap::new(),
        Some(Arc::new(
            HttpAdmission::new(excluded.iter().copied()).unwrap(),
        )),
        None,
    )
    .unwrap()
}

struct World {
    dir: tempfile::TempDir,
    server: support::Server,
    proxy: support::Server,
    ca_pem: std::path::PathBuf,
    log: std::path::PathBuf,
}

fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    let ca = test_ca(dir.path(), "corp");
    let root = dir.path().join("www");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("artifact.txt"), b"through the proxy\n").unwrap();
    let server = https_server(&root, Some(&ca), dir.path());
    let log = dir.path().join("proxy.log");
    let proxy = proxy(&log);
    World {
        ca_pem: ca.ca_pem.clone(),
        dir,
        server,
        proxy,
        log,
    }
}

fn ca_line(path: &Path) -> String {
    format!("ca_file = '{}'", path.display())
}

#[test]
fn https_tunnels_through_the_proxy_under_source_policy_with_the_ca_file() {
    let w = world();
    let session = session(w.proxy.port, &ca_line(&w.ca_pem));
    let client = bound(&session, &["github.com"]);
    let url = format!("https://{FIXTURE_HOST}:{}/artifact.txt", w.server.port);
    let body = client
        .get_anonymous(&url, &CancellationToken::new())
        .unwrap()
        .bytes()
        .unwrap();
    assert_eq!(body, b"through the proxy\n");
    assert_eq!(
        requests(&w.log),
        [format!("CONNECT {FIXTURE_HOST}:{} HTTP/1.1", w.server.port)]
    );
}

#[test]
fn policy_judges_the_requested_host_before_the_proxy_is_asked() {
    let w = world();
    let session = session(w.proxy.port, &ca_line(&w.ca_pem));
    let client = bound(&session, &[FIXTURE_HOST]);
    let url = format!("https://{FIXTURE_HOST}:{}/artifact.txt", w.server.port);
    let error = client
        .get_anonymous(&url, &CancellationToken::new())
        .err()
        .unwrap();
    assert!(
        matches!(
            error,
            HttpGetError::Refused(ContactRefusal::ExcludedHost { .. })
        ),
        "{error}"
    );
    assert!(
        requests(&w.log).is_empty(),
        "the proxy was asked: {:?}",
        requests(&w.log)
    );
}

#[test]
fn no_proxy_and_loopback_connect_directly() {
    let w = world();
    // The fixture host bypasses the proxy, so it must resolve directly —
    // and it does not: the failure proves the proxy was not used.
    let session = session(
        w.proxy.port,
        &format!("no_proxy = '{FIXTURE_HOST}'\n{}", ca_line(&w.ca_pem)),
    );
    let client = bound(&session, &[]);
    let url = format!("https://{FIXTURE_HOST}:{}/artifact.txt", w.server.port);
    assert!(
        client
            .get_anonymous(&url, &CancellationToken::new())
            .is_err()
    );
    // Loopback always connects directly, whatever the list says.
    let loopback = format!("https://127.0.0.1:{}/artifact.txt", w.server.port);
    let body = client
        .get_anonymous(&loopback, &CancellationToken::new())
        .unwrap()
        .bytes()
        .unwrap();
    assert_eq!(body, b"through the proxy\n");
    assert!(requests(&w.log).is_empty(), "{:?}", requests(&w.log));
}

#[test]
fn a_private_ca_is_trusted_only_through_ca_file_and_fails_clearly_otherwise() {
    let w = world();
    let url = format!("https://{FIXTURE_HOST}:{}/artifact.txt", w.server.port);
    let untrusted = session(w.proxy.port, "");
    let error = bound(&untrusted, &[])
        .get_anonymous(&url, &CancellationToken::new())
        .err()
        .unwrap();
    let mut chain = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(next) = source {
        chain.push_str(&format!(": {next}"));
        source = next.source();
    }
    assert!(
        chain.contains("UnknownIssuer") || chain.contains("certificate"),
        "{chain}"
    );
    assert_eq!(vibe_net::doctor::cause_of(&chain, false).0, "untrusted-ca");
    // A different CA in ca_file does not help either.
    let other = test_ca(w.dir.path(), "other");
    let wrong = session(w.proxy.port, &ca_line(&other.ca_pem));
    assert!(
        bound(&wrong, &[])
            .get_anonymous(&url, &CancellationToken::new())
            .is_err()
    );
    // An unreadable ca_file is a clear configuration error, not a TLS one.
    let missing = session(w.proxy.port, "ca_file = '/nonexistent/corp-ca.pem'");
    let built = ManagedHttpClient::captured_in(
        &missing,
        Duration::from_secs(5),
        None,
        reqwest::header::HeaderMap::new(),
        None,
        None,
    );
    let text = built.err().unwrap().to_string();
    assert!(
        text.contains("/nonexistent/corp-ca.pem") && text.contains("fix:"),
        "{text}"
    );
}

#[test]
fn a_plain_http_request_goes_through_the_http_proxy() {
    let w = world();
    let root = w.dir.path().join("plain");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"plain\n").unwrap();
    let plain = https_server(&root, None, &w.dir.path().join("plain"));
    let session = session(w.proxy.port, "");
    let url = format!("http://{FIXTURE_HOST}:{}/a.txt", plain.port);
    let body = bound(&session, &[])
        .get_anonymous(&url, &CancellationToken::new())
        .unwrap()
        .bytes()
        .unwrap();
    assert_eq!(body, b"plain\n");
    assert_eq!(
        requests(&w.log),
        [format!(
            "GET http://{FIXTURE_HOST}:{}/a.txt HTTP/1.1",
            plain.port
        )]
    );
}
