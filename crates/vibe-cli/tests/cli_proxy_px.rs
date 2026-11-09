//! `[proxy]` through the real CLI (`PROP-068#proxy`, `#proxy-px`,
//! `#proxy-doctor`) in an isolated settings home: the vibe-managed px (a
//! fake px named by `px_path`) starts only for a command that needs the
//! network, serves it through the local proxy chain, and is reaped after
//! success and after failure; commands without a network need never start
//! it; `VIBEVM_PROXY_MODE=none` turns a configured px off; `self apply
//! --show` prints the mode and its origin.

#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-px");

mod common;
#[path = "../../vibe-net/tests/support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::UserScratch;
use support::{FIXTURE_HOST, https_server, test_ca};

fn fake_px() -> PathBuf {
    support::support_dir().join("fake_px.py")
}

fn started(log: &Path) -> Vec<u32> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("start "))
        .filter_map(|rest| rest.split_whitespace().next()?.parse().ok())
        .collect()
}

fn requests(log: &Path) -> Vec<String> {
    support::requests(log)
}

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

struct World {
    user: UserScratch,
    dir: tempfile::TempDir,
    log: PathBuf,
    server: support::Server,
}

fn world(extra: &str) -> World {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let ca = test_ca(dir.path(), "corp");
    let root = dir.path().join("www");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("index.html"), "ok\n").unwrap();
    let server = https_server(&root, Some(&ca), dir.path());
    let log = dir.path().join("px.log");
    std::fs::write(
        user.settings.join("config.toml"),
        format!(
            "[proxy]\nmode = 'px'\nupstream = 'proxy.example.corp:8080'\npx_path = '{}'\n\
             px_args = ['--fake-log={}']\nca_file = '{}'\n{extra}",
            fake_px().display(),
            log.display(),
            ca.ca_pem.display()
        ),
    )
    .unwrap();
    World {
        user,
        dir,
        log,
        server,
    }
}

fn vibe(world: &World) -> assert_cmd::Command {
    let mut command = world.user.vibe();
    command
        .env("HOME", world.dir.path())
        .env_remove("VIBEVM_PROXY_MODE")
        .env_remove("VIBEVM_PROXY_PAC")
        .env_remove("HTTPS_PROXY")
        .env_remove("https_proxy")
        .env_remove("VIBEVM_USER_CONFIG");
    command
}

