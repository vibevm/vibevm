//! Real startup composition of one fixed Deploy grant across CLI and MCP.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

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
    fn mcp(&self, mode: &str, ttl: bool, request: &str) -> Output {
        let mut command = self.command();
        command
            .args(["--offline", "mcp", "serve", "--path"])
            .arg(self.dir.path().join("project"))
            .args(["--fixed-deploy", mode, "--deploy-profile", "local"]);
        if ttl {
            command.args(["--ttl-ms", "60000"]);
        }
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
    json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"fixed_deploy_operation","arguments":{
            "schema":1,"action":{"kind":kind,"key":"old-key"}}}})
    .to_string()
        + "\n"
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
