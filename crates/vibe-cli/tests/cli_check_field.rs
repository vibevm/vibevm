//! `vibe check --json` names the manifest field a finding is about, so the
//! desktop shell's package editor can put the finding under that field
//! without reading the message (VIBEVM-SPEC `#linter`).

use std::fs;
use std::path::Path;

use serde_json::Value;

mod common;

use common::UserScratch;

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// The `manifest_validity` finding of `vibe check --json` on `project`,
/// which must fail.
fn manifest_finding(user: &UserScratch, project: &Path) -> Value {
    let output = user
        .vibe()
        .args(["check", "--json", "--path"])
        .arg(project)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "a refused manifest fails the check"
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not JSON ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["check"] == "manifest_validity")
        .cloned()
        .unwrap_or_else(|| panic!("no manifest_validity finding: {report}"))
}

fn option(id: &str, arch: &str, sha256: &str) -> String {
    format!(
        "\n[[installer_options]]\nid = \"{id}\"\nname = \"{id}\"\nplatform = \"linux\"\n\
         platform_name = \"Linux\"\narch = \"{arch}\"\n\n[[installer_options.steps]]\n\
         id = \"fetch\"\ntitle = \"Fetch\"\nkind = \"archive\"\nformat = \"zip\"\n\
         source = \"dist/demo.zip\"\nsha256 = \"{sha256}\"\n"
    )
}

#[test]
fn a_refused_field_is_reported_with_its_address_path_and_line() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let manifest = format!(
        "[package]\nname = \"demo\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
         [application]\nid = \"demo\"\n{}{}",
        option("first", "x86_64", SHA),
        option("second", "aarch64", "nope"),
    );
    fs::write(project.path().join("vibe.toml"), &manifest).unwrap();
    let finding = manifest_finding(&user, project.path());
    assert_eq!(finding["field"], "installer_options[1].steps[0].sha256");
    assert_eq!(finding["path"], "vibe.toml");
    let line = finding["line"].as_u64().unwrap() as usize;
    assert_eq!(manifest.lines().nth(line - 1), Some("sha256 = \"nope\""));
    assert_eq!(finding["column"], 1);
    assert!(
        finding["message"]
            .as_str()
            .unwrap()
            .contains("`fetch`.sha256 must be a 64-digit hexadecimal digest")
    );
}

#[test]
fn a_parse_error_is_reported_with_its_line_and_column() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = = \"1\"\n",
    )
    .unwrap();
    let finding = manifest_finding(&user, project.path());
    assert_eq!(finding["path"], "vibe.toml");
    assert_eq!(finding["line"], 3);
    assert_eq!(finding["column"], 11);
    assert!(
        finding.get("field").is_none(),
        "a syntax error names no table: {finding}"
    );
}
