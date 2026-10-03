//! Invalid migration authority must refuse before any fixed host issue.
mod common;

use common::UserScratch;
use std::fs;

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='startup-order'\nversion='1.1.0'\n",
    )
    .unwrap();
    fs::write(root.path().join("source.bin"), b"fixed package input").unwrap();
    fs::write(root.path().join("run.sh"), b"#!/bin/sh\nexit 0\n").unwrap();
    root
}

#[test]
fn malformed_migration_authority_precedes_all_fixed_grant_issues() {
    for (mode, extra) in [
        ("fixed_validate", vec!["--approve-captured-script"]),
        ("fixed_package", vec![]),
        ("fixed_cargo", vec![]),
        ("default", vec![]),
    ] {
        let user = UserScratch::new();
        let root = project();
        let mut command = user.vibe();
        command
            .args(["--offline", "mcp", "serve", "--path"])
            .arg(root.path())
            .args(["--migration-recovery-store", "relative-store"]);
        if mode != "default" {
            let flag = match mode {
                "fixed_validate" => "--fixed-validate",
                "fixed_package" => "--fixed-package",
                "fixed_cargo" => "--fixed-cargo",
                _ => unreachable!(),
            };
            command.args([flag, "issue", "--ttl-ms", "60000"]);
        }
        command.args(extra);
        let output = command.write_stdin("").output().unwrap();
        assert!(!output.status.success(), "{mode}");
        assert!(output.stdout.is_empty(), "{mode}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("MCP manifest migration authority refused"),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let state = root.path().join(".vibe");
        for path in [
            "machine-submissions.json",
            "cli-host-grants",
            "cli-package-host-grants",
            "machine-cargo-host-grants",
        ] {
            assert!(
                !state.join(path).exists(),
                "{mode} published {path} before migration refusal"
            );
        }
    }
}