#[test]
fn a_network_diagnosis_starts_one_px_routes_through_it_and_reaps_it() {
    let w = world("");
    let target = format!("https://{FIXTURE_HOST}:{}/", w.server.port);
    let out = vibe(&w)
        .args(["--json", "self", "doctor", "--network", &target])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: serde_json::Value =
        serde_json::from_str(&stdout[stdout.find('{').unwrap_or(0)..]).unwrap_or_default();
    let network = report["network"].as_array().cloned().unwrap_or_default();
    let status = |step: &str| {
        network
            .iter()
            .find(|row| row["step"] == step)
            .map(|row| row["status"].as_str().unwrap_or_default().to_owned())
    };
    assert_eq!(
        status("px").as_deref(),
        Some("ok"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(status("px --test").as_deref(), Some("ok"), "{network:?}");
    assert_eq!(status("connect").as_deref(), Some("ok"), "{network:?}");
    assert_eq!(status("tls").as_deref(), Some("ok"), "{network:?}");
    let pids = started(&w.log);
    assert_eq!(pids.len(), 1, "one px for the operation: {pids:?}");
    assert!(
        requests(&w.log)
            .iter()
            .any(|r| r.starts_with(&format!("CONNECT {FIXTURE_HOST}:"))),
        "{:?}",
        requests(&w.log)
    );
    assert!(gone(pids[0]), "px {} survived the command", pids[0]);
}

#[test]
fn px_is_reaped_after_a_failed_operation_and_the_cause_is_reported() {
    let w = world("");
    // The server is up, but the CA is not trusted for this name: use an
    // unrelated CA so TLS fails after px has started.
    let other = test_ca(w.dir.path(), "other");
    let config = w.user.settings.join("config.toml");
    let text = std::fs::read_to_string(&config).unwrap();
    let ca_line = text
        .lines()
        .find(|l| l.starts_with("ca_file"))
        .unwrap()
        .to_owned();
    std::fs::write(
        &config,
        text.replace(&ca_line, &format!("ca_file = '{}'", other.ca_pem.display())),
    )
    .unwrap();
    let target = format!("https://{FIXTURE_HOST}:{}/", w.server.port);
    let out = vibe(&w)
        .args(["self", "doctor", "--network", &target])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(all.contains("network tls"), "{all}");
    assert!(
        all.contains("ca_file") || all.contains("local issuer"),
        "{all}"
    );
    let pids = started(&w.log);
    assert_eq!(pids.len(), 1, "{pids:?}");
    assert!(gone(pids[0]), "px {} survived the failed command", pids[0]);
}

#[test]
fn commands_without_a_network_need_never_start_px() {
    let w = world("");
    for args in [
        vec!["--version"],
        vec!["self", "apply", "--show"],
        vec!["transfers"],
    ] {
        let out = vibe(&w).args(&args).output().unwrap();
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(started(&w.log).is_empty(), "{:?}", started(&w.log));
}

#[test]
fn mode_none_from_the_environment_turns_a_configured_px_off() {
    let w = world("");
    let target = format!("https://{FIXTURE_HOST}:{}/", w.server.port);
    let out = vibe(&w)
        .env("VIBEVM_PROXY_MODE", "none")
        .args(["self", "doctor", "--network", &target])
        .output()
        .unwrap();
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Direct: the fixture name does not resolve, and px never started.
    assert!(!out.status.success(), "{all}");
    assert!(started(&w.log).is_empty(), "{all}");
    let show = vibe(&w)
        .env("VIBEVM_PROXY_MODE", "none")
        .args(["self", "apply", "--show"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&show.stdout);
    assert!(
        text.contains("proxy mode: none (from VIBEVM_PROXY_MODE)"),
        "{text}"
    );
    let show = vibe(&w).args(["self", "apply", "--show"]).output().unwrap();
    let text = String::from_utf8_lossy(&show.stdout);
    assert!(text.contains("proxy mode: px (from user config)"), "{text}");
    assert!(
        text.contains("proxy upstream: proxy.example.corp:8080"),
        "{text}"
    );
}

#[test]
fn the_user_turns_a_site_profiles_px_off() {
    let w = world("");
    // The px table moves into a site profile; the user's own file only
    // says `mode = "none"`.
    let px_table = std::fs::read_to_string(w.user.settings.join("config.toml")).unwrap();
    let profile = w.dir.path().join("corp.toml");
    std::fs::write(&profile, format!("schema = 1\nname = 'corp'\n{px_table}")).unwrap();
    std::fs::write(w.user.settings.join("config.toml"), "").unwrap();
    let apply = vibe(&w)
        .args(["--unattended", "self", "apply", profile.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        apply.status.success(),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );
    let show = vibe(&w).args(["self", "apply", "--show"]).output().unwrap();
    let text = String::from_utf8_lossy(&show.stdout);
    assert!(
        text.contains("proxy mode: px (from site profile)"),
        "{text}"
    );
    std::fs::write(
        w.user.settings.join("config.toml"),
        "[proxy]\nmode = 'none'\n",
    )
    .unwrap();
    let show = vibe(&w).args(["self", "apply", "--show"]).output().unwrap();
    let text = String::from_utf8_lossy(&show.stdout);
    assert!(
        text.contains("proxy mode: none (from user config)"),
        "{text}"
    );
    let target = format!("https://{FIXTURE_HOST}:{}/", w.server.port);
    let _ = vibe(&w)
        .args(["self", "doctor", "--network", &target])
        .output()
        .unwrap();
    assert!(
        started(&w.log).is_empty(),
        "the site's px ran: {:?}",
        started(&w.log)
    );
}
