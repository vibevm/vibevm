//! R2.7 commissioning: an ordinary stack manifest supplies real build/test
//! bindings, while vibe remains build-system-agnostic.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{UserScratch, fixture_registry};
use specmark::verifies;
use vibe_core::manifest::Manifest;
use vibe_wire::generated::lifecycle_plan::{LifecyclePlan, PlannedContribution};
use vibe_wire::generated::lifecycle_report::{LifecycleContributionReport, LifecycleReport};

const SELECTED: &str = "org.vibevm.fixture/lifecycle-rust-stack";
const BUILD_KEY: &str = "org.vibevm.fixture/lifecycle-rust-stack#cargo-build";
const TEST_KEY: &str = "org.vibevm.fixture/lifecycle-rust-stack#cargo-test";
const OTHER_KEY: &str = "org.vibevm.fixture/other-stack#non-selected-build";

fn path_challenge(kind: &str) -> tempfile::TempDir {
    let prefix = if cfg!(windows) {
        format!("{kind} with spaces %TEMP% ")
    } else {
        format!("{kind} with spaces ")
    };
    let directory = tempfile::Builder::new().prefix(&prefix).tempdir().unwrap();
    let rendered = directory.path().to_string_lossy();
    assert!(rendered.contains(' '));
    if cfg!(windows) {
        assert!(rendered.contains("%TEMP%"));
    }
    directory
}

fn registry() -> tempfile::TempDir {
    let registry = path_challenge("registry");
    let selected_source = fixture_registry().join("org.vibevm.fixture/lifecycle-rust-stack/v0.1.0");
    let selected = registry
        .path()
        .join("org.vibevm.fixture/lifecycle-rust-stack/v0.1.0");
    common::copy_tree(&selected_source, &selected);
    if cfg!(windows) {
        fs::remove_file(selected.join("scripts/build.sh")).unwrap();
        fs::remove_file(selected.join("scripts/test.sh")).unwrap();
    }

    let other = registry
        .path()
        .join("org.vibevm.fixture/other-stack/v0.1.0");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        other.join("vibe.toml"),
        r#"[package]
name = "other-stack"
group = "org.vibevm.fixture"
kind = "stack"
version = "0.1.0"
authors = ["vibevm test fixtures"]
license = "EULA"
description = "Non-selected stack proving AUTO-BY-FAMILY"
keywords = ["test", "fixture", "lifecycle"]

[[extension]]
id = "non-selected-build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "NON-SELECTED-STACK-RAN" }
"#,
    )
    .unwrap();
    registry
}

