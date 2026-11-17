//! Global routing preview never records remap offers or invokes installers.
mod common;
use common::UserScratch;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            let key = path.strip_prefix(root).unwrap().to_path_buf();
            if metadata.is_dir() {
                pending.push(path);
            } else {
                result.insert(key, (fs::read(path).unwrap(), metadata.modified().unwrap()));
            }
        }
    }
    result
}
#[test]
fn global_remap_plan_only_never_installs_an_offer() {
    let user = UserScratch::new();
    let registry = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    let package = registry.path().join("org.preview/remap/v1.0.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("vibe.toml"),"[package]\ngroup='org.preview'\nname='remap'\nkind='remap'\nversion='1.0.0'\n[remap]\ndescription='preview-only fixture'\n[[remap.rows]]\nprefix='https://example.invalid/source'\nurl='https://example.invalid/target'\nreason='fixture'\n").unwrap();
    let before_settings = snapshot(&user.settings);
    let before_cache = snapshot(&user.cache);
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "org.preview/remap",
            "--plan-only",
            "--registry",
        ])
        .arg(registry.path())
        .args(["--offline", "--json"])
        .output()
        .unwrap();
    assert!(
        !user.settings.join("remap").exists(),
        "preview recorded an offer: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!user.settings.join("user-project").exists());
    assert!(!user.settings.join("applications").exists());
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "preview refusal emitted a success document"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("--plan-only"));
    assert_eq!(snapshot(&user.settings), before_settings);
    assert_eq!(snapshot(&user.cache), before_cache);
}

#[test]
fn mixed_global_update_refuses_destination_cohort_before_service_refresh() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    let before = snapshot(&user.settings);
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args([
            "update",
            "-g",
            "mcp:org.preview/service",
            "org.preview/application",
            "--offline",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("destinations"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(snapshot(&user.settings), before);
    assert!(!user.settings.join("mcp").exists());
}

#[test]
fn inferred_and_explicit_personal_prestop_write_nothing() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let stop = cwd.path().join("stop");
    fs::write(&stop, "stop").unwrap();
    let before = snapshot(&user.settings);
    let inferred = user
        .vibe()
        .current_dir(cwd.path())
        .args(["install", "-g", "org.preview/personal", "--stop-file"])
        .arg(&stop)
        .args(["--offline", "--json"])
        .output()
        .unwrap();
    assert_eq!(inferred.status.code(), Some(130));
    let report: serde_json::Value = serde_json::from_slice(&inferred.stdout).unwrap();
    assert_eq!(report["protocol"], "vibe-project-cancellation/1");
    assert_eq!(report["outcome"], "cancelled");
    assert_eq!(snapshot(&user.settings), before);
    let explicit = user
        .vibe()
        .current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "--user-project",
            "org.preview/personal",
            "--stop-file",
        ])
        .arg(&stop)
        .args(["--offline", "--json"])
        .output()
        .unwrap();
    assert_eq!(explicit.status.code(), Some(130));
    assert_eq!(snapshot(&user.settings), before);
    assert!(!user.settings.join("user-project").exists());
}
