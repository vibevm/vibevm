//! Personal read commands reuse ordinary list/check without implicit initialization.
mod common;
use common::UserScratch;
use std::{collections::BTreeMap, fs, path::Path};

fn snapshot(root: &Path) -> BTreeMap<String, (Vec<u8>, std::time::SystemTime)> {
    fn visit(
        root: &Path,
        dir: &Path,
        out: &mut BTreeMap<String, (Vec<u8>, std::time::SystemTime)>,
    ) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            assert!(!metadata.file_type().is_symlink());
            out.insert(
                path.strip_prefix(root).unwrap().display().to_string(),
                (
                    if metadata.is_file() {
                        fs::read(&path).unwrap()
                    } else {
                        Vec::new()
                    },
                    metadata.modified().unwrap(),
                ),
            );
            if metadata.is_dir() {
                visit(root, &path, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

#[test]
fn personal_reads_refuse_absent_and_invalid_without_creating_or_repairing() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    for verb in ["list", "check"] {
        let before = snapshot(&user.settings);
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args([verb, "-g", "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("vibe user init"));
        assert_eq!(snapshot(&user.settings), before);
    }
    let root = user.settings.join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("vibe.toml"), "invalid [manifest").unwrap();
    for verb in ["list", "check"] {
        let before = snapshot(&user.settings);
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args([verb, "-g", "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert!(error.get("user_project").is_none(), "{error}");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("vibe-user-project/1"));
        assert_eq!(snapshot(&user.settings), before);
    }
}

#[test]
fn personal_list_and_check_use_selected_root_and_do_not_write() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .assert()
        .success();
    let root = user.settings.join("user-project");
    // Root status is not a health check: ordinary check must still report missing boot.
    let before = snapshot(&user.settings);
    let initial = user
        .vibe()
        .current_dir(cwd.path())
        .args(["check", "-g", "--json"])
        .output()
        .unwrap();
    assert!(!initial.status.success());
    let initial_report: serde_json::Value = serde_json::from_slice(&initial.stdout).unwrap();
    assert!(
        initial_report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["check"] == "boot_directory")
    );
    assert_eq!(snapshot(&user.settings), before);
    // Supply ordinary boot structure explicitly; read commands never create it.
    fs::create_dir_all(root.join(vibe_core::layout::current_boot_dir())).unwrap();
    let before = snapshot(&user.settings);
    let list = user
        .vibe()
        .current_dir(cwd.path())
        .args(["list", "-g", "--json"])
        .output()
        .unwrap();
    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(value["packages"], serde_json::json!([]));
    let check = user
        .vibe()
        .current_dir(cwd.path())
        .args(["check", "-g", "--json"])
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(value["project"], root.display().to_string());
    assert!(value["findings"].is_array());
    assert_eq!(snapshot(&user.settings), before);
    assert!(fs::read_dir(cwd.path()).unwrap().next().is_none());
}

#[test]
fn explicit_path_conflicts_even_when_dot_and_ordinary_path_remains_supported() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(
        cwd.path().join("vibe.toml"),
        "[project]\nname='ordinary'\nversion='1.0.0'\n",
    )
    .unwrap();
    for verb in ["list", "check"] {
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args([verb, "-g", "--path", "."])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
    user.vibe()
        .current_dir(cwd.path())
        .args(["list", "--path", ".", "--json"])
        .assert()
        .success();
    assert!(!user.settings.join("user-project").exists());
}

#[cfg(unix)]
#[test]
fn personal_reads_refuse_symlink_root_without_reading_target() {
    let user = UserScratch::new();
    let target = tempfile::tempdir().unwrap();
    fs::write(
        target.path().join("vibe.toml"),
        "[project]\nname='foreign'\nversion='1.0.0'\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(target.path(), user.settings.join("user-project")).unwrap();
    let before = snapshot(target.path());
    for verb in ["list", "check"] {
        let output = user.vibe().args([verb, "-g", "--json"]).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("vibe-user-project/1"));
    }
    assert_eq!(snapshot(target.path()), before);
}
