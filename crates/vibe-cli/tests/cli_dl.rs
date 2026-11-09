//! `vibe dl` end to end (PROP-068 `#dl`): local HTTPS/HTTP servers with a
//! test CA, the CONNECT-proxy fixture and the fake px; no external network.
//! Every failure class is provoked and its code and exit status checked.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;
use common::net_fixture::{
    ConnectProxy, FIXTURE_HOST, FileServer, HttpServer, HttpsServer, TestCa, fake_px,
};
use specmark::verifies;

struct World {
    dir: tempfile::TempDir,
    ca: TestCa,
    server: FileServer,
    user: UserScratch,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let ca = TestCa::new(dir.path());
        let root = dir.path().join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.txt"), b"hello\n").unwrap();
        fs::write(root.join("blob.bin"), [0u8, 159, 146, 150, 0, 1, 2]).unwrap();
        let server = HttpsServer::start(&ca, &root);
        let world = Self {
            dir,
            ca,
            server,
            user: UserScratch::new(),
        };
        world.config("");
        world
    }

    /// `~/.vibe/config.toml` with the test CA trusted and `extra` lines in
    /// `[proxy]`.
    fn config(&self, extra: &str) {
        fs::create_dir_all(&self.user.settings).unwrap();
        fs::write(
            self.user.settings.join("config.toml"),
            format!("[proxy]\nca_file = '{}'\n{extra}", self.ca.ca_pem.display()),
        )
        .unwrap();
    }

    fn url(&self, host: &str, path: &str) -> String {
        self.server.url(host, path)
    }

    fn out(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn dl(&self, args: &[&str]) -> Output {
        self.dl_env(args, &[])
    }

    fn dl_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let home = self.dir.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let mut command = self.user.vibe();
        command
            .arg("dl")
            .args(args)
            .current_dir(self.dir.path())
            .env("HOME", &home)
            .env("NO_COLOR", "1")
            .env_remove("HTTPS_PROXY")
            .env_remove("HTTP_PROXY")
            .env_remove("ALL_PROXY")
            .env_remove("https_proxy")
            .env_remove("http_proxy")
            .env_remove("VIBEVM_PROXY_MODE")
            .env_remove("VIBEVM_PROXY_PAC")
            .env_remove("FAKE_PX_FAIL");
        for (name, value) in env {
            command.env(name, value);
        }
        command.output().unwrap()
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn no_partials(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(
            !name.starts_with(".vibe-dl-"),
            "partial left behind: {name}"
        );
    }
}

/// Whether the process is gone (or a zombie nobody can hold).
fn gone(pid: &str) -> bool {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(_) => true,
        Ok(stat) => stat
            .rsplit(')')
            .next()
            .is_some_and(|rest| rest.trim_start().starts_with('Z')),
    }
}

