//! Real installs feed typed portable exports; refused snapshots preserve output files.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");
mod common;

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::{Value, json as value};
use std::fs;
use vibe_app::{PortableDocument, parse_portable_document};

const ROOT: &str = "org.example/export-root@1.0.0";
const CHILD: &str = "org.example/export-child@1.0.0";
const CANARY: &str = "SYNTHETIC_EXPORT_SECRET_4901";
const COPY: &str =
    "[[installer_options.steps]]\nid='place'\ntitle='Place payload'\nkind='copy'\nfrom='payload'\n";
const CONFIGURATION: &str =
    "[[installer_options.configuration.features]]\nid='extra'\nname='Extra'\ndefault=true\n";

fn fixture(root_common: bool, child_common: bool, secret: bool) -> AppRegistry {
    let apps = AppRegistry::new();
    let child = apps.package(
        "export-child",
        "1.0.0",
        "export-child",
        &format!("{COPY}{}", if child_common { CONFIGURATION } else { "" }),
    );
    apps.script(&child, "child", "CHILD");
    let secret_decl = if secret {
        "[[installer_options.configuration.parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\n"
    } else {
        ""
    };
    let root = apps.package(
        "export-root",
        "1.0.0",
        "export-root",
        &format!(
            "requires=[{{package='org.example/export-child@1',scope='build'}}]\n{COPY}{}{secret_decl}",
            if root_common { CONFIGURATION } else { "" }
        ),
    );
    apps.script(&root, "root", "ROOT");
    let mut args = vec!["--offline", "--unattended", "--json"];
    let setter = format!("token={CANARY}");
    if secret {
        args.extend(["--set", &setter]);
    }
    ok(&apps.app(&["install", "-g"], "org.example/export-root", &args));
    assert_eq!(apps.index()["installations"][CHILD]["mark"], "auto");
    assert_eq!(apps.index()["installations"][CHILD]["hidden"], true);
    apps
}

fn full(text: &str) -> Value {
    match parse_portable_document("actual-cli-export", text).unwrap() {
        PortableDocument::Full(set) => serde_json::to_value(set).unwrap(),
        PortableDocument::Legacy(_) => panic!("expected the full portable document"),
    }
}

fn row<'a>(set: &'a Value, key: &str) -> &'a Value {
    set["application"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["key"] == key)
        .unwrap()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
fn explicit_full_export_preserves_legacy_default_and_adds_actual_required_rows() {
    let apps = fixture(false, false, false);
    let legacy = json(&apps.vibe(&["app", "export", "--json", "--unattended"]));
    assert_eq!(legacy["protocol"], "vibe-apps/1");
    assert_eq!(legacy["application"].as_array().unwrap().len(), 1);
    assert_eq!(
        legacy["unattended"], true,
        "legacy context decoration stays"
    );
    let text = ok(&apps.vibe(&["app", "export", "--configuration", "--json", "--unattended"]));
    let set = full(&text);
    assert_eq!(set["roots"], value!([ROOT]));
    assert_eq!(set["application"].as_array().unwrap().len(), 2);
    assert_eq!(row(&set, CHILD)["mark"], "auto");
    assert_eq!(row(&set, CHILD)["hidden"], true);
    assert_eq!(row(&set, ROOT)["requires"][0]["key"], CHILD);
    assert_eq!(row(&set, ROOT)["configuration"]["kind"], "legacy-effective");
    let all = full(&ok(&apps.vibe(&[
        "app",
        "export",
        "--configuration",
        "--all",
        "--json",
    ])));
    assert_eq!(
        all["roots"],
        value!([ROOT]),
        "Auto is never promoted to a root"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
fn common_install_auto_exports_parseable_json_and_toml_without_secret_answers() {
    let apps = fixture(true, true, true);
    let file = apps.root.path().join("full.json");
    let text = ok(&apps.vibe(&[
        "app",
        "export",
        "--json",
        "--unattended",
        "--output",
        file.to_str().unwrap(),
    ]));
    assert_eq!(text, fs::read_to_string(&file).unwrap());
    let set = full(&text);
    assert!(!text.contains(CANARY));
    assert!(set.get("unattended").is_none());
    assert_eq!(set["roots"], value!([ROOT]));
    assert_eq!(row(&set, ROOT)["configuration"]["kind"], "configured");
    assert_eq!(row(&set, CHILD)["configuration"]["kind"], "configured");
    let toml = ok(&apps.vibe(&["app", "export", "--unattended"]));
    let decoded = full(&toml);
    assert!(!toml.contains(CANARY));
    assert_eq!(decoded["application"], set["application"]);
    assert_eq!(decoded["roots"], set["roots"]);
    let file = apps.root.path().join("full.toml");
    let output = apps.vibe(&[
        "app",
        "export",
        "--installations-only",
        "--output",
        file.to_str().unwrap(),
    ]);
    ok(&output);
    let saved = full(&fs::read_to_string(file).unwrap());
    assert_eq!(saved["application"], set["application"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("exported 2 application(s)"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
fn configured_auto_child_selects_full_even_when_the_manual_root_is_legacy() {
    let apps = fixture(false, true, false);
    let text = ok(&apps.vibe(&["app", "export", "--json", "--unattended"]));
    let set = full(&text);
    assert_eq!(set["roots"], value!([ROOT]));
    assert_eq!(row(&set, ROOT)["configuration"]["kind"], "legacy-effective");
    assert_eq!(row(&set, CHILD)["configuration"]["kind"], "configured");
    assert_eq!(row(&set, CHILD)["mark"], "auto");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
fn unfinished_real_replacement_refuses_export_without_truncating_output() {
    let apps = fixture(true, false, false);
    apps.package(
        "export-root",
        "2.0.0",
        "export-root",
        &format!("{COPY}{CONFIGURATION}"),
    );
    let failure = apps.app(
        &["update", "-g"],
        "org.example/export-root",
        &["--offline", "--unattended", "--json"],
    );
    failed(&failure);
    assert!(
        apps.settings()
            .join("cache/app-work/org.example.export-root@2.0.0/steps.json")
            .exists()
    );
    assert!(apps.index()["installations"][ROOT].is_object());
    let sentinel = apps.root.path().join("keep-output");
    for json in [false, true] {
        fs::write(&sentinel, b"PREEXISTING OUTPUT\n").unwrap();
        let mut args = vec![
            "app",
            "export",
            "--output",
            sentinel.to_str().unwrap(),
            "--unattended",
        ];
        if json {
            args.push("--json");
        }
        let output = apps.vibe(&args);
        let error = failed(&output);
        assert!(
            error.contains("unfinished") || error.contains("unpublished"),
            "{error}"
        );
        assert_eq!(fs::read(&sentinel).unwrap(), b"PREEXISTING OUTPUT\n");
        assert!(output.stdout.is_empty());
    }
}
