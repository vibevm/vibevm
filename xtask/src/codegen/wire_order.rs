//! Schema-declared serialization order for generated JTD object structs.
//!
//! JSON object-map iteration order is not a wire contract. The only opt-in
//! order carrier is `metadata."x-wire-order"` on a root or definition object:
//! a complete permutation of that object's required and optional wire member
//! names. This pass runs after field identifiers become snake_case and before
//! any later shape edit, then moves whole generated field chunks without
//! changing a byte inside them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::domain_types::rulings::{pascal_case, root_stem};

struct WireOrder {
    definition: String,
    emitted: String,
    members: Vec<String>,
}

struct FieldChunk {
    wire: String,
    bytes: String,
}

/// Apply every explicit wire-order ruling in `resolved` to the generated Rust
/// source. With no ruling this is the exact identity, before even inspecting
/// the generated text.
pub(super) fn apply_wire_order(
    src: &str,
    file: &str,
    resolved: &Path,
    schema: &Path,
) -> Result<String> {
    let orders = wire_orders(resolved, schema)?;
    if orders.is_empty() {
        return Ok(src.to_owned());
    }
    reorder_structs(src, file, schema, orders)
}

fn wire_orders(resolved: &Path, schema: &Path) -> Result<Vec<WireOrder>> {
    let text = std::fs::read_to_string(resolved)
        .with_context(|| format!("reading the resolved schema {}", resolved.display()))?;
    let doc: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", resolved.display()))?;
    let stem = root_stem(resolved)?;
    orders_from_doc(&doc, &stem, schema)
}

fn orders_from_doc(doc: &Value, root_stem: &str, schema: &Path) -> Result<Vec<WireOrder>> {
    reject_unsupported_locations(doc, "(the root)", true, schema)?;
    let mut orders = Vec::new();
    let root_name = pascal_case(root_stem);
    if let Some(order) = order_for(doc, "(the root)", &root_name, schema)? {
        orders.push(order);
    }
    let mut candidates = BTreeMap::<String, Vec<String>>::new();
    candidates
        .entry(root_name)
        .or_default()
        .push("(the root)".to_owned());
    if let Some(definitions) = doc.get("definitions") {
        let Some(definitions) = definitions.as_object() else {
            bail!(
                "schema {}: `definitions` is not an object, so x-wire-order cannot resolve its generated structs",
                schema.display()
            );
        };
        for (name, form) in definitions {
            let emitted = pascal_case(name);
            candidates
                .entry(emitted.clone())
                .or_default()
                .push(name.clone());
            if let Some(order) = order_for(form, name, &emitted, schema)? {
                orders.push(order);
            }
        }
    }
    for order in &orders {
        let sites = &candidates[&order.emitted];
        if sites.len() != 1 {
            bail!(
                "schema {}: x-wire-order for `{}` cannot resolve generated struct `{}` uniquely because these root/definition names collide under the pinned Pascal-case rule: {}; rename the colliding schema names",
                schema.display(),
                order.definition,
                order.emitted,
                sites.join(", ")
            );
        }
    }
    Ok(orders)
}

fn reject_unsupported_locations(
    value: &Value,
    path: &str,
    allowed: bool,
    schema: &Path,
) -> Result<()> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    if object
        .get("metadata")
        .and_then(Value::as_object)
        .is_some_and(|metadata| metadata.contains_key("x-wire-order"))
        && !allowed
    {
        bail!(
            "schema {}: `{path}` carries metadata.\"x-wire-order\", but only the schema root or a named direct definition has a deterministic generated struct name; anonymous and mapping objects are unsupported",
            schema.display()
        );
    }
    for (key, child) in object {
        if key == "metadata" {
            continue;
        }
        if path == "(the root)" && key == "definitions" {
            if let Some(definitions) = child.as_object() {
                for (name, definition) in definitions {
                    reject_unsupported_locations(definition, name, true, schema)?;
                }
            }
            continue;
        }
        reject_unsupported_locations(child, &format!("{path}.{key}"), false, schema)?;
    }
    Ok(())
}

