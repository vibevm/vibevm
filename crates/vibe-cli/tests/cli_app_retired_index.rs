//! Retired /1 is inert, read-only until publication, and preserved without import.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::AppRegistry;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use vibe_app::{EventKind, HomeEvent, Index, SettingsRoot};

const BACKUP: &str = "index.retired-v1.json";

struct Fixture {
    registry: AppRegistry,
}
impl Fixture {
    fn new() -> Self {
        let registry = AppRegistry::new();
        fs::create_dir_all(registry.settings().join("applications")).unwrap();
        Self { registry }
    }
    fn path(&self) -> PathBuf {
        self.registry.settings().join("applications/index.json")
    }
    fn backup(&self) -> PathBuf {
        self.registry.settings().join("applications").join(BACKUP)
    }
    fn command(&self, image: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(image);
        command
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.registry.home())
            .env("USERPROFILE", self.registry.home())
            .env("VIBE_SETTINGS", self.registry.settings())
            .env("VIBE_REGISTRY_CACHE", &self.registry.user.cache)
            .env("VIBEVM_SEARCH_CACHE_DIR", &self.registry.user.search_cache)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("XDG_CONFIG_HOME", self.registry.home().join("config"))
            .env("XDG_CACHE_HOME", self.registry.home().join("cache"))
            .current_dir(self.registry.root.path());
        command
    }
    fn cli(&self, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_vibe"))
            .args(args)
            .output()
            .unwrap()
    }
    fn retired(&self) -> Vec<u8> {
        // Exact schema and ownership constraints from 53f1521a5 model/store.
        let host = self.registry.settings().join("opt/apps/retired");
        let value = json!({
            "protocol": "vibe-user-applications/1",
            "applications": {"retired": {
                "application": {"id": "retired", "package": {
                    "group": "org.example", "name": "old", "version": "1.0.5"
                }, "installerPackage": {
                    "group": "org.example", "name": "installer", "version": "1.0.7"
                }, "commands": ["retired-cmd"]},
                "hostRoot": host,
                "management": {"runtime": "node", "entry": host.join("manage.js")},
                "status": "ready"
            }}
        });
        let mut bytes = serde_json::to_vec_pretty(&value).unwrap();
        bytes.extend_from_slice(b" \n\n"); // Preserve bytes, not reserialized JSON.
        bytes
    }
    fn seed(&self, bytes: &[u8]) {
        fs::write(self.path(), bytes).unwrap();
    }
    fn list(&self) -> Value {
        let output = self.cli(&["app", "list", "-g", "--json"]);
        success(&output);
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn package(&self, name: &str) {
        let package = self.registry.package(name, "1.0.0", name, &format!(
            "[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='payload'\n\
             [[installer_options.launchers]]\nid='main'\nname='Main'\nexecutable='$app_bin/{name}'\nkind='command'\n\
             [[installer_options.public_commands]]\nname='{name}'\nlauncher='main'\n"
        ));
        self.registry.script(&package, name, "modern");
    }
    fn install(&self, name: &str) -> Output {
        self.cli(&[
            "install",
            "-g",
            &format!("org.example/{name}"),
            "--registry",
            self.registry.registry().to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ])
    }
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn directory(path: &std::path::Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    names
}

#[test]
fn valid_retired_list_is_empty_and_does_not_write() {
    let f = Fixture::new();
    let bytes = f.retired();
    f.seed(&bytes);
    let before = directory(&f.registry.settings().join("applications"));
    assert_eq!(f.list()["installations"], json!([]));
    assert_eq!(f.list()["installations"], json!([]));
    assert_eq!(fs::read(f.path()).unwrap(), bytes);
    assert_eq!(
        directory(&f.registry.settings().join("applications")),
        before
    );
    assert!(!f.backup().exists());
}

#[test]
fn ordinary_install_preserves_exact_bytes_and_repeat_write_keeps_backup() {
    let f = Fixture::new();
    let bytes = f.retired();
    f.seed(&bytes);
    f.package("first");
    f.package("second");
    success(&f.install("first"));
    assert_eq!(fs::read(f.backup()).unwrap(), bytes);
    let rows = f.list()["installations"].as_array().unwrap().clone();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["key"], "org.example/first@1.0.0");
    success(&f.install("second"));
    assert_eq!(fs::read(f.backup()).unwrap(), bytes);
    assert_eq!(f.list()["installations"].as_array().unwrap().len(), 2);
    assert!(
        !String::from_utf8(fs::read(f.path()).unwrap())
            .unwrap()
            .contains("retired")
    );
}

