//! Self-service recipe creation from a non-project, with no private kit inputs.
#[path = "cli_kit/build.rs"]
mod build;
mod common;
#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[path = "cli_kit/upgrade_shim.rs"]
mod upgrade_shim;
use common::UserScratch;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Output};

fn command(user: &UserScratch, cwd: &Path, args: &[&str]) -> Output {
    let home = user.settings.parent().unwrap().join("isolated-home");
    fs::create_dir_all(&home).unwrap();
    let out = user
        .vibe()
        .current_dir(cwd)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .args(["--unattended", "--offline"])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(
        fs::read_dir(&home).unwrap().count(),
        0,
        "kit command wrote HOME"
    );
    for directory in [&user.settings, &user.cache, &user.search_cache] {
        assert_eq!(
            fs::read_dir(directory).unwrap().count(),
            0,
            "kit command wrote user state"
        );
    }
    out
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn good(out: Output) {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
fn bad(out: Output) {
    assert!(!out.status.success(), "unexpected success");
}

#[test]
fn kit_local_commands_ignore_unrelated_user_profile() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let settings = temp.path().join("settings");
    let cwd = temp.path().join("non-project");
    for path in [&home, &settings, &cwd] {
        fs::create_dir(path).unwrap();
    }
    let poisoned = "SYNTHETIC_PRIVATE_CANARY = [\n";
    for file in [
        "config.toml",
        "site-profile.toml",
        "registry.toml",
        "settings.toml",
    ] {
        fs::write(settings.join(file), poisoned).unwrap();
    }
    for args in [
        vec!["kit", "init"],
        vec![
            "kit",
            "validate",
            "--file",
            "vibe-kit.toml",
            "--structure-only",
        ],
    ] {
        let out = common::vibe()
            .current_dir(&cwd)
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("VIBE_SETTINGS", &settings)
            .args(["--unattended", "--offline"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let error = String::from_utf8_lossy(&out.stderr);
        assert!(
            !error.contains("user-level config"),
            "local kit startup read user profile"
        );
        assert!(!error.contains("SYNTHETIC_PRIVATE_CANARY"));
    }
    for file in [
        "config.toml",
        "site-profile.toml",
        "registry.toml",
        "settings.toml",
    ] {
        assert_eq!(fs::read_to_string(settings.join(file)).unwrap(), poisoned);
    }
    assert_eq!(fs::read_dir(&home).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&settings).unwrap().count(), 4);
}

#[test]
fn kit_recipe_schema_and_actual_reader_match_registered_corpus() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/kit_recipe.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    for (text, expected) in [
        (
            include_str!("../../../formats/corpora/kit-recipe/e1/example.toml"),
            true,
        ),
        (
            include_str!("../../../formats/corpora/kit-recipe/e1/invalid-unknown.toml"),
            false,
        ),
    ] {
        let data: toml::Value = toml::from_str(text).unwrap();
        assert_eq!(
            validator.is_valid(&serde_json::to_value(data).unwrap()),
            expected
        );
        fs::write(temp.path().join("vibe-kit.toml"), text).unwrap();
        let out = command(
            &user,
            temp.path(),
            &[
                "kit",
                "validate",
                "--file",
                "vibe-kit.toml",
                "--structure-only",
            ],
        );
        assert_eq!(
            out.status.success(),
            expected,
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let template = include_str!("../../../formats/corpora/kit-recipe/e1/example.toml");
    for target in vibe_wire::behaviour::release_manifest::SUPPORTED_DISTRIBUTION_TARGETS
        .into_iter()
        .chain(["riscv64-unknown-linux-gnu"])
    {
        let expected = target != "riscv64-unknown-linux-gnu";
        let text = template.replace("x86_64-unknown-linux-gnu", target);
        let data: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(
            validator.is_valid(&serde_json::to_value(data).unwrap()),
            expected
        );
        fs::write(temp.path().join("vibe-kit.toml"), text).unwrap();
        let out = command(
            &user,
            temp.path(),
            &[
                "kit",
                "validate",
                "--file",
                "vibe-kit.toml",
                "--structure-only",
            ],
        );
        assert_eq!(out.status.success(), expected);
    }
}

#[test]
fn kit_init_and_validate_explicit_local_recipe() {
    let temp = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    let root = temp.path();
    assert!(!root.join("vibe.toml").exists());
    good(command(&user, root, &["kit", "init"]));
    let file = root.join("vibe-kit.toml");
    let template = fs::read_to_string(&file).unwrap();
    assert!(template.contains("DEMONSTRATION"));
    good(command(
        &user,
        root,
        &[
            "kit",
            "validate",
            "--file",
            "vibe-kit.toml",
            "--structure-only",
        ],
    ));
    bad(command(&user, root, &["kit", "init"]));
    assert_eq!(fs::read_to_string(&file).unwrap(), template);
    bad(command(
        &user,
        root,
        &["kit", "init", "--file", "missing/recipe.toml"],
    ));
    assert!(!root.join("missing").exists());
    bad(command(
        &user,
        root,
        &["kit", "validate", "--file", "vibe-kit.toml"],
    ));
    for changed in [
        template.replace("schema_version = 1", "schema_version = 2"),
        template.replace(
            "schema_version = 1",
            "schema_version = 1\nunknown = 'SYNTHETIC_PRIVATE_CANARY'",
        ),
        template.replace("inputs/site-profile.toml", "../escape.toml"),
        template.replace("inputs/site-profile.toml", "/absolute.toml"),
        template.replace("inputs/site-profile.toml", "inputs\\\\escape.toml"),
        template.replace(&"0".repeat(64), "bad-hash"),
    ] {
        fs::write(&file, changed).unwrap();
        let out = command(
            &user,
            root,
            &[
                "kit",
                "validate",
                "--file",
                "vibe-kit.toml",
                "--structure-only",
            ],
        );
        assert!(!String::from_utf8_lossy(&out.stderr).contains("SYNTHETIC_PRIVATE_CANARY"));
        bad(out);
    }
    fs::create_dir(root.join("inputs")).unwrap();
    let inputs = [
        (
            "bundle",
            b"synthetic bundle; not a qualified distribution".as_slice(),
        ),
        ("installer", b"synthetic installer".as_slice()),
        (
            "profile.toml",
            b"schema = 1\nname = 'synthetic'\n".as_slice(),
        ),
    ];
    let recipe = format!(
        "schema_version=1\n[kit]\nname='synthetic'\ntarget='x86_64-unknown-linux-gnu'\nversion='1.0.12'\n[bundle]\npath='inputs/bundle'\nsha256='{}'\n[installer]\npath='inputs/installer'\nsha256='{}'\n[site_profile]\npath='inputs/profile.toml'\nsha256='{}'\n",
        digest(inputs[0].1),
        digest(inputs[1].1),
        digest(inputs[2].1)
    );
    for (name, bytes) in inputs {
        fs::write(root.join("inputs").join(name), bytes).unwrap();
    }
    fs::write(&file, &recipe).unwrap();
    good(command(
        &user,
        root,
        &["kit", "validate", "--file", "vibe-kit.toml"],
    ));
    fs::write(root.join("inputs/bundle"), "corrupt").unwrap();
    bad(command(
        &user,
        root,
        &["kit", "validate", "--file", "vibe-kit.toml"],
    ));
    fs::write(root.join("inputs/bundle"), inputs[0].1).unwrap();
    let invalid_profile = b"schema = 1\nunknown = 'SYNTHETIC_PRIVATE_CANARY'\n";
    fs::write(root.join("inputs/profile.toml"), invalid_profile).unwrap();
    fs::write(
        &file,
        recipe.replace(&digest(inputs[2].1), &digest(invalid_profile)),
    )
    .unwrap();
    let out = command(&user, root, &["kit", "validate", "--file", "vibe-kit.toml"]);
    assert!(!String::from_utf8_lossy(&out.stderr).contains("SYNTHETIC_PRIVATE_CANARY"));
    bad(out);
    #[cfg(unix)]
    {
        fs::write(&file, &recipe).unwrap();
        fs::write(root.join("inputs/profile.toml"), inputs[2].1).unwrap();
        fs::remove_file(root.join("inputs/bundle")).unwrap();
        fs::write(root.join("foreign-bundle"), inputs[0].1).unwrap();
        std::os::unix::fs::symlink(root.join("foreign-bundle"), root.join("inputs/bundle"))
            .unwrap();
        bad(command(
            &user,
            root,
            &["kit", "validate", "--file", "vibe-kit.toml"],
        ));
        assert_eq!(fs::read(root.join("foreign-bundle")).unwrap(), inputs[0].1);
    }
    assert!(!root.join("vibe.toml").exists());
    assert!(!root.join("vibe.lock").exists());
}

#[cfg(unix)]
#[test]
fn kit_init_and_validate_refuse_symlink_entries() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    let root = temp.path();
    fs::write(root.join("foreign"), "preserve").unwrap();
    symlink(root.join("foreign"), root.join("vibe-kit.toml")).unwrap();
    bad(command(&user, root, &["kit", "init"]));
    bad(command(
        &user,
        root,
        &[
            "kit",
            "validate",
            "--file",
            "vibe-kit.toml",
            "--structure-only",
        ],
    ));
    assert_eq!(
        fs::read_to_string(root.join("foreign")).unwrap(),
        "preserve"
    );
    symlink(root, root.join("alias")).unwrap();
    bad(command(
        &user,
        root,
        &["kit", "init", "--file", "alias/new.toml"],
    ));
    assert!(!root.join("new.toml").exists());
}
