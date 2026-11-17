//! Repeated automatic personal installs retain ordinary lock/transitive policy.
mod common;
use common::UserScratch;
use std::{fs, path::Path};
fn package(registry: &Path, name: &str, version: &str, tail: &str) {
    let root = registry.join(format!("org.auto/{name}/v{version}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.auto'\nname='{name}'\nkind='flow'\nversion='{version}'\n{tail}\n"
        ),
    )
    .unwrap();
}
#[test]
fn automatic_repeated_install_uses_personal_lock_and_declared_transitive_source() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    package(registry.path(), "first", "1.0.0", "");
    package(
        registry.path(),
        "second",
        "1.0.0",
        "[requires.packages]\n'org.auto/dependency'='*'",
    );
    package(registry.path(), "dependency", "1.0.0", "");
    user.vibe().args(["user", "init"]).assert().success();
    let personal = user.settings.join("user-project");
    let manifest = personal.join("vibe.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest).unwrap()
            + &format!(
                "\n[[registry]]\nname='personal'\nurl='file://{}'\n",
                registry.path().display()
            ),
    )
    .unwrap();
    for name in ["first", "second", "first"] {
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args(["install", "-g", &format!("org.auto/{name}"), "--registry"])
            .arg(registry.path())
            .args(["--offline", "--json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let lock = vibe_core::manifest::Lockfile::read(personal.join("vibe.lock")).unwrap();
    assert_eq!(lock.packages.len(), 3);
    assert!(
        lock.packages
            .iter()
            .any(|row| row.name.as_str() == "dependency")
    );
}

#[test]
fn automatic_preview_and_conflicting_source_leave_personal_manifest_and_lock_unchanged() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    package(first.path(), "first", "1.0.0", "");
    package(first.path(), "second", "1.0.0", "");
    package(
        second.path(),
        "first",
        "1.0.0",
        "description='other-source'",
    );
    user.vibe().args(["user", "init"]).assert().success();
    let personal = user.settings.join("user-project");
    user.vibe()
        .current_dir(cwd.path())
        .args(["install", "-g", "org.auto/first", "--registry"])
        .arg(first.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    let manifest = fs::read(personal.join("vibe.toml")).unwrap();
    let lock = fs::read(personal.join("vibe.lock")).unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args(["install", "-g", "org.auto/second", "--registry"])
        .arg(first.path())
        .args(["--offline", "--plan-only", "--json"])
        .assert()
        .success();
    assert_eq!(fs::read(personal.join("vibe.toml")).unwrap(), manifest);
    assert_eq!(fs::read(personal.join("vibe.lock")).unwrap(), lock);
    let conflict = user
        .vibe()
        .current_dir(cwd.path())
        .args(["install", "-g", "org.auto/first", "--registry"])
        .arg(second.path())
        .args(["--offline", "--json"])
        .output()
        .unwrap();
    assert!(!conflict.status.success());
    assert_eq!(fs::read(personal.join("vibe.toml")).unwrap(), manifest);
    assert_eq!(fs::read(personal.join("vibe.lock")).unwrap(), lock);
}

#[test]
fn stop_arriving_during_global_metadata_contact_exits130_without_personal_apply() {
    use std::io::{Read, Write};
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let stop = cwd.path().join("stop");
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    fs::write(
        user.settings.join("registry.toml"),
        format!("[[registry]]\nname='fixture'\nurl='{url}'\nindex_url='none'\n"),
    )
    .unwrap();
    user.vibe().args(["user", "init"]).assert().success();
    let personal = user.settings.join("user-project");
    let before = fs::read(personal.join("vibe.toml")).unwrap();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let finished = done.clone();
    let control = stop.clone();
    let worker = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !finished.load(std::sync::atomic::Ordering::SeqCst)
            && std::time::Instant::now() < deadline
        {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    socket
                        .set_read_timeout(Some(std::time::Duration::from_millis(200)))
                        .unwrap();
                    let mut bytes = [0; 4096];
                    let _ = socket.read(&mut bytes);
                    fs::write(&control, "requested after metadata contact").unwrap();
                    let body = b"[]";
                    let _ = write!(
                        socket,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = socket.write_all(body);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5))
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    let mut cmd = user.vibe();
    cmd.current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "org.auto/remote",
            "--no-default-registry",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--json");
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        cmd.env_remove(key);
    }
    let result = cmd
        .timeout(std::time::Duration::from_secs(20))
        .output()
        .unwrap();
    done.store(true, std::sync::atomic::Ordering::SeqCst);
    worker.join().unwrap();
    assert!(
        stop.exists(),
        "fixture never observed metadata contact: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        result.status.code(),
        Some(130),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read(personal.join("vibe.toml")).unwrap(), before);
    assert!(!personal.join("vibe.lock").exists());
}

#[test]
fn inferred_legacy_remap_stop_remains_unsupported_before_offer_mutation() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    let root = registry.path().join("org.auto/offer/v1.0.0");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("vibe.toml"),"[package]\ngroup='org.auto'\nname='offer'\nkind='remap'\nversion='1.0.0'\n[remap]\ndescription='fixture'\n[[remap.rows]]\nprefix='https://example.invalid/source'\nurl='https://example.invalid/target'\nreason='fixture'\n").unwrap();
    let stop = cwd.path().join("not-yet-requested");
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["install", "-g", "org.auto/offer", "--registry"])
        .arg(registry.path())
        .arg("--stop-file")
        .arg(stop)
        .args(["--offline", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not supported for global remap"));
    assert!(!user.settings.join("remap").exists());
}
