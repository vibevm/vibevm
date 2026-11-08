//! Captured package source survives the global application lifecycle.
#![cfg(target_os = "linux")]
mod common;
use common::app_registry::{AppRegistry, json, ok};
use std::fs;

fn package(apps: &AppRegistry, version: &str) {
    let dir = apps.package(
        "source",
        version,
        "source",
        "[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='payload'\n",
    );
    apps.script(&dir, "source", "SOURCE");
}

#[test]
fn local_source_reaches_plan_inventory_and_description() {
    let apps = AppRegistry::new();
    let package = apps.package(
        "source",
        "1.0.0",
        "source",
        "[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='payload'\n",
    );
    apps.script(&package, "source", "SOURCE");
    let plan = json(&apps.app(
        &["install", "-g"],
        "org.example/source",
        &["--plan-only", "--json"],
    ));
    assert_eq!(plan["source_evidence"]["source_kind"], "registry");
    assert_eq!(plan["source_evidence"]["acquisition"], "directory");
    // A local directory source is no hosting: the plan says it contacts none.
    assert_eq!(plan["hostings"], serde_json::json!([]));
    ok(&apps.app(&["install", "-g"], "org.example/source", &[]));
    let row = apps.listed("org.example/source@1.0.0");
    assert_eq!(row["source_evidence"], plan["source_evidence"]);
    let description = json(&apps.app(
        &["app", "describe", "-g"],
        "org.example/source",
        &["--json"],
    ));
    assert_eq!(description["installed_source"], row["source_evidence"]);
}

#[test]
fn global_registry_alias_is_captured_and_legacy_history_is_not_reconstructed() {
    let apps = AppRegistry::new();
    package(&apps, "1.0.0");
    fs::create_dir_all(apps.settings()).unwrap();
    fs::write(
        apps.settings().join("registry.toml"),
        format!(
            "[[registry]]\nname='original'\nurl='file://{}'\n",
            apps.registry().display()
        ),
    )
    .unwrap();
    let plan = json(&apps.vibe(&[
        "install",
        "-g",
        "org.example/source",
        "--plan-only",
        "--json",
        "--offline",
    ]));
    assert_eq!(plan["source_evidence"]["registry"], "original");
    ok(&apps.vibe(&["install", "-g", "org.example/source", "--offline"]));
    assert_eq!(
        apps.listed("org.example/source@1.0.0")["source_evidence"]["registry"],
        "original"
    );
    fs::write(
        apps.settings().join("registry.toml"),
        format!(
            "[[registry]]\nname='changed'\nurl='file://{}'\n",
            apps.registry().display()
        ),
    )
    .unwrap();
    let description = json(&apps.vibe(&[
        "app",
        "describe",
        "-g",
        "org.example/source",
        "--offline",
        "--json",
    ]));
    assert_eq!(description["source_evidence"]["registry"], "changed");
    assert_eq!(description["installed_source"]["registry"], "original");
    let mut old = apps.index();
    old["protocol"] = "vibe-user-applications/3".into();
    old["installations"]["org.example/source@1.0.0"]
        .as_object_mut()
        .unwrap()
        .remove("source_evidence");
    fs::write(
        apps.settings().join("applications/index.json"),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    assert!(apps.listed("org.example/source@1.0.0")["source_evidence"].is_null());
    let description = json(&apps.vibe(&[
        "app",
        "describe",
        "-g",
        "org.example/source",
        "--offline",
        "--json",
    ]));
    assert!(description["installed_source"].is_null());
    assert_eq!(description["source_evidence"]["registry"], "changed");
}

#[test]
fn update_keeps_previous_origin_and_rollback_restores_it() {
    let apps = AppRegistry::new();
    package(&apps, "1.0.0");
    ok(&apps.app(&["install", "-g"], "org.example/source@1.0.0", &[]));
    let original = apps.listed("org.example/source@1.0.0")["source_evidence"].clone();
    package(&apps, "2.0.0");
    ok(&apps.app(&["update", "-g"], "org.example/source", &[]));
    assert_eq!(
        apps.listed("org.example/source@1.0.0")["source_evidence"],
        original
    );
    let next = apps.listed("org.example/source@2.0.0")["source_evidence"].clone();
    assert_ne!(next["source_url"], original["source_url"]);
    ok(&apps.vibe(&["app", "rollback", "-g", "org.example/source"]));
    assert_eq!(
        apps.listed("org.example/source@1.0.0")["source_evidence"],
        original
    );
}

#[test]
fn configured_plan_and_description_use_the_same_source_facts() {
    let apps = AppRegistry::new();
    package(&apps, "1.0.0");
    let manifest = apps.registry().join("org.example/source/v1.0.0/vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        manifest,
        format!("{text}\n[[installer_options.configuration.features]]\nid='extra'\nname='Extra'\n"),
    )
    .unwrap();
    let plan = json(&apps.app(
        &["app", "plan", "-g"],
        "org.example/source",
        &["--configuration", "--json", "--unattended"],
    ));
    assert_eq!(plan["source_evidence"]["source_kind"], "registry");
    ok(&apps.app(&["install", "-g"], "org.example/source", &["--unattended"]));
    let describe = json(&apps.app(
        &["app", "describe", "-g"],
        "org.example/source",
        &["--configuration", "--json"],
    ));
    assert_eq!(describe["source_evidence"], plan["source_evidence"]);
    assert_eq!(describe["installed_source"], plan["source_evidence"]);
}

#[test]
fn resume_retains_captured_origin_and_old_work_remains_unknown() {
    for legacy in [false, true] {
        let apps = AppRegistry::new();
        package(&apps, "1.0.0");
        let gate = apps.root.path().join("release");
        let manifest = apps.registry().join("org.example/source/v1.0.0/vibe.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        fs::write(manifest, format!("{text}\n[[installer_options.steps]]\nid='release'\ntitle='Release'\nkind='shell'\nphase='prepare'\ncommand=\"test -f '{}'\"\n", gate.display())).unwrap();
        fs::create_dir_all(apps.settings()).unwrap();
        let registry = apps.settings().join("registry.toml");
        fs::write(
            &registry,
            format!(
                "[[registry]]\nname='captured'\nurl='file://{}'\n",
                apps.registry().display()
            ),
        )
        .unwrap();
        let stopped = apps.vibe(&[
            "install",
            "-g",
            "org.example/source",
            "--offline",
            "--keep-work",
            "--json",
        ]);
        let (_, failure) = common::fake_agent::refused(&stopped);
        assert_eq!(failure["code"], "step_failed");
        let listing = json(&apps.vibe(&["app", "work", "-g", "--json"]));
        let row = listing["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["key"] == "org.example/source@1.0.0")
            .unwrap();
        let path = std::path::Path::new(row["path"].as_str().unwrap()).join("steps.json");
        let mut work: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            work["source_evidence"]["org.example/source@1.0.0"]["registry"],
            "captured"
        );
        let operation = work["operation"]["target"]["operation"]
            .as_str()
            .unwrap()
            .to_owned();
        if legacy {
            work["protocol"] = "vibe-app-steps/2".into();
            work.as_object_mut().unwrap().remove("source_evidence");
            fs::write(&path, serde_json::to_vec(&work).unwrap()).unwrap();
        }
        fs::write(
            &registry,
            format!(
                "[[registry]]\nname='current'\nurl='file://{}'\n",
                apps.registry().display()
            ),
        )
        .unwrap();
        fs::write(gate, "release").unwrap();
        ok(&apps.vibe(&[
            "app",
            "resume",
            "org.example/source@1.0.0",
            "--operation",
            &operation,
            "-g",
            "--offline",
        ]));
        let evidence = apps.listed("org.example/source@1.0.0")["source_evidence"].clone();
        if legacy {
            assert!(evidence.is_null());
        } else {
            assert_eq!(evidence["registry"], "captured");
        }
    }
}

