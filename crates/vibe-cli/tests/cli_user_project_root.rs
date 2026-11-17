//! `vibe user init` / `vibe user status` — the personal project's root-only
//! lifecycle (PROP-074 §2.1).
//!
//! Every command runs under a scratch settings root (`$VIBE_SETTINGS`), so the
//! developer's real `~/.vibe` is never touched. The cwd of each run is a
//! scratch directory with no repository in it: the personal root may never be
//! derived from a working directory.
mod common;

use std::fs;
use std::time::SystemTime;

use common::UserScratch;

/// The personal root the scratch settings imply.
fn personal(user: &UserScratch) -> std::path::PathBuf {
    user.settings.join("user-project")
}

/// A cwd outside any project.
fn elsewhere() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn entries(path: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn report(output: &std::process::Output) -> serde_json::Value {
    let text = std::str::from_utf8(&output.stdout).expect("UTF-8 report");
    let value: serde_json::Value = serde_json::from_str(text).expect("JSON report");
    if value["protocol"] == "vibe-user-project/1" {
        common::wire::report::<vibe_wire::generated::user_project_report::UserProjectReport>(text)
    } else {
        value
    }
}

#[test]
fn status_on_an_absent_root_succeeds_and_creates_nothing() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report = report(&output);
    assert_eq!(report["protocol"], "vibe-user-project/1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["command"], "user:status");
    assert_eq!(report["state"], "absent");
    assert_eq!(report["outcome"], "observed");
    assert_eq!(report["effects"], "none");
    assert_eq!(report["publication"], "not-started");
    assert_eq!(report["root"], personal(&user).to_string_lossy().as_ref());
    // Both nullable members are present and null, never omitted.
    assert!(report.get("reason").is_some());
    assert!(report["reason"].is_null());
    assert!(report.get("remedy").is_some());
    assert!(report["remedy"].is_null());
    assert!(report.get("error").is_none());
    assert!(report.get("observed_state").is_none());
    assert!(!personal(&user).exists());
    // The settings root existed already and gained nothing at all.
    assert_eq!(entries(&user.settings), Vec::<String>::new());
    assert_eq!(entries(cwd.path()), Vec::<String>::new());
}

#[test]
fn init_publishes_the_project_outside_any_repository() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = report(&output);
    assert_eq!(report["command"], "user:init");
    assert_eq!(report["state"], "initialized");
    assert_eq!(report["ok"], true);
    assert_eq!(report["outcome"], "created");
    assert_eq!(report["effects"], "changed");
    assert_eq!(report["publication"], "committed");
    assert!(report["reason"].is_null());
    assert!(report["remedy"].is_null());
    let root = personal(&user);
    assert_eq!(entries(&root), ["vibe.toml", "vibevm"]);
    assert!(root.join("vibevm/vibespecs").is_dir());
    // No lock, no boot output, no ignore file, nothing in the cwd.
    assert!(!root.join("vibe.lock").exists());
    assert!(!root.join(".vibe").exists());
    assert_eq!(entries(cwd.path()), Vec::<String>::new());
    // The created manifest is an ordinary `[project]` manifest.
    let text = fs::read_to_string(root.join("vibe.toml")).unwrap();
    let parsed = vibe_core::manifest::Manifest::parse_str(&text).unwrap();
    assert_eq!(parsed.require_project().unwrap().name, "vibe-user");
}

#[test]
fn a_second_init_reports_already_initialized_and_keeps_bytes_and_times() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init"])
        .assert()
        .success();
    let manifest = personal(&user).join("vibe.toml");
    let old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    fs::File::options()
        .write(true)
        .open(&manifest)
        .unwrap()
        .set_modified(old)
        .unwrap();
    let before = fs::metadata(&manifest).unwrap().modified().unwrap();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report = report(&output);
    assert_eq!(report["ok"], true);
    assert_eq!(report["outcome"], "already-initialized");
    assert_eq!(report["state"], "initialized");
    assert_eq!(report["effects"], "none");
    assert_eq!(report["publication"], "not-started");
    assert_eq!(fs::metadata(&manifest).unwrap().modified().unwrap(), before);
    assert_eq!(entries(&personal(&user)), ["vibe.toml", "vibevm"]);
}

#[test]
fn plain_success_goes_to_stdout_once_and_stderr_stays_empty() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--plain"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("user project").count(), 1, "{stdout}");
    assert!(stdout.contains("absent"), "{stdout}");
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    let quiet = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--quiet"])
        .output()
        .unwrap();
    assert!(quiet.status.success());
    assert_eq!(String::from_utf8_lossy(&quiet.stdout).lines().count(), 1);
}

#[test]
fn a_relative_settings_root_is_unavailable_with_a_null_root() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .env(vibe_core::settings::SETTINGS_DIR_ENV, "rel/settings")
        .args(["user", "init", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "a refusal prints no report on stdout"
    );
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["ok"], false);
    let observation = &envelope["user_project"];
    assert_eq!(observation["protocol"], "vibe-user-project/1");
    assert!(observation["root"].is_null());
    assert_eq!(observation["state"], "unavailable");
    assert_eq!(observation["outcome"], "refused");
    assert_eq!(observation["effects"], "none");
    assert_eq!(observation["publication"], "not-started");
    assert!(observation["reason"].as_str().unwrap().contains("relative"));
    assert!(observation["remedy"].as_str().unwrap().contains("absolute"));
    assert_eq!(entries(cwd.path()), Vec::<String>::new());
}

