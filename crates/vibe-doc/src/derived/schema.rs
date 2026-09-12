//! `derived kind="jtd-schema"` — a field table built from the schema a
//! document is published under (PROP-045 `##ROW-DOCVOCAB-DERIVED`).
//!
//! The reference is either a path to a `.jtd.json` file or a format id in
//! the format registry, which is where every surface a foreign parser
//! reads is inventoried. Both resolve to the same file; the id is the
//! address that survives a file being moved.
//!
//! What comes out is a table of the document's members, not a copy of the
//! schema: name, whether it is required, its type, and the description
//! the schema itself carries. A nested object form gets its own table
//! after the root's, in the order the schema defines it, so the output is
//! the same bytes on every run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-DERIVED");

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::{DocError, Result};

/// Where the project inventories its formats.
pub const FORMAT_REGISTRY: &str = "formats/REGISTRY.toml";

/// Build the field table for one schema reference.
pub fn generate(reference: &str, repo_root: &Path) -> Result<String> {
    let fail = |message: String| DocError::Derived {
        kind: "jtd-schema",
        reference: reference.to_owned(),
        message,
    };
    let path = resolve(reference, repo_root).map_err(fail)?;
    let text = std::fs::read_to_string(&path)
        .map_err(|e| fail(format!("cannot read `{}`: {e}", path.display())))?;
    let schema: Value = serde_json::from_str(&text)
        .map_err(|e| fail(format!("`{}` is not JSON: {e}", path.display())))?;
    let Some(root) = schema.as_object() else {
        return Err(fail("the schema is not a JSON object".into()));
    };
    let mut out = String::new();
    render_form(&mut out, "The document", root);
    if let Some(definitions) = root.get("definitions").and_then(Value::as_object) {
        for (name, definition) in definitions {
            let Some(form) = definition.as_object() else {
                continue;
            };
            out.push('\n');
            render_form(&mut out, &format!("`{name}`"), form);
        }
    }
    Ok(out.trim_end().to_owned())
}

/// A reference is a path when it names a file, and a format id otherwise.
fn resolve(reference: &str, repo_root: &Path) -> std::result::Result<PathBuf, String> {
    if reference.ends_with(".jtd.json") {
        let path = repo_root.join(reference);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(format!("`{}` does not exist", path.display()))
        };
    }
    let registry_path = repo_root.join(FORMAT_REGISTRY);
    let registry = std::fs::read_to_string(&registry_path)
        .map_err(|e| format!("cannot read `{}`: {e}", registry_path.display()))?;
    // `toml::from_str`, not `str::parse`: the latter reads a single TOML
    // VALUE, and the registry is a document.
    let parsed: toml::Value = toml::from_str(&registry)
        .map_err(|e| format!("`{}` does not parse: {e}", registry_path.display()))?;
    let schema = parsed
        .get("format")
        .and_then(|f| f.get(reference))
        .and_then(|r| r.get("schema"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            format!("no format `{reference}` with a schema in {FORMAT_REGISTRY} — a format that is not inventoried has no address")
        })?;
    if schema == "none" {
        return Err(format!(
            "format `{reference}` is inventoried with no schema, so it has no field table"
        ));
    }
    Ok(repo_root.join(schema))
}

/// One object form as a table, headed by what it is.
fn render_form(out: &mut String, title: &str, form: &Map<String, Value>) {
    let required = form.get("properties").and_then(Value::as_object);
    let optional = form.get("optionalProperties").and_then(Value::as_object);
    if required.is_none() && optional.is_none() {
        return;
    }
    out.push_str(&format!("{title}\n\n"));
    out.push_str("| Field | Required | Type | Meaning |\n");
    out.push_str("|---|---|---|---|\n");
    for (members, mark) in [(required, "yes"), (optional, "no")] {
        let Some(members) = members else { continue };
        for (name, sub) in members {
            out.push_str(&format!(
                "| `{name}` | {mark} | {} | {} |\n",
                type_of(sub),
                description_of(sub)
            ));
        }
    }
}

/// The JTD form of one member, in the words a reader of the manual has.
fn type_of(schema: &Value) -> String {
    let Some(map) = schema.as_object() else {
        return "any".into();
    };
    let nullable = map.get("nullable").and_then(Value::as_bool) == Some(true);
    let base = if let Some(name) = map.get("ref").and_then(Value::as_str) {
        format!("`{name}`")
    } else if let Some(kind) = map.get("type").and_then(Value::as_str) {
        kind.to_owned()
    } else if let Some(values) = map.get("enum").and_then(Value::as_array) {
        let names: Vec<String> = values
            .iter()
            .filter_map(Value::as_str)
            .map(|v| format!("`{v}`"))
            .collect();
        format!("one of {}", names.join(", "))
    } else if let Some(elements) = map.get("elements") {
        format!("list of {}", type_of(elements))
    } else if let Some(values) = map.get("values") {
        format!("map of {}", type_of(values))
    } else if map.contains_key("discriminator") {
        "tagged union".into()
    } else if map.contains_key("properties") || map.contains_key("optionalProperties") {
        "object".into()
    } else {
        "any".into()
    };
    if nullable {
        format!("{base} or null")
    } else {
        base
    }
}

/// The schema's own words about a member, with the table's cell
/// separator neutralised — a description is prose and may carry one.
fn description_of(schema: &Value) -> String {
    schema
        .get("metadata")
        .and_then(|m| m.get("description"))
        .and_then(Value::as_str)
        .map(|d| d.replace('\n', " ").replace('|', "\\|"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
