//! Declared replacement authority is shared by capture, compose and layered views.
mod common;
use common::UserScratch;
use std::{fs, path::Path};

fn spec(root: &Path, name: &str) {
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let mut parsed = vibe_core::manifest::Manifest::parse_str(&text).unwrap();
    let project = parsed.project.as_mut().unwrap();
    project.group = Some(vibe_core::Group::parse("org.relations").unwrap());
    project.name = if name == "PROJECT" {
        "test-project"
    } else {
        "vibe-user"
    }
    .into();
    parsed.write(&manifest).unwrap();
    let dir = root
        .join(vibe_core::layout::current_specs_root())
        .join("common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("STYLE.md"),
        format!("# Style {{#width}}\n\n{name}_AUTHORED\n"),
    )
    .unwrap();
}
fn run(user: &UserScratch, root: &Path, args: &[&str]) -> serde_json::Value {
    let output = user.vibe().current_dir(root).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn append(root: &Path, text: &str) {
    let p = root.join("vibe.toml");
    fs::write(&p, fs::read_to_string(&p).unwrap() + text).unwrap();
}
#[test]
fn explicit_replacement_is_visible_in_tree_capture_and_compose_and_protected_target_denies_layer() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    spec(project.path(), "PROJECT");
    append(
        project.path(),
        "\n[layers]\nschema=1\n[[layers.targets]]\nid='width'\nsource='spec://org.relations/test-project/common/STYLE#width'\npolicy='overridable'\n",
    );
    // Fixture project identity is intentionally taken from its actual manifest.
    let m = vibe_core::manifest::Manifest::read(project.path().join("vibe.toml")).unwrap();
    let name = m.project.unwrap().name;
    let p = project.path().join("vibe.toml");
    fs::write(
        &p,
        fs::read_to_string(&p).unwrap().replace(
            "org.relations/test-project",
            &format!("org.relations/{name}"),
        ),
    )
    .unwrap();
    let boot = project.path().join(vibe_core::layout::current_boot_dir());
    fs::write(boot.join("SYSTEM-TEMPLATE.md"), "PROJECT_PREFIX\n").unwrap();
    run(&user, project.path(), &["install", "--offline", "--json"]);
    user.vibe().args(["user", "init"]).assert().success();
    let personal = user.settings.join("user-project");
    spec(&personal, "USER");
    append(
        &personal,
        "\n[layers]\nschema=1\n[[layers.relations]]\nid='my-width'\nkind='replace'\ntarget_layer='project'\ntarget='width'\nsource='spec://org.relations/vibe-user/common/STYLE#width'\n",
    );
    fs::create_dir_all(personal.join(vibe_core::layout::current_boot_dir())).unwrap();
    fs::write(
        personal
            .join(vibe_core::layout::current_boot_dir())
            .join("preferences.md"),
        "PERSONAL_BOOT_PAYLOAD\n",
    )
    .unwrap();
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    let prefix = fs::read(boot.join("SYSTEM.md")).unwrap();
    let tree = run(&user, project.path(), &["tree", "--layers", "--json"]);
    assert_eq!(tree["schema_version"], 2);
    assert_eq!(tree["layers"][1]["relations"][0]["state"], "active");
    let capture = run(
        &user,
        project.path(),
        &[
            "prompt",
            "capture",
            "--session-target",
            "os=linux",
            "--with-text",
            "--json",
        ],
    );
    assert!(
        capture
            .to_string()
            .contains("Declared layer relation my-width")
    );
    let composed = run(
        &user,
        project.path(),
        &[
            "prompt",
            "compose",
            "--session-target",
            "os=linux",
            "--json",
        ],
    );
    let text = fs::read(composed["path"].as_str().unwrap()).unwrap();
    assert!(text.starts_with(&prefix));
    assert!(
        String::from_utf8(text)
            .unwrap()
            .contains("Declared layer relation my-width")
    );
    #[cfg(unix)]
    actual_relation_launch(&user, project.path());
    // A lower owner revokes its grant; the optional user layer is skipped, not
    // stripped out of the already compiled shared prefix or silently authorized.
    fs::write(
        &p,
        fs::read_to_string(&p)
            .unwrap()
            .replace("policy='overridable'", "policy='protected'"),
    )
    .unwrap();
    run(&user, project.path(), &["prompt", "build", "--json"]);
    let tree = run(&user, project.path(), &["tree", "--layers", "--json"]);
    assert_eq!(tree["layers"][1]["relations"][0]["state"], "denied");
    let capture = run(
        &user,
        project.path(),
        &[
            "prompt",
            "capture",
            "--session-target",
            "os=linux",
            "--with-text",
            "--json",
        ],
    );
    assert!(!capture.to_string().contains("PERSONAL_BOOT_PAYLOAD"));
}

