//! Real local app planning: classified secret answers never enter CLI errors.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#SECRET-BOUNDARY");

#[cfg(test)]
mod common;

use common::app_registry::{AppRegistry, json};
use serde_json::Value;

const PACKAGE: &str = "org.example/secret-diagnostics";

#[cfg(test)]
fn fixture(kind: &str, fields: &str, secret: bool) -> AppRegistry {
    let registry = AppRegistry::new();
    let package = registry.package("secret-diagnostics", "1.0.0", "secret-diagnostics", &format!(
        "[[installer_options.parameters]]\nid='answer'\nname='Answer'\ntype='{kind}'\nsecret={secret}\n{fields}\n\
         [[installer_options.steps]]\nid='copy'\ntitle='Copy fixture'\nkind='copy'\nfrom='input.txt'\nto='input.txt'\n"
    ));
    std::fs::write(package.join("input.txt"), "LOCAL_FIXTURE").unwrap();
    registry
}

#[cfg(test)]
fn secret_refusal(kind: &str, fields: &str, raw: &str, forbidden: &[&str]) {
    let registry = fixture(kind, fields, true);
    let assignment = format!("answer={raw}");
    let mut exposed = Vec::new();
    for machine in [false, true] {
        let mut extra = vec!["--offline", "--unattended", "--set", assignment.as_str()];
        if machine {
            extra.push("--json");
        }
        let output = registry.app(&["app", "plan"], PACKAGE, &extra);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(output.status.code(), Some(2), "{stdout}\n{stderr}");
        assert!(
            stderr.contains("answer") && stderr.contains("main"),
            "{stderr}"
        );
        if machine {
            let envelope = stderr
                .lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .find(|value| value["ok"] == false)
                .expect("JSON refusal envelope");
            assert_eq!(envelope["code"], "parameter_invalid");
        } else {
            assert!(stderr.contains("parameter_invalid"), "{stderr}");
        }
        let text = format!("{stdout}\n{stderr}");
        if forbidden.iter().any(|value| text.contains(value)) {
            exposed.push((machine, text));
        }
    }
    assert!(
        exposed.is_empty(),
        "synthetic secret in public error output: {exposed:?}"
    );
    assert!(!registry.settings().join("cache/app-work").exists());
    assert!(!registry.settings().join("applications/index.json").exists());
}

#[test]
fn secret_bool_error_is_redacted_in_human_and_json() {
    secret_refusal("bool", "", "SYN_CLI_BOOL_CANARY", &["SYN_CLI_BOOL_CANARY"]);
}

#[test]
fn secret_integer_parse_error_is_redacted_in_human_and_json() {
    secret_refusal(
        "integer",
        "",
        "SYN_CLI_INTEGER_CANARY",
        &["SYN_CLI_INTEGER_CANARY"],
    );
}

#[test]
fn secret_canonical_integer_bound_error_is_redacted_in_human_and_json() {
    secret_refusal("integer", "min=1\nmax=10", "0004242", &["0004242", "4242"]);
}

#[test]
fn secret_choice_error_is_redacted_in_human_and_json() {
    secret_refusal(
        "choice",
        "choices=[{value='alpha',name='Alpha'},{value='beta',name='Beta'}]",
        "SYN_CLI_CHOICE_CANARY",
        &["SYN_CLI_CHOICE_CANARY"],
    );
}

#[test]
fn secret_missing_path_error_is_redacted_in_human_and_json() {
    secret_refusal(
        "path",
        "must_exist=true",
        "SYN_CLI_MISSING_PATH_CANARY",
        &["SYN_CLI_MISSING_PATH_CANARY"],
    );
}

#[test]
fn public_diagnostics_keep_the_invalid_answer_and_category() {
    let registry = fixture("bool", "", false);
    for mode in [
        vec!["--offline", "--unattended"],
        vec!["--offline", "--unattended", "--json"],
    ] {
        let mut args = mode;
        args.extend_from_slice(&["--set", "answer=PUBLIC_INVALID_BOOL"]);
        let output = registry.app(&["app", "plan"], PACKAGE, &args);
        let text = String::from_utf8(output.stderr).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(
            text.contains("PUBLIC_INVALID_BOOL") && text.contains("parameter_invalid"),
            "{text}"
        );
    }
}

#[test]
fn public_definition_default_remains_visible_while_chosen_secret_is_omitted() {
    let registry = fixture("string", "default='PUBLIC_DECLARED_DEFAULT'", true);
    let description = json(&registry.app(&["app", "describe"], PACKAGE, &["--offline", "--json"]));
    assert!(description.to_string().contains("PUBLIC_DECLARED_DEFAULT"));
    let output = registry.app(
        &["app", "plan"],
        PACKAGE,
        &[
            "--offline",
            "--unattended",
            "--json",
            "--set",
            "answer=SYN_SUCCESS_SECRET_CANARY",
        ],
    );
    let plan = json(&output);
    assert_eq!(plan["parameters"][0]["secret"], true);
    assert!(plan["parameters"][0].get("value").is_none());
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("SYN_SUCCESS_SECRET_CANARY")
    );
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("SYN_SUCCESS_SECRET_CANARY")
    );
}