fn order_for(
    form: &Value,
    definition: &str,
    emitted: &str,
    schema: &Path,
) -> Result<Option<WireOrder>> {
    let Some(metadata) = form.get("metadata") else {
        return Ok(None);
    };
    let Some(metadata) = metadata.as_object() else {
        return Ok(None);
    };
    let Some(value) = metadata.get("x-wire-order") else {
        return Ok(None);
    };
    let object_form = form.get("properties").is_some() || form.get("optionalProperties").is_some();
    if !object_form {
        bail!(
            "schema {}: `{definition}` carries metadata.\"x-wire-order\" but is not an object form; only a root/definition with properties or optionalProperties emits an orderable struct",
            schema.display()
        );
    }
    let expected = object_members(form, definition, schema)?;
    let Some(values) = value.as_array() else {
        bail!(
            "schema {}: `{definition}` metadata.\"x-wire-order\" must be an array containing every object member exactly once, found {value}",
            schema.display()
        );
    };
    let mut members = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let Some(member) = value.as_str() else {
            bail!(
                "schema {}: `{definition}` metadata.\"x-wire-order\" member {} is not a string: {value}",
                schema.display(),
                index + 1
            );
        };
        if !expected.contains(member) {
            bail!(
                "schema {}: `{definition}` metadata.\"x-wire-order\" names unknown member `{member}`; declared members are {}",
                schema.display(),
                listed(&expected)
            );
        }
        if !seen.insert(member.to_owned()) {
            bail!(
                "schema {}: `{definition}` metadata.\"x-wire-order\" repeats member `{member}`; every member must occur exactly once",
                schema.display()
            );
        }
        members.push(member.to_owned());
    }
    let missing = expected.difference(&seen).cloned().collect::<BTreeSet<_>>();
    if !missing.is_empty() {
        bail!(
            "schema {}: `{definition}` metadata.\"x-wire-order\" omits member(s) {}; every required and optional member must occur exactly once",
            schema.display(),
            listed(&missing)
        );
    }
    Ok(Some(WireOrder {
        definition: definition.to_owned(),
        emitted: emitted.to_owned(),
        members,
    }))
}

fn object_members(form: &Value, definition: &str, schema: &Path) -> Result<BTreeSet<String>> {
    let mut members = BTreeSet::new();
    for key in ["properties", "optionalProperties"] {
        let Some(block) = form.get(key) else {
            continue;
        };
        let Some(block) = block.as_object() else {
            bail!(
                "schema {}: `{definition}` `{key}` is not an object, so its x-wire-order member census cannot be read",
                schema.display()
            );
        };
        for member in block.keys() {
            if !members.insert(member.clone()) {
                bail!(
                    "schema {}: `{definition}` declares member `{member}` in both properties and optionalProperties",
                    schema.display()
                );
            }
        }
    }
    Ok(members)
}

fn reorder_structs(src: &str, file: &str, schema: &Path, orders: Vec<WireOrder>) -> Result<String> {
    let by_name = orders
        .into_iter()
        .map(|order| (order.emitted.clone(), order))
        .collect::<BTreeMap<_, _>>();
    let mut consumed = BTreeSet::new();
    let lines = src.split_inclusive('\n').collect::<Vec<_>>();
    let mut out = String::with_capacity(src.len());
    let mut index = 0;
    while index < lines.len() {
        let text = trimmed(lines[index]);
        if let Some(name) = empty_struct_name(text)
            && let Some(order) = by_name.get(name)
        {
            if !order.members.is_empty() {
                bail!(
                    "{}:{}: generated one-line empty struct `{name}` cannot carry the {} members ordered by schema {} definition `{}`",
                    file,
                    index + 1,
                    order.members.len(),
                    schema.display(),
                    order.definition
                );
            }
            if !consumed.insert(name.to_owned()) {
                bail!(
                    "{}:{}: generated struct `{name}` appears more than once for schema {} definition `{}`",
                    file,
                    index + 1,
                    schema.display(),
                    order.definition
                );
            }
            out.push_str(lines[index]);
            index += 1;
            continue;
        }
        let Some(name) = struct_name(text) else {
            out.push_str(lines[index]);
            index += 1;
            continue;
        };
        let Some(order) = by_name.get(name) else {
            out.push_str(lines[index]);
            index += 1;
            continue;
        };
        if !consumed.insert(name.to_owned()) {
            bail!(
                "{}:{}: generated struct `{name}` appears more than once for schema {} definition `{}`",
                file,
                index + 1,
                schema.display(),
                order.definition
            );
        }
        out.push_str(lines[index]);
        let (fields, separator, closing) =
            parse_fields(&lines, index + 1, file, schema, &order.definition, name)?;
        let mut by_wire = fields
            .into_iter()
            .map(|field| (field.wire.clone(), field))
            .collect::<BTreeMap<_, _>>();
        if by_wire.len() != order.members.len() {
            bail!(
                "{}:{}: generated struct `{name}` has {} distinct fields but schema {} definition `{}` orders {}; the generator shape and schema member census disagree",
                file,
                index + 1,
                by_wire.len(),
                schema.display(),
                order.definition,
                order.members.len()
            );
        }
        for (position, wire) in order.members.iter().enumerate() {
            let field = by_wire.remove(wire).ok_or_else(|| {
                anyhow::anyhow!(
                    "{}:{}: generated struct `{name}` has no field carrying schema {} definition `{}` member `{wire}`",
                    file,
                    index + 1,
                    schema.display(),
                    order.definition
                )
            })?;
            out.push_str(&field.bytes);
            if position + 1 < order.members.len() {
                out.push_str(separator);
            }
        }
        out.push_str(lines[closing]);
        index = closing + 1;
    }
    for (name, order) in &by_name {
        if !consumed.contains(name) {
            bail!(
                "{file}: schema {} definition `{}` carries x-wire-order for generated struct `{name}`, but that struct is absent or has an unfamiliar declaration shape",
                schema.display(),
                order.definition
            );
        }
    }
    Ok(out)
}

