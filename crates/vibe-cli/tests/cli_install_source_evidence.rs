//! Real project events carry held source facts through the existing deferred stream.
mod common;
use common::UserScratch;
use serde_json::Value;
use std::fs;

fn fixture() -> (UserScratch, tempfile::TempDir, tempfile::TempDir) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    for (name, extra) in [
        (
            "root",
            "[requires.packages]\n'org.example/child'='=1.0.0'\n",
        ),
        ("child", ""),
    ] {
        let path = registry.path().join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("vibe.toml"), format!("[package]\ngroup='org.example'\nname='{name}'\nversion='1.0.0'\nkind='tool'\n{extra}")).unwrap();
    }
    fs::write(project.path().join("vibe.toml"), format!("[project]\nname='event-test'\nversion='1.0.0'\n[[registry]]\nname='selected'\nurl='file://{}'\n", registry.path().display())).unwrap();
    (user, project, registry)
}
fn install(user: &UserScratch, project: &std::path::Path, explicit: bool) -> Vec<Value> {
    let mut command = user.vibe();
    command
        .args(["install", "--json", "--offline", "--assume-yes", "--path"])
        .arg(project);
    if explicit {
        command.arg("org.example/root@=1.0.0");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}
fn event(docs: &[Value]) -> &Value {
    docs.iter()
        .find(|v| v["command"] == "install:plan")
        .unwrap()
}
#[test]
fn canonical_generated_reader_decodes_the_actual_live_event() {
    let (user, project, _registry) = fixture();
    let docs = install(&user, project.path(), true);
    let _: vibe_wire::generated::install::e2::plan::InstallPlan =
        serde_json::from_value(event(&docs).clone())
            .expect("canonical reader must match the real producer");
}
#[test]
fn project_event_retains_source_for_the_whole_closure_and_observed_reuse() {
    let (user, project, registry) = fixture();
    let empty = install(&user, project.path(), false);
    assert!(empty.iter().all(|v| v["command"] != "install:plan"));
    for reused in [false, true] {
        let docs = install(&user, project.path(), true);
        let rows = event(&docs)["packages"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        for row in rows {
            let facts = &row["source_evidence"];
            assert_eq!(facts["registry"], "selected");
            assert_eq!(facts["source_kind"], "registry");
            assert_eq!(facts["acquisition"], "directory");
            assert_eq!(facts["store_reused"], reused);
            assert!(
                facts["source_url"]
                    .as_str()
                    .unwrap()
                    .contains(registry.path().to_str().unwrap())
            );
            assert!(facts["resolved_commit"].is_null());
        }
        assert_eq!(docs.last().unwrap()["command"], "install");
    }
    let fresh = install(&user, project.path(), false);
    assert!(fresh.iter().all(|v| v["command"] != "install:plan"));
    // An offline retry must project retained lock history and actual store use,
    // without turning a missing legacy kind into a freshly guessed Registry.
    let path = project.path().join("vibe.lock");
    let mut lock = vibe_core::manifest::Lockfile::read(&path).unwrap();
    for package in &mut lock.packages {
        package.source_kind = None;
    }
    lock.write(&path).unwrap();
    fs::remove_dir_all(registry.path().join("org.example")).unwrap();
    let cached = install(&user, project.path(), true);
    for row in event(&cached)["packages"].as_array().unwrap() {
        assert_eq!(row["source_evidence"]["registry"], "selected");
        assert_eq!(row["source_evidence"]["acquisition"], "store");
        assert_eq!(row["source_evidence"]["store_reused"], true);
        assert!(row["source_evidence"]["source_kind"].is_null());
        assert!(row["source_evidence"]["resolved_commit"].is_null());
    }
}

#[test]
fn retained_lock_hosting_reaches_the_real_project_event_without_new_contact() {
    let (user, project, registry) = fixture();
    install(&user, project.path(), true);
    // Projection control only: replace retained URL observations after local
    // acquisition. The second install is offline from already populated store.
    let path = project.path().join("vibe.lock");
    let mut lock = vibe_core::manifest::Lockfile::read(&path).unwrap();
    for package in &mut lock.packages {
        package.source_url = "https://github.com/example/source".into();
    }
    lock.write(&path).unwrap();
    fs::remove_dir_all(registry.path().join("org.example")).unwrap();
    let docs = install(&user, project.path(), true);
    assert_eq!(docs.last().unwrap()["command"], "install");
    let rows = event(&docs)["packages"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row["source_hosting"]["selected_origin"], "github");
        assert!(
            row["source_hosting"]
                .get("differing_acquisition_route")
                .is_none()
        );
        assert_eq!(row["source_evidence"]["acquisition"], "store");
        assert_eq!(row["source_evidence"]["store_reused"], true);
    }
    assert!(!fs::read_to_string(path).unwrap().contains("hosting"));
}
