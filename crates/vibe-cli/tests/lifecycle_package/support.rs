use std::fs;
use std::path::Path;

use serde_json::Value;
use vibe_wire::generated::lifecycle_plan::LifecyclePlan;
use vibe_wire::generated::lifecycle_report::LifecycleReport;

use crate::common::UserScratch;

pub const SKILL_KEY: &str = "@vibe/package/skill/org.example/lifecycle-skills/demo";
pub const SWEEP_KEY: &str = "@vibe/package/skill/reconcile";
pub const RECOVER_KEY: &str = "@vibe/package/skill/recover";
pub const AFTER_KEY: &str = "org.example/lifecycle-skills#after-skill";

pub fn project(with_skill: bool, agents: &[&str], with_after: bool) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let mut manifest = r#"[package]
group = "org.example"
name = "lifecycle-skills"
kind = "tool"
version = "0.1.0"
authors = ["Fixture"]
license = "EULA"
description = "fixture"
keywords = ["fixture"]
"#
    .to_string();
    if with_skill {
        let agents = agents
            .iter()
            .map(|agent| format!("\"{agent}\""))
            .collect::<Vec<_>>()
            .join(", ");
        manifest.push_str(&format!(
            r#"
[[skill]]
name = "demo"
path = "skills/demo"
include = ["SKILL.md", "references/**"]
agents = [{agents}]
"#,
        ));
        let source = project.path().join("skills/demo");
        fs::create_dir_all(source.join("references")).unwrap();
        fs::write(source.join("SKILL.md"), "first\n").unwrap();
        fs::write(source.join("references/guide.md"), "guide\n").unwrap();
        fs::write(source.join("noise.txt"), "excluded\n").unwrap();
    }
    if with_after {
        manifest.push_str(
            r#"
[[extension]]
id = "after-skill"
point = "phase:package"
handler = { kind = "builtin", name = "log" }
config = { message = "AFTER-SKILL" }
"#,
        );
    }
    fs::write(project.path().join("vibe.toml"), manifest).unwrap();
    project
}

pub fn run_json(
    user: &UserScratch,
    project: &Path,
    fake_home: &Path,
) -> (LifecyclePlan, LifecycleReport) {
    let output = command(user, project, fake_home, true).output().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let documents = json_documents(&output.stdout);
    assert_eq!(
        documents
            .iter()
            .map(|document| document["command"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["lifecycle:plan", "lifecycle"]
    );
    (
        serde_json::from_value(documents[0].clone()).unwrap(),
        serde_json::from_value(documents[1].clone()).unwrap(),
    )
}

pub fn command(
    user: &UserScratch,
    project: &Path,
    fake_home: &Path,
    json: bool,
) -> assert_cmd::Command {
    let mut command = user.vibe();
    command.arg("package");
    if json {
        command.arg("--json");
    }
    command
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .env("HOME", fake_home)
        .env("USERPROFILE", fake_home);
    command
}

pub fn json_documents(bytes: &[u8]) -> Vec<Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

pub fn report_status<'a>(report: &'a LifecycleReport, key: &str) -> &'a str {
    report
        .contributions
        .iter()
        .find(|row| row.key == key)
        .unwrap_or_else(|| panic!("missing lifecycle row `{key}`: {report:?}"))
        .status
        .as_str()
}

pub fn lifecycle_state(project: &Path) -> vibe_wire::generated::lifecycle_state::LifecycleState {
    toml::from_str(&fs::read_to_string(project.join(".vibe/lifecycle.toml")).unwrap()).unwrap()
}
