//! Real startup composition of one fixed Deploy grant across CLI and MCP.
#![cfg(target_os = "linux")]

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
};
use vibe_wire::{behaviour::artifact_record::validate, generated::artifact_record::ArtifactRecord};

#[path = "cli_mcp_fixed_deploy/frame_cross_surface.rs"]
mod frame_cross_surface;

struct Fixture {
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("project")).unwrap();
        fs::create_dir_all(dir.path().join("home")).unwrap();
        fs::write(dir.path().join("project/vibe.toml"), manifest("")).unwrap();
        Self { dir }
    }
    fn root(&self) -> PathBuf {
        self.dir.path().join("project")
    }
    fn seed_artifact(&self) {
        let artifact = self.root().join("target/debug/helper.exe");
        fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        fs::write(&artifact, b"one isolated deploy effect").unwrap();
        let digest = format!("{:x}", Sha256::digest(fs::read(&artifact).unwrap()));
        let record: ArtifactRecord = serde_json::from_value(json!({
            "schema":1,"id":"helper.exe","kind":"executable","shape":"file",
            "path_absolute":artifact.to_string_lossy().replace('\\', "/"),
            "path_relative":{"root":"project","path":"target/debug/helper.exe"},
            "digest":{"algorithm":"sha256","value":digest},
            "producer":{"target":"helper","mechanism":"build:cargo",
                "provider":{"key":"org.vibevm/vibe#cargo"}},
            "freshness":{},"created_at":"2026-10-05T12:00:00Z",
            "verification":{"status":"verified","evidence":"isolated Deploy fixture"}
        }))
        .unwrap();
        validate(&record).unwrap();
        let records = self.root().join(".vibe/state/artifacts");
        fs::create_dir_all(&records).unwrap();
        fs::write(
            records.join("helper.exe.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", self.dir.path().join("home"))
            .env("USERPROFILE", self.dir.path().join("home"))
            .env("VIBE_SETTINGS", self.dir.path().join("settings"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .current_dir(self.dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
    fn machine(&self, mode: &str, ttl: bool) -> Output {
        let mut command = self.command();
        command
            .args(["machine", "session", "--root"])
            .arg(self.dir.path().join("project"))
            .args(["--fixed-deploy", mode, "--deploy-profile", "local"]);
        if ttl {
            command.args(["--ttl-ms", "60000"]);
        }
        command.output().unwrap()
    }
    fn mcp_command(&self, mode: &str, ttl: bool) -> Command {
        let mut command = self.command();
        command
            .args(["--offline", "mcp", "serve", "--path"])
            .arg(self.dir.path().join("project"))
            .args(["--fixed-deploy", mode, "--deploy-profile", "local"]);
        if ttl {
            command.args(["--ttl-ms", "60000"]);
        }
        command
    }
    fn mcp(&self, mode: &str, ttl: bool, request: &str) -> Output {
        let mut command = self.mcp_command(mode, ttl);
        let mut child = command.spawn().unwrap();
        std::io::Write::write_all(child.stdin.as_mut().unwrap(), request.as_bytes()).unwrap();
        drop(child.stdin.take());
        child.wait_with_output().unwrap()
    }
}

fn manifest(extra: &str) -> String {
    format!(
        concat!(
            "[project]\nname='demo'\nversion='1.1.0'\n\n",
            "[[artifacts.build]]\nid='helper'\nmechanism='build:cargo'\n",
            "outputs=[{{ id='helper.exe', kind='executable' }}]\n\n",
            "[[deploy.target]]\nid='local-helper'\nartifact='helper.exe'\n",
            "mechanism='deploy:vibe-bin'\ndepends_on=[]\n",
            "config={{ command='local-helper' }}\n\n",
            "[deploy.profiles.local]\ntargets=['local-helper']\n{}"
        ),
        extra
    )
}

fn request(kind: &str) -> String {
    request_key(kind, "old-key")
}

fn request_key(kind: &str, key: &str) -> String {
    json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"fixed_deploy_operation","arguments":{
            "schema":1,"action":{"kind":kind,"key":key}}}})
    .to_string()
        + "\n"
}

fn mcp_reply(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn mcp_issue_and_cold_reattach_retain_one_real_deploy_effect() {
    let fixture = Fixture::new();
    fixture.seed_artifact();
    let first = mcp_reply(&fixture.mcp("issue", true, &request("submit")));
    assert_eq!(first["result"]["isError"], false, "{first}");
    let completed = &first["result"]["structuredContent"]["reply"];
    assert_eq!(completed["kind"], "completed", "{first}");
    let run = completed["run_id"].as_str().unwrap();
    let receipt: Value = serde_json::from_str(completed["receipt_json"].as_str().unwrap()).unwrap();
    assert_eq!(receipt["status"], "verified");
    let launcher = fs::read(fixture.dir.path().join("settings/bin/local-helper")).unwrap();
    assert!(launcher.starts_with(b"#!/bin/sh\n# vibevm-launcher"));
    let pointer = fs::read_dir(fixture.root().join(".vibe/cli-deploy-host-grants"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let pointer_before = fs::read(&pointer).unwrap();

    let cold = mcp_reply(&fixture.mcp("reattach", false, &request("submit")));
    assert_eq!(cold["result"]["isError"], false, "{cold}");
    assert_eq!(cold["result"]["structuredContent"]["reply"], *completed);
    assert_eq!(fs::read(&pointer).unwrap(), pointer_before);

    fs::write(
        fixture.root().join("vibe.toml"),
        manifest("\n# drift after effect\n"),
    )
    .unwrap();
    let observed = mcp_reply(&fixture.mcp("reattach", false, &request("inspect")));
    assert_eq!(observed["result"]["isError"], false, "{observed}");
    assert_eq!(
        observed["result"]["structuredContent"]["reply"]["run_id"],
        run
    );
    let refused = mcp_reply(&fixture.mcp("reattach", false, &request("submit")));
    assert_eq!(refused["result"]["isError"], true, "{refused}");
    assert_eq!(refused["result"]["structuredContent"]["known_run_id"], run);
    assert_eq!(fs::read(&pointer).unwrap(), pointer_before);
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
    assert_eq!(
        fs::read(fixture.dir.path().join("settings/bin/local-helper")).unwrap(),
        launcher
    );
}

#[test]
fn machine_issue_mcp_reattach_and_drifted_recovery_only() {
    let fixture = Fixture::new();
    let issued = fixture.machine("issue", true);
    assert!(
        issued.status.success(),
        "{}",
        String::from_utf8_lossy(&issued.stderr)
    );
    assert!(issued.stdout.is_empty());
    let reattached = fixture.mcp("reattach", false, &request("inspect"));
    assert!(
        reattached.status.success(),
        "{}",
        String::from_utf8_lossy(&reattached.stderr)
    );
    let response: Value = serde_json::from_slice(&reattached.stdout).unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(
        response["result"]["structuredContent"]["reply"]["kind"],
        "absent"
    );

    fs::write(
        fixture.root().join("vibe.toml"),
        manifest("\n# changed after issue\n"),
    )
    .unwrap();
    let recovered = fixture.mcp("reattach", false, &request("inspect"));
    assert!(
        recovered.status.success(),
        "{}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    let response: Value = serde_json::from_slice(&recovered.stdout).unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(
        response["result"]["structuredContent"]["reply"]["kind"],
        "absent"
    );

    let denied = fixture.mcp("reattach", false, &request("submit"));
    assert!(
        denied.status.success(),
        "{}",
        String::from_utf8_lossy(&denied.stderr)
    );
    let response: Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert_eq!(response["result"]["isError"], true, "{response}");
    let changed_profile = manifest("")
        .replace(
            "[deploy.profiles.local]",
            "[[deploy.target]]\nid='other-helper'\nartifact='helper.exe'\nmechanism='deploy:vibe-bin'\nconfig={ command='other-helper' }\n\n[deploy.profiles.local]",
        )
        .replace("targets=['local-helper']", "targets=['other-helper']");
    fs::write(fixture.root().join("vibe.toml"), changed_profile).unwrap();
    let changed = fixture.mcp("reattach", false, &request("inspect"));
    assert!(
        changed.status.success(),
        "{}",
        String::from_utf8_lossy(&changed.stderr)
    );
    let response: Value = serde_json::from_slice(&changed.stdout).unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(
        response["result"]["structuredContent"]["reply"]["kind"],
        "absent"
    );
    let denied = fixture.mcp("reattach", false, &request("submit"));
    assert!(
        denied.status.success(),
        "{}",
        String::from_utf8_lossy(&denied.stderr)
    );
    let response: Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert_eq!(response["result"]["isError"], true, "{response}");
    fs::write(
        fixture.root().join("vibe.toml"),
        manifest("").replace("[deploy.profiles.local]", "[deploy.profiles.renamed]"),
    )
    .unwrap();
    let removed = fixture.mcp("reattach", false, &request("inspect"));
    assert!(
        removed.status.success(),
        "{}",
        String::from_utf8_lossy(&removed.stderr)
    );
    let response: Value = serde_json::from_slice(&removed.stdout).unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    fs::write(
        fixture.root().join("vibe.toml"),
        "[project]\nname='demo'\nversion='1.1.0'\n",
    )
    .unwrap();
    let removed_deploy = fixture.mcp("reattach", false, &request("inspect"));
    assert!(
        removed_deploy.status.success(),
        "{}",
        String::from_utf8_lossy(&removed_deploy.stderr)
    );
    let response: Value = serde_json::from_slice(&removed_deploy.stdout).unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(
        fs::read_dir(fixture.root().join(".vibe/cli-deploy-host-grants"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn fixed_deploy_startup_requires_explicit_profile_and_existing_pointer() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.dir.path().join("settings")).unwrap();
    let absent = fixture.mcp("reattach", false, "");
    assert!(!absent.status.success());
    assert!(String::from_utf8_lossy(&absent.stderr).contains("MissingPointer"));
    assert!(absent.stdout.is_empty());
    assert!(!fixture.root().join(".vibe/cli-deploy-host-grants").exists());

    let no_profile = fixture
        .command()
        .args([
            "machine",
            "session",
            "--root",
            fixture.root().to_str().unwrap(),
            "--fixed-deploy",
            "issue",
            "--ttl-ms",
            "60000",
        ])
        .output()
        .unwrap();
    assert!(!no_profile.status.success());
    assert!(String::from_utf8_lossy(&no_profile.stderr).contains("--deploy-profile"));
    assert!(no_profile.stdout.is_empty());
}
