//! Effective-document stream assertions with explicit child-process home isolation.

use super::*;

/// Keep settings separate from the home and attach both to every child, including init.
struct UserScratch {
    user: vibe_test_support::UserScratch,
    home: tempfile::TempDir,
}

impl UserScratch {
    /// Own a fresh synthetic home and settings for one test.
    fn new() -> Self {
        Self {
            user: vibe_test_support::UserScratch::new(),
            home: tempfile::tempdir().unwrap(),
        }
    }

    /// No child may inherit the operator's home or settings roots.
    fn vibe(&self) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        command
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("VIBE_SETTINGS", &self.user.settings);
        command
    }

    /// Initialise through the same isolated command used for inspection.
    fn init_project(&self, project: &Path) {
        self.vibe()
            .args(["init", "--path"])
            .arg(project)
            .assert()
            .success();
    }
}

#[test]
fn show_effective_emits_boot_files_with_provenance() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    let assertion = user
        .vibe()
        .arg("show")
        .arg("effective")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout);
    // 00-core and 90-user boot files. The WAL is NOT created by
    // default `vibe init` — it's a project convention, not part of
    // the package manager's contract — so `show effective` should
    // simply skip it when absent, not blow up.
    assert!(
        stdout.contains("spec://project/boot/00-core.md"),
        "expected 00-core spec URI; got:\n{stdout}"
    );
    assert!(
        stdout.contains("spec://project/boot/90-user.md"),
        "expected 90-user spec URI; got:\n{stdout}"
    );
    // Provenance marker for foundation (user-owned) files.
    assert!(
        stdout.contains("(user)"),
        "expected (user) provenance marker; got:\n{stdout}"
    );
}

/// Human prose belongs beside the document, never in its stdout bytes.
#[test]
fn show_effective_summary_goes_to_stderr() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let assertion = user
        .vibe()
        .args(["show", "effective", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let output = assertion.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("vibe show effective:"), "{stdout}");
    assert!(
        stderr
            .lines()
            .any(|line| line.starts_with("  ") && line.contains(" sections · ")),
        "{stderr}"
    );
}
