#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

#[test]
fn setup_reports_registry_parse_code() {
    let fixture = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    fs::write(
        fixture.path().join("agents.toml"),
        "[launchers.fake]\nkind = \"pi\"\ncommand = \"fake\"\n",
    )
    .unwrap();
    let run = |json: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", fixture.path())
            .env("VIBE_SETTINGS", fixture.path())
            .env("PATH", "/usr/bin:/bin")
            .env("NO_COLOR", "1")
            .args([
                "ai",
                "setup",
                "--from-agent-alias",
                "fake",
                "--mode",
                "inherit-default",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if json {
            command.arg("--json");
        }
        command.output().unwrap()
    };
    let json = run(true);
    assert!(!json.status.success());
    let error: serde_json::Value = serde_json::from_slice(&json.stderr).unwrap();
    assert_eq!(error["ok"], false);
    assert_eq!(error["code"], "agents_config_invalid");
    assert!(error["error"].as_str().unwrap().contains("agents.toml"));

    let human = run(false);
    assert!(!human.status.success());
    let stderr = String::from_utf8(human.stderr).unwrap();
    assert!(stderr.starts_with("✗ agents_config_invalid  "), "{stderr}");
}
