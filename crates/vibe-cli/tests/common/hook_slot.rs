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
    /// A judged `slot:post-install` contribution whose `when` is false, so
    /// it must be planned, reported `skip` with the deciding condition, and
    /// never executed (PROP-065 `#conditions`, `#ordinary-plan`).
    JudgmentSkip,
    /// A judged contribution that will run and needs a command this host
    /// has not got, so the installation is refused up front
    /// (PROP-065 `#needs`).
    JudgmentNeed,
    /// A judged contribution that will run, for the trust gate of
    /// PROP-065 `#ordinary-trust` to be asked about.
    JudgmentTrust,
    /// A judged `slot:post-install` contribution gated on
    /// `agent_configured = true`, so the run's real agent posture decides
    /// it (PROP-065 `#ordinary-grammar`, PROP-064 `#selection`).
    JudgmentAgent,
    /// Two `slot:post-install` contributions: one that runs and writes to
    /// both streams, and one a condition skips — the shape the step
    /// evidence of PROP-065 `#ordinary-plan` is read back from.
    JudgmentEvidence,
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
        Fixture::JudgmentSkip => {
            r#"
[[extension]]
id = "judged"
point = "slot:post-install"
title = "Judged contribution"
handler = { kind = "script", base = "hooks/post" }
when = { command_present = "no-such-command-anywhere" }
"#
        }
        Fixture::JudgmentNeed => {
            r#"
[[extension]]
id = "judged"
point = "slot:post-install"
title = "Judged contribution"
handler = { kind = "script", base = "hooks/post" }
needs = ["command:no-such-command-anywhere"]
"#
        }
        Fixture::JudgmentTrust => {
            r#"
[[extension]]
id = "judged"
point = "slot:post-install"
title = "Judged contribution"
handler = { kind = "script", base = "hooks/post" }
"#
        }
        Fixture::JudgmentAgent => {
            r#"
[[extension]]
id = "judged"
point = "slot:post-install"
title = "Judged contribution"
handler = { kind = "script", base = "hooks/post" }
when = { agent_configured = true }
"#
        }
        Fixture::JudgmentEvidence => {
            r#"
[[extension]]
id = "noisy"
point = "slot:post-install"
title = "A contribution with something to say"
handler = { kind = "script", base = "hooks/noisy" }

[[extension]]
id = "quiet"
point = "slot:post-install"
title = "A contribution a condition decides against"
handler = { kind = "script", base = "hooks/post" }
when = { command_present = "no-such-command-anywhere" }
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
        "set -eu\nprintf 'pre\\n' >> hook-order.txt\nmkdir -p boot\nprintf 'generated before boot\\n' > boot/generated.md\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/post.sh"),
        "set -eu\ntest -f \"$VIBE_PROJECT_ROOT/vibe.lock\"\nprintf 'post\\n' >> hook-order.txt\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/noisy.sh"),
        "set -eu\nprintf 'NOISY-OUT\\n'\nprintf 'NOISY-ERR\\n' >&2\nprintf 'post\\n' >> hook-order.txt\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/noisy.ps1"),
        "Write-Output NOISY-OUT\n[Console]::Error.WriteLine('NOISY-ERR')\nAdd-Content -LiteralPath hook-order.txt -Value post\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/fail.sh"),
        "printf SOFT-STDOUT\nprintf SOFT-STDERR >&2\nexit 17\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/pre.ps1"),
        "Add-Content -LiteralPath hook-order.txt -Value pre\nNew-Item -ItemType Directory -Path boot -Force | Out-Null\nSet-Content -LiteralPath boot/generated.md -Value 'generated before boot'\n",
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

/// Publish the `JudgmentTrust` package again under another group, so the
/// trust gate can be asked about a group the base list covers
/// (`org.vibevm.*`) rather than only about `org.example`
/// (PROP-065 `#ordinary-trust`). Returns the pkgref to install.
pub fn judged_package_in_group(registry: &Path, group: &str) -> String {
    let package = registry.join(group).join("hooked").join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            r#"[package]
group = "{group}"
name = "hooked"
kind = "flow"
version = "0.1.0"

[[extension]]
id = "judged"
point = "slot:post-install"
title = "Judged contribution"
handler = {{ kind = "script", base = "hooks/post" }}
"#,
        ),
    )
    .unwrap();
    for (file, body) in [
        (
            "hooks/post.sh",
            "set -eu\nprintf 'post\\n' >> hook-order.txt\n",
        ),
        (
            "hooks/post.ps1",
            "Add-Content -LiteralPath hook-order.txt -Value post\n",
        ),
    ] {
        fs::write(package.join(file), body).unwrap();
    }
    format!("{group}/hooked@=0.1.0")
}

/// `vibe install <pkgref>` with no `--assume-yes`: the posture the trust
/// gate answers in, for any package of any group.
pub fn install_pkg_unconsented(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
    pkgref: &str,
) -> std::process::Output {
    no_agent_env(user.vibe())
        .arg("--json")
        .arg("install")
        .arg(pkgref)
        .arg("--registry")
        .arg(registry)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
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

/// Name an agent in this scratch home's `agents.toml`, which is where the
/// install boundary reads the run's agent posture from (PROP-064
/// `#selection`). `codex` is a built-in preset, so the file needs no
/// definition — and nothing is ever started: the condition asks whether an
/// agent is configured, not that one runs.
pub fn select_default_agent(user: &UserScratch) {
    fs::write(user.settings.join("agents.toml"), "default = \"codex\"\n").unwrap();
}

pub fn install(user: &UserScratch, project: &Path, registry: &Path) -> std::process::Output {
    no_agent_env(user.vibe())
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

/// The same install WITHOUT `--assume-yes`: `--json` already satisfies the
/// package-count confirmation, so this is the posture a programmatic client
/// has and the one the trust gate of `#ordinary-trust` answers in.
pub fn install_unconsented(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
) -> std::process::Output {
    no_agent_env(user.vibe())
        .arg("--json")
        .arg("install")
        .arg("org.example/hooked@=0.1.0")
        .arg("--registry")
        .arg(registry)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// Drop the external agent registration of PROP-064
/// `#external-registration` from the child's environment: a developer who
/// has `VIBE_AGENT_COMMAND` exported must not make an `agent_configured`
/// condition answer differently than it does in CI.
fn no_agent_env(mut command: assert_cmd::Command) -> assert_cmd::Command {
    for variable in ["VIBE_AGENT_COMMAND", "VIBE_AGENT_ARGS"] {
        command.env_remove(variable);
    }
    command
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
