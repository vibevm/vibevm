use super::*;

#[test]
fn a_pre_set_stop_file_cancels_before_the_first_write() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let stop = cwd.path().join("cancel-request");
    fs::write(&stop, b"stop").unwrap();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json", "--stop-file"])
        .arg(&stop)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(130));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["protocol"], "vibe-project-cancellation/1");
    assert_eq!(envelope["outcome"], "cancelled");
    assert_eq!(envelope["publication"], "not-started");
    // Exactly one document: the cancellation envelope, no root report too.
    assert!(envelope.get("user_project").is_none());
    assert_eq!(stdout.trim_end().lines().count(), 1, "{stdout}");
    assert!(!personal(&user).exists());
}

#[test]
fn status_accepts_the_stop_file_and_cancels_without_writes() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let stop = cwd.path().join("cancel-request");
    fs::write(&stop, b"stop").unwrap();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--json", "--stop-file"])
        .arg(&stop)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(130));
    assert_eq!(report(&output)["protocol"], "vibe-project-cancellation/1");
    assert!(!personal(&user).exists());
}

#[test]
fn the_command_no_longer_refuses_stop_file_as_unsupported() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let stop = cwd.path().join("cancel-request");
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--stop-file"])
        .arg(&stop)
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("supported only"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Both verbs are now named by the gate's own diagnostic.
    assert!(output.status.success());
}

#[test]
fn the_report_root_is_the_admitted_absolute_path() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let root = personal(&user);
    assert!(root.is_absolute());
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "status", "--json"])
        .output()
        .unwrap();
    let reported = report(&output)["root"].as_str().unwrap().to_owned();
    assert_eq!(reported, vibe_core::machine_json_path(&root));
    assert!(!reported.is_empty());
}

#[test]
fn plain_refusals_go_to_stderr_and_leave_stdout_empty() {
    let user = UserScratch::new();
    let root = personal(&user);
    fs::create_dir_all(root.join("nested")).unwrap();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--plain"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    // The refusal sentence comes from the report's reason; the actionable
    // step must reach the operator beside it.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("retry"),
        "the remedy must be shown: {stderr}"
    );
}

#[test]
fn a_deep_root_is_created_under_an_isolated_settings_home() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(report(&output)["ok"], true);
    assert!(personal(&user).join("vibe.toml").is_file());
}

#[test]
fn the_freshly_created_project_parses_with_the_ordinary_manifest_reader() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init"])
        .assert()
        .success();
    let text = fs::read_to_string(personal(&user).join("vibe.toml")).unwrap();
    let manifest = vibe_core::manifest::Manifest::parse_str(&text).unwrap();
    manifest.require_project().unwrap();
    assert!(manifest.package.is_none());
}

#[test]
fn a_stop_file_with_relative_settings_still_reports_one_cancellation() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let stop = cwd.path().join("cancel-request");
    fs::write(&stop, b"stop").unwrap();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .env(vibe_core::settings::SETTINGS_DIR_ENV, "rel/settings")
        .args(["user", "status", "--json", "--stop-file"])
        .arg(&stop)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(130));
    assert_eq!(report(&output)["protocol"], "vibe-project-cancellation/1");
    assert!(!cwd.path().join("rel").exists());
}

#[test]
fn parent_traversal_settings_refuse_with_null_root_and_no_writes() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    for path in [
        user.settings.join("missing/../other"),
        user.settings.join("../other"),
    ] {
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .env("VIBE_SETTINGS", &path)
            .args(["user", "init", "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert!(envelope["user_project"]["root"].is_null());
        assert!(
            envelope["user_project"]["reason"]
                .as_str()
                .unwrap()
                .contains("parent traversal")
        );
        assert_eq!(entries(&user.settings), Vec::<String>::new());
    }
}

#[test]
fn unsupported_component_refuses_before_any_settings_ancestor_is_created() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let settings = user.settings.join("new/.vibe-stage-reserved");
    for verb in ["init", "status"] {
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .env("VIBE_SETTINGS", &settings)
            .args(["user", verb, "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert!(envelope["user_project"]["root"].is_null());
        assert_eq!(envelope["user_project"]["outcome"], "refused");
        assert!(!user.settings.join("new").exists());
    }
}

#[test]
fn context_stamps_remain_transport_metadata_on_success_and_refusal() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args([
            "user",
            "status",
            "--json",
            "--invoked-by",
            "root-test",
            "--unattended",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value = report(&output);
    assert_eq!(value["invoked_by"], "root-test");
    assert_eq!(value["unattended"], true);
    let root = personal(&user);
    fs::create_dir(&root).unwrap();
    fs::write(root.join("foreign"), b"preserve").unwrap();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args([
            "user",
            "status",
            "--json",
            "--invoked-by",
            "root-test",
            "--unattended",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["invoked_by"], "root-test");
    assert_eq!(value["unattended"], true);
    assert_eq!(value["user_project"]["outcome"], "observed");
    assert_eq!(value["user_project"]["ok"], false);
    assert_eq!(value["user_project"].as_object().unwrap().len(), 10);
    assert_eq!(fs::read(root.join("foreign")).unwrap(), b"preserve");
}

#[test]
fn unavailable_status_nested_reports_never_claim_success() {
    let user = UserScratch::new();
    let cwd = elsewhere();
    let root = personal(&user);
    fs::create_dir(&root).unwrap();
    for text in ["private foreign bytes", "[project\nprivate = true"] {
        fs::write(root.join("vibe.toml"), text).unwrap();
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args(["user", "status", "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(value["user_project"]["ok"], false);
        assert_eq!(value["user_project"]["outcome"], "observed");
        assert_eq!(value["user_project"]["state"], "unavailable");
    }
}

#[cfg(unix)]
#[test]
fn mutating_failure_plain_and_quiet_keep_root_and_no_write_evidence() {
    use std::os::unix::fs::PermissionsExt;
    let user = UserScratch::new();
    let cwd = elsewhere();
    let root = personal(&user);
    fs::create_dir(&root).unwrap();
    for mode in ["--plain", "--quiet", "--json"] {
        fs::set_permissions(&root, fs::Permissions::from_mode(0o500)).unwrap();
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args(["user", "init", mode])
            .output()
            .unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(root.to_str().unwrap()), "{stderr}");
        if mode == "--json" {
            let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(value["user_project"]["effects"], "none");
            assert_eq!(value["user_project"]["publication"], "not-started");
            assert_eq!(value["user_project"]["outcome"], "refused");
        }
        assert!(entries(&root).is_empty());
    }
}
