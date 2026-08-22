//! End-to-end contract for the W1 `vibe facts` command family.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#cli");

mod common;

use std::fs;

use common::UserScratch;

#[test]
fn facts_crud_filters_and_spec_to_registry_sync() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().expect("tempdir");
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    fs::create_dir_all(project.path().join("spec/common")).expect("spec dir");
    let spec = project.path().join("spec/common/RULE.md");
    let spec_bytes = "# Rule {#root}\n\n@fact:RULE The rule. @status:impl/done\n";
    fs::write(&spec, spec_bytes).expect("spec");

    let host = "spec://org.example/demo/common/RULE#RULE";
    let package = "spec://org.external/flow/flows/main#PACKAGE-RULE";

    user.vibe()
        .current_dir(project.path())
        .args([
            "facts",
            "set",
            host,
            "spec/work",
            "--comment",
            "adopted locally",
        ])
        .assert()
        .success();

    let get = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "get", host])
        .output()
        .expect("get");
    assert!(get.status.success());
    let stdout = String::from_utf8_lossy(&get.stdout);
    assert!(stdout.contains(host));
    assert!(stdout.contains("spec/work"));

    let list = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "list", "--status", "spec/work"])
        .output()
        .expect("list");
    assert!(list.status.success());
    assert!(String::from_utf8_lossy(&list.stdout).contains(host));

    let deferred = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "set", package, "impl/done"])
        .output()
        .expect("set uninstalled package fact");
    assert!(deferred.status.success());
    assert!(
        String::from_utf8_lossy(&deferred.stdout).contains("not installed; will apply at install")
    );
    assert!(
        project
            .path()
            .join("vibefacts/org.external.flow.toml")
            .is_file()
    );
    user.vibe()
        .current_dir(project.path())
        .args(["facts", "rm", package])
        .assert()
        .success();
    assert!(
        !project
            .path()
            .join("vibefacts/org.external.flow.toml")
            .exists()
    );

    user.vibe()
        .current_dir(project.path())
        .args(["facts", "sync"])
        .assert()
        .failure();
    user.vibe()
        .current_dir(project.path())
        .args(["facts", "sync", "--write"])
        .assert()
        .success();
    user.vibe()
        .current_dir(project.path())
        .args(["facts", "sync"])
        .assert()
        .success();

    let get = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "get", host])
        .output()
        .expect("get after sync");
    assert!(String::from_utf8_lossy(&get.stdout).contains("impl/done"));
    assert_eq!(
        fs::read_to_string(spec).expect("spec unchanged"),
        spec_bytes
    );
}

#[test]
fn package_set_rederives_and_adopt_fills_only_absent_statuses() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().expect("project");
    let registry = tempfile::tempdir().expect("registry");
    let package = registry
        .path()
        .join("org.example")
        .join("facts-pkg")
        .join("v1.0.0");
    fs::create_dir_all(package.join("spec")).expect("package spec dir");
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup = \"org.example\"\nname = \"facts-pkg\"\nkind = \"flow\"\nversion = \"1.0.0\"\nepoch = 1\n",
    )
    .expect("package manifest");
    fs::write(
        package.join("spec/RULE.md"),
        "# Rules\n\n@fact:FIRST First. <status stage=\"spec\" state=\"work\" comment=\"author extra\"/>\n\n@fact:SECOND Second. @status:impl/done\n",
    )
    .expect("package spec");
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\ngroup = \"org.consumer\"\nname = \"demo\"\nversion = \"0.1.0\"\nspec_format = \"markdown\"\n",
    )
    .expect("project manifest");

    user.vibe()
        .args(["install", "org.example/facts-pkg@=1.0.0", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let first = "spec://org.example/facts-pkg/RULE#FIRST";
    let second = "spec://org.example/facts-pkg/RULE#SECOND";
    user.vibe()
        .current_dir(project.path())
        .args(["facts", "set", first, "impl/done"])
        .assert()
        .success();

    let slot = project.path().join("vibedeps/org.example.facts-pkg/1.0.0");
    let materialised = fs::read_to_string(slot.join("spec/RULE.md")).expect("slot spec");
    assert!(materialised.contains("stage=\"impl\" state=\"done\" comment=\"author extra\""));
    let manifest = vibe_workspace::vibedeps::read_derived_manifest(&slot).expect("manifest");
    assert!(manifest.overlay_hash.is_some());

    let adopt = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "adopt", "--package", "org.example/facts-pkg"])
        .output()
        .expect("adopt");
    assert!(
        adopt.status.success(),
        "{}",
        String::from_utf8_lossy(&adopt.stderr)
    );
    let stdout = String::from_utf8_lossy(&adopt.stdout);
    assert!(stdout.contains("added=1 kept=1"), "{stdout}");
    let facts = fs::read_to_string(project.path().join("vibefacts/org.example.facts-pkg.toml"))
        .expect("package facts");
    assert!(facts.contains(first));
    assert!(facts.contains(second));

    user.vibe()
        .current_dir(project.path())
        .args(["facts", "rm", first])
        .assert()
        .success();
    let restored = fs::read_to_string(slot.join("spec/RULE.md")).expect("restored slot spec");
    assert!(restored.contains("stage=\"spec\" state=\"work\" comment=\"author extra\""));

    user.vibe()
        .arg("check")
        .arg("--path")
        .arg(project.path())
        .arg("--quiet")
        .assert()
        .success();
}
