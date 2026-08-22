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

    user.vibe()
        .current_dir(project.path())
        .args(["facts", "set", package, "impl/done"])
        .assert()
        .success();
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
