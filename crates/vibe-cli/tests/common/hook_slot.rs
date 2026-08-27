//! Shared fixture for the `[hooks]`-sugar / explicit-slot install e2e.
//!
//! One published `org.example/hooked` package whose contribution shape is
//! chosen by [`Fixture`], plus the readers every case in the family needs: the
//! JSON document stream, the one registered install root, the slot-outcome
//! rows, and the durable lifecycle state.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

use super::UserScratch;

#[derive(Clone, Copy)]
pub enum Fixture {
    Timing,
    PreFail,
    PostFail,
}

pub fn registry(root: &Path, fixture: Fixture) -> PathBuf {
    let package = root
        .join("registry")
        .join("org.example")
        .join("hooked")
        .join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::create_dir_all(package.join("boot")).unwrap();
    let hooks = match fixture {
        Fixture::Timing => {
            r#"
[hooks]
pre-install = "hooks/pre"
post-install = "hooks/post"
"#
        }
        Fixture::PreFail => {
            r#"
[hooks]
pre-install = "hooks/fail"
"#
        }
        Fixture::PostFail => {
            r#"
[hooks]
post-install = "hooks/fail"
"#
        }
    };
    fs::write(
        package.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.example"
name = "hooked"
kind = "flow"
version = "0.1.0"

[boot_snippet]
source = "boot/generated.md"
category = "flow"
{hooks}"#,
        ),
    )
    .unwrap();
    fs::write(
        package.join("hooks/pre.sh"),
        "set -eu\nprintf 'pre\\n' >> hook-order.txt\nprintf 'generated before boot\\n' > boot/generated.md\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/post.sh"),
        "set -eu\ntest -f \"$VIBE_PROJECT_ROOT/vibe.lock\"\nprintf 'post\\n' >> hook-order.txt\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/fail.sh"),
        "printf SOFT-STDOUT\nprintf SOFT-STDERR >&2\nexit 17\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/pre.ps1"),
        "Add-Content -LiteralPath hook-order.txt -Value pre\nSet-Content -LiteralPath boot/generated.md -Value 'generated before boot'\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/post.ps1"),
        "$lock = Join-Path $env:VIBE_PROJECT_ROOT 'vibe.lock'\nif (-not (Test-Path -LiteralPath $lock)) { exit 19 }\nAdd-Content -LiteralPath hook-order.txt -Value post\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/fail.ps1"),
        "Write-Output SOFT-STDOUT\n[Console]::Error.Write('SOFT-STDERR')\nexit 17\n",
    )
    .unwrap();
    root.join("registry")
}

pub fn plain_package(registry: &Path, name: &str) {
    let package = registry.join("org.multi").join(name).join("v0.1.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!("[package]\ngroup='org.multi'\nname='{name}'\nkind='tool'\nversion='0.1.0'\n"),
    )
    .unwrap();
}

pub fn post_package(registry: &Path, name: &str, fail: bool) {
    let package = registry.join("org.continue").join(name).join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.continue'\nname='{name}'\nkind='tool'\nversion='0.1.0'\n\n[hooks]\npost-install='hooks/post'\n"
        ),
    )
    .unwrap();
    let shell = if fail {
        "printf FIRST-FAIL >&2\nexit 19\n"
    } else {
        "printf later > \"$VIBE_PROJECT_ROOT/.vibe/later-post-ran\"\n"
    };
    let powershell = if fail {
        "[Console]::Error.Write('FIRST-FAIL')\nexit 19\n"
    } else {
        "Set-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/later-post-ran') -Value later -NoNewline\n"
    };
    fs::write(package.join("hooks/post.sh"), shell).unwrap();
    fs::write(package.join("hooks/post.ps1"), powershell).unwrap();
}

pub fn install(user: &UserScratch, project: &Path, registry: &Path) -> std::process::Output {
    user.vibe()
        .arg("--json")
        .arg("install")
        .arg("org.example/hooked@=0.1.0")
        .arg("--registry")
        .arg(registry)
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .output()
        .unwrap()
}

pub fn documents(output: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(output)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// The sole `cli-install-report` this invocation emitted.
///
/// `vibe install` is the OUTERMOST command on these paths, so it — and only it
/// — emits a root document. The per-row `"command": "lifecycle"` echoes these
/// tests used to read were removed deliberately; the same rows now travel as
/// the report's typed `contributions`, streams and all.
pub fn install_report(docs: &[serde_json::Value]) -> &serde_json::Value {
    // Which root it is depends on which command was outermost: `vibe install`
    // emits `cli-install-report`, a phase verb emits `cli-lifecycle-report`.
    // Exactly one of them exists either way.
    let roots: Vec<&serde_json::Value> = docs
        .iter()
        .filter(|doc| doc["command"] == "install" || doc["command"] == "lifecycle")
        .collect();
    assert_eq!(roots.len(), 1, "exactly one root report: {docs:#?}");
    roots[0]
}

pub fn slot_outcomes(docs: &[serde_json::Value]) -> Vec<serde_json::Value> {
    install_report(docs)["contributions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| {
            row["point"]
                .as_str()
                .is_some_and(|point| point.starts_with("slot:"))
        })
        .cloned()
        .collect()
}

pub fn state_key(id: &str) -> String {
    format!("org.example/hooked#{id}@slot(org.example/hooked@0.1.0)")
}

pub fn lifecycle_state(project: &Path) -> LifecycleState {
    toml::from_str(&fs::read_to_string(project.join(".vibe/lifecycle.toml")).unwrap()).unwrap()
}

pub fn setup(fixture: Fixture) -> (tempfile::TempDir, UserScratch, tempfile::TempDir, PathBuf) {
    let outer = tempfile::tempdir().unwrap();
    let registry = registry(outer.path(), fixture);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    (outer, user, project, registry)
}
