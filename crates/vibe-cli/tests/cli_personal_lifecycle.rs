//! Actual personal package lifecycle using an isolated local registry.
mod common;
use common::UserScratch;
use std::{fs, path::Path};
use vibe_core::manifest::Lockfile;

fn package(registry: &Path, version: &str) {
    let root = registry.join(format!("org.personal/example/v{version}"));
    let source = vibe_core::layout::current_specs_root().join("boot/personal.md");
    fs::create_dir_all(root.join(source.parent().unwrap())).unwrap();
    fs::write(root.join("vibe.toml"), format!(
        "[package]\ngroup='org.personal'\nname='example'\nversion='{version}'\nkind='flow'\n[boot_snippet]\nsource='{}'\n", source.display()
    )).unwrap();
    fs::write(
        root.join(source),
        format!("# Personal\n\npersonal_marker_{version}\n"),
    )
    .unwrap();
}

#[test]
fn explicit_personal_install_list_check_update_uninstall_from_nonproject_directory() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    package(registry.path(), "1.0.0");
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .assert()
        .success();
    let root = user.settings.join("user-project");
    let manifest = fs::read_to_string(root.join("vibe.toml")).unwrap();
    fs::write(
        root.join("vibe.toml"),
        format!(
            "{manifest}\n[[registry]]\nname='personal-fixture'\nurl='file://{}'\n",
            registry.path().display()
        ),
    )
    .unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "--user-project",
            "org.personal/example",
            "--offline",
            "--json",
        ])
        .assert()
        .success();
    let locked = Lockfile::read(root.join("vibe.lock")).unwrap();
    assert_eq!(locked.packages[0].version.to_string(), "1.0.0");
    assert!(
        root.join(vibe_core::layout::current_boot_dir())
            .join("INDEX.md")
            .exists()
    );
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["list", "-g", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let listed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(listed["packages"][0]["name"], "example");
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args(["check", "-g", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    package(registry.path(), "1.1.0");
    user.vibe()
        .current_dir(cwd.path())
        .args(["update", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    assert_eq!(
        Lockfile::read(root.join("vibe.lock")).unwrap().packages[0]
            .version
            .to_string(),
        "1.1.0"
    );
    let before = fs::read(root.join("vibe.lock")).unwrap();
    for argv in [
        vec![
            "update",
            "-g",
            "org.personal/example",
            "--path",
            ".",
            "--json",
        ],
        vec![
            "update",
            "-g",
            "org.personal/example",
            "org.other/application",
            "--json",
        ],
    ] {
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args(argv)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(fs::read(root.join("vibe.lock")).unwrap(), before);
    }
    user.vibe()
        .current_dir(cwd.path())
        .args([
            "update",
            "-g",
            "org.personal/example",
            "--offline",
            "--json",
        ])
        .assert()
        .success();
    // Corrupt applicable application ownership is not absence permitting personal fallback.
    fs::create_dir_all(user.settings.join("applications")).unwrap();
    fs::write(
        user.settings.join("applications/index.json"),
        "invalid-json",
    )
    .unwrap();
    let failed = user
        .vibe()
        .current_dir(cwd.path())
        .args([
            "update",
            "-g",
            "org.personal/example",
            "--offline",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert_eq!(fs::read(root.join("vibe.lock")).unwrap(), before);
    fs::remove_file(user.settings.join("applications/index.json")).unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args([
            "uninstall",
            "-g",
            "org.personal/example",
            "--assume-yes",
            "--json",
        ])
        .assert()
        .success();
    assert!(
        Lockfile::read(root.join("vibe.lock"))
            .unwrap()
            .packages
            .is_empty()
    );
    assert!(fs::read_dir(cwd.path()).unwrap().next().is_none());
    assert!(!user.settings.join("mcp").exists());
    assert!(!user.settings.join("opt").exists());
}

#[test]
fn explicit_personal_flags_refuse_before_initialization_or_store_mutation() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    for args in [
        vec!["install", "--user-project", "org.personal/example"],
        vec![
            "install",
            "-g",
            "mcp:org.service/server",
            "org.personal/example",
        ],
        vec![
            "install",
            "-g",
            "--user-project",
            "--path",
            ".",
            "org.personal/example",
        ],
        vec![
            "install",
            "-g",
            "--user-project",
            "--configuration-input",
            "-",
            "org.personal/example",
        ],
        vec![
            "install",
            "-g",
            "--user-project",
            "mcp:org.personal/example",
        ],
        vec!["update", "-g", "--user-project", "--force"],
        vec![
            "uninstall",
            "-g",
            "--user-project",
            "--cascade",
            "org.personal/example",
        ],
    ] {
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args(args.clone())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(!user.settings.join("user-project").exists(), "{args:?}");
        assert!(!user.settings.join("mcp").exists());
        assert!(!user.settings.join("opt").exists());
    }
}

#[test]
fn personal_preview_and_preexisting_stop_leave_project_unmodified() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    package(registry.path(), "1.0.0");
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .assert()
        .success();
    let root = user.settings.join("user-project");
    let before = fs::read(root.join("vibe.toml")).unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "--user-project",
            "org.personal/example",
            "--plan-only",
            "--registry",
        ])
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    assert_eq!(fs::read(root.join("vibe.toml")).unwrap(), before);
    assert!(!root.join("vibe.lock").exists());
    assert!(!root.join(".vibe").exists());
    assert!(
        !root
            .join(vibe_core::layout::current_vibedeps_root())
            .exists()
    );
    let stop = cwd.path().join("stop");
    fs::write(&stop, b"stop").unwrap();
    let output = user
        .vibe()
        .current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "--user-project",
            "org.personal/example",
            "--registry",
        ])
        .arg(registry.path())
        .args(["--offline", "--json", "--stop-file"])
        .arg(&stop)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(root.join("vibe.toml")).unwrap(), before);
    assert!(!root.join("vibe.lock").exists());
    assert!(
        !root
            .join(vibe_core::layout::current_vibedeps_root())
            .exists()
    );
}

