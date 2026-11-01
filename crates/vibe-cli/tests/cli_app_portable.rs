//! Public Linux fixture for the portable set of applications (PROP-059
//! `#portable-set`): `vibe app export` writes each installation's installer
//! option, features, parameters and mark and the launcher arrangement;
//! `vibe app import` reproduces them in a fresh home from the same local
//! registry, `--dry-run` changes nothing, a missing package is one refused
//! row and exit 1, a differing choice is refused unless `--skip-installed`,
//! and an unattended import refuses a licence nobody accepted.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::{Value, json as value};

const EDITOR: &str = "org.example/editor@1.0.0";
const EDIT: &str = "org.example/editor@1.0.0#edit";
const TOOLCHAIN: &str = "org.example/toolchain@1.0.0";

/// An editor with two options for this host (`main`, `alt`), a default
/// feature `docs`, an optional `extras`, a `prefix` parameter and a runtime
/// requirement on a toolchain, which therefore installs as `auto`.
fn fixture() -> AppRegistry {
    let apps = AppRegistry::new();
    let toolchain = apps.package(
        "toolchain",
        "1.0.0",
        "toolchain",
        r#"
[[installer_options.steps]]
id = "place"
title = "Place the toolchain"
kind = "copy"
from = "payload"
"#,
    );
    apps.script(&toolchain, "tcc", "TCC");
    let option = |id: &str| {
        format!(
            r#"
requires = [{{ package = "org.example/toolchain@1", scope = "runtime" }}]

[[installer_options.features]]
id = "docs"
name = "Documentation"
default = true

[[installer_options.features]]
id = "extras"
name = "Extras"

[[installer_options.parameters]]
id = "prefix"
name = "Prefix"
type = "string"
default = "/opt/editor"

[[installer_options.steps]]
id = "place"
title = "Place the editor ({id})"
kind = "copy"
from = "payload"

[[installer_options.launchers]]
id = "edit"
name = "Editor"
executable = "$app_bin/edit"
kind = "command"
"#
        )
    };
    let editor = apps.package(
        "editor",
        "1.0.0",
        "editor",
        &format!(
            "{}\n[[installer_options]]\nid = \"alt\"\nname = \"Alt\"\nplatform = \"linux\"\n\
             platform_name = \"Linux\"\narch = \"{}\"\n{}",
            option("main"),
            std::env::consts::ARCH,
            option("alt")
        ),
    );
    apps.script(&editor, "edit", "EDIT");
    apps
}

/// Install the editor with a choice that differs from every default.
fn installed() -> AppRegistry {
    let apps = fixture();
    ok(&apps.app(
        &["install", "-g"],
        "org.example/editor",
        &[
            "--installer",
            "main",
            "--feature",
            "extras",
            "--no-feature",
            "docs",
            "--set",
            "prefix=/srv/editor",
        ],
    ));
    apps
}

/// `vibe app export` into a file of the scratch root; its path and text.
fn export(apps: &AppRegistry, extra: &[&str]) -> (PathBuf, String) {
    let path = apps.root.path().join("apps.toml");
    let mut args = vec!["app", "export", "--output", path.to_str().unwrap()];
    args.extend_from_slice(extra);
    ok(&apps.vibe(&args));
    let text = fs::read_to_string(&path).unwrap();
    (path, text)
}

