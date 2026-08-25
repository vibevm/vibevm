mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{UserScratch, git_available, run_git, write_project_with_per_package_registry};

fn registry_url(registry: &Path) -> String {
    format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    )
}

fn publish_repo(registry: &Path, group: &str, name: &str, versions: &[&str], extension: bool) {
    let source = registry.join(format!("src-{name}"));
    fs::create_dir_all(source.join("hooks")).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    for version in versions {
        let extra = if extension {
            r#"
[[extension]]
id = "watch-target"
point = "slot:pre-install"
handler = { kind = "script", base = "hooks/watch" }
applies_to = { packages = ["org.world/target"] }
"#
        } else {
            ""
        };
        fs::write(
            source.join("vibe.toml"),
            format!(
                "[package]\ngroup='{group}'\nname='{name}'\nkind='tool'\nversion='{version}'\n{extra}"
            ),
        )
        .unwrap();
        fs::write(source.join("payload.txt"), format!("{name}-{version}\n")).unwrap();
        if extension {
            fs::write(
                source.join("hooks/watch.sh"),
                "set -eu\ncp \"$VIBE_CONTEXT\" \"$VIBE_PROJECT_ROOT/.vibe/update-context.json\"\n",
            )
            .unwrap();
            fs::write(
                source.join("hooks/watch.ps1"),
                "Copy-Item -LiteralPath $env:VIBE_CONTEXT -Destination (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/update-context.json')\n",
            )
            .unwrap();
        }
        run_git(&source, &["add", "-A"]);
        run_git(
            &source,
            &["commit", "-m", &format!("{group}/{name}@{version}")],
        );
        run_git(&source, &["tag", &format!("v{version}")]);
    }
    let bare = registry.join(format!("{group}.{name}.git"));
    run_git(
        registry,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
}

fn setup() -> Option<(UserScratch, tempfile::TempDir, PathBuf)> {
    if !git_available() {
        return None;
    }
    let registry = tempfile::tempdir().unwrap().keep();
    publish_repo(&registry, "org.world", "provider", &["0.1.0"], true);
    publish_repo(&registry, "org.world", "target", &["0.1.0", "0.2.0"], false);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    user.vibe()
        .arg("install")
        .args(["org.world/provider@=0.1.0", "org.world/target@=0.1.0"])
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = vibe_core::manifest::Manifest::read(&manifest_path).unwrap();
    let target = manifest
        .requires
        .packages
        .iter_mut()
        .find(|package| package.name == "target")
        .unwrap();
    *target = vibe_core::PackageRef::parse("org.world/target@*").unwrap();
    manifest.write(&manifest_path).unwrap();
    let mut text = fs::read_to_string(&manifest_path).unwrap();
    text.push_str(
        r#"
[[extensions.use]]
ref = "org.world/provider#watch-target"
"#,
    );
    fs::write(manifest_path, text).unwrap();
    Some((user, project, registry))
}

#[test]
fn scoped_update_uses_full_lock_world_and_unchanged_activated_provider() {
    let Some((user, project, _registry)) = setup() else {
        eprintln!("skipping: git unavailable");
        return;
    };
    let output = user
        .vibe()
        .args(["update", "org.world/target", "--json", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let context: serde_json::Value = serde_json::from_slice(
        &fs::read(project.path().join(".vibe/update-context.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(context["execution"]["package"], "org.world/provider");
    assert_eq!(context["slot_target"]["name"], "target");
    let lock = vibe_core::manifest::Lockfile::read(project.path().join("vibe.lock")).unwrap();
    let lock_order = lock
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect::<Vec<_>>();
    let envelope_order = context["world"]["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| package["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(envelope_order, lock_order);
    assert!(envelope_order.contains(&"provider"));
    let docs = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let slot_rows = docs
        .iter()
        .filter(|doc| doc["command"] == "lifecycle")
        .flat_map(|doc| doc["contributions"].as_array().into_iter().flatten())
        .filter(|row| row["point"] == "slot:pre-install")
        .collect::<Vec<_>>();
    assert_eq!(
        slot_rows.len(),
        1,
        "unchanged provider emits no target event"
    );
    assert_eq!(slot_rows[0]["provider"], "org.world/provider");
    assert_eq!(slot_rows[0]["tier"], "host-activation");
    assert_eq!(docs.last().unwrap()["command"], "update");
    assert_eq!(docs.last().unwrap()["hooks"], serde_json::json!([]));
}
