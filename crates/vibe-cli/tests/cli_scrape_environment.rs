#![cfg(windows)]

use std::fs;
use std::path::Path;

use assert_cmd::Command;

fn fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("vibevm/test-macros/src")).unwrap();
    fs::create_dir_all(root.join("vibevm/scrape")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "scrape-env-fixture"
version = "0.1.0"
edition = "2024"

[dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/test-macros" }
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "#[specmark::spec(\"spec://fixture/answer\")]\npub fn answer() -> u32 { 42 }\n",
    )
    .unwrap();
    fs::write(
        root.join("vibevm/test-macros/Cargo.toml"),
        "[package]\nname='core-ai-native-specmark'\nversion='0.1.0'\nedition='2024'\n[lib]\nproc-macro=true\n",
    )
    .unwrap();
    fs::write(
        root.join("vibevm/test-macros/src/lib.rs"),
        "extern crate proc_macro;\nuse proc_macro::TokenStream;\n#[proc_macro_attribute]\npub fn spec(_: TokenStream, item: TokenStream) -> TokenStream { item }\n",
    )
    .unwrap();
    fs::write(root.join("vibevm/health.rs"), "pub fn health() {}\n").unwrap();
    fs::write(
        root.join("vibevm/scrape/contract.toml"),
        r#"schema = 1
id = "org.example.environment"
[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"
[scope]
closed_roots = ["vibevm"]
outside = "implicit-keep"
[commit]
contract = "delete-last"
[[classify]]
id = "remove-vibevm"
kind = "delete"
patterns = ["vibevm", "vibevm/**"]
owner = "vibe"
proof = "contract-assertion-v1"
modified = "delete"
require_match = true
[[rewrite]]
id = "strip-specmark"
kind = "rust-specmark-strip-v1"
patterns = ["src/**/*.rs"]
forms = ["spec"]
matches = "one-or-more"
[[rewrite]]
id = "remove-specmark-package"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
[[assert]]
id = "vibe-paths-absent"
kind = "paths-absent-v1"
patterns = ["vibevm", "vibevm/**"]
[[assert]]
id = "specmark-text-absent"
kind = "text-literal-absent-v1"
patterns = ["src/**/*.rs"]
needles = ["specmark"]
[[assert]]
id = "specmark-dependency-absent"
kind = "dependency-identities-absent-v1"
manager = "cargo"
manifests = ["Cargo.toml"]
identities = ["core-ai-native-specmark"]
[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "inherit"
max_stdout_bytes = 65536
max_stderr_bytes = 65536
max_result_bytes = 65536
termination_grace_seconds = 1
[[healthcheck]]
id = "sealed-rustc"
kind = "custom"
root = "."
source = "vibevm/health.rs"
snapshot = ["vibevm/health.rs"]
interpreter = "rustc"
argv = ["--cfg", "{phase}", "--crate-name", "scrape_health", "--crate-type", "lib", "--emit", "metadata", "--out-dir", "{scratch}"]
protocol = "exit-code"
reads = ["**"]
writes = []
spawn = true
network = "inherit"
timeout_seconds = 30
"#,
    )
    .unwrap();
}

fn command() -> Command {
    let mut command = Command::cargo_bin("vibe").unwrap();
    command
        .env_remove("VIBE_SETTINGS")
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .env("VIBE_NO_DEFAULT_REGISTRY", "1");
    command
}

#[test]
fn planning_succeeds_without_journal_environment() {
    let project = tempfile::tempdir().unwrap();
    fixture(project.path());
    command()
        .args(["scrape", "--plan", "--in-place", "--path"])
        .arg(project.path())
        .assert()
        .success();
}

#[test]
fn execute_and_recover_without_home_refuse_bounded() {
    let project = tempfile::tempdir().unwrap();
    fixture(project.path());
    for args in [
        ["scrape", "--in-place", "--assume-yes", "--path"].as_slice(),
        ["scrape", "--recover", "--path"].as_slice(),
    ] {
        command()
            .args(args)
            .arg(project.path())
            .assert()
            .failure()
            .stderr("error: resolving user home for scrape transaction state\n");
    }
}

#[test]
fn selected_state_root_is_only_engine_owned_argv_evidence() {
    let root = tempfile::tempdir().unwrap();
    let settings = root.path().join("selected-state-root-marker");
    let project = root.path().join("project");
    let output = root.path().join("output");
    fs::create_dir(&project).unwrap();
    fixture(&project);

    let mut scrape = command();
    let result = scrape
        .env("VIBE_SETTINGS", &settings)
        .env("HOME", root.path().join("unused-home-canary"))
        .env("USERPROFILE", root.path().join("unused-profile-canary"))
        .args(["--json", "scrape", "--output"])
        .arg(&output)
        .arg("--path")
        .arg(&project)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    for rendered in [&result.stdout, &result.stderr] {
        let rendered = String::from_utf8_lossy(rendered);
        for forbidden in [
            "VIBE_SETTINGS",
            "USERPROFILE",
            "HOME",
            "unused-home-canary",
            "unused-profile-canary",
            "credential",
        ] {
            assert!(!rendered.contains(forbidden), "{rendered}");
        }
    }

    let reports = fs::read_dir(settings.join("scrape-state/reports"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(reports.len(), 1);
    let report = fs::read(reports[0].path()).unwrap();
    assert_eq!(report, result.stdout.trim_ascii_end());
    let mut value: serde_json::Value = serde_json::from_slice(&report).unwrap();
    let health = value["health"].take();
    assert!(
        !serde_json::to_string(&value)
            .unwrap()
            .contains("selected-state-root-marker")
    );
    for row in health.as_array().unwrap() {
        let marked = row["argv"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_str)
            .filter(|arg| arg.contains("selected-state-root-marker"))
            .collect::<Vec<_>>();
        assert_eq!(marked.len(), 2, "{row}");
        for path in marked {
            let path = path.replace('\\', "/");
            assert!(path.contains("/selected-state-root-marker/scrape-state/t/"));
            assert!(path.contains("/v/s"), "{path}");
        }
    }
}
