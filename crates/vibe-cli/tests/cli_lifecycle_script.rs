mod common;

use std::fs;

use common::UserScratch;
use vibe_wire::generated::lifecycle_report::LifecycleReport;

fn project() -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Script"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/run.sh"),
        r#"mkdir -p .vibe
n=0
test ! -f .vibe/script-count || n=$(cat .vibe/script-count)
echo $((n + 1)) > .vibe/script-count
echo SCRIPT-STDOUT
printf '%s' '{"artifacts":[],"envelope":1,"status":"ok","tasks":[]}' > "$VIBE_REPLY"
"#,
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/run.ps1"),
        r#"New-Item -ItemType Directory -Force .vibe | Out-Null
$n = 0; if (Test-Path .vibe/script-count) { $n = [int](Get-Content .vibe/script-count) }
($n + 1) | Set-Content .vibe/script-count
Write-Output 'SCRIPT-STDOUT'
'{"artifacts":[],"envelope":1,"status":"ok","tasks":[]}' | Set-Content -NoNewline $env:VIBE_REPLY
"#,
    )
    .unwrap();
    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"
[[extension]]
id = "phase-script"
point = "phase:build"
handler = { kind = "script", base = "scripts/run" }
inputs = ["scripts/**"]
"#,
    );
    fs::write(manifest, body).unwrap();
    (user, project)
}

fn documents(bytes: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn script_json_is_structured_then_fresh_and_force_runs_again() {
    let (user, project) = project();
    for (force, expected) in [(false, "ok"), (false, "fresh"), (true, "ok")] {
        let mut command = user.vibe();
        command
            .args(["build", "--json", "--path"])
            .arg(project.path())
            .arg("--assume-yes");
        if force {
            command.arg("--force");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let docs = documents(&output.stdout);
        let report: LifecycleReport = serde_json::from_value(docs.last().unwrap().clone()).unwrap();
        assert_eq!(report.contributions[0].status, expected);
        if expected == "ok" {
            assert!(
                report.contributions[0]
                    .stdout
                    .as_deref()
                    .unwrap_or("")
                    .contains("SCRIPT-STDOUT")
            );
        } else {
            assert!(report.contributions[0].stdout.is_none());
        }
    }
    assert_eq!(
        fs::read_to_string(project.path().join(".vibe/script-count"))
            .unwrap()
            .trim(),
        "2"
    );
}

#[test]
fn script_quiet_is_one_line_without_stream_contamination() {
    let (user, project) = project();
    let output = user
        .vibe()
        .args(["build", "--quiet", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(!stdout.contains("SCRIPT-STDOUT"), "{stdout}");
}
