//! Root declaration selection retains the common consumer scaffold.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE");

use std::fs;
use std::path::Path;

use super::common::{self, UserScratch};
use vibe_core::manifest::Manifest;

fn manifest(root: &Path) -> Manifest {
    Manifest::parse_str(&fs::read_to_string(root.join("vibe.toml")).unwrap()).unwrap()
}

fn package_args() -> [&'static str; 12] {
    [
        "init",
        "--type",
        "package",
        "--group",
        "org.example",
        "--name",
        "demo",
        "--version",
        "1.2.3",
        "--kind",
        "flow",
        "--no-registry",
    ]
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn root_package_has_consumer_scaffold_and_metadata_roundtrip() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let author = "Oleg \"the author\" Chirukhin";
    let second_author = "Second \\ Author";
    let license = "Custom \"license\"";
    let description = "A quoted \"description\" with a \\ slash";
    user.vibe()
        .args(package_args())
        .args(["--author", author, "--author", second_author])
        .args(["--license", license, "--description", description])
        .args(["--format", "normal", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    let parsed = manifest(dir.path());
    parsed.validate().unwrap();
    assert!(parsed.project.is_none());
    let package = parsed.require_package().unwrap();
    assert_eq!(package.group.to_string(), "org.example");
    assert_eq!(package.name, "demo");
    assert_eq!(package.version.to_string(), "1.2.3");
    assert_eq!(package.kind.as_str(), "flow");
    assert_eq!(package.authors, [author, second_author]);
    assert_eq!(package.license.as_deref(), Some(license));
    assert_eq!(package.description.as_deref(), Some(description));
    assert_eq!(package.format, vibe_core::manifest::PackageFormat::Normal);
    assert!(parsed.registries.is_empty());
    for rel in [
        "vibe.lock".to_string(),
        "README.md".to_string(),
        "CLAUDE.md".to_string(),
        "AGENTS.md".to_string(),
        "GEMINI.md".to_string(),
        common::boot_rel("00-core.md"),
        common::boot_rel("90-user.md"),
        common::index_rel(),
    ] {
        assert!(dir.path().join(&rel).is_file(), "missing {rel}");
    }
    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(dir.path().join("vibe.lock")).unwrap()).unwrap();
    assert!(lock.packages.is_empty());
    assert!(
        !dir.path()
            .join(common::pack_rel("org.example/demo/v1.2.3"))
            .exists(),
        "root role selection must not create a nested package"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn root_project_can_declare_group_without_creating_package() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    user.vibe()
        .args([
            "init",
            "--type",
            "project",
            "--group",
            "org.example",
            "--name",
            "consumer",
            "--version",
            "2.3.4",
            "--path",
        ])
        .arg(dir.path())
        .assert()
        .success();
    let parsed = manifest(dir.path());
    assert!(parsed.package.is_none());
    let project = parsed.require_project().unwrap();
    assert_eq!(project.group.as_ref().unwrap().to_string(), "org.example");
    assert_eq!(project.name, "consumer");
    assert_eq!(project.version, "2.3.4");
    assert!(
        !dir.path()
            .join(common::pack_rel("org.example/consumer"))
            .exists()
    );
    assert!(dir.path().join(common::index_rel()).is_file());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn noninteractive_group_without_type_retains_project_default() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    user.vibe()
        .args([
            "init",
            "--group",
            "org.example",
            "--name",
            "consumer",
            "--path",
        ])
        .arg(dir.path())
        .assert()
        .success();
    let parsed = manifest(dir.path());
    assert!(parsed.package.is_none());
    let project = parsed.require_project().unwrap();
    assert_eq!(project.group.as_ref().unwrap().to_string(), "org.example");
    assert_eq!(project.version, "0.0.1");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn root_package_reinit_preserves_authored_files_and_role() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    user.vibe()
        .args(package_args())
        .arg("--path")
        .arg(dir.path())
        .assert()
        .success();
    let manifest_path = dir.path().join("vibe.toml");
    let original = fs::read_to_string(&manifest_path).unwrap();
    let edited = format!("# User-owned manifest comment\n{original}");
    fs::write(&manifest_path, &edited).unwrap();
    let boot_path = dir.path().join(common::boot_rel("90-user.md"));
    let boot = "# User-owned boot content\n";
    fs::write(&boot_path, boot).unwrap();

    for explicit in [true, false] {
        let mut command = user.vibe();
        if explicit {
            command.args(package_args());
        } else {
            command.arg("init");
        }
        command.arg("--path").arg(dir.path()).assert().success();
        assert_eq!(fs::read_to_string(&manifest_path).unwrap(), edited);
        assert_eq!(fs::read_to_string(&boot_path).unwrap(), boot);
        assert!(manifest(dir.path()).project.is_none());
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn invalid_root_identity_fails_before_scaffold_and_settings_writes() {
    let invalid: &[&[&str]] = &[
        &["--type", "package", "--name", "demo"],
        &["--type", "package", "--group", "invalid", "--name", "demo"],
        &["--type", "project", "--group", "invalid"],
        &[
            "--type",
            "package",
            "--group",
            "org.example",
            "--name",
            "bad/name",
        ],
        &[
            "--type",
            "package",
            "--group",
            "org.example",
            "--name",
            "demo",
            "--version",
            "bad",
        ],
        &[
            "--type",
            "package",
            "--group",
            "org.example",
            "--name",
            "demo",
            "--format",
            "bad",
        ],
        &[
            "--type",
            "package",
            "--group",
            "org.example",
            "--name",
            "demo",
            "--kind",
            "bad",
        ],
        // MCP identity is invalid without a declared server; init must not
        // invent one or write a partial scaffold before manifest validation.
        &[
            "--type",
            "package",
            "--group",
            "org.example",
            "--name",
            "demo",
            "--kind",
            "mcp",
        ],
        &["--type", "unknown"],
        &["package", "org.example/demo", "--type", "package"],
        &["group", "org.example", "--type", "project"],
        &["package", "org.example/demo", "--group", "org.example"],
        &["group", "org.example", "--group", "org.example"],
        &["--type", "project", "--name", ""],
        &["--group", "Org.Example"],
        &["--group", "org.пример"],
        &["--group", "org..example"],
        &["--group", "org.example/foreign"],
        &["package", "org.example/demo", "--name", "other"],
        &["package", "org.example/demo", "--version", "bad"],
        &["package", "org.example/demo", "--format", "bad"],
        &["package", "org.example/demo", "--link", "bad"],
        &["package", "org.example/demo", "--kind", "mcp"],
        &[
            "package",
            "org.example/demo",
            "--description",
            "unescaped \"quote\"",
        ],
        &[
            "package",
            "org.example/demo",
            "--description",
            "escaped\\nnewline",
        ],
        &[
            "package",
            "org.example/demo",
            "--kind",
            "doc",
            "--license",
            "unescaped \"quote\"",
        ],
        &["first-directory", "second-directory"],
        &["--translates", "org.example/docs"],
    ];
    for args in invalid {
        let user = UserScratch::new();
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("not-created");
        let settings = user.settings.join("config.toml");
        let sentinel = "[init]\nlast_author = \"Prior author\"\n";
        fs::write(&settings, sentinel).unwrap();
        user.vibe()
            .arg("init")
            .args(*args)
            .args(["--author", "Different author", "--path"])
            .arg(&target)
            .assert()
            .failure();
        assert!(!target.exists(), "invalid {args:?} created a scaffold");
        assert_eq!(
            fs::read_to_string(&settings).unwrap(),
            sentinel,
            "invalid {args:?} changed settings"
        );
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn invalid_package_names_are_refused_in_root_and_nested_forms() {
    for name in [
        "-package",
        "package-",
        "package--name",
        "foreign/name",
        "имя",
    ] {
        for nested in [false, true] {
            let user = UserScratch::new();
            let dir = tempfile::tempdir().unwrap();
            let target = dir.path().join("not-created");
            let settings = user.settings.join("config.toml");
            let sentinel = "[init]\nlast_author = \"Prior author\"\n";
            fs::write(&settings, sentinel).unwrap();
            let mut command = user.vibe();
            command.arg("init");
            if nested {
                command.args(["package", &format!("org.example/{name}")]);
            } else {
                command.args(["--type", "package", "--group", "org.example"]);
                command.arg(format!("--name={name}"));
            }
            command
                .args(["--author", "Changed author", "--path"])
                .arg(&target)
                .assert()
                .failure();
            assert!(
                !target.exists(),
                "invalid {name} (nested={nested}) wrote scaffold"
            );
            assert_eq!(fs::read_to_string(settings).unwrap(), sentinel);
        }
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn every_declaration_only_root_kind_preserves_format_and_quoted_metadata() {
    for kind in ["tool", "flow", "feat", "stack", "lang", "doc", "app"] {
        for format in ["normal", "simple"] {
            let user = UserScratch::new();
            let dir = tempfile::tempdir().unwrap();
            let author = "A \"Writer\"\\with a slash";
            let license = "License \"quoted\"\\text";
            let description = "Quoted \"prose\"\\path\nand a newline";
            user.vibe()
                .args([
                    "init",
                    "--type",
                    "package",
                    "--group",
                    "org.example",
                    "--name",
                    "package-name",
                    "--kind",
                    kind,
                    "--format",
                    format,
                    "--version",
                    "2.3.4",
                    "--author",
                    author,
                    "--license",
                    license,
                    "--description",
                    description,
                    "--no-registry",
                    "--path",
                ])
                .arg(dir.path())
                .assert()
                .success();
            let parsed = manifest(dir.path());
            parsed.validate().unwrap();
            let meta = parsed.require_package().unwrap();
            assert_eq!(meta.kind.as_str(), kind);
            assert_eq!(meta.version.to_string(), "2.3.4");
            assert_eq!(meta.authors, [author]);
            assert_eq!(meta.license.as_deref(), Some(license));
            assert_eq!(meta.description.as_deref(), Some(description));
            assert_eq!(
                meta.format,
                if format == "normal" {
                    vibe_core::manifest::PackageFormat::Normal
                } else {
                    vibe_core::manifest::PackageFormat::Simple
                }
            );
            if kind == "doc" {
                assert!(meta.title.is_some());
                assert!(meta.abstract_text.is_some());
                assert!(!parsed.documents.is_empty());
                assert!(dir.path().join(common::spec_rel("README.xml")).is_file());
            }
        }
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn changed_identity_and_workspace_conversion_preserve_existing_bytes() {
    for initial in ["project", "package", "workspace"] {
        let user = UserScratch::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vibe.toml");
        if initial == "workspace" {
            fs::write(&path, "# Authored workspace\n[workspace]\nmembers = []\n").unwrap();
        } else {
            user.vibe()
                .args([
                    "init",
                    "--type",
                    initial,
                    "--group",
                    "org.example",
                    "--name",
                    "demo",
                    "--path",
                ])
                .arg(dir.path())
                .assert()
                .success();
        }
        let before = fs::read(&path).unwrap();
        let settings_path = user.settings.join("config.toml");
        let settings_before = fs::read(&settings_path).ok();
        for flags in [
            ["--group", "org.other"],
            ["--name", "other-name"],
            ["--type", "package"],
        ] {
            if initial == "package" && flags[0] == "--type" {
                continue;
            }
            user.vibe()
                .arg("init")
                .args(flags)
                .args(["--author", "Changed author", "--path"])
                .arg(dir.path())
                .assert()
                .failure();
            assert_eq!(fs::read(&path).unwrap(), before);
            assert_eq!(fs::read(&settings_path).ok(), settings_before);
        }
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn explicit_root_role_cannot_convert_existing_manifest() {
    for initial in ["project", "package"] {
        let user = UserScratch::new();
        let dir = tempfile::tempdir().unwrap();
        user.vibe()
            .args([
                "init",
                "--type",
                initial,
                "--group",
                "org.example",
                "--name",
                "demo",
                "--path",
            ])
            .arg(dir.path())
            .assert()
            .success();
        let path = dir.path().join("vibe.toml");
        let before = fs::read(&path).unwrap();
        let settings_path = user.settings.join("config.toml");
        let settings_before = fs::read(&settings_path).ok();
        let other = if initial == "project" {
            "package"
        } else {
            "project"
        };
        user.vibe()
            .args([
                "init",
                "--type",
                other,
                "--group",
                "org.example",
                "--name",
                "demo",
                "--author",
                "Changed author",
                "--path",
            ])
            .arg(dir.path())
            .assert()
            .failure();
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(fs::read(settings_path).ok(), settings_before);
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn legacy_package_and_group_initializers_remain_nested() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "group", "org.example", "--path"])
        .arg(dir.path())
        .assert()
        .success();
    user.vibe()
        .args([
            "init",
            "package",
            "org.example/demo",
            "--kind",
            "flow",
            "--path",
        ])
        .arg(dir.path())
        .assert()
        .success();
    let root = manifest(dir.path());
    assert!(root.project.is_some());
    assert!(root.package.is_none());
    assert!(dir.path().join(common::pack_rel("org.example")).is_dir());
    let nested = dir.path().join(common::pack_rel("org.example/demo/v0.1.0"));
    assert_eq!(manifest(&nested).require_package().unwrap().name, "demo");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn hyphenated_names_roundtrip_in_root_and_nested_packages() {
    for nested in [false, true] {
        let user = UserScratch::new();
        let dir = tempfile::tempdir().unwrap();
        let mut command = user.vibe();
        command.arg("init");
        if nested {
            command.args(["package", "org.example/package-name"]);
        } else {
            command.args([
                "--type",
                "package",
                "--group",
                "org.example",
                "--name",
                "package-name",
            ]);
        }
        command
            .args(["--no-registry", "--path"])
            .arg(dir.path())
            .assert()
            .success();
        let package_root = if nested {
            dir.path()
                .join(common::pack_rel("org.example/package-name/v0.1.0"))
        } else {
            dir.path().to_path_buf()
        };
        let parsed = manifest(&package_root);
        parsed.validate().unwrap();
        assert_eq!(parsed.require_package().unwrap().name, "package-name");
    }
}
