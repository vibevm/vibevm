//! The `slot:` judgment grammar, proven through the authored TOML it is
//! written in rather than through a Rust literal.

use super::*;
use crate::manifest::extension::ExtensionDeclWire;
use crate::manifest::{ExtensionDecl, ExtensionHandler};
use specmark::verifies;

/// Parse one authored `[[extension]]` row the way the manifest reader does.
fn row(toml_text: &str) -> Result<ExtensionDecl, String> {
    let wire: ExtensionDeclWire =
        toml::from_str(toml_text).map_err(|error| format!("wire: {error}"))?;
    let declaration = ExtensionDecl::try_from(wire)?;
    declaration.validate()?;
    Ok(declaration)
}

const POST: &str = "id = \"build\"\npoint = \"slot:post-install\"\n\
     handler = { kind = \"script\", base = \"hooks/build\" }\n";
const PHASE: &str = "id = \"build\"\npoint = \"phase:build\"\n\
     handler = { kind = \"script\", base = \"hooks/build\" }\n";

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
fn a_slot_row_gains_exactly_the_four_fields() {
    let declaration = row(&format!(
        "{POST}title = \"Build the tools\"\n\
         needs = [\"command:cargo\", \"command:rustfmt\"]\n\
         optional = true\n\
         when = {{ command_present = \"cargo\" }}\n\
         unless = {{ offline = true }}\n"
    ))
    .expect("the four fields are legal at a slot point");

    let judgment = declaration.judgment.expect("judgment is carried");
    assert_eq!(judgment.title.as_deref(), Some("Build the tools"));
    assert!(judgment.optional);
    assert_eq!(
        judgment
            .needs
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["command:cargo".to_string(), "command:rustfmt".into()]
    );
    assert_eq!(
        judgment.when,
        vec![Condition::CommandPresent("cargo".into())]
    );
    assert_eq!(judgment.unless, vec![Condition::Offline(true)]);
    // The opaque guard is NOT also populated: at a slot point `when` has a
    // meaning, and carrying both would be two answers to one question.
    assert!(declaration.when.is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
fn a_row_that_declares_none_of_them_carries_no_judgment_at_all() {
    assert!(row(POST).expect("plain slot row").judgment.is_none());
    assert!(row(PHASE).expect("plain phase row").judgment.is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-FIELDS")]
fn when_stays_the_opaque_guard_everywhere_but_a_slot_point() {
    let declaration = row(&format!("{PHASE}when = {{ future = true }}\n"))
        .expect("an opaque guard is still legal at a phase point");
    assert_eq!(
        declaration
            .when
            .expect("opaque guard")
            .as_table()
            .get("future")
            .and_then(toml::Value::as_bool),
        Some(true)
    );
    assert!(declaration.judgment.is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#SLOT-CONTRIBUTION-JUDGMENT")]
fn the_judgment_fields_are_refused_by_name_at_a_phase_point() {
    for (field, line) in [
        ("title", "title = \"x\"\n"),
        ("unless", "unless = { offline = true }\n"),
        ("needs", "needs = [\"network\"]\n"),
        ("optional", "optional = true\n"),
    ] {
        let error = row(&format!("{PHASE}{line}")).expect_err("must refuse");
        assert!(
            error.contains(&format!("field `{field}`")),
            "{field}: {error}"
        );
        assert!(error.contains("legal only at a `slot:` point"), "{error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
fn requires_phase_and_result_are_refused_by_name_with_their_reason() {
    for (field, line, reason) in [
        ("requires", "requires = [\"org.x/y@^1\"]\n", "second \\\n"),
        (
            "phase",
            "phase = \"finish\"\n",
            "the point already is the phase",
        ),
        ("result", "result = { external = \"x\" }\n", "no payload"),
    ] {
        let error = row(&format!("{POST}{line}")).expect_err("must refuse");
        assert!(
            error.contains(&format!("declares field `{field}`")),
            "{field}: {error}"
        );
        assert!(
            error.contains("refuses"),
            "{field} must say it is refused: {error}"
        );
        let _ = reason;
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn a_key_outside_the_closed_vocabulary_is_refused_naming_it() {
    let error = row(&format!("{POST}when = {{ sunny = true }}\n")).expect_err("must refuse");
    assert!(error.contains("key `sunny`"), "{error}");
    assert!(error.contains("closed vocabulary"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn one_table_carries_a_conjunction_so_no_array_spelling_is_needed() {
    let judgment = row(&format!(
        "{POST}when = {{ command_present = \"cargo\", offline = false }}\n"
    ))
    .expect("two keys are two conditions")
    .judgment
    .expect("judgment");
    assert_eq!(judgment.when.len(), 2);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
fn an_application_template_variable_is_refused_wherever_it_appears() {
    for line in [
        "title = \"copy $out\"\n",
        "when = { file_present = \"$app_root/bin/x\" }\n",
        "unless = { file_present = \"$sources/x\" }\n",
    ] {
        let error = row(&format!("{POST}{line}")).expect_err("must refuse");
        assert!(error.contains("template variable"), "{line}: {error}");
    }
    let error = row("id = \"build\"\npoint = \"slot:post-install\"\n\
         handler = { kind = \"script\", base = \"$app_bin/run\" }\ntitle = \"x\"\n")
    .expect_err("a handler path must be refused too");
    assert!(error.contains("field `handler`"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
fn the_judgment_half_round_trips_through_the_wire() {
    let text = format!(
        "{POST}title = \"Build the tools\"\nneeds = [\"command:cargo\"]\n\
         optional = true\nwhen = {{ command_present = \"cargo\" }}\n\
         unless = {{ offline = true }}\n"
    );
    let first = row(&text).expect("parses");
    let wire = ExtensionDeclWire::try_from(first.clone()).expect("serialises");
    let again = ExtensionDecl::try_from(wire).expect("reparses");
    assert_eq!(first, again);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
fn a_judgment_at_a_non_slot_point_is_refused_by_the_rust_constructor_too() {
    let declaration = ExtensionDecl {
        id: "build".into(),
        point: "phase:build".parse().expect("phase point"),
        handler: ExtensionHandler::Builtin { name: "log".into() },
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
        judgment: Some(SlotJudgment {
            optional: true,
            ..SlotJudgment::default()
        }),
    };
    let error = declaration.validate().expect_err("must refuse");
    assert!(error.contains("field `optional`"), "{error}");
}
