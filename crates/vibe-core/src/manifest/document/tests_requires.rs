//! The env step kind and requirements between application packages
//! (`PROP-065#env-step`, `#app-dependencies`).

use super::Manifest;
use crate::manifest::{RequirementScope, StepKind, StepPhase, steps_of};

const HEADER: &str = r#"
[package]
name = "demo"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "demo"

[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
"#;

const PLACE: &str = r#"
[[installer_options.steps]]
id = "place"
title = "Place"
kind = "copy"
from = "payload"
"#;

fn manifest(body: &str) -> Result<Manifest, String> {
    Manifest::parse_str(&format!("{HEADER}{body}{PLACE}")).map_err(|error| error.to_string())
}

#[test]
fn an_env_step_is_always_a_finish_step_and_names_portable_variables() {
    let parsed = manifest(
        r#"requires = ["org.example/runtime@2"]
[[installer_options.steps]]
id = "register"
title = "Register"
kind = "env"
set = { TOOL_HOME = "$app_root", TOOL_BIN = "${app_bin}" }
unset = ["TOOL_DEBUG"]
path_prepend = ["$app_bin"]
requires = ["org.example/gcc@13", { package = "org.example/py@3", scope = "runtime" }]
"#,
    )
    .unwrap();
    let option = &parsed.installer_options[0];
    let steps = steps_of(option);
    assert_eq!(steps[0].kind.name(), "env");
    assert_eq!(steps[0].phase, StepPhase::Finish, "env always finishes");
    let StepKind::Env {
        set,
        unset,
        path_prepend,
    } = &steps[0].kind
    else {
        panic!("an env step");
    };
    assert_eq!(set.len(), 2);
    assert_eq!(unset, &["TOOL_DEBUG"]);
    assert_eq!(path_prepend, &["$app_bin"]);
    let scopes: Vec<(&str, RequirementScope)> = steps[0]
        .requires
        .iter()
        .map(|req| (req.package(), req.scope_or(RequirementScope::Build)))
        .collect();
    assert_eq!(
        scopes,
        [
            ("org.example/gcc@13", RequirementScope::Build),
            ("org.example/py@3", RequirementScope::Runtime)
        ]
    );
    assert_eq!(
        option.requires[0].scope_or(RequirementScope::Runtime),
        RequirementScope::Runtime
    );
    let round = toml::to_string(option).unwrap();
    assert!(round.contains("path_prepend"), "{round}");
}

#[test]
fn invalid_env_steps_and_requirements_are_refused() {
    for (body, needle) in [
        (
            "[[installer_options.steps]]\nid = \"e\"\ntitle = \"E\"\nkind = \"env\"\n",
            "must declare set, unset or path_prepend",
        ),
        (
            "[[installer_options.steps]]\nid = \"e\"\ntitle = \"E\"\nkind = \"env\"\nset = { PATH = \"/x\" }\n",
            "PATH is only edited through path_prepend",
        ),
        (
            "[[installer_options.steps]]\nid = \"e\"\ntitle = \"E\"\nkind = \"env\"\nset = { \"1BAD\" = \"/x\" }\n",
            "must be a letter",
        ),
        (
            "[[installer_options.steps]]\nid = \"e\"\ntitle = \"E\"\nkind = \"env\"\nset = { A = \"$nope\" }\n",
            "unknown template variable",
        ),
        (
            "[[installer_options.steps]]\nid = \"e\"\ntitle = \"E\"\nkind = \"env\"\nset = { A = \"1\" }\nunset = [\"A\"]\n",
            "both sets and unsets",
        ),
        ("requires = [\"gcc@13\"]\n", "must be fully qualified"),
    ] {
        let error = manifest(body).unwrap_err();
        assert!(error.contains(needle), "{body}: {error}");
    }
}
