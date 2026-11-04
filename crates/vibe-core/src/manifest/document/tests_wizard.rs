//! The wizard grammar of an installer option (`PROP-065#feature-tree`,
//! `#profiles`, `#parameters`, `#license`, `#sizes`, `#icon`).

use super::Manifest;
use crate::manifest::{
    Condition, InstallerOptionDecl, ParamEquals, ParameterType, ParameterValue, StepKind,
};

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn wizard() -> String {
    format!(
        r#"
[package]
name = "demo"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "demo"
icon = "share/demo.png"

[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"

[installer_options.license]
file = "LICENSE.txt"
name = "Demo License 1.0"

[[installer_options.features]]
id = "core"
name = "Core"
required = true

[[installer_options.features]]
id = "gui"
name = "GUI"
default = true

[[installer_options.features]]
id = "gtk"
name = "GTK"
parent = "gui"
exclusive_group = "toolkit"
default = true

[[installer_options.features]]
id = "qt"
name = "Qt"
parent = "gui"
exclusive_group = "toolkit"
requires = ["core"]

[[installer_options.profiles]]
id = "typical"
name = "Typical"
features = ["core", "gui"]
default = true

[[installer_options.parameters]]
id = "port"
name = "Port"
type = "integer"
default = 8080
min = 1
max = 65535

[[installer_options.parameters]]
id = "lang"
name = "Language"
type = "choice"
default = "en"
choices = [{{ value = "en", name = "English" }}, {{ value = "ru", name = "Russian" }}]

[[installer_options.parameters]]
id = "token"
name = "Token"
type = "string"
secret = true

[[installer_options.steps]]
id = "fetch"
title = "Fetch"
kind = "archive"
format = "tar_gz"
source = "https://example.com/a.tar.gz"
sha256 = "{SHA}"
size = 1024

[[installer_options.steps]]
id = "lang"
title = "Language pack"
kind = "copy"
from = "lang"
to = "share/$param:lang"
when = [{{ param = "token" }}, {{ param_equals = {{ id = "lang", value = "ru" }} }}]
"#
    )
}

fn option(text: &str) -> InstallerOptionDecl {
    Manifest::parse_str(text)
        .unwrap()
        .installer_options
        .remove(0)
}

fn refused(text: &str) -> String {
    Manifest::parse_str(text).unwrap_err().to_string()
}

#[test]
fn the_wizard_declarations_parse() {
    let manifest = Manifest::parse_str(&wizard()).unwrap();
    let icon = manifest
        .application
        .as_ref()
        .and_then(|app| app.icon.clone());
    assert_eq!(
        icon,
        Some(crate::manifest::IconDecl::Path("share/demo.png".into()))
    );
    let option = &manifest.installer_options[0];
    assert_eq!(option.license.as_ref().unwrap().name, "Demo License 1.0");
    assert_eq!(option.features[2].parent.as_deref(), Some("gui"));
    assert_eq!(option.features[3].requires, ["core"]);
    assert!(option.features[0].required);
    assert_eq!(option.profiles[0].features, ["core", "gui"]);
    assert_eq!(option.parameters[0].kind, ParameterType::Integer);
    assert_eq!(
        option.parameters[0].default,
        Some(ParameterValue::Integer(8080))
    );
    assert_eq!(option.parameters[1].choices[1].value, "ru");
    assert!(option.parameters[2].secret);
    assert!(matches!(
        option.steps[0].kind,
        StepKind::Archive {
            size: Some(1024),
            ..
        }
    ));
    assert_eq!(
        option.steps[1].when,
        [
            Condition::Param("token".into()),
            Condition::ParamEquals(ParamEquals {
                id: "lang".into(),
                value: "ru".into()
            })
        ]
    );
    let text = toml::to_string(option).unwrap();
    assert!(text.contains("exclusive_group = \"toolkit\""), "{text}");
    assert_eq!(
        &toml::from_str::<InstallerOptionDecl>(&text).unwrap(),
        option
    );
}

#[test]
fn parameter_values_are_canonical_and_typed() {
    let option = option(&wizard());
    let port = &option.parameters[0];
    assert_eq!(port.canonical("443").unwrap(), "443");
    assert!(port.canonical("70000").unwrap_err().contains("outside"));
    assert!(
        port.canonical("eighty")
            .unwrap_err()
            .contains("not an integer")
    );
    let lang = &option.parameters[1];
    assert_eq!(lang.canonical("ru").unwrap(), "ru");
    assert!(
        lang.canonical("de")
            .unwrap_err()
            .contains("not one of en, ru")
    );
    let token = &option.parameters[2];
    assert_eq!(token.canonical("").unwrap(), "");
    assert!(token.canonical("a\nb").is_err());
}

#[test]
fn wizard_rules_are_enforced_with_named_reasons() {
    let base = wizard();
    let cases: Vec<(String, &str)> = vec![
        (base.replace("parent = \"gui\"\nexclusive_group = \"toolkit\"\ndefault", "parent = \"nope\"\nexclusive_group = \"toolkit\"\ndefault"), "undeclared parent"),
        (base.replace("name = \"GUI\"\ndefault = true", "name = \"GUI\"\ndefault = true\nparent = \"gtk\""), "cycle"),
        (base.replace("requires = [\"core\"]", "requires = [\"qt\"]"), "not another declared feature"),
        (base.replace("requires = [\"core\"]", "requires = [\"core\"]\ndefault = true"), "2 default members"),
        (base.replace("features = [\"core\", \"gui\"]", "features = [\"core\", \"nope\"]"), "undeclared feature"),
        (base.replace("features = [\"core\", \"gui\"]\ndefault = true", "features = [\"core\", \"gui\"]\ndefault = true\n[[installer_options.profiles]]\nid = \"all\"\nname = \"All\"\ndefault = true"), "more than one default profile"),
        (base.replace("default = 8080", "default = 99999"), "outside"),
        (base.replace("default = 8080", "default = \"8080\""), "not a integer value"),
        (base.replace("default = \"en\"", "default = \"de\""), "not one of"),
        (base.replace("type = \"string\"\nsecret", "type = \"string\"\nmin = 1\nsecret"), "apply only to type = \"integer\""),
        (base.replace("type = \"string\"\nsecret", "type = \"choice\"\nsecret"), "nonempty choices"),
        (base.replace("{ param = \"token\" }", "{ param = \"nope\" }"), "undeclared parameter"),
        (base.replace("share/$param:lang", "share/$param:nope"), "undeclared parameter"),
        (base.replace("file = \"LICENSE.txt\"", "file = \"../LICENSE.txt\""), "license].file"),
        (base.replace("icon = \"share/demo.png\"", "icon = \"/abs/demo.png\""), "[application].icon"),
        (base.replace("size = 1024", "size = -1"), "expected u64"),
    ];
    for (text, needle) in cases {
        let error = refused(&text);
        assert!(error.contains(needle), "expected `{needle}` in: {error}");
    }
}

#[cfg(test)]
fn diagnostic_parameter(kind: &str, fields: &str, secret: bool) -> crate::manifest::ParameterDecl {
    let text = format!(
        "[package]\nname='diagnostic'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n\
         [application]\nid='diagnostic'\n\
         [[installer_options]]\nid='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\n\
         [[installer_options.parameters]]\nid='answer'\nname='Answer'\ntype='{kind}'\nsecret={secret}\n{fields}\n\
         [[installer_options.steps]]\nid='build'\ntitle='Build'\nkind='recipe'\nscript='build.sh'\n"
    );
    option(&text).parameters.remove(0)
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#SECRET-BOUNDARY")]
fn secret_canonical_errors_omit_supplied_text() {
    let mut failures = Vec::new();
    for (kind, fields, raw, reason) in [
        ("bool", "", "SYN_SECRET_BOOL_CANARY", "not a boolean"),
        ("integer", "", "SYN_SECRET_INTEGER_CANARY", "not an integer"),
        (
            "choice",
            "choices=[{value='alpha',name='Alpha'},{value='beta',name='Beta'}]",
            "SYN_SECRET_CHOICE_CANARY",
            "not one of alpha, beta",
        ),
        ("string", "", "SYN_SECRET_CONTROL\n", "control characters"),
        ("path", "", "SYN_SECRET_PATH\n", "control characters"),
    ] {
        let error = diagnostic_parameter(kind, fields, true)
            .canonical(raw)
            .unwrap_err();
        assert!(error.contains(reason), "{kind}: {error}");
        if error.contains(raw) {
            failures.push((kind, error));
        }
    }
    assert!(failures.is_empty(), "secret answer in errors: {failures:?}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#SECRET-BOUNDARY")]
fn secret_integer_bounds_do_not_echo_the_canonical_value() {
    let error = diagnostic_parameter("integer", "min=1\nmax=10", true)
        .canonical("0004242")
        .unwrap_err();
    assert!(error.contains("outside 1..=10"));
    assert!(
        !error.contains("4242"),
        "canonical secret integer was exposed: {error}"
    );
}

#[test]
fn public_parameter_diagnostics_and_valid_canonical_values_are_unchanged() {
    for (kind, fields, raw, expected) in [
        (
            "bool",
            "",
            "maybe",
            "`maybe` is not a boolean (true / false)",
        ),
        ("integer", "", "text", "`text` is not an integer"),
        (
            "integer",
            "min=1\nmax=10",
            "0004242",
            "4242 is outside 1..=10",
        ),
        (
            "choice",
            "choices=[{value='alpha',name='Alpha'}]",
            "other",
            "`other` is not one of alpha",
        ),
        ("path", "", "   ", "a path must be nonempty"),
    ] {
        assert_eq!(
            diagnostic_parameter(kind, fields, false)
                .canonical(raw)
                .unwrap_err(),
            expected
        );
    }
    for (kind, fields, raw, expected) in [
        ("bool", "default=true", " YeS ", "true"),
        ("bool", "", "0", "false"),
        ("integer", "min=1\nmax=100", " 080 ", "80"),
        (
            "choice",
            "choices=[{value='alpha',name='Alpha'}]\ndefault='alpha'",
            "alpha",
            "alpha",
        ),
        ("string", "required=true\ndefault='public default'", "", ""),
        ("string", "", " spaced value ", " spaced value "),
        ("path", "", "relative/path", "relative/path"),
    ] {
        for secret in [false, true] {
            assert_eq!(
                diagnostic_parameter(kind, fields, secret)
                    .canonical(raw)
                    .unwrap(),
                expected
            );
        }
    }
}