#[test]
fn backup_conflict_or_obstruction_refuses_before_replacing_source() {
    for obstruction in ["different", "same", "directory", "symlink"] {
        let f = Fixture::new();
        let bytes = f.retired();
        f.seed(&bytes);
        f.package("first");
        match obstruction {
            "directory" => fs::create_dir(f.backup()).unwrap(),
            "symlink" => std::os::unix::fs::symlink(f.path(), f.backup()).unwrap(),
            "same" => fs::write(f.backup(), &bytes).unwrap(),
            _ => fs::write(f.backup(), b"unrelated backup").unwrap(),
        }
        let output = f.install("first");
        assert!(
            !output.status.success(),
            "{obstruction} unexpectedly succeeded"
        );
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
        match obstruction {
            "directory" => assert!(directory(&f.backup()).is_empty()),
            "symlink" => assert_eq!(fs::read_link(f.backup()).unwrap(), f.path()),
            "same" => assert_eq!(fs::read(f.backup()).unwrap(), bytes),
            _ => assert_eq!(fs::read(f.backup()).unwrap(), b"unrelated backup"),
        }
    }
}

#[test]
fn corrupt_unknown_and_future_documents_never_become_empty_or_overwritten() {
    let f = Fixture::new();
    f.package("first");
    let original: Value = serde_json::from_slice(&f.retired()).unwrap();
    let mut variants = vec![b"{broken".to_vec()];
    for protocol in [
        "vibe-user-applications/2",
        "vibe-user-applications/999",
        "unknown",
    ] {
        let mut bad = original.clone();
        bad["protocol"] = protocol.into();
        variants.push(serde_json::to_vec(&bad).unwrap());
    }
    for pointer in [
        "/applications/retired/management/runtime",
        "/applications/retired/status",
    ] {
        let mut bad = original.clone();
        *bad.pointer_mut(pointer).unwrap() = "future".into();
        variants.push(serde_json::to_vec(&bad).unwrap());
    }
    for pointer in [
        "/applications/retired",
        "/applications/retired/application",
        "/applications/retired/application/package",
        "/applications/retired/management",
        "",
    ] {
        let mut bad = original.clone();
        bad.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("future".into(), true.into());
        variants.push(serde_json::to_vec(&bad).unwrap());
    }
    let mut missing = original.clone();
    missing["applications"]["retired"]
        .as_object_mut()
        .unwrap()
        .remove("management");
    variants.push(serde_json::to_vec(&missing).unwrap());
    for (pointer, value) in [
        ("/applications/retired/application/id", json!("mismatch")),
        ("/applications/retired/application/commands", json!([])),
        (
            "/applications/retired/application/commands",
            json!(["x", "x"]),
        ),
        (
            "/applications/retired/application/package/version",
            json!("invalid"),
        ),
        (
            "/applications/retired/management/entry",
            json!("/outside/manage.js"),
        ),
        ("/applications/retired/hostRoot", json!("/outside")),
    ] {
        let mut bad = original.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        variants.push(serde_json::to_vec(&bad).unwrap());
    }
    variants.push(
        b"{\"protocol\":\"vibe-user-applications/1\",\"applications\":{},\"applications\":{}}"
            .to_vec(),
    );
    let record = serde_json::to_string(&original["applications"]["retired"]).unwrap();
    variants.push(format!(
        "{{\"protocol\":\"vibe-user-applications/1\",\"applications\":{{\"retired\":{record},\"retired\":{record}}}}}"
    ).into_bytes());
    for bytes in variants {
        f.seed(&bytes);
        assert!(!f.cli(&["app", "list", "-g", "--json"]).status.success());
        assert!(!f.install("first").status.success());
        // Also cover the raw /3 writer, which previously skipped reader admission.
        let home = SettingsRoot::new(f.registry.settings());
        assert!(
            Index::default()
                .save(&home, &HomeEvent::new(EventKind::Updated))
                .is_err()
        );
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
        assert!(!f.backup().exists());
    }
}

