//! Unused Auto rows remain portable data, not target-source queries or jobs.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::{AppRegistry, ok};
use serde_json::{Value, json};
use std::fs;
const ORPHAN: &str = "org.example/orphan@1.0.0";
fn source() -> (AppRegistry, std::path::PathBuf, std::path::PathBuf) {
    let r = AppRegistry::new();
    let data = r.root.path().join("source-data");
    fs::create_dir(&data).unwrap();
    for name in ["main", "orphan"] {
        let configuration = if name == "orphan" {
            format!(
                "[[installer_options.configuration.parameters]]\nid='data'\nname='Data'\ntype='path'\ndefault='{}'\nmust_exist=true\n",
                data.display()
            )
        } else {
            "[installer_options.configuration]\n".into()
        };
        let p=r.package(name,"1.0.0",name,&format!("[[installer_options.steps]]\nid='place'\ntitle='Place'\nkind='copy'\nfrom='payload'\n{configuration}"));
        r.script(&p, name, name);
        ok(&r.app(
            &["install", "-g"],
            &format!("org.example/{name}"),
            &["--offline", "--unattended", "--json"],
        ));
    }
    ok(&r.vibe(&["app", "mark", "auto", ORPHAN, "-g", "--json"]));
    let file = r.root.path().join("all.json");
    ok(&r.vibe(&[
        "app",
        "export",
        "--all",
        "--configuration",
        "--installations-only",
        "--json",
        "--output",
        file.to_str().unwrap(),
    ]));
    let orphan = r.registry().join("org.example/orphan/v1.0.0");
    fs::remove_dir_all(&orphan).unwrap();
    fs::remove_dir_all(&data).unwrap();
    (r, file, orphan)
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
fn unused_auto_without_target_source_or_path_binding_is_not_consumed() {
    let (source, file, orphan) = source();
    let target = AppRegistry::new();
    let registry = source.registry();
    for dry in [true, false] {
        let mut args = vec![
            "app",
            "import",
            file.to_str().unwrap(),
            "--registry",
            registry.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ];
        if dry {
            args.push("--dry-run");
        }
        let output = target.vibe(&args);
        ok(&output);
        let view: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(view["installed"], json!(["org.example/main@1.0.0"]));
        assert_eq!(view["skipped"][0]["key"], ORPHAN);
        assert!(
            view["skipped"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("not consumed")
        );
        assert!(!orphan.exists());
    }
    assert!(target.index()["installations"].get(ORPHAN).is_none());
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
fn explicit_inputs_for_unused_rows_refuse_without_starting_a_job() {
    let (source, file, _) = source();
    for secret in [false, true] {
        let target = AppRegistry::new();
        let input = target.root.path().join("input.json");
        let (flag, body) = if secret {
            (
                "--secret-input",
                json!({"protocol":"vibe-app-import-secrets/1","applications":{(ORPHAN):{"unused":"CANARY_UNUSED_913"}}}),
            )
        } else {
            (
                "--target-input",
                json!({"protocol":"vibe-app-import-input/1","applications":{(ORPHAN):{"pathBindings":{"data":"/tmp/unconsumed"}}}}),
            )
        };
        fs::write(&input, serde_json::to_vec(&body).unwrap()).unwrap();
        let out = target.vibe(&[
            "app",
            "import",
            file.to_str().unwrap(),
            "--registry",
            source.registry().to_str().unwrap(),
            flag,
            input.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ]);
        assert!(!out.status.success());
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(text.contains("unused"), "{text}");
        assert!(!text.contains("CANARY_UNUSED_913"));
        assert!(!target.settings().join("cache/app-work").exists());
        assert!(!target.settings().join("applications/index.json").exists());
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
fn all_auto_empty_roots_do_not_resolve_sources_or_invent_installations() {
    let (source, file, _) = source();
    ok(&source.vibe(&[
        "app",
        "mark",
        "auto",
        "org.example/main@1.0.0",
        "-g",
        "--json",
    ]));
    ok(&source.vibe(&[
        "app",
        "export",
        "--all",
        "--configuration",
        "--installations-only",
        "--json",
        "--output",
        file.to_str().unwrap(),
    ]));
    fs::remove_dir_all(source.registry().join("org.example/main/v1.0.0")).unwrap();
    let target = AppRegistry::new();
    let out = target.vibe(&[
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        source.registry().to_str().unwrap(),
        "--offline",
        "--unattended",
        "--json",
    ]);
    ok(&out);
    let view: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(view["installed"], json!([]));
    assert_eq!(view["skipped"].as_array().unwrap().len(), 2);
    assert!(!target.settings().join("applications/index.json").exists());
}
