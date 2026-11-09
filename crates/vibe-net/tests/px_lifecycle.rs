//! The vibe-managed px lifecycle (`PROP-068#proxy`) against a fake px:
//! lazy start, one px per operation, killed and reaped after success,
//! failure and a stop; no start for a direct route or a bypassed host.
#![cfg(unix)]

use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use vibe_core::{cancellation::CancellationToken, net::ProxyRoute, user_config::ProxyConfig};
use vibe_net::{NetworkSession, PxError};

fn fake_px() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/fake_px.py")
}

fn route_logging(log: &Path, extra: &str) -> ProxyRoute {
    let cfg: ProxyConfig = toml::from_str(&format!(
        "mode = 'px'\nupstream = 'proxy.example.corp:8080'\npx_path = '{}'\n{extra}",
        fake_px().display()
    ))
    .unwrap();
    let mut cfg = cfg;
    cfg.px_args
        .insert(0, format!("--fake-log={}", log.display()));
    ProxyRoute::from_config(&cfg).unwrap()
}

/// Pids the fake px logged as started.
fn started(log: &Path) -> Vec<u32> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("start "))
        .filter_map(|rest| rest.split_whitespace().next()?.parse().ok())
        .collect()
}

/// Gone, or a zombie its new parent has not reaped yet.
fn gone(pid: u32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Err(_) => return true,
            Ok(stat)
                if stat
                    .rsplit(')')
                    .next()
                    .is_some_and(|s| s.trim_start().starts_with('Z')) =>
            {
                return true;
            }
            Ok(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    false
}

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("fake-px.log");
    (dir, log)
}

#[test]
fn px_starts_lazily_once_and_is_reaped_after_success() {
    let (_dir, log) = scratch();
    let session = NetworkSession::new(route_logging(&log, ""));
    assert!(!session.px_started(), "building a session starts nothing");
    let url = url::Url::parse("https://example.test/a").unwrap();
    let first = session.proxy_for(&url).unwrap().unwrap();
    let second = session.proxy_for(&url).unwrap().unwrap();
    assert_eq!(first, second, "one px per operation");
    assert_eq!(first.host_str(), Some("127.0.0.1"));
    let pids = started(&log);
    assert_eq!(pids.len(), 1, "{pids:?}");
    // Traffic reaches it.
    let probe = std::net::TcpStream::connect(("127.0.0.1", first.port().unwrap()));
    assert!(probe.is_ok());
    session.close();
    assert!(gone(pids[0]), "px {} survived close", pids[0]);
    assert!(matches!(session.px_address(), Err(PxError::Closed)));
}

#[test]
fn a_direct_route_or_a_bypassed_host_never_starts_px() {
    let (_dir, log) = scratch();
    let session = NetworkSession::new(route_logging(&log, "no_proxy = '.corp.example'"));
    for raw in [
        "https://nexus.corp.example/a",
        "http://127.0.0.1:9/a",
        "https://localhost/a",
        "ssh://example.test/repo.git",
    ] {
        let url = url::Url::parse(raw).unwrap();
        assert_eq!(session.proxy_for(&url).unwrap(), None, "{raw}");
    }
    let net = vibe_net::git::for_remote(&session, "/srv/repo.git").unwrap();
    assert!(net.is_empty());
    assert!(!session.px_started());
    let direct = NetworkSession::new(ProxyRoute::direct());
    let url = url::Url::parse("https://example.test/").unwrap();
    assert_eq!(direct.proxy_for(&url).unwrap(), None);
    assert!(!direct.px_started());
    assert!(started(&log).is_empty());
}

#[test]
fn a_px_that_exits_is_reported_with_its_output_and_not_retried() {
    let (_dir, log) = scratch();
    let session = NetworkSession::new(route_logging(&log, "px_args = ['--fake-fail=1']"));
    let error = session.px_address().unwrap_err();
    let text = error.to_string();
    assert!(matches!(error, PxError::Exited { .. }), "{text}");
    assert!(text.contains("refusing to start"), "{text}");
    assert!(text.contains("vibe self doctor"), "{text}");
    // Remembered: the second request does not spawn another px.
    assert!(matches!(session.px_address(), Err(PxError::Exited { .. })));
    assert!(session.px_failure().is_some());
}

