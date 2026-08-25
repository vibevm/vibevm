//! Review-fix integration oracles for chained-clean envelopes and preparation failure state.

mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

fn project(user: &UserScratch, extension: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Review"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let path = project.path().join("vibe.toml");
    fs::write(
        &path,
        format!("{}{}", fs::read_to_string(&path).unwrap(), extension),
    )
    .unwrap();
    project
}

fn state(root: &Path) -> LifecycleState {
    toml::from_str(&fs::read_to_string(root.join(".vibe/lifecycle.toml")).unwrap()).unwrap()
}

fn outcome(bytes: &[u8]) -> LifecycleReport {
    let docs = serde_json::Deserializer::from_slice(bytes)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    serde_json::from_value(docs.last().unwrap().clone()).unwrap()
}

#[test]
fn chained_clean_handler_sees_clean_while_state_chain_stays_default_only() {
    let user = UserScratch::new();
    let project = project(
        &user,
        r#"
[[extension]]
id="chain-probe"
point="phase:build"
handler={kind="builtin",name="log"}
config={message="CHAIN-PROBE"}
"#,
    );
    user.vibe()
        .args(["build", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let before_fp = state(project.path())
        .execution
        .values()
        .next()
        .unwrap()
        .fingerprint
        .clone();
    let output = user
        .vibe()
        .args(["clean", "build", "--json", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        outcome(&output.stdout).chain,
        ["clean", "validate", "install", "generate", "build"]
    );
    let after = state(project.path());
    assert_eq!(
        after.run.chain,
        ["validate", "install", "generate", "build"]
    );
    assert_ne!(
        after.execution.values().next().unwrap().fingerprint,
        before_fp,
        "fingerprint observes the actual clean-prefixed handler envelope"
    );
}

#[test]
fn preparation_error_replaces_stale_success_with_fail_after_successful_prefix() {
    let user = UserScratch::new();
    let project = project(
        &user,
        r#"
[[extension]]
id="prefix-ok"
point="phase:build"
handler={kind="builtin",name="log"}
config={message="PREFIX"}
[[extension]]
id="prepare-fail"
point="phase:build"
handler={kind="builtin",name="log"}
config={message="FAIL"}
inputs=["probe.txt"]
"#,
    );
    fs::write(project.path().join("probe.txt"), "probe").unwrap();
    user.vibe()
        .args(["build", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let prior = state(project.path());
    assert_eq!(
        prior
            .execution
            .values()
            .filter(|row| row.status == ExecutionRecordStatus::Ok)
            .count(),
        2
    );
    let manifest = project.path().join("vibe.toml");
    let body = fs::read_to_string(&manifest)
        .unwrap()
        .replace("inputs=[\"probe.txt\"]", "inputs=[\"../secret\"]");
    fs::write(manifest, body).unwrap();
    let output = user
        .vibe()
        .args(["build", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("../secret"));
    let state = state(project.path());
    assert_eq!(
        state
            .execution
            .values()
            .filter(|row| row.status == ExecutionRecordStatus::Ok)
            .count(),
        1
    );
    let failed = state
        .execution
        .iter()
        .find(|(key, _)| key.ends_with("#prepare-fail"))
        .unwrap()
        .1;
    assert_eq!(failed.status, ExecutionRecordStatus::Fail);
    assert!(failed.fingerprint.starts_with("sha256:"));
    assert_ne!(failed.fingerprint, "sha256:error");
}
