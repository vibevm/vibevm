//! Public authored-source decisions and managed-contact controls, using only owned loopback.
mod common;
use common::UserScratch;
use serde_json::Value;
use std::{fs, path::Path};
#[path = "source_policy/server.rs"]
mod server;
use server::Server;

fn quiet_user() -> UserScratch {
    let user = UserScratch::new();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    user
}
fn package(root: &Path, version: &str, marker: &str) {
    let dir = root.join(format!("org.policy/demo/v{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("vibe.toml"),format!("[package]\ngroup='org.policy'\nname='demo'\nversion='{version}'\nkind='tool'\ndescription='{marker}'\n")).unwrap();
}
fn run_install(user: &UserScratch, project: &Path) -> std::process::Output {
    user.vibe()
        .current_dir(project)
        .env("HOME", &user.settings)
        .args([
            "install",
            "--json",
            "--assume-yes",
            "--no-default-registry",
            "--path",
        ])
        .arg(project)
        .output()
        .unwrap()
}
fn success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn authored_pin_selects_named_registry_and_held_alias_survives_catalog_removal() {
    let user = quiet_user();
    let project = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    package(first.path(), "2.0.0", "first");
    package(second.path(), "1.0.0", "second");
    let manifest = format!(
        "[project]\nname='policy'\nversion='1.0.0'\n[requires.packages]\n'org.policy/demo'={{version='*',registry='second'}}\n[[registry]]\nname='first'\nurl='file://{}'\n[[registry]]\nname='second'\nurl='file://{}'\n",
        first.path().display(),
        second.path().display()
    );
    fs::write(project.path().join("vibe.toml"), &manifest).unwrap();
    success(&run_install(&user, project.path()));
    let before = fs::read(project.path().join("vibe.lock")).unwrap();
    let lock: toml::Value = toml::from_str(std::str::from_utf8(&before).unwrap()).unwrap();
    let rows = lock["package"].as_array().unwrap();
    assert_eq!(rows[0]["registry"].as_str(), Some("second"));
    assert_eq!(rows[0]["version"].as_str(), Some("1.0.0"));
    fs::write(
        project.path().join("vibe.toml"),
        manifest.split("[[registry]]").next().unwrap(),
    )
    .unwrap();
    success(&run_install(&user, project.path()));
    assert_eq!(fs::read(project.path().join("vibe.lock")).unwrap(), before);
}
#[test]
fn unknown_pin_is_typed_and_never_falls_back() {
    let user = quiet_user();
    let project = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    package(registry.path(), "1.0.0", "available");
    fs::write(project.path().join("vibe.toml"),format!("[project]\nname='policy'\nversion='1.0.0'\n[requires.packages]\n'org.policy/demo'={{version='*',registry='missing'}}\n[[registry]]\nname='available'\nurl='file://{}'\n",registry.path().display())).unwrap();
    let output = run_install(&user, project.path());
    assert!(!output.status.success());
    let docs: Vec<Value> = serde_json::Deserializer::from_slice(&output.stderr)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        docs.iter()
            .any(|v| v["code"] == "source_policy_unknown_registry"),
        "{docs:?}"
    );
    assert!(!project.path().join("vibe.lock").exists());
}
#[test]
fn full_scan_exclusion_reports_unknown_and_sends_zero_tcp_connections() {
    let server = Server::new("127.0.0.1", b"[]");
    let user = quiet_user();
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("vibe.toml"),"[project]\nname='search'\nversion='1.0.0'\n[[registry]]\nname='org'\nurl='https://github.com/fixture'\nindex_url='none'\n").unwrap();
    for (denied, expected) in [(false, 1), (true, 1)] {
        let mut command = user.vibe();
        command
            .current_dir(project.path())
            .env("HOME", &user.settings)
            .args([
                "search",
                "package",
                "--full-scan",
                "--no-cache",
                "--json",
                "--path",
            ])
            .arg(project.path())
            .env("VIBEVM_GITHUB_API_BASE", &server.url);
        // Both runs are controlled: positive denies a distinct loopback host.
        command.args([
            "--exclude-hosting",
            if denied {
                "host:127.0.0.1"
            } else {
                "host:127.0.0.2"
            },
        ]);
        let output = command.output().unwrap();
        success(&output);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(server.count(), expected);
        if denied {
            assert_eq!(
                report["registries_policy_skipped"][0]["observation"],
                "unknown"
            );
            assert_eq!(report["registries_unreachable"], serde_json::json!([]));
        } else {
            assert_eq!(
                report["registries_full_scanned"],
                serde_json::json!(["org"])
            );
        }
    }
}
#[cfg(target_os = "linux")]
#[test]
fn global_app_payload_obeys_exclusion_and_an_admitted_remap_still_works() {
    use common::app_registry::AppRegistry;
    use sha2::{Digest, Sha256};
    let bytes = b"#!/bin/sh\necho admitted\n";
    let denied = Server::new("127.0.0.1", bytes);
    let allowed = Server::new("127.0.0.2", bytes);
    for mirror in [false, true] {
        let apps = AppRegistry::new();
        fs::write(
            apps.settings().join("registry.toml"),
            "[source_policy]\nexclude_hosting=['host:127.0.0.1']\n",
        )
        .unwrap();
        let option = format!(
            "[installer_options.installation]\ntype='source_build'\n[[installer_options.installation.sources]]\nname='payload'\nurl='{}/payload.txt'\nsha256='{:x}'\n[installer_options.installation.recipe]\ninterpreter='posix_sh'\nscript='build.sh'\n",
            denied.url,
            Sha256::digest(bytes)
        );
        let dir = apps.package("payload", "1.0.0", "payload", &option);
        fs::write(dir.join("build.sh"),"set -e\nmkdir -p \"$VIBE_OUT/bin\"\ncp \"$VIBE_SOURCES/payload/payload.txt\" \"$VIBE_OUT/bin/payload\"\nchmod 755 \"$VIBE_OUT/bin/payload\"\n").unwrap();
        if mirror {
            fs::write(
                apps.settings().join("settings.toml"),
                format!(
                    "[[remap]]\nprefix='{}/'\nurl='{}/'\n",
                    denied.url, allowed.url
                ),
            )
            .unwrap();
        }
        let output = apps.app(
            &["install", "-g"],
            "org.example/payload",
            &["--assume-yes", "--json"],
        );
        if mirror {
            success(&output);
            assert_eq!(allowed.count(), 1);
        } else {
            assert!(!output.status.success());
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(text.contains("source_host_excluded"), "{text}");
            assert_eq!(allowed.count(), 0);
        }
        assert_eq!(denied.count(), 0);
    }
}

#[cfg(unix)]
#[path = "source_policy/hosting.rs"]
mod hosting;

#[test]
fn generic_app_metadata_honors_explicit_project_policy_before_contact() {
    let server = Server::new("127.0.0.1", b"{}");
    let user = quiet_user();
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("vibe.toml"), "[project]\nname='scoped'\nversion='1.0.0'\n[source_policy]\nexclude_hosting=['host:127.0.0.1']\n").unwrap();
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='fixture'\nurl='{}/org'\nindex_url='{}/index'\n",
            server.url, server.url
        ),
    )
    .unwrap();
    let output = user
        .vibe()
        .current_dir(project.path())
        .env("HOME", &user.settings)
        .args(["app", "--path"])
        .arg(project.path())
        .args(["installers", "org.example/demo", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(server.count(), 0);
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(rendered.contains("source_host_excluded"), "{rendered}");
}
