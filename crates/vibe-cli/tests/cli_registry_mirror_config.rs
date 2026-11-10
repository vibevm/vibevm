//! `vibe registry mirror --config mirror.toml` — declarative selection and
//! the selection the target records (PROP-075 `#MIRROR-CONFIG`,
//! `#MIRROR-CONFIG-SCHEMA`). Local directories only, no network.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;

/// A `localdir` source registry: one directory per version, whose index
/// `load_or_project` derives from the manifests themselves.
fn publish(root: &Path, group: &str, name: &str, version: &str) {
    let dir = root.join(group).join(name).join(format!("v{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\ngroup='{group}'\nname='{name}'\nkind='feat'\nversion='{version}'\n\
             description='The quokkaword {name} package'\nkeywords=['quokkaword']\n"
        ),
    )
    .unwrap();
}

fn project(root: &Path, source: &Path, target: &Path) -> PathBuf {
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        format!(
            "[project]\nname='consumer'\nversion='0.1.0'\n\
             [[registry]]\nname='src'\ntype='localdir'\nurl='file://{}'\n\
             [[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
            source.display(),
            target.display()
        ),
    )
    .unwrap();
    project
}

fn vibe(user: &UserScratch, project: &Path, args: &[&str]) -> Output {
    user.vibe()
        .current_dir(project)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .output()
        .unwrap()
}

fn json(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn refusal(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "expected a refusal, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// `<group>/<name>@<version>` of every version the report carries.
fn copied(report: &serde_json::Value) -> Vec<String> {
    let mut rows: Vec<String> = report["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            format!(
                "{}@{}",
                row["package"].as_str().unwrap(),
                row["version"].as_str().unwrap()
            )
        })
        .collect();
    rows.sort();
    rows
}