#[test]
fn a_relative_fallback_home_is_refused_without_using_the_cwd() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .env_remove(vibe_core::settings::SETTINGS_DIR_ENV)
        .env("HOME", "relative-home")
        .env("USERPROFILE", "relative-home")
        .args(["user", "status", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(envelope["user_project"]["root"].is_null());
    assert_eq!(entries(cwd.path()), Vec::<String>::new());
}

#[test]
fn a_foreign_nonempty_root_refuses_and_keeps_its_bytes() {
    let user = UserScratch::new();
    let root = personal(&user);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("notes.txt"), b"mine").unwrap();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["user_project"]["outcome"], "refused");
    assert_eq!(envelope["user_project"]["state"], "unavailable");
    assert_eq!(fs::read(root.join("notes.txt")).unwrap(), b"mine");
    assert!(!root.join("vibe.toml").exists());
}

#[test]
fn status_distinguishes_absent_initialized_and_unavailable() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let root = personal(&user);
    fs::create_dir(&root).unwrap();
    // An empty directory without a manifest is an absent project.
    let absent = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--json"])
        .output()
        .unwrap();
    assert!(absent.status.success());
    assert_eq!(absent.status.code(), Some(0));
    assert_eq!(report(&absent)["state"], "absent");
    assert_eq!(report(&absent)["ok"], true);
    // A valid project is initialized.
    fs::write(
        root.join("vibe.toml"),
        "[project]\nname = \"x\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let initialized = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--json"])
        .output()
        .unwrap();
    assert!(initialized.status.success());
    assert_eq!(report(&initialized)["state"], "initialized");
    assert_eq!(report(&initialized)["outcome"], "observed");
    assert_eq!(report(&initialized)["effects"], "none");
    // A package-only manifest is unavailable, with a nonzero exit.
    fs::write(
        root.join("vibe.toml"),
        "[package]\ngroup = \"org.example\"\nname = \"x\"\nversion = \"1.0.0\"\nkind = \"tool\"\n",
    )
    .unwrap();
    let unavailable = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--json"])
        .output()
        .unwrap();
    assert!(!unavailable.status.success());
    let envelope: serde_json::Value = serde_json::from_slice(&unavailable.stderr).unwrap();
    assert_eq!(envelope["user_project"]["state"], "unavailable");
}

#[test]
fn an_existing_empty_directory_is_initialized() {
    let user = UserScratch::new();
    let root = personal(&user);
    fs::create_dir_all(&root).unwrap();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report(&output)["outcome"], "created");
    assert!(root.join("vibe.toml").is_file());
}

#[test]
fn a_directory_holding_only_a_subdirectory_is_unavailable() {
    let user = UserScratch::new();
    let root = personal(&user);
    fs::create_dir_all(root.join("nested/deeper")).unwrap();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["user_project"]["state"], "unavailable");
    assert!(root.join("nested/deeper").is_dir());
}

#[cfg(unix)]
#[test]
fn a_linked_root_is_unavailable_and_untouched() {
    let user = UserScratch::new();
    let real = user.settings.join("real-project");
    fs::create_dir(&real).unwrap();
    let root = personal(&user);
    std::os::unix::fs::symlink(&real, &root).unwrap();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        fs::symlink_metadata(&root)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(!real.join("vibe.toml").exists());
}

#[test]
fn a_manifest_that_is_a_directory_is_unavailable() {
    let user = UserScratch::new();
    let root = personal(&user);
    fs::create_dir_all(root.join("vibe.toml")).unwrap();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn a_concurrent_manifest_written_between_status_and_init_survives() {
    let user = UserScratch::new();
    let root = personal(&user);
    fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    // A racing initializer publishes first; the loser must not overwrite it.
    let cwd = elsewhere();
    let theirs = "[project]\nname = \"theirs\"\nversion = \"1.0.0\"\n";
    fs::write(root.join("vibe.toml"), theirs).unwrap();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read_to_string(root.join("vibe.toml")).unwrap(), theirs);
}

#[test]
fn status_leaves_an_initialized_project_byte_identical() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init"])
        .assert()
        .success();
    let root = personal(&user);
    let before = fs::read(root.join("vibe.toml")).unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "status"])
        .assert()
        .success();
    assert_eq!(fs::read(root.join("vibe.toml")).unwrap(), before);
    assert_eq!(entries(&root), ["vibe.toml", "vibevm"]);
}

#[test]
fn nothing_outside_the_personal_root_changes() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init"])
        .assert()
        .success();
    let settings: Vec<String> = entries(&user.settings);
    assert_eq!(settings, ["user-project"]);
    assert!(!user.settings.join("mcp").exists());
    assert_eq!(entries(cwd.path()), Vec::<String>::new());
}

#[test]
fn the_verbs_accept_no_root_flag() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    for argv in [
        &["user", "status", "--path", "."][..],
        &["user", "status", "-g"][..],
        &["user", "status", "--force"][..],
    ] {
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args(argv)
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "{argv:?} must not be accepted: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[path = "cli_user_project_root/transport.rs"]
mod transport;
