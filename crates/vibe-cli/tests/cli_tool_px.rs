//! `vibe tool px exec|serve|status` through the real CLI
//! (`PROP-068#proxy-tool`) in an isolated settings home: the child sees
//! the proxy environment, reaches a loopback server through px, and its
//! exit code is vibe's; px is reaped after the child exits and after it
//! is killed; a route with no proxy refuses unless `--allow-direct`.
//!
//! px is the repository's fake px (`px_path`), which logs `start <pid>`
//! and every request it forwards, so "the child went through px" is read
//! from px's own log rather than inferred.

#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool");

mod common;
#[path = "../../vibe-net/tests/support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::UserScratch;

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

struct World {
    user: UserScratch,
    dir: tempfile::TempDir,
    log: PathBuf,
    /// A plain HTTP server the child fetches through px.
    port: u16,
    bin: PathBuf,
}

/// A loopback HTTP server that answers one line, and its port.
fn http_server(dir: &Path) -> (u16, std::process::Child) {
    let root = dir.join("www");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("hello.txt"), "through-px\n").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let child = std::process::Command::new("python3")
        .args([
            "-m",
            "http.server",
            &port.to_string(),
            "--bind",
            "127.0.0.1",
        ])
        .current_dir(&root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    (port, child)
}

/// A directory holding the little programs `exec` is asked to run, and
/// the `PATH` entry that makes them bare command names.
fn tools(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    bin
}

fn script(bin: &Path, name: &str, body: &str) {
    let path = bin.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&path, perms).unwrap();
}

fn world(proxy_table: &str) -> World {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("px.log");
    std::fs::write(
        user.settings.join("config.toml"),
        proxy_table
            .replace("{px}", &fake_px().display().to_string())
            .replace("{log}", &log.display().to_string()),
    )
    .unwrap();
    let (port, _server) = http_server(dir.path());
    // The server is a child of this test process; it is killed with the
    // temporary directory at the end of the test.
    std::mem::forget(_server);
    let bin = tools(dir.path());
    World {
        user,
        dir,
        log,
        port,
        bin,
    }
}

fn px_world() -> World {
    world(
        "[proxy]\nmode = 'px'\nupstream = 'proxy.example.corp:8080'\n\
         px_path = '{px}'\npx_args = ['--fake-log={log}']\nno_proxy = '.corp.example'\n",
    )
}