fn parse_fields<'a>(
    lines: &[&'a str],
    mut index: usize,
    file: &str,
    schema: &Path,
    definition: &str,
    name: &str,
) -> Result<(Vec<FieldChunk>, &'a str, usize)> {
    let mut fields = Vec::new();
    let mut separators = Vec::new();
    loop {
        if index >= lines.len() {
            bail!(
                "{file}: generated struct `{name}` for schema {} definition `{definition}` never closes",
                schema.display()
            );
        }
        if trimmed(lines[index]) == "}" {
            break;
        }
        let start = index;
        while index < lines.len() {
            let text = trimmed(lines[index]);
            if text.starts_with("///") || (text.starts_with("#[") && text.ends_with(']')) {
                index += 1;
            } else {
                break;
            }
        }
        if index >= lines.len() {
            bail!(
                "{file}: generated struct `{name}` for schema {} definition `{definition}` ends inside a field chunk",
                schema.display()
            );
        }
        let field_line = index;
        let rust = field_name(trimmed(lines[field_line])).ok_or_else(|| {
            anyhow::anyhow!(
                "{}:{}: generated struct `{name}` for schema {} definition `{definition}` contains an unfamiliar field line `{}`",
                file,
                field_line + 1,
                schema.display(),
                trimmed(lines[field_line])
            )
        })?;
        index += 1;
        let bytes = lines[start..index].concat();
        let renames = lines[start..index]
            .iter()
            .filter_map(|line| rename_wire(trimmed(line)))
            .collect::<Vec<_>>();
        let wire = match renames.as_slice() {
            [] => rust.to_owned(),
            [wire] => (*wire).to_owned(),
            _ => bail!(
                "{}:{}: generated field `{rust}` in struct `{name}` carries {} serde rename attributes; schema {} definition `{definition}` requires one wire identity",
                file,
                field_line + 1,
                renames.len(),
                schema.display()
            ),
        };
        if fields.iter().any(|field: &FieldChunk| field.wire == wire) {
            bail!(
                "{}:{}: generated struct `{name}` carries wire member `{wire}` more than once for schema {} definition `{definition}`",
                file,
                field_line + 1,
                schema.display()
            );
        }
        fields.push(FieldChunk { wire, bytes });
        if index < lines.len() && trimmed(lines[index]).is_empty() {
            separators.push(lines[index]);
            index += 1;
            if index >= lines.len() || trimmed(lines[index]) == "}" {
                bail!(
                    "{}:{}: generated struct `{name}` has a trailing blank inside its field list; schema {} definition `{definition}` expects the pinned field-chunk shape",
                    file,
                    index,
                    schema.display()
                );
            }
        } else if index < lines.len() && trimmed(lines[index]) != "}" {
            bail!(
                "{}:{}: generated struct `{name}` has no blank separator between field chunks for schema {} definition `{definition}`",
                file,
                index + 1,
                schema.display()
            );
        }
    }
    if fields.len() > 1 && separators.len() != fields.len() - 1 {
        bail!(
            "{file}: generated struct `{name}` has an unfamiliar separator count for schema {} definition `{definition}`",
            schema.display()
        );
    }
    let separator = separators.first().copied().unwrap_or("");
    if separators.iter().any(|candidate| *candidate != separator) {
        bail!(
            "{file}: generated struct `{name}` mixes field separators for schema {} definition `{definition}`; refusing to normalize generator bytes",
            schema.display()
        );
    }
    Ok((fields, separator, index))
}

fn struct_name(text: &str) -> Option<&str> {
    text.strip_prefix("pub struct ")?.strip_suffix(" {")
}

fn empty_struct_name(text: &str) -> Option<&str> {
    text.strip_prefix("pub struct ")?.strip_suffix(" {}")
}

fn field_name(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("pub ")?;
    let (name, _) = rest.split_once(':')?;
    (text.ends_with(',')
        && !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    .then_some(name)
}

fn rename_wire(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("#[serde(rename = \"")?;
    rest.strip_suffix("\")]")
}

fn trimmed(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n']).trim()
}

fn listed(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
#[path = "wire_order/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "wire_order/resolution_tests.rs"]
mod resolution_tests;

#[cfg(test)]
#[path = "wire_order/pipeline_tests.rs"]
mod pipeline_tests;
