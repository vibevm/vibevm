use anyhow::Result;
use serde_json::{Value, json};

use super::apply_wire_order;

fn apply(doc: &Value, src: &str) -> Result<String> {
    let temp = tempfile::tempdir()?;
    let schema = temp.path().join("fixture.jtd.json");
    std::fs::write(&schema, serde_json::to_vec_pretty(doc)?)?;
    apply_wire_order(src, "generated/fixture/mod.rs", &schema, &schema)
}

fn root(order: Option<Value>) -> Value {
    let mut metadata = serde_json::Map::new();
    if let Some(order) = order {
        metadata.insert("x-wire-order".to_owned(), order);
    }
    json!({
        "metadata": metadata,
        "properties": {
            "displayName": {"type": "string"},
            "zeta_value": {"type": "string"}
        },
        "optionalProperties": {
            "optional_value": {
                "type": "string",
                "metadata": {"x-default": null}
            }
        }
    })
}

const GENERATED: &str = r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    /// Display docs stay with displayName.
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Optional docs and serde policy move together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_value: Option<String>,

    /// Zeta docs stay byte-exact.
    pub zeta_value: String,
}
"#;

#[test]
fn nonalphabetic_required_and_optional_order_moves_complete_chunks() -> Result<()> {
    let out = apply(
        &root(Some(json!(["zeta_value", "displayName", "optional_value"]))),
        GENERATED,
    )?;
    assert_eq!(
        out,
        r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    /// Zeta docs stay byte-exact.
    pub zeta_value: String,

    /// Display docs stay with displayName.
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Optional docs and serde policy move together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_value: Option<String>,
}
"#
    );
    Ok(())
}

#[test]
fn resolved_shared_definition_is_ordered_without_touching_the_root() -> Result<()> {
    let doc = json!({
        "properties": {"root": {"ref": "shared_record"}},
        "definitions": {
            "shared_record": {
                "metadata": {"x-wire-order": ["second", "first"]},
                "properties": {
                    "first": {"type": "string"},
                    "second": {"type": "string"}
                }
            }
        }
    });
    let src = r#"pub struct Fixture {
    pub root: SharedRecord,
}

pub struct SharedRecord {
    pub first: String,

    pub second: String,
}
"#;
    let out = apply(&doc, src)?;
    assert!(out.starts_with("pub struct Fixture {\n    pub root: SharedRecord,\n}"));
    assert!(
        out.contains(
            "pub struct SharedRecord {\n    pub second: String,\n\n    pub first: String,\n}"
        ),
        "{out}"
    );
    Ok(())
}

#[test]
fn relevant_full_pipeline_uses_post_snake_post_optional_chunks() -> Result<()> {
    let doc = root(Some(json!(["optional_value", "displayName", "zeta_value"])));
    let temp = tempfile::tempdir()?;
    let schema = temp.path().join("fixture.jtd.json");
    std::fs::write(&schema, serde_json::to_vec_pretty(&doc)?)?;
    let emitted = r#"pub struct Fixture {
    #[serde(rename = "displayName")]
    pub displayName: String,

    #[serde(rename = "optional_value")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optionalValue: Option<Box<String>>,

    #[serde(rename = "zeta_value")]
    pub zetaValue: String,
}
"#;
    let snaked = crate::codegen::snake_case::snake_case_fields(emitted, "fixture")?;
    let unboxed = crate::codegen::optional_shapes::apply_optional_shapes(
        &snaked, "fixture", &schema, &schema,
    )?;
    let ordered = apply_wire_order(&unboxed, "fixture", &schema, &schema)?;
    assert!(
        ordered.find("pub optional_value").unwrap() < ordered.find("pub display_name").unwrap()
    );
    assert!(ordered.find("pub display_name").unwrap() < ordered.find("pub zeta_value").unwrap());
    assert!(ordered.contains("pub optional_value: Option<String>"));
    assert!(ordered.contains("#[serde(rename = \"displayName\")]"));
    Ok(())
}

#[test]
fn pass_is_idempotent_and_preserves_crlf_and_eof() -> Result<()> {
    let doc = root(Some(json!(["zeta_value", "displayName", "optional_value"])));
    let crlf = GENERATED.replace('\n', "\r\n");
    let once = apply(&doc, &crlf)?;
    let twice = apply(&doc, &once)?;
    assert_eq!(once, twice);
    assert!(once.contains("\r\n\r\n"));

    let no_final_newline = GENERATED.trim_end_matches('\n');
    let reordered = apply(&doc, no_final_newline)?;
    assert!(!reordered.ends_with('\n'));
    Ok(())
}

