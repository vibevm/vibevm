//! A real recipe, separate processes and original common input throughout maintenance.
#![cfg(target_os = "linux")]
mod common;
use common::app_registry::{AppRegistry, json, ok};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
const NAME: &str = "org.example/common-journey";
fn package(r: &AppRegistry, version: &str) -> PathBuf {
    let path = r.package(
        "common-journey",
        version,
        "common-journey",
        r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.configuration.features]]
id='gui'
name='Graphical interface'
[[installer_options.configuration.features]]
id='sqlite'
name='SQLite'
effects=[{kind='config-value',name='scope'}]
[[installer_options.configuration.features]]
id='mysql'
name='MySQL'
[[installer_options.configuration.groups]]
mode='choice'
id='ui'
name='Interface'
cardinality='zero-or-one'
members=['gui']
[[installer_options.configuration.groups]]
mode='choice'
id='database'
name='Database'
cardinality='exactly-one'
members=['sqlite','mysql']
guards=[{member='gui'}]
[[installer_options.configuration.parameters]]
id='flag'
name='Flag'
type='bool'
default=true
[[installer_options.configuration.parameters]]
id='root'
name='Data directory'
type='path'
default='other'
[installer_options.configuration.config_values]
scope='installer'
[[configuration.features]]
id='sqlite'
name='Package sqlite'
required=true
effects=[{kind='config-value',name='scope'}]
[configuration.config_values]
scope='package'
"#,
    );
    fs::write(path.join("build.sh"),format!("set -e\nmkdir -p \"$VIBE_OUT\"\nprintf '%s|%s|%s\\n' \"$VIBE_PARAM_FLAG\" \"$VIBE_PARAM_ROOT\" \"$VIBE_FEATURE_SQLITE\" >> '{}'\nprintf 'built' > \"$VIBE_OUT/receipt\"\n",r.root.path().join("executed").display())).unwrap();
    path
}
fn submission(gui: bool) -> Value {
    json!({"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","defaultFeatures":false,"features":[],"selections":{"ui":{"intent":"exact","members":if gui{json!(["gui"])}else{json!([])}},"database":{"intent":"exact","members":["sqlite"]}},"parameters":{"flag":{"kind":"bool","value":false},"root":{"kind":"text","value":"data"}}},"disabledFeatures":[]}})
}
fn save_input(r: &AppRegistry, value: &Value) -> PathBuf {
    let p = r.root.path().join("input.json");
    fs::write(&p, serde_json::to_vec(value).unwrap()).unwrap();
    p
}
fn record(r: &AppRegistry, version: &str) -> Value {
    r.index()["installations"][format!("{NAME}@{version}")].clone()
}
#[test]
fn actual_common_install_modify_update_rollback_and_repair_keep_original_paths_and_dormant_answers()
{
    let r = AppRegistry::new();
    package(&r, "1.0.0");
    let description = json(&r.app(
        &["app", "describe"],
        NAME,
        &["--configuration", "--json", "--offline", "--unattended"],
    ));
    assert_eq!(description["protocol"], "vibe-configuration-description/1");
    assert_eq!(
        description["configuration"]["configValues"]["scope"]["value"],
        "package"
    );
    assert_eq!(
        description["options"][0]["configuration"]["configValues"]["scope"]["value"],
        "installer"
    );
    let input = save_input(&r, &submission(true));
    let input_arg = input.to_str().unwrap();
    let planned = json(&r.app(
        &["app", "plan"],
        NAME,
        &[
            "--configuration",
            "--configuration-input",
            input_arg,
            "--json",
            "--offline",
            "--unattended",
        ],
    ));
    assert!(planned["execution"]["steps"].is_array());
    assert_eq!(
        planned["configuration"]["resolved"]["features"]["selected"],
        json!(["gui", "sqlite"])
    );
    assert!(!r.root.path().join("executed").exists());
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &[
            "--configuration-input",
            input_arg,
            "--json",
            "--offline",
            "--unattended",
        ],
    ));
    let first = record(&r, "1.0.0");
    assert_eq!(r.index()["protocol"], "vibe-user-applications/5");
    assert!(first["source_evidence"].is_object());
    assert_eq!(
        first["configuration"]["evaluation"]["input"],
        submission(true)["input"]
    );
    assert_eq!(
        first["configuration"]["evaluation"]["resolved"]["data"][0]["value"]["value"],
        "installer"
    );
    let path = first["parameters"]["root"].as_str().unwrap().to_owned();
    let elsewhere = r.root.path().join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    ok(&r.vibe_in(
        &elsewhere,
        &[
            "app",
            "modify",
            NAME,
            "-g",
            "--registry",
            r.registry().to_str().unwrap(),
            "--set",
            "flag=yes",
            "--json",
            "--offline",
            "--unattended",
        ],
    ));
    assert_eq!(record(&r, "1.0.0")["parameters"]["root"], path);
    assert_eq!(record(&r, "1.0.0")["parameters"]["flag"], "true");
    save_input(&r, &submission(false));
    ok(&r.app(
        &["app", "modify"],
        NAME,
        &[
            "-g",
            "--configuration-input",
            input_arg,
            "--json",
            "--offline",
            "--unattended",
        ],
    ));
    let dormant = record(&r, "1.0.0");
    assert_eq!(
        dormant["configuration"]["evaluation"]["input"]["intent"]["selections"]["database"]["members"],
        json!(["sqlite"])
    );
    assert_eq!(
        dormant["configuration"]["evaluation"]["resolved"]["features"]["selected"],
        json!([])
    );
    package(&r, "1.1.0");
    fs::remove_file(input).unwrap();
    ok(&r.app(
        &["update", "-g"],
        NAME,
        &["--json", "--offline", "--unattended"],
    ));
    assert_eq!(
        record(&r, "1.1.0")["configuration"]["evaluation"]["input"],
        dormant["configuration"]["evaluation"]["input"]
    );
    ok(&r.vibe(&[
        "app",
        "rollback",
        NAME,
        "-g",
        "--json",
        "--offline",
        "--unattended",
    ]));
    assert_eq!(
        record(&r, "1.0.0")["configuration"]["evaluation"]["input"],
        dormant["configuration"]["evaluation"]["input"]
    );
    ok(&r.app(
        &["install", "-g"],
        "org.example/common-journey@=1.0.0",
        &["--force", "--json", "--offline", "--unattended"],
    ));
    assert_eq!(
        record(&r, "1.0.0")["configuration"]["evaluation"]["input"],
        dormant["configuration"]["evaluation"]["input"]
    );
}
#[test]
fn missing_choice_is_a_structured_question_and_install_runs_nothing() {
    let r = AppRegistry::new();
    package(&r, "1.0.0");
    let mut request = submission(true);
    request["input"]["intent"]["selections"]
        .as_object_mut()
        .unwrap()
        .remove("database");
    let input = save_input(&r, &request);
    let view = json(&r.app(
        &["app", "plan"],
        NAME,
        &[
            "--configuration",
            "--configuration-input",
            input.to_str().unwrap(),
            "--json",
            "--offline",
            "--unattended",
        ],
    ));
    assert!(view.get("execution").is_none());
    assert!(view["configuration"].get("resolved").is_none());
    assert_eq!(view["configuration"]["questions"][0]["kind"], "choice");
    let refusal = r.app(
        &["install", "-g"],
        NAME,
        &[
            "--configuration-input",
            input.to_str().unwrap(),
            "--json",
            "--offline",
            "--unattended",
        ],
    );
    assert_eq!(
        refusal.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&refusal.stderr)
    );
    assert!(!r.root.path().join("executed").exists());
    assert!(!r.settings().join("applications/index.json").exists());
}

