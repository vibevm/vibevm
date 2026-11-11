//! The isolated Git fixtures `materialisation_tests` drives: a controlled
//! backend over a selected installation, a warm origin, and the marker scripts
//! that prove nothing external ran. Split out of that file to hold the
//! file-length budget.
use super::*;
use std::os::unix::fs::PermissionsExt;

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: this cfg(test, unix) helper constructs isolated test-only Git fixtures; invalid fixture paths or configuration must abort the test, never become a product fallback"
)]
pub(super) fn selected(
    f: &Fixture,
    cwd: &Path,
    extra: &[(&str, String)],
    deny_b: bool,
) -> ShellGit {
    let mut selection =
        NativeConfigSelection::new(cwd.into(), Path::new(f.get("root")).join("home"))
            .unwrap()
            .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
            .unwrap()
            .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
            .unwrap();
    for (key, value) in extra {
        selection = selection.with_variable(key, value.into()).unwrap();
    }
    let profile = NativeGitAdmission::direct_https(
        Arc::new(
            crate::HttpAdmission::new(if deny_b { vec!["127.0.0.2"] } else { vec![] }).unwrap(),
        ),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(std::fs::read(f.get("ca")).unwrap()),
    )
    .with_config_selection(selection);
    ShellGit::new().with_native_admission(Arc::new(profile))
}
pub(super) fn warm_origin(f: &Fixture) -> PathBuf {
    let warm = PathBuf::from(f.get("work"));
    assert!(
        fixture_git(
            &f.info,
            &[
                "remote",
                "add",
                "origin",
                &format!("{}/repo.git", f.get("a"))
            ],
            &warm
        )
        .status
        .success()
    );
    warm
}
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: this cfg(test, unix) helper writes only isolated test marker scripts; invalid fixture paths or setup I/O must abort the test before asserting non-execution"
)]
pub(super) fn script(path: &Path, marker: &Path) {
    std::fs::write(path,format!("#!/usr/bin/python3\nimport pathlib,sys\npathlib.Path({:?}).write_text('ran')\nsys.stdout.buffer.write(sys.stdin.buffer.read().replace(b'retained',b'FILTERED'))\n",marker.to_str().unwrap())).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}
