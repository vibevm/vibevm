//! Registry edits draw one result block, a final trailer and coded refusals.

mod common;

use common::UserScratch;
use std::path::Path;
use std::process::Output;

fn setup() -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    (user, project)
}

fn run(user: &UserScratch, project: &Path, args: &[&str]) -> Output {
    user.vibe()
        .env("HOME", project)
        .env("USERPROFILE", project)
        .env("VIBE_SETTINGS", &user.settings)
        .env("NO_COLOR", "1")
        .args(args)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

fn add(user: &UserScratch, project: &Path) -> Output {
    run(
        user,
        project,
        &[
            "registry",
            "add",
            "corp2",
            "file:///srv/gitreg",
            "--type",
            "localdir",
        ],
    )
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn registry_add_ends_with_a_registered_trailer() {
    let (user, project) = setup();
    let output = add(&user, project.path());
    success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.lines().next(),
        Some("✓ added registry corp2 · localdir · project · publish localdir")
    );
    assert!(
        stdout
            .lines()
            .any(|line| line == "  url   file:///srv/gitreg")
    );
    let last = stdout.lines().rfind(|line| !line.is_empty()).unwrap();
    assert!(last.starts_with("✻ registered corp2 · "), "{stdout}");
    assert!(
        last.ends_with(&project.path().join("vibe.toml").display().to_string()),
        "{stdout}"
    );
    assert!(!stdout.contains('`'), "{stdout}");
}

#[test]
fn registry_add_twice_is_registry_exists() {
    let (user, project) = setup();
    success(&add(&user, project.path()));
    let output = add(&user, project.path());
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(
            "✗ registry_exists  a registry named corp2 already exists in the project layer"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains("vibe registry remove registry corp2"),
        "{stderr}"
    );
}

#[test]
fn registry_add_to_the_site_scope_is_scope_read_only() {
    let (user, project) = setup();
    let output = run(
        &user,
        project.path(),
        &["registry", "add", "x", "file:///x", "--scope", "site"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("✗ scope_read_only  the site layer is not editable here"),
        "{stderr}"
    );
    assert!(
        stderr.contains("the site profile is applied as a whole"),
        "{stderr}"
    );
    assert!(stderr.contains("vibe self apply <file>"), "{stderr}");
}

#[test]
fn registry_remove_ends_with_a_removed_trailer() {
    let (user, project) = setup();
    success(&add(&user, project.path()));
    let output = run(
        &user,
        project.path(),
        &["registry", "remove", "registry", "corp2"],
    );
    success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let last = stdout.lines().rfind(|line| !line.is_empty()).unwrap();
    assert!(last.starts_with("✻ removed registry corp2 · "), "{stdout}");
    assert!(
        last.ends_with(&project.path().join("vibe.toml").display().to_string()),
        "{stdout}"
    );
    let output = run(
        &user,
        project.path(),
        &["registry", "remove", "registry", "nosuch"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("✗ registry_not_found  no registry named nosuch in the project layer"),
        "{stderr}"
    );
    assert!(stderr.contains("known: none"), "{stderr}");
}