#[test]
fn public_proxy_never_executes_retired_management_and_never_rewrites() {
    let f = Fixture::new();
    let bytes = f.retired();
    f.seed(&bytes);
    let host = f.registry.settings().join("opt/apps/retired");
    // AppRegistry is the sole executable-fixture creator; the marker would
    // be emitted if this script were ever run, directly or through Node.
    f.registry.script(&host, "manage.js", "RETIRED_EXECUTED");
    let marker = f.registry.root.path().join("retired-was-executed");
    let trap = format!(
        "#!/bin/sh\nprintf called > '{}'\necho RETIRED_EXECUTED\n",
        marker.display()
    );
    fs::write(host.join("payload/bin/manage.js"), &trap).unwrap();
    fs::copy(host.join("payload/bin/manage.js"), host.join("manage.js")).unwrap();
    // Trap even a legacy dispatcher that calls Node instead of the entry.
    f.registry.script(&host, "node", "RETIRED_EXECUTED");
    fs::write(host.join("payload/bin/node"), trap).unwrap();
    let proxy = f.registry.settings().join("opt/bin/retired-cmd");
    fs::create_dir_all(proxy.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_vibe-app-proxy"), &proxy).unwrap();
    let output = f
        .command(&proxy)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", host.join("payload/bin").display()),
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(127));
    assert!(!marker.exists(), "retired executable was run");
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("RETIRED_EXECUTED"));
    assert_eq!(fs::read(f.path()).unwrap(), bytes);
    assert!(!f.backup().exists());
}

#[test]
fn preservation_precedes_modern_staging_and_a_failed_write_keeps_both_copies() {
    let f = Fixture::new();
    let bytes = f.retired();
    f.seed(&bytes);
    // write_atomic cannot stage into a directory. The raw writer uses this
    // process's pid, so the obstruction is deterministic without child hooks.
    let staging = f
        .path()
        .parent()
        .unwrap()
        .join(format!(".index.json.{}.tmp", std::process::id()));
    fs::create_dir(&staging).unwrap();
    let home = SettingsRoot::new(f.registry.settings());
    let lock = home.lock().unwrap();
    assert!(
        Index::default()
            .save_locked(&home, &HomeEvent::new(EventKind::Updated), &lock)
            .is_err()
    );
    assert_eq!(fs::read(f.path()).unwrap(), bytes);
    assert_eq!(fs::read(f.backup()).unwrap(), bytes);
    assert!(directory(&staging).is_empty());
    // The interrupted transition cannot guess that an existing same-byte
    // backup is ours; it refuses and preserves both on retry as well.
    assert!(
        Index::default()
            .save_locked(&home, &HomeEvent::new(EventKind::Updated), &lock)
            .is_err()
    );
    assert_eq!(fs::read(f.path()).unwrap(), bytes);
    assert_eq!(fs::read(f.backup()).unwrap(), bytes);
}

#[test]
fn foreign_home_lock_cannot_preserve_or_replace_retired_index() {
    let f = Fixture::new();
    let bytes = f.retired();
    f.seed(&bytes);
    let other = tempfile::tempdir().unwrap();
    let lock = SettingsRoot::new(other.path()).lock().unwrap();
    assert!(
        Index::default()
            .save_locked(
                &SettingsRoot::new(f.registry.settings()),
                &HomeEvent::new(EventKind::Updated),
                &lock
            )
            .is_err()
    );
    assert_eq!(fs::read(f.path()).unwrap(), bytes);
    assert!(!f.backup().exists());
}
