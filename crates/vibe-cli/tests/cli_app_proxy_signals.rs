//! A direct SIGINT to the PATH proxy must not release a live payload lease.

#![cfg(target_os = "linux")]

use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

#[test]
fn proxy_sigint_keeps_lease_until_child_exits_with_status_or_signal() {
    let scratch = tempfile::tempdir().unwrap();
    let companion = PathBuf::from(env!("CARGO_BIN_EXE_vibe-app-proxy"));
    assert!(companion.is_file());
    let alias = scratch.path().join("lease-alias");
    fs::copy(&companion, &alias).unwrap();
    fs::set_permissions(&alias, fs::Permissions::from_mode(0o755)).unwrap();
    let sidecar = scratch.path().join("lease-alias.proxy.json");

    let target_source = scratch.path().join("waiting-child.rs");
    let target = scratch.path().join("waiting-child");
    fs::write(&target_source, CHILD_SOURCE).unwrap();
    let built = Command::new("rustc")
        .args([
            "--edition=2021",
            target_source.to_str().unwrap(),
            "-o",
            target.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "child compile: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let lease = scratch
        .path()
        .join("applications/leases")
        .join(format!("{}.lock", "a".repeat(64)));
    fs::create_dir_all(lease.parent().unwrap()).unwrap();
    for (mode, expected) in [("exit", 42), ("signal", 143)] {
        fs::write(
            &sidecar,
            serde_json::to_vec(&json!({
                "protocol": "vibe-app-proxy/1",
                "mode": "direct",
                "target": target,
                "args": [mode],
                "cwd": scratch.path(),
                "lease": lease,
            }))
            .unwrap(),
        )
        .unwrap();
        let started = scratch.path().join(format!("{mode}.started"));
        let release = scratch.path().join(format!("{mode}.release"));
        let mut proxy = Command::new(&alias)
            .args([&started, &release])
            .spawn()
            .unwrap();
        wait_for_marker(&started, &mut proxy);

        let delivered = Command::new("kill")
            .args(["-INT", &proxy.id().to_string()])
            .status()
            .unwrap();
        assert!(delivered.success());
        thread::sleep(Duration::from_millis(50));
        assert_eq!(
            proxy.try_wait().unwrap(),
            None,
            "proxy exited while child was live"
        );
        let probe = File::options().read(true).write(true).open(&lease).unwrap();
        assert!(
            matches!(probe.try_lock(), Err(std::fs::TryLockError::WouldBlock)),
            "exclusive lease became available before child exit"
        );

        if mode == "exit" {
            fs::write(&release, b"release").unwrap();
        } else {
            let child_pid = fs::read_to_string(&started).unwrap();
            let killed = Command::new("kill")
                .args(["-TERM", child_pid.trim()])
                .status()
                .unwrap();
            assert!(killed.success());
        }
        let status = wait_for_exit(&mut proxy, Duration::from_secs(5));
        assert_eq!(status.code(), Some(expected), "mode={mode}: {status}");
        assert!(
            probe.try_lock().is_ok(),
            "lease remained held after child exit"
        );
    }
}

fn wait_for_marker(marker: &Path, proxy: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() {
        if let Some(status) = proxy.try_wait().unwrap() {
            panic!("proxy exited before child marker: {status}");
        }
        if Instant::now() >= deadline {
            let _ = proxy.kill();
            panic!("child did not create marker {}", marker.display());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_exit(proxy: &mut Child, timeout: Duration) -> std::process::ExitStatus {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = proxy.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = proxy.kill();
            panic!("proxy failed to exit after child finished");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

const CHILD_SOURCE: &str = r#"
use std::{env, fs, thread, time::{Duration, Instant}};
fn main() {
    let mut args = env::args().skip(1);
    let mode = args.next().expect("mode");
    let marker = args.next().expect("marker");
    let release = args.next().expect("release");
    fs::write(&marker, std::process::id().to_string()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !std::path::Path::new(&release).exists() {
        if Instant::now() >= deadline { std::process::exit(99); }
        thread::sleep(Duration::from_millis(10));
    }
    if mode == "exit" { std::process::exit(42); }
    std::process::exit(98);
}
"#;
