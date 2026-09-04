//! CLI grammar and A+B planning projection for PROP-056 `vibe scrape`.

use std::fs;

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
        .stdout(predicate::str::contains("health-unsupported"))
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
    assert!(document["healthchecks"].as_array().unwrap().is_empty());
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
            .any(|blocker| { blocker["code"] == "health-unsupported" })
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
fn execution_modes_refuse_explicitly_until_their_transaction_atoms_land() {
    let settings = tempfile::tempdir().unwrap();

    vibe(&settings)
        .args(["scrape", "--output", "scraped"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("execution is not implemented yet"));

    vibe(&settings)
        .args(["scrape", "--in-place", "--assume-yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("execution is not implemented yet"));

    vibe(&settings)
        .args(["scrape", "--recover"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires an explicit `--path"));

    vibe(&settings)
        .args(["scrape", "--recover", "--path", "."])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not implemented yet"));
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
