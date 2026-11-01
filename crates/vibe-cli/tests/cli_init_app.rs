//! `vibe init --kind app` scaffolds an application package that plans
//! (PROP-059 `#declaration`): the desktop shell's "create an application
//! package" wizard runs this command and must not patch the result by
//! hand before `vibe check` and `vibe app plan` accept it.

use std::fs;
use std::path::Path;

use predicates::prelude::*;
use vibe_core::manifest::Manifest;

mod common;

use common::UserScratch;

fn recipe_rel() -> &'static str {
    if cfg!(windows) {
        "recipes/build.ps1"
    } else {
        "recipes/build.sh"
    }
}

/// Copy a package directory the way a registry holds it,
/// `<registry>/<group>/<name>/v<version>/`.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// `vibe check` on the project, then `vibe app plan` on the package copied
/// into a registry: both must succeed with nothing edited in between.
fn assert_checks_and_plans(user: &UserScratch, project: &Path, package: &Path, name: &str) {
    user.vibe()
        .args(["check", "--path"])
        .arg(project)
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest_validity").not());

    let registry = tempfile::tempdir().unwrap();
    copy_tree(
        package,
        &registry.path().join(format!("org.example/{name}/v0.1.0")),
    );
    user.vibe()
        .args([
            "app",
            "plan",
            &format!("org.example/{name}@0.1.0"),
            "--registry",
        ])
        .arg(registry.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("build").and(predicate::str::contains("recipe")));
}

fn assert_declares_the_host(package: &Path, name: &str) {
    let manifest =
        Manifest::parse_str(&fs::read_to_string(package.join("vibe.toml")).unwrap()).unwrap();
    manifest.validate().unwrap();
    assert_eq!(manifest.application.as_ref().unwrap().id, name);
    let [option] = manifest.installer_options.as_slice() else {
        panic!("one installer option, got {:?}", manifest.installer_options);
    };
    assert_eq!(option.id, "main");
    assert_eq!(option.arch, std::env::consts::ARCH);
    assert!(option.launchers.is_empty(), "no launcher is invented");
    assert_eq!(option.steps.len(), 1);
    assert_eq!(option.steps[0].id, "build");

    let recipe = fs::read_to_string(package.join(recipe_rel())).unwrap();
    assert!(recipe.contains("echo"), "{recipe}");
    assert!(
        !recipe.contains('<'),
        "no angle-bracket placeholder: {recipe}"
    );
    assert!(package.join("icons/.gitkeep").is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(package.join(recipe_rel()))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "the recipe is executable");
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
fn root_app_package_checks_and_plans_as_scaffolded() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("demo");
    user.vibe()
        .args(["init", "--type", "package", "--group", "org.example"])
        .args(["--name", "demo", "--kind", "app", "--no-registry", "--path"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains(recipe_rel()))
        .stdout(predicate::str::contains("icons/.gitkeep"));

    assert_declares_the_host(&root, "demo");
    assert_checks_and_plans(&user, &root, &root, "demo");

    // A second init keeps what the first wrote and adds nothing.
    let recipe = root.join(recipe_rel());
    fs::write(&recipe, "echo edited\n").unwrap();
    user.vibe()
        .args(["init", "--kind", "app", "--path"])
        .arg(&root)
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&recipe).unwrap(), "echo edited\n");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
fn nested_app_package_checks_and_plans_as_scaffolded() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    user.vibe()
        .args([
            "init",
            "package",
            "org.example/tool",
            "--kind",
            "app",
            "--path",
        ])
        .arg(dir.path())
        .assert()
        .success();

    let package = dir.path().join(common::pack_rel("org.example/tool/v0.1.0"));
    assert_declares_the_host(&package, "tool");
    assert_checks_and_plans(&user, dir.path(), &package, "tool");
}