/// The environment `vibe(&world)` sets, for a test that needs a raw
/// `std::process::Command` (one whose output it reads while it runs).
fn env_of(world: &World) -> Vec<(String, String)> {
    let path = format!(
        "{}:{}",
        world.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    vec![
        ("HOME".to_owned(), world.dir.path().display().to_string()),
        ("PATH".to_owned(), path),
        (
            vibe_core::settings::SETTINGS_DIR_ENV.to_owned(),
            world.user.settings.display().to_string(),
        ),
    ]
}

fn vibe(world: &World) -> assert_cmd::Command {
    let mut command = world.user.vibe();
    let path = format!(
        "{}:{}",
        world.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    command
        .env("HOME", world.dir.path())
        .env("PATH", path)
        .env_remove("VIBEVM_PROXY_MODE")
        .env_remove("VIBEVM_PROXY_PAC")
        .env_remove("VIBEVM_PROXY_REVOCATION")
        .env_remove("HTTPS_PROXY")
        .env_remove("https_proxy")
        .env_remove("HTTP_PROXY")
        .env_remove("http_proxy")
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .env_remove("VIBEVM_USER_CONFIG");
    command
}

/// The whole point: one px for the invocation, the child sees its address
/// in every spelling, reaches a server through it, and px is gone after.
#[test]
fn the_child_reaches_a_server_through_px_and_px_is_reaped_after_it() {
    let w = px_world();
    script(
        &w.bin,
        "fetch",
        "echo \"HTTPS_PROXY=$HTTPS_PROXY\"\n\
         echo \"https_proxy=$https_proxy\"\n\
         echo \"HTTP_PROXY=$HTTP_PROXY\"\n\
         echo \"NO_PROXY=$NO_PROXY\"\n\
         echo \"extra=$MY_VAR npm=$npm_config_proxy\"\n\
         exec python3 -c \"import os,urllib.request;\\
p=os.environ['HTTP_PROXY'];\\
o=urllib.request.build_opener(urllib.request.ProxyHandler({'http':p}));\\
print(o.open(os.environ['TARGET']).read().decode().strip())\"",
    );
    let target = format!("http://vibe-fixture.test:{}/hello.txt", w.port);
    let output = vibe(&w)
        .env("TARGET", &target)
        .args([
            "tool",
            "px",
            "exec",
            "--env",
            "MY_VAR=set",
            "--proxy-env",
            "npm_config_proxy",
            "--",
            "fetch",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{stdout}{stderr}");

    let pids = started(&w.log);
    assert_eq!(pids.len(), 1, "one px per invocation: {pids:?}");
    let address = stdout
        .lines()
        .find_map(|line| line.strip_prefix("HTTPS_PROXY="))
        .expect("the child is told the proxy")
        .to_owned();
    assert!(address.starts_with("http://127.0.0.1:"), "{stdout}");
    // Every spelling the tools that need px disagree about.
    assert!(
        stdout.contains(&format!("https_proxy={address}")),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("HTTP_PROXY={address}")),
        "{stdout}"
    );
    // Loopback first, then the configured list.
    assert!(
        stdout.contains("NO_PROXY=localhost,127.0.0.1,::1,.corp.example"),
        "{stdout}"
    );
    // `--env` is literal; `--proxy-env` names a variable to fill.
    assert!(
        stdout.contains(&format!("extra=set npm={address}")),
        "{stdout}"
    );
    // The body came back through px, and px saw the request.
    assert!(stdout.contains("through-px"), "{stdout}");
    let requests = support::requests(&w.log);
    assert!(
        requests.iter().any(|line| line.contains("hello.txt")),
        "px forwarded the child's request: {requests:?}"
    );
    assert!(gone(pids[0]), "px {} is reaped", pids[0]);
}

/// The child's exit code is vibe's, and px is reaped whichever way the
/// child ended — returning a code, or dying on a signal.
#[test]
fn the_exit_code_is_the_childs_and_px_is_reaped_either_way() {
    let w = px_world();
    script(&w.bin, "refuse", "exit 7");
    let output = vibe(&w)
        .args(["tool", "px", "exec", "--", "refuse"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7), "the child's own code");
    let pids = started(&w.log);
    assert_eq!(pids.len(), 1, "{pids:?}");
    assert!(gone(pids[0]), "px is reaped after a failing child");

    let w = px_world();
    script(&w.bin, "die", "kill -9 $$");
    let output = vibe(&w)
        .args(["tool", "px", "exec", "--", "die"])
        .output()
        .unwrap();
    // A child killed by a signal has no code of its own.
    assert_eq!(output.status.code(), Some(1), "a killed child");
    let pids = started(&w.log);
    assert_eq!(pids.len(), 1, "{pids:?}");
    assert!(gone(pids[0]), "px is reaped after a killed child");
}

/// `explicit` lends the configured proxy URL and starts no px; a route
/// with no proxy refuses, and `--allow-direct` runs the child clean.
#[test]
fn an_explicit_route_is_lent_as_written_and_no_proxy_refuses() {
    let w = world("[proxy]\nhttps = 'http://proxy.example.corp:3128'\n");
    script(&w.bin, "show", "echo \"P=$HTTPS_PROXY\"");
    let output = vibe(&w)
        .args(["tool", "px", "exec", "--", "show"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "{stdout}");
    // Lent as the user wrote it: parsing a bare `host:port` URL adds a
    // path of `/`, and the child would then report a proxy nobody set.
    assert_eq!(
        stdout.trim(),
        "P=http://proxy.example.corp:3128",
        "{stdout}"
    );
    assert!(started(&w.log).is_empty(), "no px for an explicit route");

    let w = world("[init]\nlast_author = 'Me <me@example.test>'\n");
    script(&w.bin, "show", "echo \"P=[$HTTPS_PROXY]\"");
    let output = vibe(&w)
        .args(["tool", "px", "exec", "--", "show"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("configures no proxy"), "{stderr}");
    assert!(stderr.contains("--allow-direct"), "{stderr}");

    // With the flag the child runs, and does not inherit a stray proxy.
    let output = vibe(&w)
        .env("HTTPS_PROXY", "http://inherited.example.invalid:1")
        .args(["tool", "px", "exec", "--allow-direct", "--", "show"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("P=[]"), "{stdout}");
}

/// `exec` is transparent: only the child writes to stdout, so a wrapper
/// can be used in a pipeline (`pxpip freeze > requirements.txt`). What
/// vibe has to say about the route is said on stderr, and only with
/// `--verbose`.
#[test]
fn only_the_child_writes_to_stdout() {
    let w = px_world();
    script(&w.bin, "say", "echo only-this");
    let plain = vibe(&w)
        .args(["tool", "px", "exec", "--", "say"])
        .output()
        .unwrap();
    assert!(plain.status.success());
    assert_eq!(
        String::from_utf8_lossy(&plain.stdout),
        "only-this\n",
        "stdout is the child's alone"
    );
    assert!(
        String::from_utf8_lossy(&plain.stderr).trim().is_empty(),
        "and nothing is said unless asked: {}",
        String::from_utf8_lossy(&plain.stderr)
    );

    let verbose = vibe(&w)
        .args(["--verbose", "tool", "px", "exec", "--", "say"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&verbose.stdout),
        "only-this\n",
        "still only the child's"
    );
    let stderr = String::from_utf8_lossy(&verbose.stderr).into_owned();
    assert!(
        stderr.contains("through the vibe-managed px on 127.0.0.1:"),
        "{stderr}"
    );
}

/// A command that is not on PATH is a refusal naming it, not a px that
/// started for nothing.
#[test]
fn a_command_that_is_not_on_path_is_refused_by_name() {
    let w = px_world();
    let output = vibe(&w)
        .args(["tool", "px", "exec", "--", "no-such-tool-here"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(!output.status.success(), "{stderr}");
    assert_eq!(
        output.status.code(),
        Some(127),
        "the shell's own code for a command that is not there: {stderr}"
    );
    assert!(stderr.contains("no-such-tool-here"), "{stderr}");
    assert!(stderr.contains("is not on PATH"), "{stderr}");
    assert!(started(&w.log).is_empty(), "px did not start: {stderr}");

    // `--env` without a value says so instead of exporting nonsense.
    let output = vibe(&w)
        .args(["tool", "px", "exec", "--env", "JUSTNAME", "--", "true"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("is not `NAME=VALUE`"), "{stderr}");
}

/// `serve` prints the address and the resolved route, keeps px alive, and
/// ends with the stop the invocation was given — reaping px.
#[test]
fn serve_reports_the_address_and_stops_with_the_invocation() {
    let w = px_world();
    let stop = w.dir.path().join("stop");
    // The document is printed only after px accepts connections, so
    // reading it is how the test knows px is serving — waiting for px's
    // own start line would race the readiness poll.
    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("vibe"))
        .args([
            "--json",
            "--stop-file",
            stop.to_str().unwrap(),
            "tool",
            "px",
            "serve",
        ])
        .envs(env_of(&w))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut document = String::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !document.contains('}') && Instant::now() < deadline {
        let mut line = String::new();
        if std::io::BufRead::read_line(&mut stdout, &mut line).unwrap_or(0) == 0 {
            break;
        }
        document.push_str(&line);
    }
    assert!(
        document.contains("\"protocol\": \"vibe-px-serve/1\""),
        "{document}"
    );
    assert!(
        document.contains("\"url\": \"http://127.0.0.1:"),
        "{document}"
    );
    assert!(document.contains("proxy.example.corp:8080"), "{document}");
    let pids = started(&w.log);
    assert_eq!(pids.len(), 1, "one px while serving: {pids:?}");

    std::fs::write(&stop, "").unwrap();
    let status = {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match child.try_wait().unwrap() {
                Some(status) => break status,
                None if Instant::now() > deadline => {
                    let _ = child.kill();
                    panic!("`serve` did not end when the invocation was stopped");
                }
                None => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    };
    assert!(gone(pids[0]), "px is reaped when serve ends");
    // A stop is not a failure of the command that honoured it.
    assert!(status.code().is_some(), "serve ended on its own terms");
}

/// `--port` binds px where the caller asked, so a policy that allows one
/// loopback port can be satisfied.
#[test]
fn a_fixed_port_is_used_as_given() {
    let w = px_world();
    let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = free.local_addr().unwrap().port();
    drop(free);
    script(&w.bin, "show", "echo \"P=$HTTPS_PROXY\"");
    let output = vibe(&w)
        .args([
            "tool",
            "px",
            "exec",
            "--port",
            &port.to_string(),
            "--",
            "show",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "{stdout}");
    assert!(
        stdout.contains(&format!("P=http://127.0.0.1:{port}")),
        "{stdout}"
    );
}

/// `vibe tool px status` is the companion report under the name a user of
/// `vibe tool px` already has in hand.
#[test]
fn status_is_the_companion_report() {
    let w = px_world();
    let mine = vibe(&w)
        .args(["--json", "tool", "px", "status"])
        .output()
        .unwrap();
    let theirs = vibe(&w)
        .args(["--json", "self", "px", "status"])
        .output()
        .unwrap();
    assert!(
        mine.status.success(),
        "{}",
        String::from_utf8_lossy(&mine.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&mine.stdout),
        String::from_utf8_lossy(&theirs.stdout),
        "the same document"
    );
}