#[test]
fn automatic_global_spec_install_selects_personal_project_without_explicit_selector() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    package(registry.path(), "1.0.0");
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .assert()
        .success();
    let root = user.settings.join("user-project");
    let before = fs::read(root.join("vibe.toml")).unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "org.personal/example",
            "--plan-only",
            "--registry",
        ])
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    assert_eq!(fs::read(root.join("vibe.toml")).unwrap(), before);
    assert!(!root.join("vibe.lock").exists());
    assert!(!root.join(".vibe").exists());
    user.vibe()
        .current_dir(cwd.path())
        .args(["install", "-g", "org.personal/example", "--registry"])
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    let root = user.settings.join("user-project");
    let lock = Lockfile::read(root.join("vibe.lock")).unwrap();
    assert_eq!(lock.packages[0].version.to_string(), "1.0.0");
    assert!(!user.settings.join("opt").exists());
    assert!(fs::read_dir(cwd.path()).unwrap().next().is_none());
}

#[test]
fn automatic_personal_held_source_refuses_without_writes_and_explicit_uses_held_policy() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let held_registry = tempfile::tempdir().unwrap();
    let global_registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    package(held_registry.path(), "1.0.0");
    package(global_registry.path(), "2.0.0");
    user.vibe()
        .current_dir(cwd.path())
        .args(["user", "init", "--json"])
        .assert()
        .success();
    let root = user.settings.join("user-project");
    let manifest = fs::read_to_string(root.join("vibe.toml")).unwrap();
    fs::write(
        root.join("vibe.toml"),
        format!(
            "{manifest}\n[[registry]]\nname='held-fixture'\nurl='file://{}'\n",
            held_registry.path().display()
        ),
    )
    .unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "--user-project",
            "org.personal/example",
            "--offline",
            "--json",
        ])
        .assert()
        .success();
    let before_manifest = fs::read(root.join("vibe.toml")).unwrap();
    let before_lock = fs::read(root.join("vibe.lock")).unwrap();
    let before_index = fs::read(
        root.join(vibe_core::layout::current_boot_dir())
            .join("INDEX.md"),
    )
    .unwrap();
    for extra in [vec![], vec!["--plan-only"]] {
        let output = user
            .vibe()
            .current_dir(cwd.path())
            .args(["install", "-g", "org.personal/example", "--registry"])
            .arg(global_registry.path())
            .args(["--offline", "--json"])
            .args(extra)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(error.contains("--user-project"), "{error}");
        assert_eq!(fs::read(root.join("vibe.toml")).unwrap(), before_manifest);
        assert_eq!(fs::read(root.join("vibe.lock")).unwrap(), before_lock);
        assert_eq!(
            fs::read(
                root.join(vibe_core::layout::current_boot_dir())
                    .join("INDEX.md")
            )
            .unwrap(),
            before_index
        );
    }
    user.vibe()
        .current_dir(cwd.path())
        .args([
            "install",
            "-g",
            "--user-project",
            "org.personal/example",
            "--offline",
            "--json",
        ])
        .assert()
        .success();
    assert_eq!(
        Lockfile::read(root.join("vibe.lock")).unwrap().packages[0]
            .version
            .to_string(),
        "1.0.0"
    );
    assert!(!user.settings.join("mcp").exists());
    assert!(!user.settings.join("opt").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn global_application_does_not_consult_unavailable_personal_project() {
    let user = UserScratch::new();
    let cwd = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    let personal = user.settings.join("user-project");
    fs::create_dir_all(&personal).unwrap();
    fs::write(personal.join("vibe.toml"), "invalid manifest").unwrap();
    let app = registry.path().join("org.personal/example/v2.0.0");
    fs::create_dir_all(&app).unwrap();
    let installed = cwd.path().join("installed-app");
    fs::write(app.join("vibe.toml"), format!(
        "[package]\ngroup='org.personal'\nname='example'\nversion='2.0.0'\nkind='app'\n\
         [application]\nid='personal-priority-fixture'\n\
         [[installer_options]]\nid='shell'\nname='Shell'\nplatform='linux'\nplatform_name='Linux'\narch='{}'\n\
         [installer_options.installation]\ntype='shell'\napp_root='{}'\ninstall=\"mkdir -p '{}'\"\nforce_install='true'\nuninstall='true'\nforce_uninstall='true'\n",
        std::env::consts::ARCH, installed.display(), installed.display()
    )).unwrap();
    user.vibe()
        .current_dir(cwd.path())
        .args(["install", "-g", "org.personal/example", "--registry"])
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    assert!(installed.is_dir());
    assert_eq!(
        fs::read_to_string(personal.join("vibe.toml")).unwrap(),
        "invalid manifest"
    );
    assert!(!personal.join("vibe.lock").exists());
    assert!(user.settings.join("applications/index.json").exists());
}