fn project(user: &UserScratch, registry: &Path) -> tempfile::TempDir {
    let project = path_challenge("project");
    user.vibe()
        .args(["init", "--no-registry", "--author", "Preset Test"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();

    let manifest = Manifest::read(project.path().join("vibe.toml")).unwrap();
    assert!(manifest.project.unwrap().group.is_none());

    fs::write(
        project.path().join("Cargo.toml"),
        r#"[package]
name = "lifecycle-preset-demo"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::create_dir_all(project.path().join("tests")).unwrap();
    fs::write(
        project.path().join("src/main.rs"),
        "fn main() { println!(\"preset-build\"); }\n",
    )
    .unwrap();
    write_test(project.path(), "first");

    user.vibe()
        .args([
            "install",
            "stack:org.vibevm.fixture/lifecycle-rust-stack",
            "stack:org.vibevm.fixture/other-stack",
        ])
        .arg("--registry")
        .arg(registry)
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let selected_slot = project.path().join(common::slot_dir(
        "org.vibevm.fixture.lifecycle-rust-stack",
        "0.1.0",
    ));
    assert!(selected_slot.is_dir());
    assert!(selected_slot.to_string_lossy().contains(' '));
    if cfg!(windows) {
        assert!(selected_slot.to_string_lossy().contains("%TEMP%"));
        assert!(!selected_slot.join("scripts/build.sh").exists());
        assert!(!selected_slot.join("scripts/test.sh").exists());
        assert!(selected_slot.join("scripts/build.ps1").is_file());
        assert!(selected_slot.join("scripts/test.ps1").is_file());
    }

    let manifest_path = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest_path).unwrap();
    body.push_str(
        r#"

[active]
stack = "lifecycle-rust-stack"
"#,
    );
    fs::write(manifest_path, body).unwrap();
    project
}

fn write_test(project: &Path, value: &str) {
    fs::write(
        project.join("tests/preset.rs"),
        format!(
            r#"#[test]
fn preset_test_executes() {{
    let marker = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/preset-test-ran.txt");
    std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
    std::fs::write(marker, {value:?}).unwrap();
}}
"#,
        ),
    )
    .unwrap();
}

fn run_test(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
) -> (LifecyclePlan, LifecycleReport) {
    let output = user
        .vibe()
        .args(["test", "--json", "--path"])
        .arg(project)
        .arg("--registry")
        .arg(registry)
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful JSON lifecycle leaked stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let documents = json_documents(&output.stdout);
    assert_eq!(
        document_commands(&documents),
        ["lifecycle:plan", "lifecycle"]
    );
    let plan = serde_json::from_value(documents[0].clone()).unwrap();
    let report = serde_json::from_value(documents[1].clone()).unwrap();
    (plan, report)
}

fn json_documents(bytes: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn document_commands(documents: &[serde_json::Value]) -> Vec<&str> {
    documents
        .iter()
        .map(|document| document["command"].as_str().unwrap())
        .collect()
}

fn planned<'a>(plan: &'a LifecyclePlan, key: &str) -> &'a PlannedContribution {
    plan.contributions
        .iter()
        .find(|row| row.key == key)
        .unwrap_or_else(|| panic!("missing planned contribution {key}: {plan:?}"))
}

fn outcome<'a>(report: &'a LifecycleReport, key: &str) -> &'a LifecycleContributionReport {
    report
        .contributions
        .iter()
        .find(|row| row.key == key)
        .unwrap_or_else(|| panic!("missing contribution outcome {key}: {report:?}"))
}

fn assert_status(report: &LifecycleReport, expected: [(&str, &str); 3]) {
    for (key, status) in expected {
        assert_eq!(outcome(report, key).status, status, "{key}");
    }
}

fn assert_initial_script_shape(report: &LifecycleReport) {
    let build = outcome(report, BUILD_KEY);
    assert_eq!(build.handler, "script");
    assert_eq!(build.status, "ok");
    assert!(build.message.is_none());
    assert!(build.flagged.is_none());
    assert!(build.stdout.is_none());
    assert!(build.stderr.is_none());
    assert!(build.stdout_truncated.is_none());
    assert!(build.stderr_truncated.is_none());

    let test = outcome(report, TEST_KEY);
    assert_eq!(test.handler, "script");
    assert_eq!(test.status, "ok");
    assert!(test.message.is_none());
    assert!(test.flagged.is_none());
    assert!(
        test.stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("test result: ok")),
        "real cargo test stream was not captured: {test:?}"
    );
    assert!(test.stderr.is_none());
    assert!(test.stdout_truncated.is_none());
    assert!(test.stderr_truncated.is_none());
}

fn run_forced_failure(user: &UserScratch, project: &Path, registry: &Path) -> LifecycleReport {
    let output = user
        .vibe()
        .args(["test", "--json", "--force", "--path"])
        .arg(project)
        .arg("--registry")
        .arg(registry)
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "invalid Rust source was swallowed"
    );
    let documents = json_documents(&output.stdout);
    assert_eq!(
        document_commands(&documents),
        ["lifecycle:plan", "lifecycle"]
    );
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["ok"], false);
    assert!(
        error["error"]
            .as_str()
            .is_some_and(|text| text.contains(BUILD_KEY))
    );
    serde_json::from_value(documents[1].clone()).unwrap()
}