/// A source carrying three groups, so every axis of `[select]` has
/// something to decide.
fn fixture(root: &Path) -> (PathBuf, PathBuf) {
    let source = root.join("source");
    publish(&source, "org.corp", "demo", "1.0.0");
    publish(&source, "org.corp", "demo", "1.1.0");
    publish(&source, "org.corp", "legacy", "0.9.0");
    publish(&source, "org.other", "tool", "2.0.0");
    let target = root.join("target");
    fs::create_dir_all(&target).unwrap();
    (source, target)
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG")]
fn a_config_selects_declaratively_and_the_target_records_what_it_selected() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let (source, target) = fixture(root);
    let project = project(root, &source, &target);
    let config = root.join("mirror.toml");
    fs::write(
        &config,
        "schema = 1\n[select]\ngroups = ['org.corp']\nexclude = ['org.corp/legacy']\n",
    )
    .unwrap();

    let report = json(&vibe(
        &user,
        &project,
        &[
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            config.to_str().unwrap(),
        ],
    ));
    // The group axis admits `org.corp`, the exclude subtracts `legacy`,
    // and `org.other` was never in.
    assert_eq!(
        copied(&report),
        ["org.corp/demo@1.0.0", "org.corp/demo@1.1.0"],
        "{report}"
    );
    assert!(target.join("org.corp/demo/v1.1.0/vibe.toml").is_file());
    assert!(!target.join("org.corp/legacy").exists());
    assert!(!target.join("org.other").exists());

    // The selection is recorded beside the provenance, in a record whose
    // schema is still 1.
    let record: toml::Value =
        toml::from_str(&fs::read_to_string(target.join("_index/mirrors/src.toml")).unwrap())
            .unwrap();
    assert_eq!(record["schema"].as_integer(), Some(1));
    assert_eq!(
        record["select"]["groups"].as_array().unwrap()[0].as_str(),
        Some("org.corp")
    );
    assert_eq!(
        record["select"]["exclude"].as_array().unwrap()[0].as_str(),
        Some("org.corp/legacy")
    );

    // A later bare run repeats the recorded selection: the new version of
    // an admitted package is copied, the excluded one still is not.
    publish(&source, "org.corp", "demo", "1.2.0");
    publish(&source, "org.corp", "legacy", "0.10.0");
    publish(&source, "org.other", "tool", "2.1.0");
    let repeat = json(&vibe(
        &user,
        &project,
        &["--json", "registry", "mirror", "src", "--into", "dst"],
    ));
    assert_eq!(
        copied(&repeat),
        [
            "org.corp/demo@1.0.0",
            "org.corp/demo@1.1.0",
            "org.corp/demo@1.2.0"
        ],
        "{repeat}"
    );
    assert!(target.join("org.corp/demo/v1.2.0/vibe.toml").is_file());
    assert!(!target.join("org.corp/legacy").exists());
    assert!(!target.join("org.other").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG")]
fn package_flags_override_the_file_wholesale() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let (source, target) = fixture(root);
    let project = project(root, &source, &target);
    let config = root.join("mirror.toml");
    fs::write(&config, "schema = 1\n[select]\ngroups = ['org.corp']\n").unwrap();

    let report = json(&vibe(
        &user,
        &project,
        &[
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            config.to_str().unwrap(),
            "--package",
            "org.other/tool",
        ],
    ));
    // Not the intersection (which would be empty) and not the union: the
    // flag replaced `[select]`.
    assert_eq!(copied(&report), ["org.other/tool@2.0.0"], "{report}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn every_document_level_refusal_reaches_the_command() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let (source, target) = fixture(root);
    let project = project(root, &source, &target);
    let config = root.join("mirror.toml");
    let run = |user: &UserScratch| {
        vibe(
            user,
            &project,
            &[
                "registry",
                "mirror",
                "src",
                "--into",
                "dst",
                "--config",
                config.to_str().unwrap(),
            ],
        )
    };

    // A `--config` naming a file that does not exist.
    let message = refusal(&run(&user));
    assert!(
        message.contains("names a file that does not exist"),
        "{message}"
    );

    // Any schema but 1.
    fs::write(&config, "schema = 7\n[select]\nall = true\n").unwrap();
    let message = refusal(&run(&user));
    assert!(
        message.contains("has schema 7, this vibe reads 1"),
        "{message}"
    );

    // A selection of nothing is never a silent empty run.
    fs::write(&config, "schema = 1\n[select]\nall = false\n").unwrap();
    let message = refusal(&run(&user));
    assert!(message.contains("selects nothing"), "{message}");

    // Payloads with no drop to write them into.
    fs::write(
        &config,
        "schema = 1\n[[template]]\npackage = 'org.corp/demo'\npayloads = true\n",
    )
    .unwrap();
    let message = refusal(&run(&user));
    assert!(message.contains("[artifacts] packages"), "{message}");

    // Payloads into the registry target itself.
    fs::write(
        &config,
        format!(
            "schema = 1\n[[template]]\npackage = 'org.corp/demo'\npayloads = true\n\
             [payloads]\ninto = '{}'\n",
            target.join("payloads").display()
        ),
    )
    .unwrap();
    let message = refusal(&run(&user));
    assert!(message.contains("is the registry target"), "{message}");

    // Nothing was written by any of them.
    assert!(!target.join("org.corp").exists());
    assert!(!target.join("_index").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY")]
fn a_template_row_against_a_per_package_git_source_refuses_by_name() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let target = root.join("target");
    fs::create_dir_all(&target).unwrap();
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        format!(
            "[project]\nname='consumer'\nversion='0.1.0'\n\
             [[registry]]\nname='src'\ntype='git'\nurl='file://{}/org'\nindex_url='none'\n\
             [[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
            root.display(),
            target.display()
        ),
    )
    .unwrap();
    let config = root.join("mirror.toml");
    fs::write(
        &config,
        "schema = 1\n[[template]]\npackage = 'org.vibevm.node/node'\nversions = ['>=22']\n",
    )
    .unwrap();

    let message = refusal(&vibe(
        &user,
        &project,
        &[
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            config.to_str().unwrap(),
        ],
    ));
    assert!(message.contains("org.vibevm.node/node"), "{message}");
    assert!(message.contains("per-package Git registry"), "{message}");
    assert!(message.contains("#TEMPLATE-DISCOVERY"), "{message}");
}