#[test]
fn a_missing_px_names_where_it_looked_and_the_fix() {
    let cfg: ProxyConfig = toml::from_str(
        "mode = 'px'\npac = 'https://pac.example.corp/proxy.pac'\npx_path = '/nonexistent/px'",
    )
    .unwrap();
    let session = NetworkSession::new(ProxyRoute::from_config(&cfg).unwrap());
    let error = session.px_address().unwrap_err();
    assert!(matches!(error, PxError::NotInstalled { .. }));
    assert!(error.to_string().contains("/nonexistent/px"));
    assert!(error.to_string().contains("px_path"));
}

#[test]
fn the_operation_guard_reaps_px_and_a_stop_prevents_a_start() {
    let (_dir, log) = scratch();
    let token = CancellationToken::new();
    let guard = vibe_net::enter(&token, route_logging(&log, ""));
    let session: Arc<NetworkSession> = guard.session().clone();
    let address = session.px_address().unwrap();
    assert!(TcpListener::bind(address).is_err(), "px holds its port");
    let pid = started(&log)[0];
    drop(guard);
    assert!(gone(pid), "px {pid} survived the end of the operation");

    let stopped = CancellationToken::new();
    let guard = vibe_net::enter(&stopped, route_logging(&log, ""));
    stopped.cancel();
    assert!(matches!(
        guard.session().px_address(),
        Err(PxError::Cancelled)
    ));
    assert_eq!(started(&log).len(), 1, "no px after the stop");
}

#[test]
fn px_dies_with_its_owner_even_when_the_owner_is_killed() {
    // The supervisor's liveness pipe: a process that started px and is
    // then SIGKILLed — no destructor runs — still leaves no px behind.
    let (_dir, log) = scratch();
    let exe = std::env::current_exe().unwrap();
    let mut child = std::process::Command::new(exe)
        .args(["--exact", "owner_process_body", "--ignored", "--nocapture"])
        .env("VIBE_NET_OWNER_BODY", &log)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while started(&log).is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let pid = *started(&log).first().expect("the owner started px");
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(gone(pid), "px {pid} outlived its SIGKILLed owner");
}

#[test]
#[ignore = "body of px_dies_with_its_owner_even_when_the_owner_is_killed"]
fn owner_process_body() {
    let Some(log) = std::env::var_os("VIBE_NET_OWNER_BODY") else {
        return;
    };
    let log = PathBuf::from(log);
    let session = NetworkSession::new(route_logging(&log, ""));
    session.px_address().unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

#[test]
fn a_busy_fixed_px_port_fails_clearly_and_never_moves() {
    let (_dir, log) = scratch();
    let holder = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = holder.local_addr().unwrap().port();
    let session = NetworkSession::new(route_logging(&log, &format!("px_port = {port}")));
    let error = session.px_address().unwrap_err();
    let text = error.to_string();
    assert!(
        matches!(error, PxError::PortBusy { port: p, .. } if p == port),
        "{text}"
    );
    assert!(
        text.contains("drop `px_port`") && text.contains(&format!("127.0.0.1:{port}")),
        "{text}"
    );
    assert!(
        started(&log).is_empty(),
        "px must not start on another port"
    );
    drop(holder);
    // Free again: px listens exactly there.
    let session = NetworkSession::new(route_logging(&log, &format!("px_port = {port}")));
    assert_eq!(session.px_address().unwrap().port(), port);
    session.close();
}

#[test]
fn modes_none_and_explicit_never_spawn_px_even_with_px_keys() {
    let (_dir, log) = scratch();
    for mode in ["none", "explicit"] {
        let cfg: ProxyConfig = toml::from_str(&format!(
            "mode = '{mode}'\nhttps = 'http://127.0.0.1:9'\nupstream = 'proxy.example.corp:8080'\n\
             px_path = '{}'\npx_args = ['--fake-log={}']",
            fake_px().display(),
            log.display()
        ))
        .unwrap();
        let session = NetworkSession::new(ProxyRoute::from_config(&cfg).unwrap());
        let url = url::Url::parse("https://example.test/").unwrap();
        let proxy = session.proxy_for(&url).unwrap();
        match mode {
            "none" => assert_eq!(proxy, None),
            _ => assert_eq!(proxy.unwrap().as_str(), "http://127.0.0.1:9/"),
        }
        assert!(!session.px_started(), "{mode}");
        assert!(matches!(session.px_address(), Err(PxError::Closed)));
    }
    // The one-shot override turns px off as well.
    let off = NetworkSession::new(route_logging(&log, "").without_px());
    let url = url::Url::parse("https://example.test/").unwrap();
    assert_eq!(off.proxy_for(&url).unwrap(), None);
    assert!(started(&log).is_empty());
}