#[test]
fn package_declared_targets_keep_owner_and_version_across_independent_layers() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    for version in ["1.0.0", "2.0.0"] {
        let root = registry
            .path()
            .join(format!("org.relations/style/v{version}"));
        fs::create_dir_all(root.join("vibevm/vibespecs/common")).unwrap();
        let relation = if version == "2.0.0" {
            "[[layers.relations]]\nid='personal-width'\nkind='replace'\ntarget_layer='project'\ntarget='width'\nsource='spec://org.relations/style/common/STYLE#width'\n"
        } else {
            ""
        };
        fs::write(root.join("vibe.toml"),format!("[package]\ngroup='org.relations'\nname='style'\nkind='flow'\nversion='{version}'\n[boot_snippet]\nsource='vibevm/vibespecs/common/STYLE.md'\n[layers]\nschema=1\n[[layers.targets]]\nid='width'\nsource='spec://org.relations/style/common/STYLE#width'\npolicy='overridable'\n{relation}")).unwrap();
        fs::write(
            root.join("vibevm/vibespecs/common/STYLE.md"),
            format!("# Style {{#width}}\n\nPACKAGE_WIDTH_{version}\n"),
        )
        .unwrap();
    }
    let output = user
        .vibe()
        .args(["install", "org.relations/style@=1.0.0", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .args(["--offline", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(
        project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("SYSTEM-TEMPLATE.md"),
        "PACKAGE_PROJECT_PREFIX\n",
    )
    .unwrap();
    run(&user, project.path(), &["prompt", "build", "--json"]);
    user.vibe().args(["user", "init"]).assert().success();
    user.vibe()
        .args([
            "install",
            "-g",
            "--user-project",
            "org.relations/style@=2.0.0",
            "--registry",
        ])
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    let tree = run(&user, project.path(), &["tree", "--layers", "--json"]);
    let relation = &tree["layers"][1]["relations"][0];
    assert_eq!(relation["state"], "active");
    assert_eq!(relation["source_owner"], "org.relations/style");
    assert_eq!(relation["target_owner"], "org.relations/style");
    assert_eq!(relation["source_version"], "2.0.0");
    assert_eq!(relation["target_version"], "1.0.0");
    let plain = user
        .vibe()
        .current_dir(project.path())
        .args(["tree", "--layers"])
        .output()
        .unwrap();
    assert!(plain.status.success());
    let plain = String::from_utf8(plain.stdout).unwrap();
    assert!(plain.contains("org.relations/style@2.0.0"), "{plain}");
    assert!(plain.contains("org.relations/style@1.0.0"), "{plain}");
    let capture = run(
        &user,
        project.path(),
        &[
            "prompt",
            "capture",
            "--session-target",
            "os=linux",
            "--with-text",
            "--json",
        ],
    );
    assert!(
        capture.to_string().contains(
            "Declared effective source unit spec://org.relations/style/common/STYLE#width"
        )
    );
    user.vibe()
        .args(["user", "disable", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let disabled = run(
        &user,
        project.path(),
        &[
            "prompt",
            "capture",
            "--session-target",
            "os=linux",
            "--with-text",
            "--json",
        ],
    );
    assert!(!disabled.to_string().contains("PACKAGE_WIDTH_2.0.0"));
    assert!(disabled.to_string().contains("PACKAGE_WIDTH_1.0.0"));
    let tree = run(&user, project.path(), &["tree", "--layers", "--json"]);
    assert_eq!(tree["layers"][1]["relations"][0]["state"], "disabled");
    assert_eq!(
        tree["layers"][1]["relations"][0]["source_owner"],
        "org.relations/style"
    );
    assert_eq!(tree["layers"][1]["relations"][0]["source_version"], "2.0.0");
}

#[cfg(unix)]
fn actual_relation_launch(user: &UserScratch, project: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let shim = tempfile::tempdir().unwrap();
    let executable = shim.path().join("record-agent");
    let argv = shim.path().join("argv");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$RELATION_TEST_ARGV\"\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        user.settings.join("agents.toml"),
        format!(
            "[launchers.relation-test]\nkind='claude'\ncommand=['{}']\n",
            executable.display()
        ),
    )
    .unwrap();
    let launch = || {
        user.vibe()
            .current_dir(project)
            .env("RELATION_TEST_ARGV", &argv)
            .args(["agent", "relation-test"])
            .assert()
            .success();
        let arguments = fs::read_to_string(&argv).unwrap();
        let arguments: Vec<_> = arguments.lines().collect();
        let path = arguments
            .windows(2)
            .find(|p| p[0] == "--append-system-prompt-file")
            .unwrap()[1];
        fs::read_to_string(path).unwrap()
    };
    let enabled = launch();
    assert!(enabled.contains("Declared layer relation my-width"));
    assert!(enabled.contains("USER_AUTHORED"));
    user.vibe()
        .args(["user", "disable", "--path"])
        .arg(project)
        .assert()
        .success();
    let disabled = launch();
    assert!(!disabled.contains("Declared layer relation my-width"));
    assert!(!disabled.contains("PERSONAL_BOOT_PAYLOAD"));
    user.vibe()
        .args(["user", "enable", "--path"])
        .arg(project)
        .assert()
        .success();
    assert!(launch().contains("Declared layer relation my-width"));
}
