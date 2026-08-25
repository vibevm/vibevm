use std::fs;

use specmark::verifies;
use tempfile::tempdir;

use crate::Error;
use crate::manifest::{ExtensionDecl, ExtensionHandler, ExtensionPassKind, Manifest};

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
const PACKAGE: &str =
    "[package]\ngroup = \"org.demo\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n";

fn parse(role: &str, declarations: &str) -> Manifest {
    Manifest::parse_str(&format!("{role}\n{declarations}")).unwrap()
}

fn parse_error(role: &str, declarations: &str) -> String {
    Manifest::parse_str(&format!("{role}\n{declarations}"))
        .unwrap_err()
        .to_string()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES")]
fn all_five_handler_shapes_parse_without_restricting_agent_to_create() {
    let manifest = parse(
        PACKAGE,
        r#"
[[extension]]
id = "builtin"
point = "phase:validate"
handler = { kind = "builtin", name = "" }

[[extension]]
id = "script"
point = "slot:pre-install"
handler = { kind = "script", base = "hooks/prepare" }

[[extension]]
id = "binary"
point = "phase:build"
handler = { kind = "binary", name = "tool" }

[[extension]]
id = "native-source"
point = "compile:emitted"
handler = { kind = "native", crate_dir = "ext/squeeze" }

[[extension]]
id = "native-prebuilt"
point = "phase:test"
handler = { kind = "native", prebuilt = {} }

[[extension]]
id = "agent-not-create"
point = "phase:test"
handler = { kind = "agent", prompt = "not-yet-resolved" }
"#,
    );
    assert_eq!(manifest.extensions.len(), 6);
    assert_eq!(manifest.extensions[0].handler.kind(), "builtin");
    assert_eq!(manifest.extensions[5].handler.kind(), "agent");
    assert!(matches!(
        manifest.extensions[4].handler,
        ExtensionHandler::Native {
            prebuilt: Some(_),
            ..
        }
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn every_optional_field_preserves_presence_and_structural_roundtrip() {
    let manifest = parse(
        PROJECT,
        r#"
[[extension]]
id = "source"
point = "compile:source"
handler = { kind = "builtin", name = "transform" }
config = { nan = nan, zero = -0.0, at = 1979-05-27T07:32:00Z, nested = { values = [1, 1.0, true] } }
auto = false
applies_to = { packages = ["org.demo/*"], paths = ["vibevm/vibespecs/**"] }
when = { future = { mode = "opaque" } }

[[extension]]
id = "phase"
point = "phase:test"
handler = { kind = "agent", prompt = "anything" }
config = {}
inputs = []
when = {}

[[extension]]
id = "pass"
point = "compile:pass"
handler = { kind = "native", crate_dir = "ext/pass" }
auto = true
compiler_internals = true
pass = { kind = "transform", level = "closure", from = "source", to = "document", after = "qualify", before = "link", replace = "anything", formats = [], artifact = "" }
"#,
    );

    let source = &manifest.extensions[0];
    assert!(source.config.is_some());
    assert_eq!(source.auto, Some(false));
    assert!(source.applies_to.is_some());
    assert!(source.when.is_some());
    let zero = source.config.as_ref().unwrap().as_table()["zero"]
        .as_float()
        .unwrap();
    assert_eq!(zero.to_bits(), (-0.0_f64).to_bits());
    assert!(matches!(
        source.config.as_ref().unwrap().as_table().get("at"),
        Some(toml::Value::Datetime(_))
    ));

    let phase = &manifest.extensions[1];
    assert!(phase.config.as_ref().unwrap().is_empty());
    assert_eq!(phase.inputs, Some(Vec::new()));
    assert!(phase.when.as_ref().unwrap().is_empty());

    let pass = manifest.extensions[2].pass.as_ref().unwrap();
    assert_eq!(pass.kind, ExtensionPassKind::Transform);
    assert_eq!(pass.formats, Some(Vec::new()));

    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
    assert_eq!(
        reparsed
            .extensions
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        ["source", "phase", "pass"]
    );
    let reparsed_zero = reparsed.extensions[0].config.as_ref().unwrap().as_table()["zero"]
        .as_float()
        .unwrap();
    assert_eq!(reparsed_zero.to_bits(), (-0.0_f64).to_bits());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn manifest_write_preserves_commented_nested_extension_tables() {
    let project = tempdir().unwrap();
    let path = project.path().join(Manifest::FILENAME);
    fs::write(
        &path,
        r#"[project]
name = "demo"
version = "0.1.0"

[[extension]]
id = "pass"
point = "compile:pass"
compiler_internals = true

[extension.handler]
# handler-comment
kind = "builtin"
name = "log"

[extension.config]
# config-comment
message = "hello"

[extension.pass]
# pass-comment
kind = "transform"
level = "closure"
"#,
    )
    .unwrap();

    let mut manifest = Manifest::read(&path).unwrap();
    manifest.project.as_mut().unwrap().version = "0.2.0".into();
    manifest.write(&path).unwrap();

    let written = fs::read_to_string(&path).unwrap();
    for comment in ["# handler-comment", "# config-comment", "# pass-comment"] {
        assert!(written.contains(comment), "missing {comment}:\n{written}");
    }
    assert!(written.contains("version = \"0.2.0\""), "{written}");
    assert!(!written.contains("version = \"0.1.0\""), "{written}");
    let reparsed = Manifest::read(&path).unwrap();
    assert_eq!(reparsed.project.unwrap().version, "0.2.0");
    assert_eq!(reparsed.extensions.len(), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn declarations_are_role_blind_between_project_and_package_but_not_virtual_workspace() {
    let row = r#"
[[extension]]
id = "announce"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
"#;
    assert_eq!(parse(PROJECT, row).extensions.len(), 1);
    assert_eq!(parse(PACKAGE, row).extensions.len(), 1);
    assert_eq!(
        parse(&format!("{PROJECT}\n[workspace]\nmembers = []\n"), row)
            .extensions
            .len(),
        1
    );
    assert_eq!(
        parse(&format!("{PACKAGE}\n[workspace]\nmembers = []\n"), row)
            .extensions
            .len(),
        1
    );

    let error = parse_error("[workspace]\nmembers = []\n", row);
    assert!(error.contains("pure virtual `[workspace]`"), "{error}");
    assert!(error.contains("PROP-054#CONTRIB-GRAMMAR"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn required_fields_fail_as_toml_shape_errors_and_name_the_field() {
    for (row, field) in [
        (
            "[[extension]]\npoint = \"phase:build\"\nhandler = { kind = \"builtin\", name = \"x\" }\n",
            "id",
        ),
        (
            "[[extension]]\nid = \"x\"\nhandler = { kind = \"builtin\", name = \"x\" }\n",
            "point",
        ),
        (
            "[[extension]]\nid = \"x\"\npoint = \"phase:build\"\n",
            "handler",
        ),
    ] {
        match Manifest::parse_str(&format!("{PROJECT}\n{row}")) {
            Err(Error::ParseToml { detail, .. }) => {
                assert!(detail.contains(field), "field={field}: {detail}");
            }
            other => panic!("expected ParseToml for missing `{field}`, got {other:?}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn strict_unknown_fields_cover_the_row_selector_and_pass_tables() {
    for row in [
        "[[extension]]\nid = \"x\"\npoint = \"phase:build\"\nhandler = { kind = \"builtin\", name = \"x\" }\nmystery = true\n",
        "[[extension]]\nid = \"x\"\npoint = \"compile:source\"\nhandler = { kind = \"builtin\", name = \"x\" }\napplies_to = { packages = [], mystery = true }\n",
        "[[extension]]\nid = \"x\"\npoint = \"compile:pass\"\nhandler = { kind = \"builtin\", name = \"x\" }\ncompiler_internals = true\npass = { kind = \"transform\", mystery = true }\n",
    ] {
        match Manifest::parse_str(&format!("{PROJECT}\n{row}")) {
            Err(Error::ParseToml { detail, .. }) => {
                assert!(detail.contains("unknown field"), "{detail}");
                assert!(detail.contains("mystery"), "{detail}");
            }
            other => panic!("expected ParseToml for unknown field, got {other:?}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
fn bad_point_is_a_typed_toml_shape_error() {
    match Manifest::parse_str(&format!(
        "{PROJECT}\n[[extension]]\nid = \"x\"\npoint = \"phase:BUILD\"\nhandler = {{ kind = \"builtin\", name = \"x\" }}\n"
    )) {
        Err(Error::ParseToml { detail, .. }) => {
            assert!(
                detail.contains("field `point` value `phase:BUILD`"),
                "{detail}"
            );
            assert!(detail.contains("PROP-054#POINT-GRAMMAR"), "{detail}");
        }
        other => panic!("expected ParseToml for typed point, got {other:?}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-FIELDS")]
fn duplicate_ids_are_semantic_invalid_manifest_errors() {
    let declarations = r#"
[[extension]]
id = "same"
point = "phase:build"
handler = { kind = "builtin", name = "a" }
[[extension]]
id = "same"
point = "phase:test"
handler = { kind = "builtin", name = "b" }
"#;
    match Manifest::parse_str(&format!("{PROJECT}\n{declarations}")) {
        Err(Error::InvalidManifest { reason }) => {
            assert!(reason.contains("duplicate [[extension]]"), "{reason}");
            assert!(reason.contains("value `same`"), "{reason}");
        }
        other => panic!("expected InvalidManifest for duplicate id, got {other:?}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn semantic_extension_errors_follow_legacy_role_precedence() {
    let invalid_extension = r#"
[[extension]]
id = "x"
point = "phase:build"
handler = { kind = "builtin", name = "x" }
auto = false
"#;

    match Manifest::parse_str(&format!("{PROJECT}\n{invalid_extension}")) {
        Err(Error::InvalidManifest { reason }) => {
            assert!(reason.contains("field `auto`"), "{reason}");
        }
        other => panic!("expected semantic InvalidManifest, got {other:?}"),
    }

    match Manifest::parse_str(&format!("{PROJECT}\n{PACKAGE}\n{invalid_extension}")) {
        Err(Error::InvalidManifest { reason }) => {
            assert!(reason.contains("mutually exclusive"), "{reason}");
            assert!(!reason.contains("field `auto`"), "{reason}");
        }
        other => panic!("expected role InvalidManifest first, got {other:?}"),
    }

    match Manifest::parse_str(invalid_extension) {
        Err(Error::InvalidManifest { reason }) => {
            assert!(reason.contains("manifest declares no role"), "{reason}");
            assert!(!reason.contains("field `auto`"), "{reason}");
        }
        other => panic!("expected no-role InvalidManifest first, got {other:?}"),
    }

    let unknown = "[[extension]]\nid = \"x\"\npoint = \"phase:build\"\nhandler = { kind = \"builtin\", name = \"x\", mystery = true }\n";
    match Manifest::parse_str(&format!("{PROJECT}\n{unknown}")) {
        Err(Error::ParseToml { detail, .. }) => {
            assert!(detail.contains("mystery"), "{detail}");
        }
        other => panic!("expected unknown field to stay ParseToml, got {other:?}"),
    }
}

#[test]
fn required_strings_do_not_acquire_an_unwritten_content_grammar() {
    let manifest = parse(
        PROJECT,
        r#"
[[extension]]
id = ""
point = "phase:build"
handler = { kind = "builtin", name = "" }
"#,
    );
    assert_eq!(manifest.extensions[0].id, "");
    assert!(matches!(
        &manifest.extensions[0].handler,
        ExtensionHandler::Builtin { name } if name.is_empty()
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILER-INTERNALS-FLAG")]
fn pass_wire_accepts_every_kind_and_level_without_placement_policy() {
    let manifest = parse(
        PROJECT,
        r#"
[[extension]]
id = "transform"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "transform", level = "source" }

[[extension]]
id = "lowering"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "lowering", from = "document", to = "closure" }

[[extension]]
id = "frontend"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "frontend", level = "lane", formats = ["txt"] }

[[extension]]
id = "backend"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "backend", level = "emitted", artifact = "json" }
"#,
    );
    assert_eq!(
        manifest
            .extensions
            .iter()
            .map(|row| row.pass.as_ref().unwrap().kind)
            .collect::<Vec<_>>(),
        [
            ExtensionPassKind::Transform,
            ExtensionPassKind::Lowering,
            ExtensionPassKind::Frontend,
            ExtensionPassKind::Backend,
        ]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn optional_field_presence_laws_are_enforced_through_the_wire() {
    for (point, field, value) in [
        ("phase:build", "auto", "auto = false"),
        ("slot:pre-install", "inputs", "inputs = []"),
        (
            "compile:lane",
            "applies_to",
            "applies_to = { packages = [] }",
        ),
        (
            "phase:test",
            "compiler_internals",
            "compiler_internals = false",
        ),
        ("phase:test", "pass", "pass = { kind = \"frontend\" }"),
    ] {
        let body = format!(
            "[[extension]]\nid = \"x\"\npoint = \"{point}\"\nhandler = {{ kind = \"builtin\", name = \"x\" }}\n{value}\n"
        );
        let error = parse_error(PROJECT, &body);
        assert!(error.contains(field), "{error}");
    }

    for flag in ["", "compiler_internals = false"] {
        let body = format!(
            "[[extension]]\nid = \"x\"\npoint = \"compile:pass\"\nhandler = {{ kind = \"builtin\", name = \"x\" }}\n{flag}\n"
        );
        let error = parse_error(PROJECT, &body);
        assert!(
            error.contains("requires field `compiler_internals = true`"),
            "{error}"
        );
    }
}

#[test]
fn extension_use_spelling_remains_rejected_until_its_owner_ruling() {
    let error = parse_error(
        PROJECT,
        r#"
[[extension]]
id = "x"
point = "compile:emitted"
handler = { kind = "builtin", name = "x" }

[[extension.use]]
ref = "org.demo/pkg#x"
"#,
    );
    assert!(error.contains("unknown field"), "{error}");
    assert!(error.contains("use"), "{error}");
}

#[test]
fn programmatic_declarations_revalidate_before_validation_and_serialization() {
    let mut manifest = Manifest::new_project("demo", "0.1.0");
    manifest.extensions.push(ExtensionDecl {
        id: "programmatic".into(),
        point: "phase:build".parse().unwrap(),
        handler: ExtensionHandler::Builtin { name: "log".into() },
        config: None,
        auto: Some(false),
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    });

    let validation = manifest.validate().unwrap_err().to_string();
    assert!(validation.contains("field `auto`"), "{validation}");
    let serialization = toml::to_string_pretty(&manifest).unwrap_err().to_string();
    assert!(serialization.contains("field `auto`"), "{serialization}");
}
