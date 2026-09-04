//! CLI grammar and A+B planning projection for PROP-056 `vibe scrape`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

use assert_cmd::Command;
use predicates::prelude::*;

fn vibe(settings: &tempfile::TempDir) -> Command {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("vibe"));
    command.env("VIBE_SETTINGS", settings.path());
    command.env("VIBE_NO_DEFAULT_REGISTRY", "1");
    command
}

fn init_contract(settings: &tempfile::TempDir, project: &tempfile::TempDir) {
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname='scrape-cli-fixture'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    fs::write(project.path().join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
    vibe(settings)
        .args(["scrape", "contract", "init", "--path"])
        .arg(project.path())
        .assert()
        .success();
}

#[cfg(windows)]
fn write_real_scrape_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("vibevm/test-macros/src")).unwrap();
    fs::create_dir_all(root.join("vibevm/scrape")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "scrape-real-fixture"
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
        r#"[package]
name = "core-ai-native-specmark"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true
"#,
    )
    .unwrap();
    fs::write(
        root.join("vibevm/test-macros/src/lib.rs"),
        "extern crate proc_macro;\nuse proc_macro::TokenStream;\n#[proc_macro_attribute]\npub fn spec(_: TokenStream, item: TokenStream) -> TokenStream { item }\n",
    )
    .unwrap();
    fs::write(
        root.join("vibevm/health.rs"),
        "pub fn sealed_health_fixture() {}\n",
    )
    .unwrap();
    fs::write(
        root.join("vibevm/scrape/contract.toml"),
        r#"schema = 1
id = "org.example.real-scrape"

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

#[cfg(windows)]
fn cargo_check(root: &Path, target: &Path) {
    let status = ProcessCommand::new("cargo")
        .args(["check", "--offline", "--quiet"])
        .current_dir(root)
        .env("CARGO_TARGET_DIR", target)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "scraped fixture must remain cargo-checkable"
    );
}

