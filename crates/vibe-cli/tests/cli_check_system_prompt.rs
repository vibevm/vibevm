//! `vibe check` carries the system-prompt freshness verdict — PROP-076
//! `SP-FMT-008`.
//!
//! The point of the id is that a developer who never ran `vibe prompt
//! check` still sees a stale prefix, so the assertions are made on the
//! ordinary report rather than on the verb: the id appears, it names the
//! build that resolves it, and a project that has not opted in is absent
//! from the report entirely.

use std::fs;
use std::path::Path;

use serde_json::Value;

mod common;

use common::UserScratch;

const TEMPLATE: &str = "# My frame\n\nPermanent instructions.\n";

/// Every `system_prompt` finding of `vibe check --json` on `project`.
fn findings(user: &UserScratch, project: &Path) -> Vec<Value> {
    let output = user
        .vibe()
        .args(["check", "--json", "--path"])
        .arg(project)
        .output()
        .unwrap();
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
        .filter(|finding| finding["check"] == "system_prompt")
        .cloned()
        .collect()
}

fn template(project: &Path) -> std::path::PathBuf {
    project.join(common::boot_rel("SYSTEM-TEMPLATE.md"))
}

/// The report's own spelling of a generated artifact's path.
fn shown(name: &str) -> String {
    common::boot_rel(name)
}

#[test]
fn a_project_that_has_not_opted_in_is_absent_from_the_report() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    assert!(
        findings(&user, project.path()).is_empty(),
        "the feature is invisible to a project that does not want it"
    );
}

#[test]
fn an_opted_in_project_whose_prefix_was_never_built_is_warned_with_the_build() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    fs::write(template(project.path()), TEMPLATE).unwrap();
    let found = findings(&user, project.path());
    assert!(!found.is_empty(), "no system_prompt finding");
    for finding in &found {
        assert_eq!(finding["severity"], "warning");
        assert!(
            finding["message"]
                .as_str()
                .unwrap()
                .contains("vibe prompt build"),
            "{finding}"
        );
        assert_eq!(finding["path"], shown("SYSTEM.md"));
    }
}

#[test]
fn a_built_prefix_leaves_the_report_clean_and_a_stale_one_does_not() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    fs::write(template(project.path()), TEMPLATE).unwrap();
    user.vibe()
        .args(["prompt", "build", "--path"])
        .arg(project.path())
        .assert()
        .success();
    assert!(
        findings(&user, project.path()).is_empty(),
        "a current prefix is no finding"
    );

    // Edit the one input the user writes. The prefix is now stale, and
    // this is the whole reason the id exists: it shows without being asked.
    fs::write(template(project.path()), "# Another frame\n").unwrap();
    let found = findings(&user, project.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["severity"], "warning");
    assert!(
        found[0]["message"].as_str().unwrap().contains("stale"),
        "{}",
        found[0]["message"]
    );
    // A stale prefix must not fail `vibe check`: it is derivable from
    // committed inputs by one command, and the caller who needs a hard
    // gate has `vibe prompt check`'s non-zero exit.
    user.vibe()
        .args(["check", "--path"])
        .arg(project.path())
        .assert()
        .success();
}
