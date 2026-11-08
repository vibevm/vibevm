//! `[[registry]].type`, the project-local override layer and
//! `vibe registry add|list|remove --scope/--type`
//! (PROP-002 `#registry-type`, `#registry-layers`).

mod common;

use std::fs;
use std::path::Path;

use common::{UserScratch, make_wal_dir_registry};

fn json(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json report")
}

fn file_url(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    if raw.starts_with('/') {
        format!("file://{raw}")
    } else {
        format!("file:///{raw}")
    }
}

fn registry_names(list: &serde_json::Value) -> Vec<(String, String, String)> {
    list["registries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap().to_string(),
                row["type"].as_str().unwrap().to_string(),
                row["scope"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn add_writes_each_scope_with_a_case_insensitive_type() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    for (scope, name, kind, url) in [
        (
            "local",
            "corp",
            "MonoRepo",
            "ssh://git@git.corp.example/tools/vibe.git",
        ),
        ("project", "corp", "github", "https://github.com/corp"),
        ("user", "drop", "LocalDir", "file:///srv/vibe"),
    ] {
        user.vibe()
            .args([
                "registry", "add", name, url, "--type", kind, "--scope", scope, "--path",
            ])
            .arg(project.path())
            .assert()
            .success();
    }

    let local = fs::read_to_string(project.path().join(".vibe/registry.local.toml")).unwrap();
    assert!(local.contains("type = \"monorepo\""), "{local}");
    let user_file = fs::read_to_string(user.settings.join("registry.toml")).unwrap();
    assert!(user_file.contains("type = \"localdir\""), "{user_file}");
    let manifest = fs::read_to_string(project.path().join("vibe.toml")).unwrap();
    assert!(manifest.contains("type = \"github\""), "{manifest}");

    // The merged view: the local row hides the project's `corp`; the user
    // row follows. Each row names its layer and effective type.
    let list = json(
        &user
            .vibe()
            .args(["--json", "registry", "list", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let rows = registry_names(&list);
    assert_eq!(rows[0], ("corp".into(), "monorepo".into(), "local".into()));
    assert!(rows.contains(&("drop".into(), "localdir".into(), "user".into())));
    assert!(!rows.iter().any(|row| row.2 == "project" && row.0 == "corp"));

    // One layer only.
    let project_only = json(
        &user
            .vibe()
            .args(["--json", "registry", "list", "--scope", "project", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    assert_eq!(
        registry_names(&project_only),
        vec![("corp".into(), "github".into(), "project".into())]
    );

    user.vibe()
        .args([
            "registry", "remove", "registry", "corp", "--scope", "local", "--path",
        ])
        .arg(project.path())
        .assert()
        .success();
    let local = fs::read_to_string(project.path().join(".vibe/registry.local.toml")).unwrap();
    assert!(!local.contains("corp"), "{local}");
}

/// `--scope user|local` edits `registry.toml` / `registry.local.toml` in
/// place: comments and unrelated tables keep their exact bytes, and a
/// removal takes away exactly what the add put there.
#[test]
fn user_and_local_scope_writes_keep_comments_and_unrelated_tables() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let original = "# machine registries — keep this note\n\
                    [[registry]]\n\
                    # the public org\n\
                    name = 'vibespecs'\n\
                    url = 'https://github.com/vibespecs'   # canonical\n\n\
                    [source_policy]\n\
                    # never github here\n\
                    exclude_hosting = ['github']\n";
    let files = [
        ("user", user.settings.join("registry.toml")),
        ("local", project.path().join(".vibe/registry.local.toml")),
    ];
    for (scope, file) in &files {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, original).unwrap();
        user.vibe()
            .args([
                "registry",
                "add",
                "corp",
                "https://git.corp.example/vibe",
                "--type",
                "git",
                "--scope",
                scope,
                "--path",
            ])
            .arg(project.path())
            .assert()
            .success();
        let written = fs::read_to_string(file).unwrap();
        let registries_end = original.find("[source_policy]").unwrap();
        assert!(
            written.starts_with(&original[..registries_end]),
            "{scope}: {written}"
        );
        assert!(
            written.ends_with(&original[registries_end..]),
            "{scope}: {written}"
        );
        assert!(written.contains("name = \"corp\""), "{scope}: {written}");

        user.vibe()
            .args([
                "registry", "remove", "registry", "corp", "--scope", scope, "--path",
            ])
            .arg(project.path())
            .assert()
            .success();
        assert_eq!(
            fs::read_to_string(file).unwrap(),
            original,
            "{scope}: remove undoes the add byte for byte"
        );
    }
}

#[test]
fn inferred_types_are_reported_and_monorepo_is_never_inferred() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    for (name, url) in [
        ("gh", "https://github.com/corp"),
        ("gv", "https://gitverse.ru/corp"),
        ("generic", "ssh://git@git.corp.example/tools/vibe.git"),
        ("dir", "file:///srv/vibe"),
    ] {
        user.vibe()
            .args(["registry", "add", name, url, "--path"])
            .arg(project.path())
            .assert()
            .success();
    }
    let manifest = fs::read_to_string(project.path().join("vibe.toml")).unwrap();
    assert!(
        !manifest.contains("type ="),
        "inferred types are not written: {manifest}"
    );
    let list = json(
        &user
            .vibe()
            .args(["--json", "registry", "list", "--scope", "project", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let types: Vec<String> = registry_names(&list).into_iter().map(|row| row.1).collect();
    assert_eq!(types, ["github", "gitverse", "git", "localdir"]);
}

#[test]
fn bad_type_values_are_refused_with_the_choices() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    let unknown = user
        .vibe()
        .args([
            "registry",
            "add",
            "x",
            "https://git.corp/x",
            "--type",
            "svn",
            "--path",
        ])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!unknown.status.success());
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(stderr.contains("unknown registry type `svn`"), "{stderr}");
    assert!(stderr.contains("monorepo, localdir"), "{stderr}");

    let remote_dir = user
        .vibe()
        .args([
            "registry",
            "add",
            "x",
            "ssh://git@git.corp/x.git",
            "--type",
            "localdir",
            "--path",
        ])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!remote_dir.status.success());
    let stderr = String::from_utf8_lossy(&remote_dir.stderr);
    assert!(stderr.contains("#registry-type"), "{stderr}");

    // A hand-written unknown type in the local override is refused when read.
    fs::write(
        project.path().join(".vibe/registry.local.toml"),
        "[[registry]]\nname = \"x\"\ntype = \"svn\"\nurl = \"file:///x\"\n",
    )
    .unwrap();
    let list = user
        .vibe()
        .args(["registry", "list", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!list.status.success());
    let stderr = String::from_utf8_lossy(&list.stderr);
    assert!(stderr.contains("registry.local.toml"), "{stderr}");
}

/// A registry declared only in the never-committed local override serves
/// an install, and a `localdir` type on a bare path is read as a directory.
#[test]
fn install_resolves_through_the_project_local_override() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = make_wal_dir_registry(project.path());
    fs::write(
        project.path().join(".vibe/registry.local.toml"),
        format!(
            "[[registry]]\nname = \"drop\"\ntype = \"LocalDir\"\nurl = \"{}\"\n",
            registry.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    user.vibe()
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["install", "org.vibevm.world/wal", "--assume-yes", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let lock = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    assert!(lock.contains("registry = \"drop\""), "{lock}");
    assert!(
        lock.contains(&file_url(&registry)) || lock.contains("wal-registry"),
        "{lock}"
    );
    // `vibe init` keeps the override out of version control.
    let ignore = fs::read_to_string(project.path().join(".vibe/.gitignore")).unwrap();
    assert_eq!(ignore.trim(), "*");
}
