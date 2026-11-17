//! Ordinary install owns authority production; init and read commands do not.
mod common;
use common::UserScratch;
use std::{fs, path::Path};
fn authority(root: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(root.join(".vibe/specmap-authority.json")).unwrap()).unwrap()
}
#[test]
fn member_path_owner_replays_from_sibling_without_granting_sibling_authority() {
    let user = UserScratch::new();
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    fs::write(
        root.join("vibe.toml"),
        "[project]\nname='mono'\nversion='1.0.0'\n[workspace]\nmembers=['a','b']\n",
    )
    .unwrap();
    for name in ["a", "b", "dependency"] {
        fs::create_dir_all(root.join(name)).unwrap();
    }
    fs::write(root.join("a/vibe.toml"), "[project]\nname='a'\nversion='1.0.0'\n[requires.packages]\n'org.authority/path'={path='../dependency'}\n").unwrap();
    fs::write(
        root.join("b/vibe.toml"),
        "[project]\nname='b'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(
        root.join("dependency/vibe.toml"),
        "[package]\ngroup='org.authority'\nname='path'\nversion='1.0.0'\nkind='flow'\n",
    )
    .unwrap();
    for name in ["a", "b", "a"] {
        let node = root.join(name);
        user.vibe()
            .args(["install", "--path"])
            .arg(&node)
            .args(["--offline", "--json"])
            .assert()
            .success();
        let value = authority(&node);
        assert_eq!(
            value["dependencies"].as_array().unwrap().len(),
            usize::from(name == "a")
        );
    }
}
#[test]
fn authored_path_root_installs_and_is_exact_authority() {
    let user = UserScratch::new();
    let outer = tempfile::tempdir().unwrap();
    let project = outer.path().join("project");
    let dependency = outer.path().join("dependency");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&dependency).unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    fs::write(project.join("vibe.toml"), "[project]\nname='path-demo'\nversion='1.0.0'\n[requires.packages]\n'org.authority/path'={path='../dependency'}\n").unwrap();
    fs::write(
        dependency.join("vibe.toml"),
        "[package]\ngroup='org.authority'\nname='path'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    user.vibe()
        .args(["install", "--path"])
        .arg(&project)
        .args(["--offline", "--json"])
        .assert()
        .success();
    let value = authority(&project);
    assert_eq!(value["dependencies"][0]["coordinate"], "org.authority/path");
    assert_eq!(value["node_roots"]["."].as_array().unwrap().len(), 1);
    assert!(
        value["node_roots"]["."][0]
            .as_str()
            .unwrap()
            .contains("org.authority/path@=1.0.0")
    );
}
#[test]
fn legacy_reinstall_without_authority_repairs_boot_without_creating_context() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    user.vibe()
        .arg("reinstall")
        .arg(project.path())
        .args(["--json", "--assume-yes"])
        .assert()
        .success();
    assert!(!project.path().join(".vibe/specmap-authority.json").exists());
}
#[test]
fn workspace_projection_is_selected_and_repeated_publication_preserves_mtime() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='mono'\nversion='1.0.0'\n[workspace]\nmembers=['a','b']\n",
    )
    .unwrap();
    for name in ["a", "b"] {
        let node = root.path().join(name);
        fs::create_dir_all(&node).unwrap();
        fs::write(node.join("vibe.toml"), format!("[project]\nname='{name}'\nversion='1.0.0'\n[requires.packages]\n'org.authority/{name}'='*'\n")).unwrap();
        let package = registry.path().join(format!("org.authority/{name}/v1.0.0"));
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("vibe.toml"),
            format!(
                "[package]\ngroup='org.authority'\nname='{name}'\nversion='1.0.0'\nkind='flow'\n"
            ),
        )
        .unwrap();
    }
    for name in ["a", "b"] {
        let node = root.path().join(name);
        let install = || {
            user.vibe()
                .args(["install", "--path"])
                .arg(&node)
                .arg("--registry")
                .arg(registry.path())
                .args(["--offline", "--json"])
                .assert()
                .success();
        };
        install();
        let value = authority(&node);
        assert_eq!(value["dependencies"].as_array().unwrap().len(), 1);
        assert_eq!(
            value["dependencies"][0]["coordinate"],
            format!("org.authority/{name}")
        );
        let path = node.join(".vibe/specmap-authority.json");
        let bytes = fs::read(&path).unwrap();
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        for _ in 0..4 {
            install();
        }
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), mtime);
    }
}
#[test]
fn empty_ready_and_noop_install_publish_current_selected_node_authority() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    assert!(!project.path().join(".vibe/specmap-authority.json").exists());
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    user.vibe()
        .args(["install", "--path"])
        .arg(project.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    let empty = authority(project.path());
    assert!(empty["dependencies"].as_array().unwrap().is_empty());
    let pkg = registry.path().join("org.authority/example/v1.0.0");
    fs::create_dir_all(pkg.join("vibevm/vibespecs/common")).unwrap();
    fs::write(
        pkg.join("vibe.toml"),
        "[package]\ngroup='org.authority'\nname='example'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(
        pkg.join("vibevm/vibespecs/common/RULE.md"),
        "# Rule {#rule}\n",
    )
    .unwrap();
    user.vibe()
        .args(["install", "org.authority/example", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    let ready = authority(project.path());
    assert_eq!(
        ready["dependencies"][0]["coordinate"],
        "org.authority/example"
    );
    let path = project.path().join(".vibe/specmap-authority.json");
    fs::remove_file(&path).unwrap();
    user.vibe()
        .args(["install", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    assert_eq!(authority(project.path()), ready);
    let before = fs::read(&path).unwrap();
    user.vibe()
        .args(["check", "--path"])
        .arg(project.path())
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
    let mtime = fs::metadata(&path).unwrap().modified().unwrap();
    user.vibe()
        .arg("reinstall")
        .arg(project.path())
        .args(["--json", "--assume-yes"])
        .assert()
        .success();
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), mtime);
    user.vibe()
        .args(["update", "org.authority/example", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .args(["--offline", "--json", "--assume-yes"])
        .assert()
        .success();
    assert_eq!(
        authority(project.path())["dependencies"][0]["coordinate"],
        "org.authority/example"
    );
    user.vibe()
        .args(["uninstall", "org.authority/example", "--path"])
        .arg(project.path())
        .args(["--json", "--assume-yes"])
        .assert()
        .success();
    assert!(
        authority(project.path())["dependencies"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
