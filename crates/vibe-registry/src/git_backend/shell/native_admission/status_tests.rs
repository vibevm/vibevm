//! Native local status for an untrusted checkout: no legacy shell, no
//! authored config execution, untracked and ignored files count.
use super::*;
use crate::{GitBackend, ShellGit};
use std::process::Command;

fn native() -> ShellGit {
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::System,
    );
    ShellGit::new().with_native_admission(Arc::new(profile))
}
fn fixture_git(work: &Path, args: &[&str]) {
    let output = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", work)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(work)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION")]
fn native_status_reports_tracked_untracked_and_ignored_changes() {
    let temp = tempfile::tempdir().unwrap();
    let work = temp.path().join("work");
    std::fs::create_dir(&work).unwrap();
    fixture_git(&work, &["init", "--initial-branch=main"]);
    std::fs::write(work.join("payload.txt"), "clean\n").unwrap();
    std::fs::write(work.join(".gitignore"), "ignored.tmp\n").unwrap();
    fixture_git(&work, &["add", "-A"]);
    fixture_git(&work, &["commit", "-m", "fixture"]);
    let git = native();
    assert!(!git.working_tree_dirty(&work).unwrap());
    std::fs::write(work.join("ignored.tmp"), "ignored\n").unwrap();
    assert!(git.working_tree_dirty(&work).unwrap(), "ignored file");
    std::fs::remove_file(work.join("ignored.tmp")).unwrap();
    std::fs::write(work.join("untracked.tmp"), "untracked\n").unwrap();
    assert!(git.working_tree_dirty(&work).unwrap(), "untracked file");
    std::fs::remove_file(work.join("untracked.tmp")).unwrap();
    std::fs::write(work.join("payload.txt"), "modified\n").unwrap();
    assert!(git.working_tree_dirty(&work).unwrap(), "tracked change");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION")]
fn native_status_refuses_checkout_config_that_could_execute() {
    let temp = tempfile::tempdir().unwrap();
    let work = temp.path().join("work");
    std::fs::create_dir(&work).unwrap();
    fixture_git(&work, &["init", "--initial-branch=main"]);
    let canary = temp.path().join("fsmonitor-ran");
    fixture_git(
        &work,
        &[
            "config",
            "core.fsmonitor",
            &format!("touch {}", canary.display()),
        ],
    );
    assert!(native().working_tree_dirty(&work).is_err());
    assert!(!canary.exists(), "checkout-authored fsmonitor executed");
}