fn px_pids(log: &Path) -> Vec<String> {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("start "))
        .map(str::to_owned)
        .collect()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn a_direct_https_download_names_its_route_tls_and_trust_and_writes_atomically() {
    let w = World::new();
    let dest = w.out("got.txt");
    let output = w.dl(&[
        &w.url("127.0.0.1", "a.txt"),
        dest.to_str().unwrap(),
        "--sha256",
        "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03",
    ]);
    let err = stderr(&output);
    assert_eq!(code(&output), 0, "{err}");
    assert_eq!(fs::read(&dest).unwrap(), b"hello\n");
    for needle in [
        "route",
        "direct",
        "TLS 1.3",
        "trusted by ca_file",
        "200 OK",
        "6 B",
    ] {
        assert!(err.contains(needle), "missing {needle:?} in:\n{err}");
    }
    assert!(stdout(&output).is_empty(), "stdout must stay clean");
    // A directory destination takes the URL's last segment; an existing
    // file is refused without --force and replaced with it.
    let again = w.dl(&[&w.url("127.0.0.1", "a.txt"), w.dir.path().to_str().unwrap()]);
    assert_eq!(code(&again), 0, "{}", stderr(&again));
    assert!(w.out("a.txt").is_file());
    let refused = w.dl(&[&w.url("127.0.0.1", "a.txt"), dest.to_str().unwrap()]);
    assert_eq!(code(&refused), 17, "{}", stderr(&refused));
    assert!(stderr(&refused).contains("dl_destination_exists"));
    let forced = w.dl(&[
        &w.url("127.0.0.1", "a.txt"),
        dest.to_str().unwrap(),
        "--force",
    ]);
    assert_eq!(code(&forced), 0, "{}", stderr(&forced));
    no_partials(w.dir.path());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn a_digest_mismatch_keeps_nothing() {
    let w = World::new();
    let dest = w.out("bad.txt");
    let output = w.dl(&[
        &w.url("127.0.0.1", "a.txt"),
        dest.to_str().unwrap(),
        "--sha256",
        &"0".repeat(64),
    ]);
    assert_eq!(code(&output), 16, "{}", stderr(&output));
    assert!(stderr(&output).contains("dl_sha256_mismatch"));
    assert!(!dest.exists());
    no_partials(w.dir.path());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn an_untrusted_server_is_named_with_its_issuer_and_never_receives_a_request() {
    let w = World::new();
    let other = TestCa::untrusted(w.dir.path());
    let root = w.dir.path().join("root");
    let rogue = HttpsServer::start(&other, &root);
    let output = w.dl(&["--show", &rogue.url("127.0.0.1", "a.txt")]);
    let err = stderr(&output);
    assert_eq!(code(&output), 11, "{err}");
    assert!(err.contains("dl_untrusted_certificate"), "{err}");
    assert!(err.contains("unable to get local issuer"), "{err}");
    assert!(err.contains("Vibe Fixture untrusted CA"), "{err}");
    assert!(err.contains("ca_file"), "{err}");
    assert!(rogue.requests().is_empty(), "{:?}", rogue.requests());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn a_certificate_for_another_name_is_a_name_mismatch() {
    let w = World::new();
    // The proxy maps every name to the fixture; `other.test` is not among
    // the leaf's names.
    let proxy = ConnectProxy::start();
    let url = w.url("other.test", "a.txt");
    let output = w.dl(&["--proxy", &proxy.url(), "--show", &url]);
    assert_eq!(code(&output), 12, "{}", stderr(&output));
    assert!(stderr(&output).contains("dl_certificate_name"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn http_statuses_redirects_dns_and_refused_connections_are_classified() {
    let w = World::new();
    let missing = w.dl(&["--show", &w.url("127.0.0.1", "status/404")]);
    assert_eq!(code(&missing), 14, "{}", stderr(&missing));
    assert!(stderr(&missing).contains("dl_http_client_error"));
    let broken = w.dl(&["--show", &w.url("127.0.0.1", "status/503")]);
    assert_eq!(code(&broken), 15, "{}", stderr(&broken));
    let redirected = w.dl(&["-v", "--show", &w.url("127.0.0.1", "redirect/a.txt")]);
    assert_eq!(code(&redirected), 0, "{}", stderr(&redirected));
    assert_eq!(stdout(&redirected), "hello\n");
    let err = stderr(&redirected);
    assert!(
        err.contains("302 Found") && err.contains("redirect"),
        "{err}"
    );
    assert!(
        err.contains("Location: /a.txt"),
        "-v shows the redirect's headers:\n{err}"
    );
    // The fixture name resolves only through the proxy.
    let dns = w.dl(&["--show", &w.url(FIXTURE_HOST, "a.txt")]);
    assert_eq!(code(&dns), 6, "{}", stderr(&dns));
    assert!(stderr(&dns).contains("dl_dns"));
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let refused = w.dl(&["--show", &format!("https://127.0.0.1:{port}/a.txt")]);
    assert_eq!(code(&refused), 8, "{}", stderr(&refused));
    assert!(stderr(&refused).contains("dl_connect_failed"));
    let offline = w.dl(&["--offline", "--show", &w.url("127.0.0.1", "a.txt")]);
    assert_eq!(code(&offline), 3, "{}", stderr(&offline));
    let not_http = w.dl(&["--show", "ftp://example.test/a"]);
    assert_eq!(code(&not_http), 1, "{}", stderr(&not_http));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn show_and_headers_keep_stdout_for_the_response_and_diagnostics_on_stderr() {
    let w = World::new();
    let url = w.url("127.0.0.1", "a.txt");
    let body = w.dl(&["--show", &url]);
    assert_eq!(code(&body), 0, "{}", stderr(&body));
    assert_eq!(body.stdout, b"hello\n", "byte-exact body");
    assert!(stderr(&body).contains("TLS"));
    let head = w.dl(&["--headers", &url]);
    let text = stdout(&head);
    assert_eq!(code(&head), 0, "{}", stderr(&head));
    assert!(text.starts_with("HTTP/1.1 200 OK\n"), "{text}");
    assert!(
        text.to_ascii_lowercase().contains("content-length: 6\n"),
        "{text}"
    );
    assert!(!text.contains("hello"), "--headers alone prints no body");
    let both = w.dl(&["--headers", "--show", &url]);
    let text = stdout(&both);
    assert!(text.starts_with("HTTP/1.1 200 OK\n"), "{text}");
    assert!(
        text.ends_with("\n\nhello\n"),
        "headers, blank line, body: {text:?}"
    );
    // A destination with --show is a usage error.
    let usage = w.dl(&["--show", &url, "x.txt"]);
    assert_eq!(code(&usage), 2, "{}", stderr(&usage));
    // A binary body to a pipe is written byte-exact.
    let blob = w.dl(&["--show", &w.url("127.0.0.1", "blob.bin")]);
    assert_eq!(blob.stdout, [0u8, 159, 146, 150, 0, 1, 2]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn the_json_report_carries_the_route_tls_and_download() {
    let w = World::new();
    let dest = w.out("j.txt");
    let output = w.dl(&[
        "--json",
        &w.url("127.0.0.1", "a.txt"),
        dest.to_str().unwrap(),
    ]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["protocol"], "vibe-dl/1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["mode"], "direct");
    assert_eq!(report["hops"][0]["tls"]["trusted_by"], "ca_file");
    assert_eq!(report["download"]["bytes"], 6);
    let failed = w.dl(&[
        "--json",
        &w.url("127.0.0.1", "status/404"),
        dest.to_str().unwrap(),
        "--force",
    ]);
    assert_eq!(code(&failed), 14);
    let report: serde_json::Value = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(report["failure"]["class"], "dl_http_client_error");
    assert_eq!(report["failure"]["exit_code"], 14);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn a_configured_proxy_carries_the_request_and_no_proxy_bypasses_it() {
    let w = World::new();
    let proxy = ConnectProxy::start();
    w.config(&format!(
        "https = '{}'\nno_proxy = 'internal.example'",
        proxy.url()
    ));
    let output = w.dl(&["--show", &w.url(FIXTURE_HOST, "a.txt")]);
    let err = stderr(&output);
    assert_eq!(code(&output), 0, "{err}");
    assert_eq!(stdout(&output), "hello\n");
    assert!(
        err.contains("via http://127.0.0.1") && err.contains("200 Connection established"),
        "{err}"
    );
    let connects: Vec<String> = proxy
        .requests()
        .into_iter()
        .filter(|line| line.starts_with(&format!("CONNECT {FIXTURE_HOST}:")))
        .collect();
    // The probe's tunnel and the transfer's tunnel.
    assert!(connects.len() >= 2, "{:?}", proxy.requests());
    // Loopback always bypasses the proxy.
    let before = proxy.requests().len();
    let direct = w.dl(&["--show", &w.url("127.0.0.1", "a.txt")]);
    assert_eq!(code(&direct), 0, "{}", stderr(&direct));
    assert_eq!(
        proxy.requests().len(),
        before,
        "loopback went through the proxy"
    );
    assert!(stderr(&direct).contains("direct (no_proxy)"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn a_proxy_that_wants_authentication_is_a_407_with_the_px_fix() {
    let w = World::new();
    let proxy = ConnectProxy::start_requiring_auth();
    let output = w.dl(&[
        "--proxy",
        &proxy.url(),
        "--show",
        &w.url(FIXTURE_HOST, "a.txt"),
    ]);
    let err = stderr(&output);
    assert_eq!(code(&output), 9, "{err}");
    assert!(
        err.contains("dl_proxy_auth") && err.contains("407"),
        "{err}"
    );
    assert!(err.contains("mode = \"px\""), "{err}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn source_policy_refuses_an_excluded_host_before_any_contact() {
    let w = World::new();
    let proxy = ConnectProxy::start();
    fs::write(
        w.user.settings.join("registry.toml"),
        format!("[source_policy]\nexclude_hosting = ['host:{FIXTURE_HOST}']\n"),
    )
    .unwrap();
    let output = w.dl(&[
        "--proxy",
        &proxy.url(),
        "--show",
        &w.url(FIXTURE_HOST, "a.txt"),
    ]);
    let err = stderr(&output);
    assert_eq!(code(&output), 4, "{err}");
    assert!(err.contains("dl_source_policy"), "{err}");
    assert!(proxy.requests().is_empty(), "{:?}", proxy.requests());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn a_proxy_is_an_option_never_the_default() {
    let w = World::new();
    // Nothing configured: bare --proxy has nothing to run.
    let output = w.dl(&["--proxy", "--show", &w.url("127.0.0.1", "a.txt")]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    assert!(stderr(&output).contains("no proxy is configured"));
    let output = w.dl(&[
        "--proxy",
        "nonsense",
        "--show",
        &w.url("127.0.0.1", "a.txt"),
    ]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    // A configured px is not used without the table asking for it.
    w.config("pac = 'https://pac.example.corp/proxy.pac'");
    let plain = w.dl(&["--show", &w.url("127.0.0.1", "a.txt")]);
    assert_eq!(code(&plain), 0, "{}", stderr(&plain));
    assert!(stderr(&plain).contains("mode direct"), "{}", stderr(&plain));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn managed_px_starts_for_the_download_and_is_reaped_after_success_and_failure() {
    let w = World::new();
    let log = w.out("px.log");
    w.config(&format!(
        "mode = 'px'\nupstream = 'proxy.example.corp:8080'\npx_path = '{}'",
        fake_px().display()
    ));
    let env = [("FAKE_PX_LOG", log.to_str().unwrap())];
    let output = w.dl_env(&["--show", &w.url(FIXTURE_HOST, "a.txt")], &env);
    let err = stderr(&output);
    assert_eq!(code(&output), 0, "{err}");
    assert_eq!(stdout(&output), "hello\n");
    assert!(err.contains("via px http://127.0.0.1:"), "{err}");
    let pids = px_pids(&log);
    assert_eq!(pids.len(), 1, "one px per operation: {pids:?}");
    assert!(
        pids.iter().all(|pid| gone(pid)),
        "px left running: {pids:?}"
    );
    // A failing px is reported as such, with its own exit status.
    let failed = w.dl_env(
        &["--show", &w.url(FIXTURE_HOST, "a.txt")],
        &[
            ("FAKE_PX_LOG", log.to_str().unwrap()),
            ("FAKE_PX_FAIL", "1"),
        ],
    );
    let err = stderr(&failed);
    assert_eq!(code(&failed), 5, "{err}");
    assert!(
        err.contains("dl_px") && err.contains("FAKE_PX_FAIL"),
        "{err}"
    );
    // An HTTP failure through px still reaps it.
    let missing = w.dl_env(&["--show", &w.url(FIXTURE_HOST, "status/404")], &env);
    assert_eq!(code(&missing), 14, "{}", stderr(&missing));
    let pids = px_pids(&log);
    assert_eq!(pids.len(), 2, "{pids:?}");
    assert!(
        pids.iter().all(|pid| gone(pid)),
        "px left running: {pids:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn a_stop_file_cancels_the_transfer_removes_the_partial_and_reaps_px() {
    let w = World::new();
    let log = w.out("px.log");
    w.config(&format!(
        "mode = 'px'\nupstream = 'proxy.example.corp:8080'\npx_path = '{}'",
        fake_px().display()
    ));
    let stop = w.out("stop");
    let dest = w.out("slow.bin");
    let home = w.dir.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_vibe"));
    command
        .args(["--stop-file", stop.to_str().unwrap(), "dl"])
        .arg(w.url(FIXTURE_HOST, "slow/20"))
        .arg(&dest)
        .env("VIBE_SETTINGS", &w.user.settings)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("FAKE_PX_LOG", &log)
        .current_dir(w.dir.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = command.spawn().unwrap();
    let started = std::time::Instant::now();
    while px_pids(&log).is_empty() && started.elapsed().as_secs() < 20 {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    std::thread::sleep(std::time::Duration::from_millis(800));
    fs::write(&stop, b"").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(code(&output), 130, "{}", stderr(&output));
    assert!(!dest.exists());
    no_partials(w.dir.path());
    let pids = px_pids(&log);
    assert_eq!(pids.len(), 1, "{pids:?}");
    assert!(
        pids.iter().all(|pid| gone(pid)),
        "px left running: {pids:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn plain_http_through_the_proxy_uses_absolute_form() {
    let w = World::new();
    let root = w.dir.path().join("root");
    let http = HttpServer::start(&root);
    let proxy = ConnectProxy::start();
    let output = w.dl(&[
        "--proxy",
        &proxy.url(),
        "--show",
        &http.url(FIXTURE_HOST, "a.txt"),
    ]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert!(
        proxy
            .requests()
            .iter()
            .any(|line| line.starts_with(&format!("GET http://{FIXTURE_HOST}:"))),
        "{:?}",
        proxy.requests()
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn test_proxy_reports_every_step_and_separates_proxy_from_target_problems() {
    let w = World::new();
    // No proxy configured: nothing to test.
    let none = w.dl(&["--test-proxy", &w.url(FIXTURE_HOST, "a.txt")]);
    assert_eq!(code(&none), 2, "{}", stderr(&none));
    let proxy = ConnectProxy::start();
    w.config(&format!("https = '{}'", proxy.url()));
    let ok = w.dl(&["--test-proxy", &w.url(FIXTURE_HOST, "redirect/a.txt")]);
    let err = stderr(&ok);
    assert_eq!(code(&ok), 0, "{err}");
    for needle in [
        "config",
        "mode explicit (default)",
        "no_proxy",
        "CONNECT",
        "TLS",
        "redirect",
        "proxy test passed",
    ] {
        assert!(err.contains(needle), "missing {needle:?}:\n{err}");
    }
    // A 407 is a proxy problem.
    let auth = ConnectProxy::start_requiring_auth();
    let denied = w.dl(&[
        "--test-proxy",
        "--proxy",
        &auth.url(),
        &w.url(FIXTURE_HOST, "a.txt"),
    ]);
    let err = stderr(&denied);
    assert_eq!(code(&denied), 21, "{err}");
    assert!(
        err.contains("proxy auth") && err.contains("dl_proxy_auth"),
        "{err}"
    );
    // An untrusted target certificate is a target problem.
    let other = TestCa::untrusted(w.dir.path());
    let rogue = HttpsServer::start(&other, &w.dir.path().join("root"));
    let untrusted = w.dl(&["--test-proxy", &rogue.url(FIXTURE_HOST, "a.txt")]);
    let err = stderr(&untrusted);
    assert_eq!(code(&untrusted), 22, "{err}");
    assert!(err.contains("TLS trust"), "{err}");
    // The JSON form carries the same rows.
    let json = w.dl(&["--json", "--test-proxy", &auth_url(&w)]);
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["protocol"], "vibe-dl/1");
    assert!(
        report["checks"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    );
}

fn auth_url(w: &World) -> String {
    w.url(FIXTURE_HOST, "a.txt")
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
fn test_proxy_in_px_mode_runs_px_test_and_reaps_px() {
    let w = World::new();
    let log = w.out("px.log");
    w.config(&format!(
        "mode = 'px'\nupstream = 'proxy.example.corp:8080'\npx_path = '{}'",
        fake_px().display()
    ));
    let env = [("FAKE_PX_LOG", log.to_str().unwrap())];
    let ok = w.dl_env(&["--test-proxy", &w.url(FIXTURE_HOST, "a.txt")], &env);
    let err = stderr(&ok);
    assert_eq!(code(&ok), 0, "{err}");
    for needle in ["px present", "px start", "px --test", "via px"] {
        assert!(err.contains(needle), "missing {needle:?}:\n{err}");
    }
    let failing = w.dl_env(
        &["--test-proxy", &w.url(FIXTURE_HOST, "a.txt")],
        &[
            ("FAKE_PX_LOG", log.to_str().unwrap()),
            ("FAKE_PX_TEST_FAIL", "1"),
        ],
    );
    let err = stderr(&failing);
    assert_eq!(code(&failing), 21, "{err}");
    // It went on past `px --test` and still reached the target.
    assert!(err.contains("px --test") && err.contains("200 OK"), "{err}");
    let pids = px_pids(&log);
    assert!(!pids.is_empty());
    assert!(
        pids.iter().all(|pid| gone(pid)),
        "px left running: {pids:?}"
    );
}