#[test]
fn submitted_common_input_is_never_ignored_by_legacy_or_workspace_routes() {
    let r = AppRegistry::new();
    let sentinel = r.root.path().join("legacy-ran");
    r.package("legacy","1.0.0","legacy",&format!("result={{external='{}'}}\n[[installer_options.steps]]\nid='run'\ntitle='Run'\nkind='shell'\ncommand=\"touch '{}'\"\n",r.root.path().display(),sentinel.display()));
    let input = save_input(&r, &submission(false));
    let refusal = r.app(
        &["install", "-g"],
        "org.example/legacy",
        &[
            "--configuration-input",
            input.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    );
    assert_eq!(refusal.status.code(), Some(2));
    assert!(!sentinel.exists());
    assert!(
        String::from_utf8_lossy(&refusal.stderr).contains("no common configuration definition")
    );
    let project = r.root.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let manifest = "[project]\nname='test'\nversion='1.0.0'\n";
    fs::write(project.join("vibe.toml"), manifest).unwrap();
    for verb in ["install", "update"] {
        let output = r.vibe(&[
            verb,
            "--path",
            project.to_str().unwrap(),
            "--configuration-input",
            input.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(project.join("vibe.toml")).unwrap(),
            manifest
        );
        assert!(!project.join("vibe.lock").exists());
    }
}
#[test]
fn mixed_secrecy_batch_refuses_before_any_root_receives_the_shared_answer() {
    let r = AppRegistry::new();
    let make = |name: &str, version: &str, secret: bool| {
        let p=r.package(name,version,name,&format!("result='owned'\n[[installer_options.steps]]\nid='build'\ntitle='Build'\nkind='recipe'\nscript='build.sh'\n[[installer_options.parameters]]\nid='cross'\nname='Cross'\ntype='string'\ndefault='base'\nsecret={secret}\n"));
        fs::write(
            p.join("build.sh"),
            format!(
                "set -e\nmkdir -p \"$VIBE_OUT\"\necho built > \"$VIBE_OUT/receipt\"\ntouch '{}'\n",
                r.root.path().join(format!("{name}-ran")).display()
            ),
        )
        .unwrap();
    };
    for name in ["a-public", "b-secret"] {
        make(name, "1.0.0", false);
        ok(&r.app(
            &["install", "-g"],
            &format!("org.example/{name}"),
            &["--offline", "--unattended", "--json"],
        ));
        fs::remove_file(r.root.path().join(format!("{name}-ran"))).unwrap();
    }
    let before = fs::read(r.settings().join("applications/index.json")).unwrap();
    make("a-public", "1.1.0", false);
    make("b-secret", "1.1.0", true);
    let output = r.vibe(&[
        "update",
        "-g",
        "--all",
        "--registry",
        r.registry().to_str().unwrap(),
        "--set",
        "cross=SYN_BATCH_CANARY",
        "--offline",
        "--unattended",
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("a-public") && error.contains("b-secret"));
    assert!(!error.contains("SYN_BATCH_CANARY"));
    assert!(!r.root.path().join("a-public-ran").exists());
    assert!(!r.root.path().join("b-secret-ran").exists());
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        before
    );
}

#[test]
fn resolved_feature_component_remains_visible_when_a_required_secret_blocks_execution() {
    let r = AppRegistry::new();
    let package = r.package(
        "partial",
        "1.0.0",
        "partial",
        r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.configuration.features]]
id='a'
name='A'
default=true
requires=['b']
[[installer_options.configuration.features]]
id='b'
name='B'
[[installer_options.configuration.parameters]]
id='token'
name='Token'
type='string'
secret=true
required=true
"#,
    );
    let sentinel = r.root.path().join("should-not-run");
    fs::write(
        package.join("build.sh"),
        format!("touch '{}'\n", sentinel.display()),
    )
    .unwrap();
    let planned = json(&r.app(
        &["app", "plan"],
        "org.example/partial",
        &["--configuration", "--json", "--offline", "--unattended"],
    ));
    assert!(planned.get("execution").is_none());
    assert!(planned["configuration"].get("resolved").is_none());
    assert_eq!(
        planned["configuration"]["featureResult"]["selected"],
        json!(["a", "b"])
    );
    assert!(
        !planned["configuration"]["featureResult"]["implied"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let output = r.app(
        &["install", "-g"],
        "org.example/partial",
        &["--json", "--offline", "--unattended"],
    );
    let (status, error) = common::fake_agent::refused(&output);
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "configuration_unresolved");
    assert!(!sentinel.exists());
    assert!(!r.settings().join("applications/index.json").exists());
}

#[cfg(test)]
#[path = "cli_app_common_journey/review.rs"]
mod review;

#[test]
fn malformed_submitted_values_are_not_echoed_and_never_reach_execution() {
    let r = AppRegistry::new();
    package(&r, "1.0.0");
    let canary = "SYN_INVALID_INPUT_PRIVATE_ANSWER";
    let mut unknown = submission(true);
    unknown["input"]["intent"]["unexpected"] = canary.into();
    let mut bad_scalar = submission(true);
    bad_scalar["input"]["intent"]["parameters"]["flag"] = json!({"kind":canary,"value":true});
    for request in [unknown, bad_scalar] {
        let input = save_input(&r, &request);
        let output = r.app(
            &["install", "-g"],
            NAME,
            &[
                "--configuration-input",
                input.to_str().unwrap(),
                "--offline",
                "--unattended",
                "--json",
            ],
        );
        let (code, envelope) = common::fake_agent::refused(&output);
        assert_eq!(code, Some(2));
        assert_eq!(envelope["code"], "configuration_input_invalid");
        assert!(envelope["error"].as_str().unwrap().contains("line"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains(canary));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(canary));
        assert!(!r.root.path().join("executed").exists());
        assert!(!r.settings().join("applications/index.json").exists());
        assert!(
            !r.settings()
                .join("cache/app-work/org.example.common-journey@1.0.0")
                .exists()
        );
    }
}