#[test]
fn portable_import_records_new_acquisition_and_skip_keeps_existing_origin() {
    let a = AppRegistry::new();
    let b = AppRegistry::new();
    package(&a, "1.0.0");
    package(&b, "1.0.0");
    ok(&a.app(&["install", "-g"], "org.example/source", &[]));
    let file = a.root.path().join("apps.toml");
    ok(&a.vibe(&["app", "export", "--output", file.to_str().unwrap()]));
    ok(&b.vibe(&[
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        b.registry().to_str().unwrap(),
        "--unattended",
        "--json",
    ]));
    let acquired = b.listed("org.example/source@1.0.0")["source_evidence"].clone();
    assert!(
        acquired["source_url"]
            .as_str()
            .unwrap()
            .contains(b.registry().to_str().unwrap())
    );
    assert_ne!(
        acquired,
        a.listed("org.example/source@1.0.0")["source_evidence"]
    );
    ok(&b.vibe(&[
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        a.registry().to_str().unwrap(),
        "--skip-installed",
        "--unattended",
        "--json",
    ]));
    assert_eq!(
        b.listed("org.example/source@1.0.0")["source_evidence"],
        acquired
    );
}

#[test]
fn retained_hosting_is_a_report_projection_not_new_index_state() {
    let apps = AppRegistry::new();
    package(&apps, "1.0.0");
    ok(&apps.app(&["install", "-g"], "org.example/source", &[]));
    // Synthetic retained observations test projection only: acquisition above
    // is entirely local and never contacts either hosting service.
    let mut index = apps.index();
    let evidence = &mut index["installations"]["org.example/source@1.0.0"]["source_evidence"];
    evidence["source_url"] = "https://github.com/example/source".into();
    evidence["fetched_from"] = "ssh://git@gitverse.ru/example/source".into();
    let path = apps.settings().join("applications/index.json");
    let bytes = serde_json::to_vec(&index).unwrap();
    fs::write(&path, &bytes).unwrap();
    let row = apps.listed("org.example/source@1.0.0");
    assert_eq!(row["source_hosting"]["selected_origin"], "github");
    assert_eq!(
        row["source_hosting"]["differing_acquisition_route"],
        "gitverse"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let description = json(&apps.app(
        &["app", "describe", "-g"],
        "org.example/source",
        &["--json"],
    ));
    assert!(description.get("source_hosting").is_none());
    assert_eq!(description["installed_hosting"], row["source_hosting"]);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let manifest = apps.registry().join("org.example/source/v1.0.0/vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        manifest,
        format!("{text}\n[[installer_options.configuration.features]]\nid='extra'\nname='Extra'\n"),
    )
    .unwrap();
    let description = json(&apps.app(
        &["app", "describe", "-g"],
        "org.example/source",
        &["--configuration", "--json"],
    ));
    assert!(description.get("source_hosting").is_none());
    assert_eq!(description["installed_hosting"], row["source_hosting"]);
    assert_eq!(fs::read(&path).unwrap(), bytes);
}