fn import(
    target: &AppRegistry,
    source: &AppRegistry,
    file: &Path,
    extra: &[&str],
) -> std::process::Output {
    let registry = source.registry();
    let mut args = vec![
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        registry.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    target.vibe(&args)
}

fn report(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn export_writes_the_choice_and_keeps_requirements_out_unless_all() {
    let apps = installed();
    let (_, text) = export(&apps, &[]);
    eprintln!("exported set:\n{text}");
    let set: toml::Table = toml::from_str(&text).unwrap();
    assert_eq!(set["protocol"].as_str(), Some("vibe-apps/1"));
    assert_eq!(set["home"].as_str(), Some("user"));
    let rows = set["application"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "the auto toolchain stays out: {text}");
    let editor = rows[0].as_table().unwrap();
    assert_eq!(editor["key"].as_str(), Some(EDITOR));
    assert_eq!(editor["installer"].as_str(), Some("main"));
    assert_eq!(editor["features"].as_array().unwrap().len(), 1);
    assert_eq!(editor["features"][0].as_str(), Some("extras"));
    assert_eq!(editor["parameters"]["prefix"].as_str(), Some("/srv/editor"));
    assert_eq!(editor["mark"].as_str(), Some("manual"));
    assert!(!editor.contains_key("held") && !editor.contains_key("license"));

    let all = json(&apps.vibe(&["app", "export", "--all", "--json"]));
    let keys: Vec<&str> = all["application"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, [EDITOR, TOOLCHAIN]);
    assert_eq!(all["application"][1]["mark"], "auto");
}

#[test]
fn import_reproduces_the_choice_in_a_fresh_home() {
    let source = installed();
    ok(&source.vibe(&["app", "hold", "-g", EDITOR]));
    let (file, _) = export(&source, &[]);

    let target = AppRegistry::new();
    let output = import(&target, &source, &file, &["--json"]);
    let answer = report(&output);
    eprintln!("import report: {answer}");
    assert!(output.status.success(), "{answer}");
    assert_eq!(answer["protocol"], "vibe-app-import/1");
    assert_eq!(answer["dry_run"], false);
    assert_eq!(answer["installed"], value!([EDITOR]));
    assert_eq!(answer["refused"], value!([]));

    let index = target.index();
    let editor = &index["installations"][EDITOR];
    assert_eq!(editor["option"], "main");
    assert_eq!(editor["features"], value!(["extras"]));
    assert_eq!(editor["parameters"]["prefix"], "/srv/editor");
    assert_eq!(editor["held"], true, "{editor}");
    assert!(
        index["installations"][TOOLCHAIN].is_object(),
        "the requirement came with it"
    );

    // The same set again: every row is installed with the same choice.
    let again = report(&import(&target, &source, &file, &["--json"]));
    assert_eq!(again["installed"], value!([]));
    assert_eq!(again["skipped"][0]["key"], EDITOR);
}

#[test]
fn dry_run_changes_nothing() {
    let source = installed();
    let (file, _) = export(&source, &[]);
    let target = AppRegistry::new();
    let answer = report(&import(&target, &source, &file, &["--dry-run", "--json"]));
    assert_eq!(answer["dry_run"], true);
    assert_eq!(answer["installed"], value!([EDITOR]));
    assert!(!target.settings().join("applications/index.json").exists());

    let text = ok(&import(&target, &source, &file, &["--dry-run"]));
    assert!(text.contains("would install 1"), "{text}");
    assert!(!target.settings().join("applications/index.json").exists());
}

#[test]
fn a_missing_package_refuses_its_row_alone() {
    let source = installed();
    let (file, text) = export(&source, &[]);
    fs::write(
        &file,
        format!(
            "{text}\n[[application]]\nkey = \"org.example/ghost@1.0.0\"\ninstaller = \"main\"\nmark = \"manual\"\n"
        ),
    )
    .unwrap();
    let target = AppRegistry::new();
    let output = import(&target, &source, &file, &["--json", "--offline"]);
    assert_eq!(output.status.code(), Some(1));
    let answer = report(&output);
    assert_eq!(answer["installed"], value!([EDITOR]));
    assert_eq!(answer["refused"][0]["key"], "org.example/ghost@1.0.0");
    assert!(answer["refused"][0]["code"].is_string(), "{answer}");
}

#[test]
fn a_differing_choice_is_refused_unless_skip_installed() {
    let source = installed();
    let (file, text) = export(&source, &[]);
    fs::write(
        &file,
        text.replace("installer = \"main\"", "installer = \"alt\""),
    )
    .unwrap();

    let refused = import(&source, &source, &file, &["--json"]);
    assert_eq!(refused.status.code(), Some(1));
    let answer = report(&refused);
    assert_eq!(answer["refused"][0]["code"], "choice_differs");
    assert!(
        answer["refused"][0]["message"]
            .as_str()
            .unwrap()
            .contains("vibe app modify"),
        "{answer}"
    );

    let kept = report(&import(
        &source,
        &source,
        &file,
        &["--json", "--skip-installed"],
    ));
    assert_eq!(kept["refused"], value!([]));
    assert_eq!(kept["skipped"][0]["key"], EDITOR);
    assert_eq!(source.index()["installations"][EDITOR]["option"], "main");
}

#[test]
fn the_launcher_arrangement_travels_with_the_set() {
    let source = installed();
    ok(&source.vibe(&["app", "collection", "-g", "create", "Writing"]));
    ok(&source.vibe(&["app", "collection", "-g", "add", "Writing", EDIT]));
    ok(&source.vibe(&["app", "pin", "-g", EDIT, "--to", "favorites"]));
    ok(&source.vibe(&["app", "launcher", "-g", EDIT, "--name", "My Editor"]));
    let (file, text) = export(&source, &[]);
    assert!(
        text.contains("[[collection]]") && text.contains("[[pin]]"),
        "{text}"
    );

    let target = AppRegistry::new();
    ok(&import(&target, &source, &file, &[]));
    let layer = json(&target.vibe(&["app", "collection", "-g", "list", "--json"]));
    let writing = layer["collections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Writing")
        .cloned()
        .unwrap_or_else(|| panic!("no Writing collection in {layer}"));
    assert_eq!(writing["launchers"], value!([EDIT]));
    assert_eq!(layer["pins"]["favorites"], value!([EDIT]));
    assert_eq!(layer["overrides"][0]["key"], EDIT);
    assert_eq!(layer["overrides"][0]["name"], "My Editor");

    let bare = export(&source, &["--installations-only"]).1;
    assert!(
        !bare.contains("[[collection]]") && !bare.contains("[[launcher]]"),
        "{bare}"
    );
}

#[test]
fn an_unattended_import_refuses_a_licence_nobody_accepted() {
    let source = AppRegistry::new();
    let licensed = source.package(
        "licensed",
        "1.0.0",
        "licensed",
        "[installer_options.license]\nfile = \"LICENSE.txt\"\nname = \"Test License 1.0\"\n\n\
         [[installer_options.steps]]\nid = \"place\"\ntitle = \"Place\"\nkind = \"copy\"\nfrom = \"payload\"\n",
    );
    fs::write(licensed.join("LICENSE.txt"), "Be kind.\n").unwrap();
    source.script(&licensed, "licensed", "LICENSED");
    ok(&source.app(
        &["install", "-g"],
        "org.example/licensed",
        &["--accept-license"],
    ));
    let (file, text) = export(&source, &[]);
    assert!(text.contains("license = \"accepted\""), "{text}");

    let target = AppRegistry::new();
    let output = import(&target, &source, &file, &["--json", "--unattended"]);
    assert_eq!(output.status.code(), Some(1), "{}", failed(&output));
    assert_eq!(report(&output)["refused"][0]["code"], "license_required");
    let accepted = import(
        &target,
        &source,
        &file,
        &["--json", "--unattended", "--accept-license"],
    );
    assert_eq!(
        report(&accepted)["installed"],
        value!(["org.example/licensed@1.0.0"])
    );
}
