//! Explicit omission/null equivalence at a named optional member, never a global rule.
use super::Required;
use anyhow::{Result, bail};
use serde_json::{Map, Value};
use std::path::Path;
const MARKER: &str = "x-null-alias";

pub(super) fn declared(node: &Map<String, Value>) -> bool {
    node.get("metadata")
        .and_then(Value::as_object)
        .is_some_and(|m| m.contains_key(MARKER))
}
pub(super) fn validate(doc: &Value, schema: &Path) -> Result<()> {
    walk(doc, "root", None, schema)
}
fn walk(value: &Value, path: &str, member: Option<Required>, schema: &Path) -> Result<()> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    if let Some(alias) = object
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|m| m.get(MARKER))
    {
        let metadata = &object["metadata"];
        if alias.as_str() != Some("absent")
            || member != Some(Required::Optional)
            || object.get("nullable") != Some(&Value::Bool(true))
            || metadata.get("x-default") != Some(&Value::Null)
            || metadata
                .get("x-empty")
                .is_some_and(|v| v.as_str() != Some("preserve"))
        {
            bail!(
                "schema {}: `{path}` metadata.{MARKER} must be the string `absent` on an optional nullable member with x-default:null and no conflicting empty policy; fix that exact member before codegen",
                schema.display()
            );
        }
    }
    for (key, required) in [
        ("properties", Required::Required),
        ("optionalProperties", Required::Optional),
    ] {
        if let Some(fields) = object.get(key).and_then(Value::as_object) {
            for (name, field) in fields {
                walk(
                    field,
                    &format!("{path}.{key}.{name}"),
                    Some(required),
                    schema,
                )?;
            }
        }
    }
    for key in ["definitions", "mapping"] {
        if let Some(named) = object.get(key).and_then(Value::as_object) {
            for (name, child) in named {
                walk(child, &format!("{path}.{key}.{name}"), None, schema)?;
            }
        }
    }
    for key in ["elements", "values"] {
        if let Some(child) = object.get(key) {
            walk(child, &format!("{path}.{key}"), None, schema)?;
        }
    }
    Ok(())
}
