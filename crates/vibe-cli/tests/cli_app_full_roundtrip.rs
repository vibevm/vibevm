//! Real CLI full transport and maintenance preserve original choices and target paths.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");
mod common;
use common::app_registry::{AppRegistry, json, ok};
use serde_json::{Value, json as value};
use std::{
    fs,
    path::{Path, PathBuf},
};
use vibe_app::{PortableDocument, parse_portable_document};

const NAME: &str = "org.example/full-roundtrip";
const FIRST: &str = "org.example/full-roundtrip@1.0.0";
const NEXT: &str = "org.example/full-roundtrip@1.1.0";

fn package(apps: &AppRegistry, version: &str) -> PathBuf {
    let package = apps.package(
        "full-roundtrip",
        version,
        "full-roundtrip",
        r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Write bound path receipt'
kind='recipe'
script='build.sh'
[[installer_options.configuration.features]]
id='ordinary'
name='Profile feature'
[[installer_options.configuration.features]]
id='extra'
name='Maintenance delta'
[[installer_options.configuration.features]]
id='gate'
name='Dormant branch gate'
[[installer_options.configuration.features]]
id='a'
name='Retained dormant choice'
[[installer_options.configuration.features]]
id='b'
name='Suppressed profile choice'
[[installer_options.configuration.features]]
id='c'
name='Explicit empty contribution'
[[installer_options.configuration.groups]]
mode='choice'
id='activation'
name='Activation'
cardinality='zero-or-one'
members=['gate']
defaults=[]
[[installer_options.configuration.groups]]
mode='choice'
id='dormant-choice'
name='Dormant exact choice'
cardinality='zero-or-more'
members=['a']
guards=[{member='gate'}]
[[installer_options.configuration.groups]]
mode='choice'
id='omitted-choice'
name='Present but omitted'
cardinality='zero-or-more'
members=['b']
guards=[{member='gate'}]
defaults=[]
[[installer_options.configuration.groups]]
mode='choice'
id='empty-choice'
name='Explicit additive empty'
cardinality='zero-or-more'
members=['c']
guards=[{member='gate'}]
defaults=['c']
[[installer_options.configuration.parameters]]
id='root'
name='Data directory'
type='path'
must_exist=true
default='unused-default'
[[installer_options.configuration.profiles]]
id='selected'
name='Selected profile'
default=true
features=['ordinary']
parameters={root='source-data'}
selections={'omitted-choice'={intent='exact',members=['b']}}
"#,
    );
    fs::write(package.join("build.sh"), "set -e\nmkdir -p \"$VIBE_OUT\"\nprintf '%s\\n' \"$VIBE_PARAM_ROOT\" > \"$VIBE_OUT/receipt\"\nprintf 'run\\n' >> \"$VIBE_PACKAGE/executions\"\n").unwrap();
    package
}
fn original() -> Value {
    value!({"protocol":"vibe-configuration-input/1","input":{
        "intent":{"policy":"use-declared-defaults","profile":"selected","features":[],"defaultFeatures":false,
        "selections":{
            "activation":{"intent":"exact","members":[]},
            "dormant-choice":{"intent":"exact","members":["a"]},
            "omitted-choice":{"intent":"exact","members":null},
            "empty-choice":{"intent":"additive","members":[]}
        },"parameters":{}},"disabledFeatures":[]
    }})
}
fn record(apps: &AppRegistry, key: &str) -> Value {
    apps.index()["installations"][key].clone()
}
fn receipt(apps: &AppRegistry, key: &str) -> String {
    let settings = vibe_app::SettingsRoot::new(apps.settings());
    let index = vibe_app::Index::load(&settings).unwrap();
    let storage = vibe_app::Storage::load(&settings).unwrap();
    let root = vibe_app::app_root(&index.installations[key], &storage).unwrap();
    fs::read_to_string(root.join("receipt")).unwrap()
}
fn exported(apps: &AppRegistry, path: &Path, json_encoding: bool) -> Value {
    let mut args = vec![
        "app",
        "export",
        "--configuration",
        "--output",
        path.to_str().unwrap(),
        "--unattended",
    ];
    if json_encoding {
        args.push("--json");
    }
    let output = apps.vibe(&args);
    ok(&output);
    let text = fs::read_to_string(path).unwrap();
    assert!(!text.trim().is_empty());
    if json_encoding {
        assert_eq!(output.stdout, text.as_bytes());
    }
    let PortableDocument::Full(set) = parse_portable_document("actual-cli-export", &text).unwrap()
    else {
        panic!("full document required");
    };
    serde_json::to_value(set).unwrap()
}
fn history(set: &Value, key: &str) -> Value {
    assert_eq!(set["protocol"], "vibe-apps/2");
    assert_eq!(set["roots"], value!([key]));
    let rows = set["application"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["key"], key);
    assert_eq!(rows[0]["configuration"]["kind"], "configured");
    rows[0]["configuration"]["history"].clone()
}
fn assert_portable_intent(history: &Value, extra: bool) {
    let intent = &history["input"]["intent"];
    assert_eq!(intent["profile"], "selected");
    assert_eq!(intent["policy"], "use-declared-defaults");
    assert_eq!(intent["defaultFeatures"], false);
    assert_eq!(
        intent["features"],
        if extra { value!(["extra"]) } else { value!([]) }
    );
    let selections = &intent["selections"];
    assert_eq!(
        selections["dormant-choice"],
        value!({"intent":"exact","members":["a"]})
    );
    let omitted = selections["omitted-choice"]
        .as_object()
        .expect("omitted-member parent selection must survive");
    assert_eq!(omitted["intent"], "exact");
    assert!(
        !omitted.contains_key("members"),
        "portable None is omission, not explicit empty"
    );
    assert_eq!(
        selections["empty-choice"],
        value!({"intent":"additive","members":[]})
    );
    assert_eq!(selections["activation"]["members"], value!([]));
}
fn assert_installed(apps: &AppRegistry, key: &str, input: &Value, path: &Path, extra: bool) {
    let row = record(apps, key);
    assert!(row.is_object());
    assert!(row.get("pending").is_none_or(Value::is_null));
    assert_eq!(row["parameters"]["root"], path.to_str().unwrap());
    let evaluation = &row["configuration"]["evaluation"];
    assert_eq!(evaluation["input"], *input);
    assert!(evaluation["resolved"].is_object());
    assert!(evaluation["questions"].as_array().unwrap().is_empty());
    assert!(evaluation["problems"].as_array().unwrap().is_empty());
    let chosen = row["features"].as_array().unwrap();
    assert!(chosen.contains(&value!("ordinary")));
    assert_eq!(chosen.contains(&value!("extra")), extra);
    for dormant in ["a", "b", "c", "gate"] {
        assert!(!chosen.contains(&value!(dormant)));
    }
    let root = evaluation["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "root")
        .unwrap();
    assert_eq!(root["origin"]["kind"], "profile");
    assert_eq!(root["origin"]["profile"], "selected");
    assert_eq!(root["source"]["value"], "source-data");
    assert_eq!(receipt(apps, key), format!("{}\n", path.display()));
}
fn changed_cwd(apps: &AppRegistry, name: &str) -> PathBuf {
    let cwd = apps.root.path().join(name);
    fs::create_dir(&cwd).unwrap();
    assert!(!cwd.join("source-data").exists());
    cwd
}
fn journey(json_encoding: bool) {
    let a = AppRegistry::new();
    package(&a, "1.0.0");
    let a_data = a.root.path().join("source-data");
    fs::create_dir(&a_data).unwrap();
    let input_file = a.root.path().join("original.json");
    fs::write(&input_file, serde_json::to_vec(&original()).unwrap()).unwrap();
    let installed = json(&a.app(
        &["install", "-g"],
        NAME,
        &[
            "--configuration-input",
            input_file.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    assert_eq!(installed["ok"], true);
    assert_installed(&a, FIRST, &original()["input"], &a_data, false);
    let file = a.root.path().join(if json_encoding {
        "apps.json"
    } else {
        "apps.toml"
    });
    let source = exported(&a, &file, json_encoding);
    assert_portable_intent(&history(&source, FIRST), false);
    fs::remove_dir(&a_data).unwrap();
    fs::remove_file(input_file).unwrap();
    assert!(!a_data.exists());

    let b = AppRegistry::new();
    let first_source = package(&b, "1.0.0");
    let b_data = b.root.path().join("bound-data");
    fs::create_dir(&b_data).unwrap();
    let target = b.root.path().join("target.json");
    fs::write(&target,serde_json::to_vec(&value!({"protocol":"vibe-app-import-input/1","applications":{(FIRST):{"installer":"main","pathBindings":{"root":b_data.to_str().unwrap()}}}})).unwrap()).unwrap();
    let import_cwd = changed_cwd(&b, "import-cwd");
    let answer = json(&b.vibe_in(
        &import_cwd,
        &[
            "app",
            "import",
            file.to_str().unwrap(),
            "--target-input",
            target.to_str().unwrap(),
            "--registry",
            b.registry().to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    assert_eq!(answer["dry_run"], false);
    assert_eq!(answer["installed"], value!([FIRST]));
    assert!(answer["refused"].as_array().unwrap().is_empty());
    assert_installed(&b, FIRST, &original()["input"], &b_data, false);
    fs::remove_file(file).unwrap();
    fs::remove_file(target).unwrap();

    let modify_cwd = changed_cwd(&b, "modify-cwd");
    let modified = json(&b.vibe_in(
        &modify_cwd,
        &[
            "app",
            "modify",
            NAME,
            "-g",
            "--feature",
            "extra",
            "--registry",
            b.registry().to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    assert_eq!(modified["ok"], true);
    let mut changed = original()["input"].clone();
    changed["intent"]["features"] = value!(["extra"]);
    assert_installed(&b, FIRST, &changed, &b_data, true);
    let repair_cwd = changed_cwd(&b, "repair-cwd");
    let repaired = json(&b.vibe_in(
        &repair_cwd,
        &[
            "install",
            "-g",
            "org.example/full-roundtrip@=1.0.0",
            "--force",
            "--registry",
            b.registry().to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    assert_eq!(repaired["ok"], true);
    assert_installed(&b, FIRST, &changed, &b_data, true);
    assert_eq!(
        fs::read_to_string(first_source.join("executions"))
            .unwrap()
            .lines()
            .count(),
        3
    );

    let next_source = package(&b, "1.1.0");
    let update_cwd = changed_cwd(&b, "update-cwd");
    let updated = json(&b.vibe_in(
        &update_cwd,
        &[
            "update",
            "-g",
            NAME,
            "--registry",
            b.registry().to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    assert_eq!(updated["ok"], true);
    assert_installed(&b, NEXT, &changed, &b_data, true);
    assert_eq!(
        record(&b, NEXT)["configuration"]["evaluation"]["definition"],
        record(&a, FIRST)["configuration"]["evaluation"]["definition"],
        "the local next version retains the same configuration meaning"
    );
    assert_eq!(
        fs::read_to_string(next_source.join("executions"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(!a_data.exists());
    let output = b.root.path().join(if json_encoding {
        "again.json"
    } else {
        "again.toml"
    });
    let next = exported(&b, &output, json_encoding);
    let retained = history(&next, NEXT);
    assert_portable_intent(&retained, true);
    let root = retained["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "root")
        .unwrap();
    assert_eq!(root["origin"]["kind"], "profile");
    assert_eq!(root["source"]["value"], "source-data");
    assert_eq!(root["realization"]["value"], b_data.to_str().unwrap());
    assert!(!b_data.join("source-data").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn full_json_cli_roundtrip_and_maintenance_keep_profile_dormant_intent_and_bound_path() {
    journey(true);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn full_toml_cli_roundtrip_and_maintenance_keep_profile_dormant_intent_and_bound_path() {
    journey(false);
}
