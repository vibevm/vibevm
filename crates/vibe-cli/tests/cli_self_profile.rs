//! Import consent and doctor observations through the real CLI, isolated from user state.
#![cfg(unix)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);
use assert_cmd::Command;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Output,
};

fn command(base: &Path, home: &Path) -> Command {
    let mut command = vibe_test_support::vibe();
    command
        .env("VIBEVM_INSTALL_ROOT", base)
        .env("HOME", home)
        .env("SHELL", "/bin/bash")
        .env("PATH", "/usr/bin:/bin")
        .env_remove("VIBEVM_HOME")
        .env_remove("VIBEVM_SHELL_HOME")
        .env_remove("VIBE_UNATTENDED");
    command
}
fn payload(root: &Path) -> PathBuf {
    let path = root.join("ready-vibe");
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}
fn import(base: &Path, home: &Path, executable: &Path, flags: &[&str]) -> Output {
    command(base, home)
        .args(["--json", "--unattended", "self", "import"])
        .arg(executable)
        .args(["--tag", "1.2.3"])
        .args(flags)
        .output()
        .unwrap()
}
fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("{e}: {output:?}"))
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_owned()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                result.insert(relative, None);
                pending.push(path);
            } else {
                result.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }
    result
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#CMD-IMPORT")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn import_default_and_explicit_optout_activate_without_profile_or_home_edits() {
    for existing in [false, true] {
        for flags in [
            &["--use"][..],
            &["--use", "--no-profile"][..],
            &["--use", "--yes", "--no-profile"][..],
        ] {
            let base = tempfile::tempdir().unwrap();
            let home = tempfile::tempdir().unwrap();
            let source = tempfile::tempdir().unwrap();
            let executable = payload(source.path());
            if existing {
                fs::write(
                    home.path().join(".bashrc"),
                    b"\xef\xbb\xbf# foreign\r\nexport EDITOR=vim\r\n",
                )
                .unwrap();
            }
            let before = snapshot(home.path());
            let output = import(base.path(), home.path(), &executable, flags);
            assert!(output.status.success(), "{output:?}");
            let value = report(&output);
            assert_eq!(value["durable_environment_policy"], "preserve");
            assert_eq!(value["durable_path_changed"], false);
            assert!(base.path().join("opt/bin/vibe").is_file());
            assert!(base.path().join("opt/vibevm/current").is_file());
            assert_eq!(snapshot(home.path()), before, "{flags:?}");
        }
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#CMD-IMPORT")]
fn inactive_import_even_with_yes_does_not_open_a_profile() {
    let base = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let before = snapshot(home.path());
    let output = import(
        base.path(),
        home.path(),
        &payload(source.path()),
        &["--yes"],
    );
    assert!(output.status.success(), "{output:?}");
    assert!(!base.path().join("opt/vibevm/current").exists());
    assert_eq!(snapshot(home.path()), before);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#CMD-IMPORT")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn default_activation_does_not_follow_or_reject_an_unselected_symlink_profile() {
    let base = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let target = source.path().join("foreign-profile");
    let original = b"export VIBEVM_HOME='/old'\nforeign=\xff\n";
    fs::write(&target, original).unwrap();
    let path = home.path().join(".bashrc");
    std::os::unix::fs::symlink(&target, &path).unwrap();
    let output = import(
        base.path(),
        home.path(),
        &payload(source.path()),
        &["--use"],
    );
    assert!(output.status.success(), "{output:?}");
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
    assert_eq!(fs::read(target).unwrap(), original);
    assert_eq!(report(&output)["durable_environment_policy"], "preserve");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#CMD-IMPORT")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn explicit_import_consent_is_idempotent_and_doctor_separates_finder_path() {
    let base = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let executable = payload(source.path());
    let path = home.path().join(".bashrc");
    let foreign = b"\xef\xbb\xbf# user bytes\r\nexport EDITOR=vim\r\n";
    fs::write(&path, foreign).unwrap();
    let first = import(base.path(), home.path(), &executable, &["--use", "--yes"]);
    assert!(first.status.success(), "{first:?}");
    assert_eq!(report(&first)["durable_environment_policy"], "update");
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.starts_with(foreign));
    assert!(
        std::str::from_utf8(&bytes)
            .unwrap()
            .contains("export VIBEVM_HOME=")
    );
    let second = import(base.path(), home.path(), &executable, &["--use", "--yes"]);
    assert!(second.status.success(), "{second:?}");
    assert_eq!(report(&second)["durable_path_changed"], false);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let before_home = snapshot(home.path());
    let before_store = snapshot(base.path());
    let doctor = command(base.path(), home.path())
        .args(["--json", "self", "doctor"])
        .output()
        .unwrap();
    let value = report(&doctor);
    assert_eq!(
        value["shim_dir_on_path"], false,
        "Finder-like process PATH lacks the shim directory"
    );
    let durable = &value["durable_environment"];
    assert_eq!(durable["target"], path.display().to_string());
    assert_eq!(durable["shell"], "bash");
    assert_eq!(durable["managed_block"], "present");
    assert_eq!(durable["path_entry"], "correct");
    assert_eq!(durable["login_probe"], "not_run");
    assert_eq!(durable["login_effectiveness"], "unknown");
    assert_eq!(snapshot(home.path()), before_home);
    assert_eq!(snapshot(base.path()), before_store);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn doctor_fix_requires_explicit_consent_and_does_not_claim_process_path_repaired() {
    let base = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let executable = payload(source.path());
    assert!(
        import(base.path(), home.path(), &executable, &["--use"])
            .status
            .success()
    );
    let before_home = snapshot(home.path());
    let before_store = snapshot(base.path());
    let refused = command(base.path(), home.path())
        .args(["--json", "--unattended", "self", "doctor", "--fix"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("explicit --yes"),
        "{refused:?}"
    );
    assert_eq!(snapshot(home.path()), before_home);
    assert_eq!(snapshot(base.path()), before_store);
    let fixed = command(base.path(), home.path())
        .args(["--json", "--unattended", "self", "doctor", "--fix", "--yes"])
        .output()
        .unwrap();
    let value = report(&fixed);
    assert_eq!(value["fixed"], true);
    assert_eq!(value["durable_environment"]["path_entry"], "correct");
    assert_eq!(value["shim_dir_on_path"], false);
    assert_eq!(value["process_path_refresh_needed"], true);
    assert_eq!(value["ok"], false);
    assert!(value["problems"].as_u64().unwrap() >= 1);
    assert_eq!(
        value["durable_environment"]["login_effectiveness"],
        "unknown"
    );
}

/// Owner decision 2026-11-08 (Need 8): the login-shell probe runs only with
/// consent — `--probe-login-shell` for non-interactive callers. A real
/// `bash -l` in an isolated HOME reads `.bash_profile`, not `.bashrc`, so the
/// managed `.bashrc` block is not in effect until a profile sources it.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn consented_login_probe_reports_whether_the_managed_path_reaches_login_shells() {
    if !Path::new("/bin/bash").is_file() {
        eprintln!("skipping: /bin/bash is not available");
        return;
    }
    let base = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let executable = payload(source.path());
    let first = import(base.path(), home.path(), &executable, &["--use", "--yes"]);
    assert!(first.status.success(), "{first:?}");
    let doctor = |flags: &[&str]| {
        report(
            &command(base.path(), home.path())
                .args(["--json", "--unattended", "self", "doctor"])
                .args(flags)
                .output()
                .unwrap(),
        )["durable_environment"]
            .clone()
    };
    let unconsented = doctor(&[]);
    assert_eq!(unconsented["login_probe"], "not_run");
    assert_eq!(unconsented["login_effectiveness"], "unknown");

    let probed = doctor(&["--probe-login-shell"]);
    assert_eq!(probed["login_probe"], "ran");
    assert_eq!(
        probed["login_effectiveness"], "not_effective",
        "a login bash does not read .bashrc on its own"
    );

    fs::write(
        home.path().join(".bash_profile"),
        "[ -f \"$HOME/.bashrc\" ] && . \"$HOME/.bashrc\"\n",
    )
    .unwrap();
    let sourced = doctor(&["--probe-login-shell"]);
    assert_eq!(sourced["login_probe"], "ran");
    assert_eq!(sourced["login_effectiveness"], "effective");
}