#[test]
fn absent_key_is_an_exact_noop_even_for_unfamiliar_rust() -> Result<()> {
    let src = "this is deliberately not generated Rust\r\n";
    assert_eq!(apply(&root(None), src)?, src);
    Ok(())
}

#[test]
fn annotated_empty_root_accepts_the_pinned_one_line_struct_as_exact_noop() -> Result<()> {
    let doc = json!({
        "metadata": {"x-wire-order": []},
        "properties": {}
    });
    let src = "#[derive(Serialize, Deserialize)]\r\npub struct Fixture {}\r\n";
    assert_eq!(apply(&doc, src)?, src);
    Ok(())
}

#[test]
fn annotated_empty_named_definition_accepts_its_one_line_struct_as_exact_noop() -> Result<()> {
    let doc = json!({
        "definitions": {
            "empty_row": {
                "metadata": {"x-wire-order": []},
                "properties": {}
            }
        },
        "properties": {"row": {"ref": "empty_row"}}
    });
    let src = "pub struct Fixture {\n    pub row: EmptyRow,\n}\n\npub struct EmptyRow {}\n";
    assert_eq!(apply(&doc, src)?, src);
    Ok(())
}

#[test]
fn every_invalid_array_shape_refuses_with_schema_and_member_context() {
    for (label, order, needle) in [
        ("not-array", json!("zeta_value"), "must be an array"),
        (
            "non-string",
            json!(["zeta_value", 7, "optional_value"]),
            "not a string",
        ),
        (
            "duplicate",
            json!(["zeta_value", "zeta_value", "displayName", "optional_value"]),
            "repeats member",
        ),
        (
            "unknown",
            json!(["zeta_value", "displayName", "ghost"]),
            "unknown member `ghost`",
        ),
        (
            "missing",
            json!(["zeta_value", "displayName"]),
            "omits member(s) optional_value",
        ),
    ] {
        let error = apply(&root(Some(order)), GENERATED)
            .expect_err(label)
            .to_string();
        assert!(error.contains("fixture.jtd.json"), "{label}: {error}");
        assert!(error.contains("(the root)"), "{label}: {error}");
        assert!(error.contains(needle), "{label}: {error}");
    }
}

#[test]
fn non_object_anonymous_mapping_and_name_collisions_refuse() {
    let non_object = json!({
        "metadata": {"x-wire-order": []},
        "type": "string"
    });
    assert!(
        apply(&non_object, "pub type Fixture = String;\n")
            .unwrap_err()
            .to_string()
            .contains("not an object form")
    );

    let anonymous = json!({
        "properties": {
            "outer": {
                "metadata": {"x-wire-order": ["inner"]},
                "properties": {"inner": {"type": "string"}}
            }
        }
    });
    assert!(
        apply(
            &anonymous,
            "pub struct Fixture {\n    pub outer: Outer,\n}\n"
        )
        .unwrap_err()
        .to_string()
        .contains("anonymous and mapping objects are unsupported")
    );

    let mapping = json!({
        "discriminator": "kind",
        "mapping": {
            "one": {
                "metadata": {"x-wire-order": ["value"]},
                "properties": {"value": {"type": "string"}}
            }
        }
    });
    assert!(
        apply(&mapping, "pub enum Fixture {}\n")
            .unwrap_err()
            .to_string()
            .contains("anonymous and mapping objects are unsupported")
    );

    let collision = json!({
        "definitions": {
            "foo_bar": {
                "metadata": {"x-wire-order": ["value"]},
                "properties": {"value": {"type": "string"}}
            },
            "foo__bar": {
                "properties": {"value": {"type": "string"}}
            }
        }
    });
    assert!(
        apply(&collision, "")
            .unwrap_err()
            .to_string()
            .contains("cannot resolve generated struct `FooBar` uniquely")
    );
}

#[test]
fn unfamiliar_generated_shape_refuses_with_type_and_definition() {
    let error = apply(
        &root(Some(json!(["displayName", "zeta_value", "optional_value"]))),
        "pub struct Fixture {\n    pub display_name: String,\n    pub zeta_value: String,\n}\n",
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("generated struct `Fixture`"), "{error}");
    assert!(error.contains("definition `(the root)`"), "{error}");
    assert!(error.contains("no blank separator"), "{error}");
}

#[test]
fn generated_field_missing_from_schema_order_refuses_by_wire_name() {
    let src = GENERATED.replace(
        "    /// Optional docs and serde policy move together.\n    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub optional_value: Option<String>,\n\n",
        "",
    );
    let error = apply(
        &root(Some(json!(["displayName", "zeta_value", "optional_value"]))),
        &src,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("schema member census disagree"), "{error}");
}
