//! The system-prompt cell: silence for a project that never opted in, a
//! warning for a stale or missing prefix, an error for a tracked one.
//!
//! The artifacts are written by hand rather than generated: this cell's job
//! is to *report* a verdict `vibe-workspace` computes, and composing a real
//! prefix here would test the composer instead. The states below are
//! exactly the ones the verdict distinguishes.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::tempdir;
use vibe_core::layout;

use super::*;
use crate::test_support::{opts, write_minimal_project};

/// Run only this cell, so a finding cannot have come from another.
fn findings(root: &Path) -> Vec<crate::Finding> {
    let mut report = CheckReport::default();
    SystemPromptCheck.run(root, &opts(), &mut report);
    report.findings
}

fn boot(root: &Path, name: &str) -> std::path::PathBuf {
    root.join(layout::current_boot_dir()).join(name)
}

/// Opt in the way a user does, and nothing else.
fn opt_in(root: &Path) {
    fs::write(
        boot(root, layout::SYSTEM_TEMPLATE_MD),
        "# My frame\n\nPermanent instructions.\n",
    )
    .unwrap();
}

#[test]
fn a_project_that_never_opted_in_is_not_reported_at_all() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    assert!(findings(project.path()).is_empty());
}

#[test]
fn an_opted_in_project_with_no_artifacts_is_a_warning_naming_the_build() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    opt_in(project.path());
    let found = findings(project.path());
    assert_eq!(found.len(), 3, "one per absent artifact: {found:?}");
    for finding in &found {
        assert_eq!(finding.check, CheckId::SystemPrompt);
        assert_eq!(
            finding.severity,
            crate::Severity::Warning,
            "a derivable artifact's absence is not an error"
        );
        assert!(finding.message.contains("missing"), "{}", finding.message);
        assert!(
            finding.message.contains("vibe prompt build"),
            "{}",
            finding.message
        );
    }
}

#[test]
fn a_project_that_opted_out_with_an_artifact_left_behind_is_still_reported() {
    // The one exception to "not opted in means silent": the inputs say
    // there should be no artifact, and a file on disk nothing can explain
    // is exactly the quiet state the removal rule exists to prevent.
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    fs::write(boot(project.path(), layout::SYSTEM_MD), "leftover\n").unwrap();
    let found = findings(project.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, crate::Severity::Warning);
    assert!(
        found[0].message.contains("opted out"),
        "{}",
        found[0].message
    );
}

#[test]
fn a_sidecar_that_does_not_parse_is_reported_rather_than_believed() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    opt_in(project.path());
    fs::write(boot(project.path(), layout::SYSTEM_MD), "whatever\n").unwrap();
    fs::write(boot(project.path(), layout::SYSTEM_META_JSON), "{\n").unwrap();
    fs::write(boot(project.path(), layout::AGENT_LANE_MD), "whatever\n").unwrap();
    let found = findings(project.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, crate::Severity::Warning);
    assert!(
        found[0].message.contains("freshness_unverifiable"),
        "{}",
        found[0].message
    );
}

#[test]
fn the_finding_points_at_the_generated_path_under_the_live_boot_directory() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    opt_in(project.path());
    let found = findings(project.path());
    let path = found[0].path.as_ref().expect("the finding names a path");
    assert_eq!(path, &boot_relative(layout::SYSTEM_MD));
    assert!(path.is_relative(), "{path:?}");
}

#[test]
fn a_git_tracked_artifact_is_an_error_and_names_the_remedy() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    opt_in(project.path());
    fs::write(boot(project.path(), layout::SYSTEM_MD), "committed\n").unwrap();
    if !git_add(project.path()) {
        // No usable git on this machine: the cell then answers "cannot
        // tell" by design, and there is nothing to assert.
        return;
    }
    let found = findings(project.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].severity,
        crate::Severity::Error,
        "a tracked generated prefix is the one state the rule exists to prevent"
    );
    assert!(
        found[0].message.contains("tracked by git"),
        "{}",
        found[0].message
    );
    assert!(
        found[0].message.contains("git rm --cached"),
        "{}",
        found[0].message
    );
}

#[test]
fn a_project_that_is_no_repository_says_nothing_about_tracking() {
    // "Cannot tell" is a third answer: with no git index to ask, the
    // ownership half is silent and only freshness is reported.
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    opt_in(project.path());
    fs::write(boot(project.path(), layout::SYSTEM_MD), "untracked\n").unwrap();
    let found = findings(project.path());
    for finding in &found {
        assert_eq!(
            finding.severity,
            crate::Severity::Warning,
            "tracking was reported without an index to read it from: {finding:?}"
        );
    }
}

/// Initialise a repository and stage everything. `false` when git is not
/// usable here, which the caller treats as "nothing to assert".
fn git_add(root: &Path) -> bool {
    let run = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };
    run(&["init", "-q"]) && run(&["add", "-A"])
}
