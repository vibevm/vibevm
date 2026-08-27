mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use vibe_wire::generated::lifecycle_report::LifecycleReport;

enum Failure {
    NonZero,
    Malformed,
}

fn script_project(failure: Failure) -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    fs::create_dir_all(project.path().join("scripts")).unwrap();
    let (shell, powershell) = match failure {
        Failure::NonZero => (
            "printf PHASE-OUT\nprintf PHASE-ERR >&2\nexit 29\n",
            "Write-Output PHASE-OUT\n[Console]::Error.Write('PHASE-ERR')\nexit 29\n",
        ),
        Failure::Malformed => (
            "printf MALFORMED-OUT\nprintf MALFORMED-ERR >&2\nprintf '{bad json' > \"$VIBE_REPLY\"\n",
            "Write-Output MALFORMED-OUT\n[Console]::Error.Write('MALFORMED-ERR')\n'{bad json' | Set-Content -LiteralPath $env:VIBE_REPLY -NoNewline\n",
        ),
    };
    fs::write(project.path().join("scripts/fail.sh"), shell).unwrap();
    fs::write(project.path().join("scripts/fail.ps1"), powershell).unwrap();
    append_extension(project.path(), "script", "scripts/fail");
    (user, project)
}

fn append_extension(project: &Path, kind: &str, coordinate: &str) {
    let manifest = project.join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    let handler = if kind == "script" {
        format!("{{ kind = \"script\", base = \"{coordinate}\" }}")
    } else {
        format!("{{ kind = \"binary\", name = \"{coordinate}\" }}")
    };
    text.push_str(&format!(
        "\n[[extension]]\nid='fatal'\npoint='phase:build'\nhandler={handler}\n"
    ));
    fs::write(manifest, text).unwrap();
}

fn binary_project() -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[package]\ngroup='org.fatal'\nname='root'\nkind='tool'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        r#"[package]
name="fatal-binary-fixture"
version="0.1.0"
edition="2024"
[[bin]]
name="runner"
path="src/main.rs"
"#,
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.rs"),
        "use std::io::Write;fn main(){eprint!(\"BINARY-DIAGNOSTIC\");std::io::stdout().write_all(b\"{bad json\").unwrap();}",
    )
    .unwrap();
    let manifest = project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[[binary]]\nname='runner'\ncrate='.'\n");
    fs::write(&manifest, text).unwrap();
    append_extension(project.path(), "binary", "runner");
    (user, project)
}

fn run_json(user: &UserScratch, project: &Path) -> std::process::Output {
    user.vibe()
        .args(["build", "--json", "--path"])
        .arg(project)
        .arg("--assume-yes")
        .output()
        .unwrap()
}

