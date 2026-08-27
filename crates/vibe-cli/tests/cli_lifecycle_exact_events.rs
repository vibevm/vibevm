mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;

fn package(registry: &Path, name: &str, hooked: bool) {
    let root = registry.join("org.phantom").join(name).join("v0.1.0");
    fs::create_dir_all(root.join("hooks")).unwrap();
    let hooks = if hooked {
        "\n[hooks]\npost-install='hooks/post'\n"
    } else {
        ""
    };
    fs::write(
        root.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.phantom'\nname='{name}'\nkind='tool'\nversion='0.1.0'\n{hooks}"
        ),
    )
    .unwrap();
    if hooked {
        fs::write(root.join("hooks/post.sh"), "printf ran > .post-ran\n").unwrap();
        fs::write(
            root.join("hooks/post.ps1"),
            "Set-Content -LiteralPath .post-ran -Value ran\n",
        )
        .unwrap();
    }
}

#[test]
fn adding_new_package_surfaces_only_exact_changed_target_rows() {
    let registry = tempfile::tempdir().unwrap();
    package(registry.path(), "hooked", true);
    package(registry.path(), "new", false);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    user.vibe()
        .args(["install", "org.phantom/hooked@=0.1.0", "--registry"])
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let hooked_slot = project
        .path()
        .join(common::slot_dir("org.phantom.hooked", "0.1.0"));
    assert!(hooked_slot.join(".post-ran").is_file());

    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/exact.sh"),
        "printf \"$VIBE_PACKAGE_NAME\" > \"$VIBE_PROJECT_ROOT/.vibe/exact-target\"\n",
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/exact.ps1"),
        "Set-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/exact-target') -Value $env:VIBE_PACKAGE_NAME -NoNewline\n",
    )
    .unwrap();
    let manifest = project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(
        r#"
[[extension]]
id="exact-target"
point="slot:pre-install"
handler={kind="script",base="scripts/exact"}
applies_to={packages=["org.phantom/*"]}
"#,
    );
    fs::write(manifest, text).unwrap();

    let output = user
        .vibe()
        .args(["install", "org.phantom/new@=0.1.0", "--json", "--registry"])
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(project.path().join(".vibe/exact-target")).unwrap(),
        "new"
    );
    let docs = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let plans = docs
        .iter()
        .filter(|doc| doc["command"] == "lifecycle:plan")
        .flat_map(|doc| doc["contributions"].as_array().into_iter().flatten())
        .filter(|row| {
            row["point"]
                .as_str()
                .is_some_and(|point| point.starts_with("slot:"))
        })
        .collect::<Vec<_>>();
    let install_roots = docs
        .iter()
        .filter(|doc| doc["command"] == "install")
        .collect::<Vec<_>>();
    assert_eq!(
        install_roots.len(),
        1,
        "exactly one install root: {docs:#?}"
    );
    assert_eq!(
        docs.last(),
        install_roots.first().copied(),
        "the command root is the final document"
    );
    assert!(
        docs.iter().all(|doc| doc["command"] != "lifecycle"),
        "slot rows belong to the command root, never a lifecycle echo: {docs:#?}"
    );
    let outcomes = install_roots[0]["contributions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| {
            row["point"]
                .as_str()
                .is_some_and(|point| point.starts_with("slot:"))
        })
        .collect::<Vec<_>>();
    assert_eq!(plans.len(), 1);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(plans[0]["slot_target"]["name"], "new");
    assert_eq!(outcomes[0]["slot_target"]["name"], "new");
    assert_eq!(plans[0]["key"], outcomes[0]["key"]);
    assert!(
        plans
            .iter()
            .all(|row| row["slot_target"]["name"] != "hooked")
    );
}
