//! The `compile:pass` wire of an extension row: each frozen pass-kind shape
//! parses and roundtrips, and every missing, conflicting, blank or foreign
//! field is refused.

use specmark::verifies;

use super::tests_extensions::{PROJECT, parse, parse_error};
use crate::manifest::{ExtensionPassKind, Manifest};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILER-INTERNALS-FLAG")]
fn pass_wire_accepts_each_frozen_kind_shape_and_roundtrips_it() {
    let manifest = parse(
        PROJECT,
        r#"
[[extension]]
id = "transform"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "transform", level = "source", after = "parse" }
[[extension]]
id = "lowering"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "lowering", from = "document", to = "closure", before = "close" }
[[extension]]
id = "replacement-lowering"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "lowering", replace = "emit", artifact = "static-xml" }
[[extension]]
id = "frontend"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "frontend", formats = ["txt"] }
[[extension]]
id = "backend"
point = "compile:pass"
handler = { kind = "builtin", name = "x" }
compiler_internals = true
pass = { kind = "backend", artifact = "json" }
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
            ExtensionPassKind::Lowering,
            ExtensionPassKind::Frontend,
            ExtensionPassKind::Backend,
        ]
    );
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert_eq!(manifest, Manifest::parse_str(&rendered).unwrap());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW")]
fn pass_wire_refuses_missing_conflicting_blank_and_foreign_fields() {
    for (pass, expected) in [
        (
            r#"pass = { kind = "transform", after = "parse" }"#,
            "requires field `level`",
        ),
        (
            r#"pass = { kind = "transform", level = "source" }"#,
            "exactly one",
        ),
        (
            r#"pass = { kind = "transform", level = "source", after = "parse", from = "source" }"#,
            "forbids field `from`",
        ),
        (
            r#"pass = { kind = "lowering", from = "source", after = "parse" }"#,
            "requires fields `from` and `to`",
        ),
        (
            r#"pass = { kind = "lowering" }"#,
            "requires fields `from` and `to`",
        ),
        (
            r#"pass = { kind = "lowering", after = "parse", artifact = "  " }"#,
            "must not be blank",
        ),
        (
            r#"pass = { kind = "frontend" }"#,
            "requires field `formats`",
        ),
        (
            r#"pass = { kind = "frontend", formats = [] }"#,
            "field `formats` is empty",
        ),
        (
            r#"pass = { kind = "frontend", formats = ["txt"], after = "parse" }"#,
            "forbids field `after`",
        ),
        (
            r#"pass = { kind = "backend" }"#,
            "requires field `artifact`",
        ),
        (
            r#"pass = { kind = "backend", artifact = " " }"#,
            "must not be blank",
        ),
    ] {
        let error = parse_error(
            PROJECT,
            &format!(
                "[[extension]]\nid = \"x\"\npoint = \"compile:pass\"\nhandler = {{ kind = \"builtin\", name = \"x\" }}\ncompiler_internals = true\n{pass}\n"
            ),
        );
        assert!(error.contains(expected), "expected {expected:?}: {error}");
    }
}
