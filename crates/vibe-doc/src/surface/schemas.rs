//! The document surface — the format registry and the members of every
//! schema it names (PROP-057 `##OBS-SURFACE-SNAPSHOTS`, PROP-044
//! `##M-FORMAT-REGISTRY`).
//!
//! A document a foreign parser reads is a contract in the same sense a
//! command is, and the registry is where every such surface is
//! inventoried. So a snapshot carries the registry verbatim — a record's
//! epoch, its recoverability, how many parsers read it — and, for each
//! record that has a schema, the members that schema states.
//!
//! Members and not the schema text. A page quotes a field table, and a
//! field table is names, requiredness and form; a reworded description
//! inside a schema is not a change to the contract and must not report as
//! one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-SURFACE-SNAPSHOTS");

use std::path::Path;

use serde_json::{Map, Value};
use vibe_wire::generated::doc_surface::{SurfaceFormat, SurfaceSchema, SurfaceSchemaField};

use crate::derived::schema::FORMAT_REGISTRY;
use crate::error::{DocError, Result};

/// Read the registry and every schema it names.
///
/// The two halves come back together because they are read from one
/// file: a schema enters this surface because a record points at it, and
/// a schema nothing points at is not part of any contract.
pub fn read(repo_root: &Path) -> Result<(Vec<SurfaceSchema>, Vec<SurfaceFormat>)> {
    let path = repo_root.join(FORMAT_REGISTRY);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::Surface {
        message: format!("cannot read `{}`: {e}", path.display()),
    })?;
    // `toml::from_str`, not `str::parse`: the latter reads a single TOML
    // VALUE, and the registry is a document.
    let parsed: toml::Value = toml::from_str(&text).map_err(|e| DocError::Surface {
        message: format!("`{}` does not parse: {e}", path.display()),
    })?;
    let Some(records) = parsed.get("format").and_then(toml::Value::as_table) else {
        return Err(DocError::Surface {
            message: format!("`{}` carries no `[format.*]` table", path.display()),
        });
    };

    let mut formats = Vec::new();
    let mut schemas = Vec::new();
    for (id, record) in records {
        let field = |key: &str| {
            record
                .get(key)
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let schema = field("schema");
        formats.push(SurfaceFormat {
            id: id.clone(),
            epoch: record
                .get("epoch")
                .and_then(toml::Value::as_integer)
                .unwrap_or_default()
                .max(0) as u32,
            schema: schema.clone(),
            recoverable: record
                .get("recoverable")
                .and_then(toml::Value::as_bool)
                .unwrap_or_default(),
            foreign_parsers: field("foreign_parsers"),
            corpus: field("corpus"),
            sunset: field("sunset"),
        });
        // A record without a schema is inventoried all the same — that
        // is the point of the axis — but it has no members to record.
        // A schema the checkout does not hold is skipped rather than
        // fatal: a planned format's path is in the registry before the
        // file is.
        if schema.is_empty() || schema == "none" {
            continue;
        }
        let file = repo_root.join(&schema);
        if !file.is_file() {
            continue;
        }
        schemas.push(SurfaceSchema {
            id: id.clone(),
            path: schema,
            fields: members(&file)?,
        });
    }
    formats.sort_by(|a, b| a.id.cmp(&b.id));
    schemas.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((schemas, formats))
}

/// Every member of every form one schema states, sorted by form and then
/// by name so two readings of one file are the same list.
fn members(path: &Path) -> Result<Vec<SurfaceSchemaField>> {
    let text = std::fs::read_to_string(path).map_err(|e| DocError::Surface {
        message: format!("cannot read `{}`: {e}", path.display()),
    })?;
    let schema: Value = serde_json::from_str(&text).map_err(|e| DocError::Surface {
        message: format!("`{}` is not JSON: {e}", path.display()),
    })?;
    let Some(root) = schema.as_object() else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    collect(&mut out, "", root);
    if let Some(definitions) = root.get("definitions").and_then(Value::as_object) {
        for (name, definition) in definitions {
            if let Some(form) = definition.as_object() {
                collect(&mut out, name, form);
            }
        }
    }
    out.sort_by(|a, b| (&a.form, &a.name).cmp(&(&b.form, &b.name)));
    Ok(out)
}

/// One form's members: the required ones, the optional ones, and — for a
/// form that is an enum rather than a struct — its values, which are a
/// contract of exactly the same kind.
fn collect(out: &mut Vec<SurfaceSchemaField>, form: &str, object: &Map<String, Value>) {
    for (key, required) in [("properties", true), ("optionalProperties", false)] {
        let Some(members) = object.get(key).and_then(Value::as_object) else {
            continue;
        };
        for (name, definition) in members {
            out.push(SurfaceSchemaField {
                form: form.to_owned(),
                name: name.clone(),
                required,
                shape: shape(definition),
            });
        }
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        for value in values {
            let Some(name) = value.as_str() else {
                continue;
            };
            out.push(SurfaceSchemaField {
                form: form.to_owned(),
                name: name.to_owned(),
                required: true,
                shape: "enum value".into(),
            });
        }
    }
}

/// One member's form in one phrase.
fn shape(definition: &Value) -> String {
    let Some(object) = definition.as_object() else {
        return "any".into();
    };
    let nullable = object
        .get("nullable")
        .and_then(Value::as_bool)
        .unwrap_or_default();
    let base = if let Some(kind) = object.get("type").and_then(Value::as_str) {
        kind.to_owned()
    } else if let Some(target) = object.get("ref").and_then(Value::as_str) {
        format!("ref {target}")
    } else if let Some(elements) = object.get("elements") {
        format!("elements of {}", shape(elements))
    } else if let Some(values) = object.get("values") {
        format!("values of {}", shape(values))
    } else if object.contains_key("enum") {
        "enum".into()
    } else if object.contains_key("properties") || object.contains_key("optionalProperties") {
        "object".into()
    } else if object.contains_key("discriminator") {
        "discriminated union".into()
    } else {
        "any".into()
    };
    if nullable {
        format!("{base}, nullable")
    } else {
        base
    }
}

#[cfg(test)]
mod tests;