#[cfg(windows)]
fn write_builtin_cargo_fixture(root: &Path) {
    fs::create_dir_all(root.join("app/src")).unwrap();
    fs::create_dir_all(root.join("vibevm/scrape")).unwrap();
    fs::write(
        root.join("app/Cargo.toml"),
        "[package]\nname='scrape-cargo-health'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    fs::write(
        root.join("app/src/lib.rs"),
        "pub fn healthy() -> bool { true }\n",
    )
    .unwrap();
    fs::create_dir(root.join("notes")).unwrap();
    fs::write(
        root.join("notes/doc.md"),
        b"before\n<vibevm>\nmanaged\n</vibevm>\nafter\n",
    )
    .unwrap();
    fs::write(
        root.join("app/Cargo.lock"),
        "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"scrape-cargo-health\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("vibevm/scrape/contract.toml"),
        r#"schema = 1
id = "org.example.cargo-health"
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
[[assert]]
id = "vibe-paths-absent"
kind = "paths-absent-v1"
patterns = ["vibevm", "vibevm/**"]
[[baseline]]
path = "notes/doc.md"
sha256 = "sha256:5ef52e4abfa8db9bad10b21c9791b22d1d02ae9249051b7f84d450c994b52b37"
[[rewrite]]
id = "strip-managed-note"
kind = "managed-block-remove-v1"
paths = ["notes/doc.md"]
marker = "vibevm"
matches = "exactly-one-per-file"
[[relocate]]
id = "move-rewritten-note"
from = "notes/doc.md"
to = "new/nested/doc.md"
conflict = "refuse"
required = true
[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "tool-offline"
max_stdout_bytes = 65536
max_stderr_bytes = 65536
max_result_bytes = 65536
termination_grace_seconds = 1
[[healthcheck]]
id = "cargo"
kind = "cargo"
root = "app"
build = "check"
workspace = false
locked = false
all_targets = false
tests = "skip"
profile = "dev"
features = []
timeout_seconds = 60
"#,
    )
    .unwrap();
}

fn exact_project_snapshot(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn visit(root: &Path, current: &Path, out: &mut BTreeMap<String, Option<Vec<u8>>>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .display()
                .to_string()
                .replace('\\', "/");
            if entry.file_type().unwrap().is_dir() {
                out.insert(relative, None);
                visit(root, &path, out);
            } else {
                out.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }

    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

#[cfg(not(windows))]
#[test]
fn non_windows_plans_typed_blocker_and_execute_recover_create_no_state() {
    let settings = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    init_contract(&settings, &project);
    let before = exact_project_snapshot(project.path());

    let plan = vibe(&settings)
        .args(["--json", "scrape", "--plan", "--in-place", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!plan.status.success());
    assert!(String::from_utf8_lossy(&plan.stdout).contains("scrape-platform-unsupported"));

    for arguments in [
        vec!["scrape", "--in-place", "--assume-yes", "--path"],
        vec!["scrape", "--recover", "--path"],
    ] {
        vibe(&settings)
            .args(arguments)
            .arg(project.path())
            .assert()
            .failure()
            .stderr(predicate::str::contains("scrape-platform-unsupported"));
    }
    assert!(!settings.path().join("scrape-state").exists());
    assert_eq!(exact_project_snapshot(project.path()), before);
}

#[cfg(windows)]
fn assert_stable_report_matches_stdout(settings: &Path, stdout: &[u8], transaction_id: &str) {
    let mut immediate = stdout.to_vec();
    while immediate
        .last()
        .is_some_and(|byte| matches!(byte, b'\r' | b'\n'))
    {
        immediate.pop();
    }
    let stable = fs::read(
        settings
            .join("scrape-state/reports")
            .join(format!("{transaction_id}.json")),
    )
    .unwrap();
    assert_eq!(
        stable, immediate,
        "stdout and stable report must be canonical-byte identical"
    );
}

#[test]
fn contract_init_works_in_human_quiet_and_json_modes() {
    let settings = tempfile::tempdir().unwrap();

    let human = tempfile::tempdir().unwrap();
    vibe(&settings)
        .args(["scrape", "contract", "init", "--path"])
        .arg(human.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Scrape contract"));
    assert!(human.path().join("vibevm/scrape/contract.toml").is_file());

    let quiet = tempfile::tempdir().unwrap();
    let output = vibe(&settings)
        .args(["--quiet", "scrape", "contract", "init", "--path"])
        .arg(quiet.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "quiet is exactly one line");
    assert!(stdout.contains("scrape contract created:"));

    let json = tempfile::tempdir().unwrap();
    let output = vibe(&settings)
        .args(["--json", "scrape", "contract", "init", "--path"])
        .arg(json.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["command"], "scrape-contract-init");
    assert!(
        document["created"]
            .as_str()
            .unwrap()
            .ends_with("contract.toml")
    );
}

#[test]
fn plan_and_contract_check_project_the_generated_plan_in_all_modes() {
    let settings = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    init_contract(&settings, &project);

    vibe(&settings)
        .args(["scrape", "--plan", "--path"])
        .arg(project.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("Scrape plan"))
        .stdout(predicate::str::contains("contract-boundary"))
        .stdout(predicate::str::contains("  item      "))
        .stdout(predicate::str::contains("  health    "))
        .stdout(predicate::str::contains("modified-policy-refusal"))
        .stdout(predicate::str::contains("health-preparation-required").not());

    let quiet = vibe(&settings)
        .args(["--quiet", "scrape", "contract", "check", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!quiet.status.success());
    let stdout = String::from_utf8(quiet.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "quiet is exactly one line");
    assert!(stdout.starts_with("scrape plan sha256:"));

    let json = vibe(&settings)
        .args(["--json", "scrape", "--plan", "--in-place", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!json.status.success());
    let document: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(document["schema"], 1);
    assert_eq!(document["command"], "scrape");
    assert_eq!(document["mode"], "in-place");
    assert!(document.get("project").is_some());
    assert!(document.get("contract").is_some());
    assert!(document.get("health_plan_id").is_some());
    assert_eq!(document["healthchecks"].as_array().unwrap().len(), 1);
    assert!(!document.to_string().contains("health-preparation-required"));
    assert!(document.get("tree_digest").is_none(), "no shadow core DTO");

    let export = vibe(&settings)
        .args(["--json", "scrape", "--plan", "--output"])
        .arg(settings.path().join("scraped-output"))
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!export.status.success());
    let export: serde_json::Value = serde_json::from_slice(&export.stdout).unwrap();
    assert_eq!(export["mode"], "export");
    assert!(
        export["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|blocker| { blocker["code"] == "modified-policy-refusal" })
    );
}

#[test]
fn output_and_in_place_are_exclusive_and_recover_owns_its_inputs() {
    let settings = tempfile::tempdir().unwrap();
    vibe(&settings)
        .args(["scrape", "--plan", "--output", "out", "--in-place"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));

    vibe(&settings)
        .args(["scrape", "--recover", "--contract", "contract.toml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));

    vibe(&settings)
        .args(["scrape", "--recover", "--plan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn execution_modes_enter_real_planning_and_recovery_paths() {
    let settings = tempfile::tempdir().unwrap();

    vibe(&settings)
        .args(["scrape", "--output", "scraped"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not implemented yet").not());

    vibe(&settings)
        .args(["scrape", "--in-place", "--assume-yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not implemented yet").not());

    vibe(&settings)
        .args(["scrape", "--recover"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires an explicit `--path"));

    vibe(&settings)
        .args(["scrape", "--recover", "--path", "."])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no pending scrape transaction"));
}

#[test]
fn unattended_never_supplies_destructive_scrape_confirmation() {
    let settings = tempfile::tempdir().unwrap();
    vibe(&settings)
        .args(["--unattended", "scrape", "--in-place"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "explicit scrape-local `--assume-yes`",
        ));
}

#[test]
fn detach_is_rejected_and_reserved_instead_of_aliasing_scrape() {
    let settings = tempfile::tempdir().unwrap();
    vibe(&settings)
        .arg("detach")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand 'detach'"));
}

#[cfg(windows)]
#[test]
fn real_export_and_in_place_scrape_strip_specmark_and_persist_canonical_reports() {
    let settings = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_real_scrape_fixture(source.path());

    let export = settings.path().join("exported");
    let export_result = vibe(&settings)
        .args(["--json", "scrape", "--output"])
        .arg(&export)
        .arg("--path")
        .arg(source.path())
        .output()
        .unwrap();
    assert!(
        export_result.status.success(),
        "export scrape failed: {}",
        String::from_utf8_lossy(&export_result.stderr)
    );
    let export_report: serde_json::Value = serde_json::from_slice(&export_result.stdout).unwrap();
    assert_eq!(export_report["outcome"], "verified");
    assert_eq!(export_report["assurance"], "reduced");
    assert_eq!(export_report["cleanup"], "complete");
    assert_eq!(export_report["health"].as_array().unwrap().len(), 2);
    assert!(!export_report["events"].as_array().unwrap().is_empty());
    assert!(!export_report["apply"].as_array().unwrap().is_empty());
    assert_stable_report_matches_stdout(
        settings.path(),
        &export_result.stdout,
        export_report["transaction_id"].as_str().unwrap(),
    );
    assert_eq!(
        export_report["rewrites"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == "strip-specmark")
            .unwrap()["matches"],
        1
    );
    assert!(
        export_report["health"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["argv"].as_array().is_some_and(|argv| !argv.is_empty()))
    );
    assert!(source.path().join("vibevm/scrape/contract.toml").is_file());
    assert!(!export.join("vibevm").exists());
    assert!(
        !fs::read_to_string(export.join("src/lib.rs"))
            .unwrap()
            .contains("specmark")
    );
    assert!(
        !fs::read_to_string(export.join("Cargo.toml"))
            .unwrap()
            .contains("core-ai-native-specmark")
    );
    cargo_check(&export, &settings.path().join("target-export"));

    let inplace_result = vibe(&settings)
        .args(["--json", "scrape", "--in-place", "--assume-yes", "--path"])
        .arg(source.path())
        .output()
        .unwrap();
    assert!(
        inplace_result.status.success(),
        "in-place scrape failed: {}; stdout: {}",
        String::from_utf8_lossy(&inplace_result.stderr),
        String::from_utf8_lossy(&inplace_result.stdout)
    );
    let inplace_report: serde_json::Value = serde_json::from_slice(&inplace_result.stdout).unwrap();
    assert_eq!(inplace_report["outcome"], "verified");
    assert_eq!(inplace_report["assurance"], "reduced");
    assert_eq!(inplace_report["cleanup"], "complete");
    assert_eq!(inplace_report["health"].as_array().unwrap().len(), 2);
    assert_stable_report_matches_stdout(
        settings.path(),
        &inplace_result.stdout,
        inplace_report["transaction_id"].as_str().unwrap(),
    );
    assert!(!source.path().join("vibevm").exists());
    assert!(
        !fs::read_to_string(source.path().join("src/lib.rs"))
            .unwrap()
            .contains("specmark")
    );
    assert!(
        !fs::read_to_string(source.path().join("Cargo.toml"))
            .unwrap()
            .contains("core-ai-native-specmark")
    );
    cargo_check(source.path(), &settings.path().join("target-in-place"));

    let reports = fs::read_dir(settings.path().join("scrape-state/reports"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(reports.len(), 2);
    for report in reports {
        let stable: serde_json::Value =
            serde_json::from_slice(&fs::read(report.path()).unwrap()).unwrap();
        assert_eq!(stable["schema"], 1);
        assert_eq!(stable["command"], "scrape");
        assert_eq!(stable["outcome"], "verified");
        assert_eq!(stable["cleanup"], "complete");
    }

    vibe(&settings)
        .args(["scrape", "--plan", "--in-place", "--path"])
        .arg(source.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a Vibe project"));
}

#[cfg(windows)]
#[test]
fn real_health_red_refuses_before_and_rolls_back_after() {
    let settings = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_real_scrape_fixture(source.path());
    let contract_before = fs::read(source.path().join("vibevm/scrape/contract.toml")).unwrap();

    fs::write(
        source.path().join("vibevm/health.rs"),
        "#[cfg(before)] compile_error!(\"red before\");\npub fn health() {}\n",
    )
    .unwrap();
    let project_temp = source.path().join("project-temp");
    fs::create_dir(&project_temp).unwrap();
    let before_refusal = exact_project_snapshot(source.path());
    let refused_output = settings.path().join("refused-output");
    let mut refused_command = vibe(&settings);
    refused_command
        .env("TMP", &project_temp)
        .env("TEMP", &project_temp);
    let refused = refused_command
        .args(["--json", "scrape", "--output"])
        .arg(&refused_output)
        .arg("--path")
        .arg(source.path())
        .output()
        .unwrap();
    assert!(!refused.status.success());
    let refused: serde_json::Value =
        serde_json::from_slice(&refused.stdout).unwrap_or_else(|error| {
            panic!(
                "refusal emitted invalid JSON ({error}); stderr: {}",
                String::from_utf8_lossy(&refused.stderr)
            )
        });
    assert_eq!(refused["outcome"], "refused");
    assert_eq!(refused["cleanup"], "complete");
    assert!(!refused_output.exists());
    assert_eq!(refused["health"].as_array().unwrap().len(), 1);
    assert_eq!(refused["health"][0]["phase"], "before");
    assert_eq!(refused["health"][0]["terminal"], "execution-failed");
    assert!(!refused["health"][0]["argv"].as_array().unwrap().is_empty());
    assert_eq!(exact_project_snapshot(source.path()), before_refusal);
    assert!(
        exact_project_snapshot(source.path())
            .keys()
            .all(|path| !path.contains("vibe-scrape-health"))
    );
    let transaction_id = refused["transaction_id"].as_str().unwrap();
    let state_entries = exact_project_snapshot(&settings.path().join("scrape-state"));
    assert!(
        state_entries
            .keys()
            .all(|path| { path.starts_with("reports/") || !path.contains(transaction_id) }),
        "completed refusal must retire its transaction workspace/home"
    );
    assert_eq!(
        fs::read(source.path().join("vibevm/scrape/contract.toml")).unwrap(),
        contract_before
    );

    fs::write(
        source.path().join("vibevm/health.rs"),
        "#[cfg(after)] compile_error!(\"red after\");\npub fn health() {}\n",
    )
    .unwrap();
    let before_rollback = exact_project_snapshot(source.path());
    let rolled_back = vibe(&settings)
        .args(["--json", "scrape", "--in-place", "--assume-yes", "--path"])
        .arg(source.path())
        .output()
        .unwrap();
    assert!(!rolled_back.status.success());
    let rolled_back: serde_json::Value = serde_json::from_slice(&rolled_back.stdout).unwrap();
    assert_eq!(rolled_back["outcome"], "rolled-back");
    assert_eq!(rolled_back["cleanup"], "complete");
    assert_eq!(rolled_back["health"].as_array().unwrap().len(), 2);
    assert_eq!(rolled_back["health"][0]["phase"], "before");
    assert_eq!(rolled_back["health"][0]["terminal"], "pass");
    assert_eq!(rolled_back["health"][1]["phase"], "after");
    assert_eq!(rolled_back["health"][1]["terminal"], "execution-failed");
    assert!(!rolled_back["rollback"].as_array().unwrap().is_empty());
    assert_eq!(exact_project_snapshot(source.path()), before_rollback);
    assert!(source.path().join("vibevm/scrape").is_dir());
    assert_eq!(
        fs::read(source.path().join("vibevm/scrape/contract.toml")).unwrap(),
        contract_before
    );
}

#[cfg(windows)]
#[test]
fn real_builtin_cargo_health_runs_before_and_after_from_exact_copies() {
    let settings = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_builtin_cargo_fixture(source.path());

    let result = vibe(&settings)
        .args(["--json", "scrape", "--in-place", "--assume-yes", "--path"])
        .arg(source.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "built-in Cargo scrape failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["outcome"], "verified");
    assert_eq!(report["assurance"], "reduced");
    assert_eq!(report["cleanup"], "complete");
    let health = report["health"].as_array().unwrap();
    assert_eq!(health.len(), 2);
    assert_eq!(health[0]["phase"], "before");
    assert_eq!(health[0]["step"], "build");
    assert_eq!(health[0]["terminal"], "pass");
    assert_eq!(health[1]["phase"], "after");
    assert_eq!(health[1]["step"], "build");
    assert_eq!(health[1]["terminal"], "pass");
    assert!(health.iter().all(|row| {
        row["argv"]
            .as_array()
            .is_some_and(|argv| argv.iter().any(|arg| arg == "--offline"))
    }));
    assert!(health.iter().all(|row| {
        row["stdout"]["redacted"] == true
            && row["stderr"]["redacted"] == true
            && row["stdout"]["head"] == ""
            && row["stderr"]["tail"] == ""
    }));
    assert!(!source.path().join("vibevm").exists());
    assert!(!source.path().join("notes/doc.md").exists());
    assert_eq!(
        fs::read(source.path().join("new/nested/doc.md")).unwrap(),
        b"before\nafter\n"
    );
    assert_eq!(report["rewrites"].as_array().unwrap().len(), 1);
    assert_eq!(report["relocations"].as_array().unwrap().len(), 1);
    assert!(source.path().join("app/Cargo.lock").is_file());
    cargo_check(
        &source.path().join("app"),
        &settings.path().join("cargo-health-target"),
    );
}

#[test]
fn contract_init_refuses_replacement() {
    let settings = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    init_contract(&settings, &project);
    let before = fs::read(project.path().join("vibevm/scrape/contract.toml")).unwrap();

    vibe(&settings)
        .args(["scrape", "contract", "init", "--path"])
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("without replacement"));
    assert_eq!(
        fs::read(project.path().join("vibevm/scrape/contract.toml")).unwrap(),
        before
    );
}

#[cfg(windows)]
fn cargo_generate_lockfile(root: &Path) {
    let output = ProcessCommand::new("cargo")
        .args(["generate-lockfile", "--offline", "--quiet"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "fixture lock generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
fn cargo_check_locked(root: &Path, target: &Path) {
    let output = ProcessCommand::new("cargo")
        .args(["check", "--locked", "--offline", "--quiet"])
        .current_dir(root)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "scraped fixture must satisfy cargo check --locked: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[test]
fn preexisting_cargo_lock_is_reconciled_with_plan_and_report_graph_evidence() {
    let settings = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_real_scrape_fixture(source.path());
    fs::write(
        source.path().join("vibevm/test-macros/Cargo.toml"),
        "[package]\nname='core-ai-native-specmark'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    fs::write(
        source.path().join("vibevm/test-macros/src/lib.rs"),
        "#[macro_export]\nmacro_rules! scope { ($uri:literal) => {}; }\n",
    )
    .unwrap();
    fs::write(
        source.path().join("src/lib.rs"),
        "specmark::scope!(\"spec://fixture/answer\");\npub fn answer() -> u32 { 42 }\n",
    )
    .unwrap();
    cargo_generate_lockfile(source.path());

    let contract_path = source.path().join("vibevm/scrape/contract.toml");
    let mut contract = fs::read_to_string(&contract_path)
        .unwrap()
        .replace("network = \"inherit\"", "network = \"tool-offline\"")
        .replace("forms = [\"spec\"]", "forms = [\"scope\"]");
    let healthcheck = contract
        .find("[[healthcheck]]")
        .expect("fixture contract has one healthcheck");
    contract.truncate(healthcheck);
    contract.push_str(
        r#"[[healthcheck]]
id = "cargo-locked"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = true
all_targets = false
tests = "skip"
profile = "dev"
features = []
timeout_seconds = 60
"#,
    );
    fs::write(&contract_path, contract).unwrap();
    cargo_check_locked(source.path(), &settings.path().join("locked-before-target"));

    let plan_output = vibe(&settings)
        .args(["--json", "scrape", "--plan", "--in-place", "--path"])
        .arg(source.path())
        .output()
        .unwrap();
    assert!(
        plan_output.status.success(),
        "locked Cargo scrape plan failed: {}",
        String::from_utf8_lossy(&plan_output.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&plan_output.stdout).unwrap();
    assert!(!plan["blockers"].as_array().unwrap().iter().any(|blocker| {
        blocker["code"] == "native-lock-evidence-required"
            || blocker["code"] == "native-lock-reconciliation-required"
    }));
    let lock_changes = plan["native_lock_changes"].as_array().unwrap();
    assert_eq!(lock_changes.len(), 1);
    let lock_change = &lock_changes[0];
    assert_eq!(lock_change["manager"], "cargo");
    assert_eq!(lock_change["path"], "Cargo.lock");
    assert_eq!(
        lock_change["authorizing_rewrite_id"],
        "remove-specmark-package"
    );
    assert!(!lock_change["before_graph"].as_array().unwrap().is_empty());
    assert!(!lock_change["after_graph"].as_array().unwrap().is_empty());
    assert!(!lock_change["removed"].as_array().unwrap().is_empty());

    let result = vibe(&settings)
        .args(["--json", "scrape", "--in-place", "--assume-yes", "--path"])
        .arg(source.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "locked Cargo scrape failed; stderr: {}; stdout: {}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["outcome"], "verified");
    let dependency_graphs = report["dependency_graphs"].as_array().unwrap();
    assert_eq!(dependency_graphs.len(), 1);
    assert_eq!(dependency_graphs[0]["manager"], "cargo");
    assert_eq!(dependency_graphs[0]["path"], "Cargo.lock");
    assert!(
        !dependency_graphs[0]["before"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!dependency_graphs[0]["after"].as_array().unwrap().is_empty());
    assert!(
        !dependency_graphs[0]["removed"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(report["health"].as_array().unwrap().iter().all(|row| {
        row["argv"]
            .as_array()
            .is_some_and(|argv| argv.iter().any(|arg| arg == "--locked"))
    }));
    assert!(!source.path().join("vibevm").exists());
    assert!(source.path().join("Cargo.lock").is_file());
    cargo_check_locked(source.path(), &settings.path().join("locked-target"));
}
