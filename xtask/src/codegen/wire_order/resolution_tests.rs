//! Resolver-backed evidence for wire-order metadata. These tests use the same
//! vocabulary substitution that production codegen uses, then pass its
//! resolved document to the ordering pass while retaining the authored schema
//! as the human-facing diagnostic authority.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::{Value, json};

use super::apply_wire_order;
use crate::codegen::vocabulary::Vocabularies;

fn write_json(path: &Path, value: &Value) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn resolver(
    home: Value,
    schema: Value,
    name: &str,
) -> Result<(tempfile::TempDir, Vocabularies, PathBuf)> {
    let temp = tempfile::tempdir()?;
    let home_path = temp.path().join("vocabularies.json");
    let schema_path = temp.path().join(name);
    write_json(&home_path, &home)?;
    write_json(&schema_path, &schema)?;
    Ok((temp, Vocabularies::load(&home_path)?, schema_path))
}

#[test]
fn resolved_consumer_keeps_local_root_and_definition_orders_beside_shared_refs() -> Result<()> {
    let schema_doc = json!({
        "metadata": {
            "x-vocabularies": ["shared_text"],
            "x-wire-order": ["second", "first"]
        },
        "definitions": {
            "local_row": {
                "metadata": {"x-wire-order": ["beta", "alpha"]},
                "properties": {
                    "alpha": {"ref": "shared_text"},
                    "beta": {"type": "string"}
                }
            }
        },
        "properties": {
            "first": {"ref": "local_row"},
            "second": {"ref": "shared_text"}
        }
    });
    let (_temp, mut vocabularies, schema) = resolver(
        json!({"shared_text": {"type": "string"}}),
        schema_doc,
        "consumer.jtd.json",
    )?;
    let resolved = vocabularies.resolve(&schema)?;
    assert_ne!(resolved.doc, schema, "the real resolver must issue a copy");
    let src = r#"pub struct Consumer {
    pub first: LocalRow,

    pub second: SharedText,
}

pub struct LocalRow {
    pub alpha: SharedText,

    pub beta: String,
}

pub type SharedText = String;
"#;
    let out = apply_wire_order(src, "generated/consumer/mod.rs", &resolved.doc, &schema)?;
    assert!(out.contains(
        "pub struct Consumer {\n    pub second: SharedText,\n\n    pub first: LocalRow,\n}"
    ));
    assert!(
        out.contains(
            "pub struct LocalRow {\n    pub beta: String,\n\n    pub alpha: SharedText,\n}"
        )
    );
    Ok(())
}

#[test]
fn resolved_vocabulary_home_keeps_order_on_its_owned_definition() -> Result<()> {
    let (temp, mut vocabularies, _schema) = resolver(
        json!({
            "ordered_row": {
                "metadata": {"x-wire-order": ["second", "first"]},
                "properties": {
                    "first": {"type": "string"},
                    "second": {"type": "string"}
                }
            }
        }),
        json!({"properties": {}}),
        "consumer.jtd.json",
    )?;
    let resolved = vocabularies.shared_schema()?;
    let home = temp.path().join("vocabularies.json");
    let src = r#"pub type Shared = Option<Value>;

pub struct OrderedRow {
    pub first: String,

    pub second: String,
}
"#;
    let out = apply_wire_order(src, "generated/shared/mod.rs", &resolved, &home)?;
    assert!(
        out.contains(
            "pub struct OrderedRow {\n    pub second: String,\n\n    pub first: String,\n}"
        )
    );
    Ok(())
}

#[test]
fn resolved_validation_reads_the_copy_but_names_only_the_authored_schema() -> Result<()> {
    let (_temp, mut vocabularies, schema) = resolver(
        json!({"shared_text": {"type": "string"}}),
        json!({
            "metadata": {
                "x-vocabularies": ["shared_text"],
                "x-wire-order": ["ghost"]
            },
            "properties": {"real": {"ref": "shared_text"}}
        }),
        "diagnostic.jtd.json",
    )?;
    let resolved = vocabularies.resolve(&schema)?;
    let error = apply_wire_order("", "generated/diagnostic/mod.rs", &resolved.doc, &schema)
        .expect_err("the resolved ruling is invalid")
        .to_string();
    assert!(error.contains(&schema.display().to_string()), "{error}");
    assert!(
        !error.contains(&resolved.doc.parent().unwrap().display().to_string()),
        "scratch paths are implementation detail: {error}"
    );
    assert!(error.contains("unknown member `ghost`"), "{error}");
    Ok(())
}
