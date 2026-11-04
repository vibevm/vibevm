//! Real installed/renamed PATH proxies consume both closed application index versions.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::{AppRegistry, ok};
use serde_json::Value;
use std::{
    fs,
    process::{Command, Output},
};
fn package(r: &AppRegistry, name: &str, command: &str, rich: bool) {
    let definition = if rich {
        "[[installer_options.configuration.features]]\nid='extra'\nname='Extra'\n"
    } else {
        ""
    };
    let root = r.package(
        name,
        "1.0.0",
        name,
        &format!(
            r#"
[[installer_options.steps]]
id='copy'
title='Copy executable'
kind='copy'
from='payload'
[[installer_options.launchers]]
id='main'
name='Command'
executable='$app_bin/{command}'
args=['fixed value']
working_directory='.'
kind='command'
[[installer_options.public_commands]]
name='{command}'
launcher='main'
{definition}
"#
        ),
    );
    r.script(&root, command, name);
}
fn launch(r: &AppRegistry, command: &str) -> Output {
    Command::new(r.settings().join("opt/bin").join(command))
        .args(["two words", "$literal"])
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap()
}
#[test]
fn installing_common_configuration_keeps_legacy_and_rich_path_commands_working() {
    let r = AppRegistry::new();
    package(&r, "legacy", "legacy-cmd", false);
    package(&r, "rich", "rich-cmd", true);
    ok(&r.app(
        &["install", "-g"],
        "org.example/legacy",
        &["--offline", "--unattended", "--json"],
    ));
    assert_eq!(r.index()["protocol"], "vibe-user-applications/3");
    let before = launch(&r, "legacy-cmd");
    assert!(
        before.status.success(),
        "{}",
        String::from_utf8_lossy(&before.stderr)
    );
    ok(&r.app(
        &["install", "-g"],
        "org.example/rich",
        &["--offline", "--unattended", "--json"],
    ));
    assert_eq!(r.index()["protocol"], "vibe-user-applications/4");
    let path = r.settings().join("applications/index.json");
    let bytes = fs::read(&path).unwrap();
    for (command, expected) in [("legacy-cmd", "legacy"), ("rich-cmd", "rich")] {
        let output = launch(&r, command);
        assert!(
            output.status.success(),
            "{command}: status={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).starts_with(expected));
    }
    assert_eq!(launch(&r, "legacy-cmd").stdout, before.stdout);
    assert_eq!(
        fs::read(path).unwrap(),
        bytes,
        "proxy must not migrate or rewrite the index"
    );
}
#[test]
fn actual_proxy_refuses_unknown_protocol_and_nested_fields_in_both_index_versions() {
    let r = AppRegistry::new();
    package(&r, "legacy", "legacy-cmd", false);
    package(&r, "rich", "rich-cmd", true);
    ok(&r.app(
        &["install", "-g"],
        "org.example/legacy",
        &["--offline", "--unattended", "--json"],
    ));
    let old = r.index();
    ok(&r.app(
        &["install", "-g"],
        "org.example/rich",
        &["--offline", "--unattended", "--json"],
    ));
    let new = r.index();
    let path = r.settings().join("applications/index.json");
    for original in [old, new.clone()] {
        let mut variants = Vec::new();
        let mut bad = original.clone();
        bad["protocol"] = "vibe-user-applications/999".into();
        variants.push(bad);
        let mut bad = original.clone();
        bad["future"] = true.into();
        variants.push(bad);
        let mut bad = original.clone();
        bad["installations"]["org.example/legacy@1.0.0"]["future"] = true.into();
        variants.push(bad);
        if original["protocol"] == "vibe-user-applications/4" {
            let mut bad = original;
            bad["installations"]["org.example/rich@1.0.0"]["configuration"]["evaluation"]["input"]
                ["intent"]["future"] = true.into();
            variants.push(bad);
        }
        for value in variants {
            let bytes = serde_json::to_vec_pretty(&value).unwrap();
            fs::write(&path, &bytes).unwrap();
            let output = launch(&r, "legacy-cmd");
            assert_eq!(
                output.status.code(),
                Some(127),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            assert!(output.stdout.is_empty());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }
    // Protocol3 must not admit4-only fields under a legacy tag.
    let mut forged: Value = new;
    forged["protocol"] = "vibe-user-applications/3".into();
    fs::write(&path, serde_json::to_vec(&forged).unwrap()).unwrap();
    assert_eq!(launch(&r, "legacy-cmd").status.code(), Some(127));
}

#[test]
fn original_image_reports_exact_reader_capabilities_without_changing_probe() {
    let image = std::path::Path::new(env!("CARGO_BIN_EXE_vibe-app-proxy"));
    for protocol in ["vibe-user-applications/3", "vibe-user-applications/4"] {
        let out = Command::new(image)
            .args(["--supports-index", protocol])
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(out.stdout, format!("{protocol}\n").as_bytes());
        assert!(out.stderr.is_empty());
    }
    let unknown = Command::new(image)
        .args(["--supports-index", "vibe-user-applications/999"])
        .output()
        .unwrap();
    assert!(!unknown.status.success());
    assert!(unknown.stdout.is_empty());
    let probe = Command::new(image).arg("--probe").output().unwrap();
    assert!(probe.status.success());
    assert_eq!(probe.stdout, b"vibe-app-proxy/2\n");
}
