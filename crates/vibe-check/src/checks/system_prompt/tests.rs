//! The system-prompt cell: silence for a project that never opted in, a
//! warning for a stale or missing prefix, silence for a tracked one.
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
fn tracked_prompt_reported_once_not_by_system_prompt() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    opt_in(project.path());
    fs::write(boot(project.path(), layout::SYSTEM_MD), "committed\n").unwrap();
    assert!(git_add(project.path()), "git init and git add must succeed");
    let found = findings(project.path());
    assert!(found.is_empty(), "{found:?}");
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

/// Initialise a private fixture repository and stage everything. Callers
/// require both git operations to succeed.
fn git_add(root: &Path) -> bool {
    let home = root.join(".private-home");
    fs::create_dir_all(home.join(".vibe")).unwrap();
    let run = |args: &[&str]| {
        Command::new("git")
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", &home)
            .env("GIT_CONFIG_GLOBAL", home.join(".gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
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

/// Count tracking findings across both cells from the actual dispatcher,
/// including old conflict wording, rather than only new-message prefixes.
#[test]
fn combined_check_project_reports_each_tracked_artifact_once() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    opt_in(project.path());
    let artifacts = layout::current_boot_prompt_artifacts();
    for path in &artifacts {
        fs::write(project.path().join(path), "generated\n").unwrap();
    }
    fs::write(
        project.path().join(layout::current_boot_index()),
        "generated\n",
    )
    .unwrap();
    let selected = |finding: &&crate::Finding| {
        matches!(
            finding.check,
            CheckId::BootDirectory | CheckId::SystemPrompt
        )
    };
    let tracking = |finding: &&crate::Finding| {
        finding.message.contains("git tracks") || finding.message.contains("tracked by git")
    };
    let before = crate::check_project(project.path(), &opts());
    assert_eq!(
        before
            .findings
            .iter()
            .filter(selected)
            .filter(tracking)
            .count(),
        0
    );
    assert!(git_add(project.path()), "fixture staging must succeed");
    let report = crate::check_project(project.path(), &opts());
    let tracked: Vec<_> = report
        .findings
        .iter()
        .filter(selected)
        .filter(tracking)
        .collect();
    assert_eq!(tracked.len(), 3, "{tracked:?}");
    for path in &artifacts {
        let shown = path.to_string_lossy().replace('\\', "/");
        let about_path: Vec<_> = tracked
            .iter()
            .filter(|finding| {
                finding.path.as_ref() == Some(path) || finding.message.contains(&shown)
            })
            .collect();
        assert_eq!(
            about_path.len(),
            1,
            "{shown}: {about_path:?}; all: {tracked:?}"
        );
        let finding = about_path[0];
        assert_eq!(finding.check, CheckId::BootDirectory);
        assert_eq!(finding.severity, crate::Severity::Error);
        assert_eq!(finding.path.as_ref(), Some(path));
        assert_eq!(
            finding.message,
            format!("git tracks the generated artifact {shown}")
        );
        assert_eq!(
            finding.why.as_deref(),
            Some(
                "it is derived from committed inputs, so a tracked copy can go stale and be believed"
            )
        );
        assert_eq!(finding.fix, Some(format!("git rm --cached {shown}")));
    }
    assert!(
        !tracked
            .iter()
            .any(|finding| { finding.path.as_ref() == Some(&layout::current_boot_index()) })
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|finding| finding.check == CheckId::SystemPrompt)
    );
}