fn target_files(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

fn has_binary_artifact(project: &Path) -> bool {
    target_files(&project.join("target")).iter().any(|path| {
        let name = path.file_name().unwrap().to_string_lossy();
        if cfg!(windows) {
            matches!(
                name.as_ref(),
                "lifecycle-preset-demo.exe" | "lifecycle_preset_demo.exe"
            )
        } else {
            matches!(
                name.as_ref(),
                "lifecycle-preset-demo" | "lifecycle_preset_demo"
            )
        }
    })
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#PRESET-LAW")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STACK-CONTRIBUTES-PRESET")]
fn stack_manifest_drives_real_cargo_with_ordered_presets_and_selective_freshness() {
    let user = UserScratch::new();
    let registry = registry();
    let project = project(&user, registry.path());

    let (plan, first) = run_test(&user, project.path(), registry.path());
    let build = planned(&plan, BUILD_KEY);
    assert_eq!(build.provider, SELECTED);
    assert_eq!(build.version.as_deref(), Some("0.1.0"));
    assert_eq!(build.handler, "script");
    assert_eq!(build.tier, "preset");
    let test = planned(&plan, TEST_KEY);
    assert_eq!(test.provider, SELECTED);
    assert_eq!(test.handler, "script");
    assert_eq!(test.tier, "preset");
    let other = planned(&plan, OTHER_KEY);
    assert_eq!(other.provider, "org.vibevm.fixture/other-stack");
    assert_eq!(other.handler, "builtin");
    assert_eq!(other.tier, "dependency");
    let selected_at = plan
        .contributions
        .iter()
        .position(|row| row.key == BUILD_KEY)
        .unwrap();
    let other_at = plan
        .contributions
        .iter()
        .position(|row| row.key == OTHER_KEY)
        .unwrap();
    assert!(selected_at < other_at, "{plan:?}");
    assert_status(
        &first,
        [(BUILD_KEY, "ok"), (OTHER_KEY, "ok"), (TEST_KEY, "ok")],
    );
    assert_initial_script_shape(&first);
    assert!(
        has_binary_artifact(project.path()),
        "cargo build left no demo binary: {:?}; report: {first:?}",
        target_files(&project.path().join("target")),
    );
    assert_eq!(
        fs::read_to_string(project.path().join("target/preset-test-ran.txt")).unwrap(),
        "first"
    );

    let (_, second) = run_test(&user, project.path(), registry.path());
    assert_status(
        &second,
        [
            (BUILD_KEY, "fresh"),
            (OTHER_KEY, "fresh"),
            (TEST_KEY, "fresh"),
        ],
    );

    fs::write(
        project.path().join("README.md"),
        "outside every preset glob\n",
    )
    .unwrap();
    let (_, unrelated) = run_test(&user, project.path(), registry.path());
    assert_status(
        &unrelated,
        [
            (BUILD_KEY, "fresh"),
            (OTHER_KEY, "fresh"),
            (TEST_KEY, "fresh"),
        ],
    );

    write_test(project.path(), "second");
    let (_, test_only) = run_test(&user, project.path(), registry.path());
    assert_status(
        &test_only,
        [(BUILD_KEY, "fresh"), (OTHER_KEY, "fresh"), (TEST_KEY, "ok")],
    );
    assert_eq!(
        fs::read_to_string(project.path().join("target/preset-test-ran.txt")).unwrap(),
        "second"
    );

    fs::write(
        project.path().join("src/main.rs"),
        "fn main() { println!(\"project source changed\"); }\n",
    )
    .unwrap();
    let (_, source_changed) = run_test(&user, project.path(), registry.path());
    assert_status(
        &source_changed,
        [(BUILD_KEY, "ok"), (OTHER_KEY, "fresh"), (TEST_KEY, "ok")],
    );

    fs::write(project.path().join("src/main.rs"), "fn main( {\n").unwrap();
    let failed = run_forced_failure(&user, project.path(), registry.path());
    assert!(!failed.ok);
    assert_eq!(failed.contributions.len(), 1);
    let build_failure = outcome(&failed, BUILD_KEY);
    assert_eq!(build_failure.handler, "script");
    assert_eq!(build_failure.status, "fail");
    assert!(build_failure.message.is_some());
    assert!(
        build_failure
            .stderr
            .as_deref()
            .is_some_and(|stderr| !stderr.is_empty())
    );
}
