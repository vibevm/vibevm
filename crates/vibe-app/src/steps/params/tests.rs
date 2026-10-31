//! Parameter values and license admission at planning time.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use vibe_core::manifest::{InstallerOptionDecl, Manifest};

use super::*;

fn option(dir: &std::path::Path) -> InstallerOptionDecl {
    let text = format!(
        r#"[package]
name = "tool"
group = "org.example"
kind = "app"
version = "1.0.0"
[application]
id = "tool"
[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
[installer_options.license]
file = "LICENSE"
name = "Tool License"
[[installer_options.parameters]]
id = "port"
name = "Port"
type = "integer"
default = 8080
max = 9000
[[installer_options.parameters]]
id = "verbose"
name = "Verbose"
type = "bool"
[[installer_options.parameters]]
id = "data-dir"
name = "Data"
type = "path"
must_exist = true
default = "{}"
[[installer_options.parameters]]
id = "token"
name = "Token"
type = "string"
required = true
secret = true
[[installer_options.steps]]
id = "build"
title = "Build"
kind = "recipe"
script = "b.sh"
"#,
        dir.display()
    );
    Manifest::parse_str(&text)
        .unwrap()
        .installer_options
        .remove(0)
}

fn identity(path: &str) -> PathBuf {
    PathBuf::from(path)
}

fn facts() -> HostFacts<'static> {
    HostFacts {
        resolve_path: &identity,
        ..HostFacts::default()
    }
}

fn set(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(id, value)| ((*id).to_owned(), (*value).to_owned()))
        .collect()
}

fn values(resolved: &[ParamValue]) -> Vec<(&str, &str, bool)> {
    resolved
        .iter()
        .map(|value| (value.id.as_str(), value.value.as_str(), value.secret))
        .collect()
}

#[test]
fn values_come_from_set_then_record_then_prompt_then_default() {
    let dir = tempfile::tempdir().unwrap();
    let option = option(dir.path());
    let dir_text = dir.path().display().to_string();
    let resolved = resolve_parameters(
        &option,
        &set(&[("token", "s3"), ("port", "81"), ("port", "82")]),
        None,
        facts(),
        None,
    )
    .unwrap();
    assert_eq!(
        values(&resolved),
        [
            ("port", "82", false),
            ("data-dir", dir_text.as_str(), false),
            ("token", "s3", true)
        ]
    );
    let previous: BTreeMap<String, String> = [("port".to_owned(), "90".to_owned())].into();
    let resolved = resolve_parameters(
        &option,
        &set(&[("token", "x")]),
        Some(&previous),
        facts(),
        None,
    )
    .unwrap();
    assert_eq!(resolved[0].value, "90");
    let asked = RefCell::new(Vec::new());
    let ask = |parameter: &vibe_core::manifest::ParameterDecl, preset: Option<&str>| {
        asked
            .borrow_mut()
            .push((parameter.id.clone(), preset.map(str::to_owned)));
        Ok(match parameter.id.as_str() {
            "verbose" => Some("yes".to_owned()),
            "token" => Some("typed".to_owned()),
            _ => None,
        })
    };
    let resolved = resolve_parameters(&option, &[], None, facts(), Some(&ask)).unwrap();
    assert_eq!(resolved[0].value, "8080", "None keeps the preselection");
    assert_eq!(resolved[1].value, "true");
    assert_eq!(
        asked.borrow()[0],
        ("port".to_owned(), Some("8080".to_owned()))
    );
    assert!(param_holds(&option, &resolved, "verbose"));
    assert!(param_holds(&option, &resolved, "token"));
    assert!(param_equals(&option, &resolved, "port", "08080"));
    assert!(!param_equals(&option, &resolved, "port", "81"));
}

#[test]
fn invalid_values_are_refused_at_planning_time() {
    let dir = tempfile::tempdir().unwrap();
    let option = option(dir.path());
    let cases = [
        (set(&[]), "token", "required and has no value"),
        (set(&[("token", "t"), ("port", "9001")]), "port", "outside"),
        (
            set(&[("token", "t"), ("verbose", "maybe")]),
            "verbose",
            "not a boolean",
        ),
        (
            set(&[("token", "t"), ("data-dir", "/surely/absent/dir")]),
            "data-dir",
            "does not exist",
        ),
        (
            set(&[("token", "t"), ("nope", "1")]),
            "nope",
            "no such parameter",
        ),
    ];
    for (pairs, parameter, needle) in cases {
        let error = resolve_parameters(&option, &pairs, None, facts(), None).unwrap_err();
        assert_eq!(error.code(), "parameter_invalid");
        let text = error.to_string();
        assert!(
            text.contains(&format!("`{parameter}`")) && text.contains(needle),
            "{text}"
        );
    }
    assert_eq!(
        step_env_name("VIBE_FEATURE_", "gui.qt"),
        "VIBE_FEATURE_GUI_QT"
    );
}

#[test]
fn a_license_needs_acceptance_before_anything_is_fetched() {
    let dir = tempfile::tempdir().unwrap();
    let option = option(dir.path());
    let error = admit_license(&option, "k", dir.path(), true, None, None).unwrap_err();
    assert_eq!(error.code(), "license_unreadable");
    std::fs::write(dir.path().join("LICENSE"), "Be nice.").unwrap();
    let error = admit_license(&option, "k", dir.path(), false, None, None).unwrap_err();
    assert_eq!(error.code(), "license_required");
    assert!(error.to_string().contains("--accept-license"));
    let admitted = admit_license(&option, "k", dir.path(), true, None, None)
        .unwrap()
        .unwrap();
    assert_eq!(admitted.name, "Tool License");
    assert!(admit_license(&option, "k", dir.path(), false, Some("Tool License"), None).is_ok());
    let shown = RefCell::new(String::new());
    let decline = |_: &str, text: &str| {
        shown.replace(text.to_owned());
        Ok(false)
    };
    assert!(admit_license(&option, "k", dir.path(), false, None, Some(&decline)).is_err());
    assert_eq!(shown.borrow().as_str(), "Be nice.");
    let accept = |_: &str, _: &str| Ok(true);
    assert!(admit_license(&option, "k", dir.path(), false, None, Some(&accept)).is_ok());
}
