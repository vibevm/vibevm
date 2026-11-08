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

fn file_inventory(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(
                    vibe_core::machine_json_path(path.strip_prefix(root).unwrap()),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    files
}

fn init_args(nested: bool) -> Vec<&'static str> {
    if nested {
        vec![
            "--json",
            "init",
            "package",
            "org.example/selected",
            "--kind",
            "app",
            "--no-registry",
        ]
    } else {
        vec![
            "--json",
            "init",
            "--type",
            "package",
            "--group",
            "org.example",
            "--name",
            "selected",
            "--kind",
            "app",
            "--no-registry",
        ]
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
fn selected_app_scaffolds_share_models_and_report_all_root_and_nested_files() {
    use vibe_core::manifest::{ArchiveFormatDecl, ShellInterpreter, StepKind};
    for nested in [false, true] {
        for (way, platform, archive) in [
            ("recipe", "macos", None),
            ("source", "windows", None),
            ("source", "linux", None),
            ("prebuilt", "linux", Some("zip")),
            ("prebuilt", "windows", Some("tar-gz")),
        ] {
            let user = UserScratch::new();
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("project");
            let hash = "f".repeat(64);
            let mut args: Vec<&str> = init_args(nested);
            args.extend([
                "--app-id",
                "chosen-slot",
                "--app-platform",
                platform,
                "--app-arch",
                "aarch64",
                "--app-install-way",
                way,
            ]);
            if way != "recipe" {
                args.extend([
                    "--app-source-url",
                    if archive == Some("zip") {
                        "https://example.invalid/author-supplied.zip"
                    } else {
                        "https://example.invalid/author-supplied.tar.gz"
                    },
                    "--app-source-sha256",
                    &hash,
                ]);
            }
            if way == "source" {
                args.extend(["--app-build-command", if platform == "windows" { "Set-Content -Path (Join-Path $env:VIBE_OUT 'payload.txt') -Value 'authored payload'" } else { "printf payload > \"$VIBE_OUT/payload.txt\"" }]);
            }
            if let Some(format) = archive {
                args.extend([
                    "--app-archive-format",
                    format,
                    "--app-payload-root",
                    "payload",
                ]);
            }
            let output = user
                .vibe()
                .args(&args)
                .arg("--path")
                .arg(&root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            // One complete response, including a newly-created nested container.
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let paths: std::collections::BTreeSet<_> = report["outcomes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["path"].as_str().unwrap().to_owned())
                .collect();
            let files = file_inventory(&root);
            assert_eq!(paths, files.keys().cloned().collect(), "{report}");
            assert_eq!(
                report["outcomes"].as_array().unwrap().len(),
                paths.len(),
                "one row per file"
            );
            let package = if nested {
                root.join(common::pack_rel("org.example/selected/v0.1.0"))
            } else {
                root.clone()
            };
            let manifest = Manifest::read(package.join("vibe.toml")).unwrap();
            manifest.validate().unwrap();
            assert_eq!(manifest.application.as_ref().unwrap().id, "chosen-slot");
            let choice = vibe_app::InstallChoice {
                platform: Some(platform.into()),
                arch: Some("aarch64".into()),
                installer: Some("main".into()),
            };
            let option =
                vibe_app::select_option(&manifest.installer_options, &choice, None).unwrap();
            assert!(option.launchers.is_empty());
            match way {
                "source" => {
                    assert!(
                        matches!(&option.steps[0].kind, StepKind::Sources { sources } if sources.len() == 1 && sources[0].sha256 == hash)
                    );
                    let (interpreter, script, command_marker, absent) = if platform == "windows" {
                        (
                            ShellInterpreter::WindowsPowershell,
                            "recipes/build.ps1",
                            "Set-Content",
                            "recipes/build.sh",
                        )
                    } else {
                        (
                            ShellInterpreter::PosixSh,
                            "recipes/build.sh",
                            "printf payload",
                            "recipes/build.ps1",
                        )
                    };
                    assert!(
                        matches!(&option.steps[1].kind, StepKind::Recipe { interpreter: Some(found), script: path } if *found == interpreter && path == Path::new(script))
                    );
                    assert!(
                        fs::read_to_string(package.join(script))
                            .unwrap()
                            .contains(command_marker)
                    );
                    assert!(!package.join(absent).exists());
                }
                "prebuilt" => {
                    assert_eq!(option.steps.len(), 1);
                    assert!(
                        matches!(&option.steps[0].kind, StepKind::Archive { format, sha256, payload_root: Some(path), .. } if sha256 == &hash && path == Path::new("payload") && *format == if archive == Some("zip") { ArchiveFormatDecl::Zip } else { ArchiveFormatDecl::TarGz })
                    );
                    assert!(!package.join("recipes").exists());
                }
                _ => assert!(matches!(
                    &option.steps[0].kind,
                    StepKind::Recipe {
                        interpreter: Some(ShellInterpreter::PosixSh),
                        ..
                    }
                )),
            }
            user.vibe()
                .args(["check", "--path"])
                .arg(&root)
                .assert()
                .success();
            let registry = tempfile::tempdir().unwrap();
            copy_tree(
                &package,
                &registry.path().join("org.example/selected/v0.1.0"),
            );
            user.vibe()
                .args([
                    "--json",
                    "app",
                    "plan",
                    "org.example/selected@0.1.0",
                    "--platform",
                    platform,
                    "--arch",
                    "aarch64",
                    "--installer",
                    "main",
                    "--registry",
                ])
                .arg(registry.path())
                .assert()
                .success();
            let authored_manifest = fs::read(package.join("vibe.toml")).unwrap();
            let script = package.join(if platform == "windows" {
                "recipes/build.ps1"
            } else {
                "recipes/build.sh"
            });
            if way != "prebuilt" {
                fs::write(&script, "# author-edited recipe\n").unwrap();
            }
            user.vibe()
                .args(&args)
                .arg("--path")
                .arg(&root)
                .assert()
                .success();
            assert_eq!(
                fs::read(package.join("vibe.toml")).unwrap(),
                authored_manifest
            );
            if way != "prebuilt" {
                assert_eq!(
                    fs::read_to_string(&script).unwrap(),
                    "# author-edited recipe\n"
                );
            }
            let before = file_inventory(&root);
            let mut conflicting = args.clone();
            let index = conflicting
                .iter()
                .position(|arg| *arg == "chosen-slot")
                .unwrap();
            conflicting[index] = "another-slot";
            user.vibe()
                .args(conflicting)
                .arg("--path")
                .arg(&root)
                .assert()
                .failure();
            assert_eq!(
                file_inventory(&root),
                before,
                "selection refusal must precede writes"
            );
        }
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
fn unsupported_app_authoring_inputs_refuse_before_root_or_container_writes() {
    for nested in [false, true] {
        for selected in [
            vec!["--app-install-way", "prebuilt"],
            vec![
                "--app-install-way",
                "source",
                "--app-source-url",
                "https://example.invalid/source.zip",
                "--app-source-sha256",
                "invalid",
                "--app-build-command",
                "echo invalid-hash-control",
            ],
            vec![
                "--app-install-way",
                "source",
                "--app-source-url",
                "https://example.invalid/source.zip",
                "--app-source-sha256",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ],
            vec![
                "--app-install-way",
                "prebuilt",
                "--app-source-url",
                "https://example.invalid/archive.zip",
                "--app-source-sha256",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "--app-archive-format",
                "zip",
                "--app-payload-root",
                "../outside",
            ],
            vec!["--app-platform", "unknown-platform"],
            vec!["--app-id", "../outside"],
            vec![
                "--app-install-way",
                "prebuilt",
                "--app-build-command",
                "echo no",
            ],
            vec!["--app-install-way", "native"],
            vec![
                "--app-install-way",
                "recipe",
                "--app-source-url",
                "https://example.invalid/source.zip",
            ],
        ] {
            let user = UserScratch::new();
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("must-not-exist");
            user.vibe()
                .args(init_args(nested))
                .args(selected)
                .arg("--path")
                .arg(&root)
                .assert()
                .failure();
            assert!(!root.exists());
        }
    }
    for args in [
        vec!["init"],
        vec![
            "init",
            "--type",
            "package",
            "--group",
            "org.example",
            "--kind",
            "tool",
        ],
        vec!["init", "group", "org.example"],
    ] {
        let user = UserScratch::new();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("must-not-exist");
        user.vibe()
            .args(args)
            .args(["--app-id", "selected", "--path"])
            .arg(&root)
            .assert()
            .failure();
        assert!(!root.exists());
    }
}