fn documents(output: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(output)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn failure_report(output: &std::process::Output) -> LifecycleReport {
    assert!(!output.status.success());
    let docs = documents(&output.stdout);
    assert!(
        !docs.is_empty(),
        "no structured output before terminal failure: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(docs[0]["command"], "lifecycle:plan");
    let report: LifecycleReport = serde_json::from_value(docs[1].clone()).unwrap();
    assert!(!report.ok);
    assert_eq!(report.contributions.len(), 1);
    assert_eq!(report.contributions[0].status, "fail");
    assert!(
        !output.stderr.is_empty(),
        "terminal error must follow outcome"
    );
    report
}

#[test]
fn phase_nonzero_emits_failure_outcome_before_terminal_error_and_quiet_is_clean() {
    let (user, project) = script_project(Failure::NonZero);
    let report = failure_report(&run_json(&user, project.path()));
    assert!(
        report.contributions[0]
            .stdout
            .as_deref()
            .unwrap()
            .contains("PHASE-OUT")
    );
    assert!(
        report.contributions[0]
            .stderr
            .as_deref()
            .unwrap()
            .contains("PHASE-ERR")
    );

    let (user, project) = script_project(Failure::NonZero);
    let output = user
        .vibe()
        .args(["build", "--quiet", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(combined.lines().count(), 1, "{combined}");
    assert!(!combined.contains("PHASE-OUT"));
    assert!(!combined.contains("PHASE-ERR"));
}

#[test]
fn malformed_script_reply_retains_post_spawn_stdout_and_stderr() {
    let (user, project) = script_project(Failure::Malformed);
    let report = failure_report(&run_json(&user, project.path()));
    assert!(
        report.contributions[0]
            .stdout
            .as_deref()
            .unwrap()
            .contains("MALFORMED-OUT")
    );
    assert!(
        report.contributions[0]
            .stderr
            .as_deref()
            .unwrap()
            .contains("MALFORMED-ERR")
    );
}

#[test]
fn malformed_binary_reply_retains_stderr_but_never_protocol_stdout() {
    let (user, project) = binary_project();
    let report = failure_report(&run_json(&user, project.path()));
    assert_eq!(report.contributions[0].handler, "binary");
    assert!(report.contributions[0].stdout.is_none());
    assert!(
        report.contributions[0]
            .stderr
            .as_deref()
            .is_some_and(|stderr| stderr.contains("BINARY-DIAGNOSTIC")),
        "{report:#?}"
    );
}

fn checkpoint_package(registry: &Path, name: &str, sabotage: bool) {
    let package = registry.join("org.checkpoint").join(name).join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.checkpoint'\nname='{name}'\nkind='tool'\nversion='0.1.0'\n\n[hooks]\npost-install='hooks/post'\n"
        ),
    )
    .unwrap();
    let shell = if sabotage {
        "rm -f \"$VIBE_PROJECT_ROOT/.vibe/lifecycle.toml\"\nmkdir \"$VIBE_PROJECT_ROOT/.vibe/lifecycle.toml\"\nprintf CHECKPOINT-DIAG >&2\nexit 17\n"
    } else {
        "printf later > \"$VIBE_PROJECT_ROOT/.vibe/later-checkpoint-post\"\n"
    };
    let powershell = if sabotage {
        "$state=Join-Path $env:VIBE_PROJECT_ROOT '.vibe/lifecycle.toml'\nRemove-Item -LiteralPath $state -Force\nNew-Item -ItemType Directory -Path $state | Out-Null\n[Console]::Error.Write('CHECKPOINT-DIAG')\nexit 17\n"
    } else {
        "Set-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/later-checkpoint-post') -Value later\n"
    };
    fs::write(package.join("hooks/post.sh"), shell).unwrap();
    fs::write(package.join("hooks/post.ps1"), powershell).unwrap();
}

#[test]
fn post_handler_plus_checkpoint_failure_is_hard_and_stops_later_targets() {
    let registry = tempfile::tempdir().unwrap();
    checkpoint_package(registry.path(), "a-sabotage", true);
    checkpoint_package(registry.path(), "z-later", false);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let output = user
        .vibe()
        .args([
            "install",
            "org.checkpoint/a-sabotage@=0.1.0",
            "org.checkpoint/z-later@=0.1.0",
            "--json",
            "--registry",
        ])
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(project.path().join(".vibe/lifecycle.toml").is_dir());
    assert!(!project.path().join(".vibe/later-checkpoint-post").exists());
    let docs = documents(&output.stdout);
    let plan = docs
        .iter()
        .position(|doc| doc["command"] == "lifecycle:plan")
        .unwrap();
    let failed = docs
        .iter()
        .position(|doc| {
            doc["command"] == "install"
                && doc["contributions"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["status"] == "fail"))
        })
        .unwrap();
    assert!(plan < failed);
    let row = &docs[failed]["contributions"][0];
    assert_eq!(row["status"], "fail");
    assert!(row.get("flagged").is_none());
    assert!(row["stderr"].as_str().unwrap().contains("CHECKPOINT-DIAG"));
    assert_eq!(docs.last().unwrap()["command"], "install");
    assert!(docs.iter().all(|doc| doc["command"] != "lifecycle"));
    assert!(!output.stderr.is_empty());
}
