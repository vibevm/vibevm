//! Both px status names report local bytes without doing redundant work.

#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools");

mod common;

use common::UserScratch;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Output;

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn no_completion(output: &Output) {
    let out = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.trim().is_empty(), "unexpected progress: {err}");
    assert!(!out.contains('✻') && !out.contains("[done]"), "{out}");
    assert_eq!(
        out.lines().filter(|line| !line.starts_with(' ')).count(),
        1,
        "{out}"
    );
}

#[test]
fn px_status_aliases_render_one_block() {
    let user = UserScratch::new();
    let base = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let empty_path = tempfile::tempdir().unwrap();
    let cmd = || {
        let mut command = user.vibe();
        command
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .env("VIBE_SETTINGS", &user.settings)
            .env("VIBEVM_INSTALL_ROOT", base.path())
            .env("VIBEVM_PROXY_MODE", "none")
            .env("PATH", empty_path.path())
            .env("NO_COLOR", "1")
            .env("TERM", "xterm-256color")
            .env_remove("VIBEVM_HOME")
            .env_remove("VIBEVM_USER_CONFIG")
            .env_remove("VIBEVM_RELEASE_ORIGIN")
            .env_remove("VIBE_OFFLINE");
        command
    };
    let aliases = [["self", "px", "status"], ["tool", "px", "status"]];
    let pin = vibe_net::px_store::PxPin::embedded();
    let asset = vibe_net::px_store::running_target()
        .and_then(|target| pin.asset(target))
        .expect("pinned Linux platform asset");
    let asset_name = pin.release_asset_name(asset);
    let root = base.path().join("opt/vibevm/tools/px");
    let expected = format!(
        "○ px  not installed · px {} · {}",
        pin.version,
        root.display()
    );
    let mut missing = Vec::new();
    for alias in aliases {
        let output = cmd().args(alias).output().unwrap();
        let text = stdout(&output);
        no_completion(&output);
        assert!(text.starts_with(&expected), "{text}");
        assert!(text.contains(&format!("  asset  {asset_name}")), "{text}");
        assert!(
            text.lines().any(|line| line == "  vibe self px install"),
            "{text}"
        );
        assert!(
            text.contains("  vibe self px install --from <archive|directory|executable>"),
            "{text}"
        );
        assert!(!text.contains('`') && !text.contains('\x1b'), "{text}");
        missing.push(text);
    }
    assert_eq!(missing[0], missing[1]);
    assert!(!root.exists(), "status must not initialize the tool store");

    // Only synthetic local bytes: the absolute interpreter works with an empty PATH.
    let own = base.path().join("my-px");
    let bytes = b"#!/bin/sh\necho 'px fixture 9.9.9'\n";
    fs::write(&own, bytes).unwrap();
    fs::set_permissions(&own, fs::Permissions::from_mode(0o755)).unwrap();
    let digest = format!("{:x}", Sha256::digest(bytes));
    let output = cmd()
        .args(["--json", "self", "px", "install", "--from"])
        .arg(&own)
        .output()
        .unwrap();
    let installed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(installed["sha256"], digest);
    assert_eq!(installed["source"], own.display().to_string());
    let program = installed["installed"].as_str().unwrap();
    let mut reports = Vec::new();
    for alias in aliases {
        let output = cmd().args(alias).output().unwrap();
        let text = stdout(&output);
        no_completion(&output);
        assert!(
            text.starts_with(&format!("✓ px  installed · {program}")),
            "{text}"
        );
        for (label, value) in [
            ("reports", "px fixture 9.9.9"),
            ("read from", own.to_str().unwrap()),
            ("sha256", digest.as_str()),
        ] {
            assert!(
                text.lines().any(|line| {
                    line.trim_start()
                        .strip_prefix(label)
                        .is_some_and(|tail| tail.starts_with("   ") && tail.trim() == value)
                }),
                "missing {label} {value}: {text}"
            );
        }
        assert!(!text.to_lowercase().contains("verif"), "{text}");
        assert!(!text.contains("vibe self px install"), "{text}");
        reports.push(text);

        let output = cmd().arg("--quiet").args(alias).output().unwrap();
        assert_eq!(stdout(&output), reports.last().unwrap().as_str());
        no_completion(&output);
        let output = cmd().arg("--json").args(alias).output().unwrap();
        let doc: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
        assert_eq!(doc["ok"], true);
        assert_eq!(doc["in_store"], true);
        assert_eq!(doc["needed"], false);
        assert_eq!(doc["installed"], program);
        assert_eq!(doc["sha256"], digest);
        assert_eq!(doc["source"], own.display().to_string());
        assert_eq!(doc["px_version"], "px fixture 9.9.9");
        assert!(output.stderr.is_empty());
    }
    assert_eq!(reports[0], reports[1]);
}
