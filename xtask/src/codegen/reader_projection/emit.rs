//! Emission of the consumer-local serde adapter from a resolved JTD closure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

use super::{ProjectionUse, field_name, rust_field, struct_name};
use crate::codegen::shared_module::emitted_name;

const DUPLICATE_SAFE_VALUE: &str = r#"    #[derive(Clone, Copy)]
    struct NoDuplicateValue;

    impl<'de> serde::de::DeserializeSeed<'de> for NoDuplicateValue {
        type Value = serde_json::Value;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(NoDuplicateValueVisitor)
        }
    }

    struct NoDuplicateValueVisitor;

    impl<'de> serde::de::Visitor<'de> for NoDuplicateValueVisitor {
        type Value = serde_json::Value;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON value without duplicate object members")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Bool(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Number(value.into()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Number(value.into()))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            serde_json::Number::from_f64(value)
                .map(serde_json::Value::Number)
                .ok_or_else(|| E::custom("non-finite JSON number"))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
            Ok(serde_json::Value::String(value.to_string()))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
            Ok(serde_json::Value::String(value))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            serde::de::DeserializeSeed::deserialize(NoDuplicateValue, deserializer)
        }

        fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            serde::de::DeserializeSeed::deserialize(NoDuplicateValue, deserializer)
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut items = Vec::new();
            while let Some(item) = sequence.next_element_seed(NoDuplicateValue)? {
                items.push(item);
            }
            Ok(serde_json::Value::Array(items))
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut object = serde_json::Map::new();
            while let Some(key) = map.next_key::<String>()? {
                if object.contains_key(&key) {
                    return Err(<A::Error as serde::de::Error>::custom(
                        "duplicate object member",
                    ));
                }
                let value = map.next_value_seed(NoDuplicateValue)?;
                object.insert(key, value);
            }
            Ok(serde_json::Value::Object(object))
        }
    }

"#;

pub(crate) fn rewrite_consumer(
    file: &Path,
    resolved_doc: &Path,
    schema: &Path,
    projections: &[ProjectionUse],
) -> Result<()> {
    if projections.is_empty() {
        return Ok(());
    }
    let source = std::fs::read_to_string(file)
        .with_context(|| format!("reading generated projection consumer {}", file.display()))?;
    let document: Value = serde_json::from_str(
        &std::fs::read_to_string(resolved_doc)
            .with_context(|| format!("reading resolved schema {}", resolved_doc.display()))?,
    )
    .with_context(|| format!("parsing resolved schema {}", resolved_doc.display()))?;
    let definitions = document
        .get("definitions")
        .and_then(Value::as_object)
        .with_context(|| {
            format!(
                "schema {} declares projection but its resolved document has no definitions",
                schema.display()
            )
        })?;

    let mut functions = Vec::with_capacity(projections.len());
    for (index, projection) in projections.iter().enumerate() {
        functions.push(format!(
            "deserialize_{}_{}",
            rust_field(&projection.target),
            index
        ));
    }
    let injected = inject_attributes(&source, file, projections, &functions)?;
    let adapter = emit_adapter(definitions, projections, &functions, schema)?;
    let mut output = injected;
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push('\n');
    output.push_str(&adapter);
    super::super::write::write_generated(file, &output)
}

fn inject_attributes(
    source: &str,
    file: &Path,
    projections: &[ProjectionUse],
    functions: &[String],
) -> Result<String> {
    let mut wanted: BTreeMap<(&str, &str), (usize, &str)> = BTreeMap::new();
    for (index, projection) in projections.iter().enumerate() {
        if wanted
            .insert(
                (&projection.owner_type, &projection.rust_field),
                (index, functions[index].as_str()),
            )
            .is_some()
        {
            bail!(
                "{}: two projection markers resolve to `{}::{}`; a marker must be consumed exactly once.",
                file.display(),
                projection.owner_type,
                projection.rust_field
            );
        }
    }
    let mut consumed = vec![0usize; projections.len()];
    let mut current_struct: Option<String> = None;
    let mut output = String::with_capacity(source.len() + projections.len() * 96);
    for chunk in source.split_inclusive('\n') {
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();
        if let Some(name) = struct_name(text) {
            current_struct = Some(name.to_string());
        } else if body == "}" {
            current_struct = None;
        }
        if let Some(owner) = current_struct.as_deref()
            && let Some(field) = field_name(text)
            && let Some((index, function)) = wanted.get(&(owner, field))
        {
            let indent = &body[..body.len() - body.trim_start().len()];
            output.push_str(indent);
            output.push_str("#[serde(deserialize_with = \"__reader_projection::");
            output.push_str(function);
            output.push_str("\")]\n");
            consumed[*index] += 1;
        }
        output.push_str(chunk);
    }
    for (index, count) in consumed.iter().enumerate() {
        if *count != 1 {
            let projection = &projections[index];
            bail!(
                "{}: projection marker at {} resolved to `{}::{}` but the generated field was consumed {count} times (expected exactly once). The pinned generator shape moved or the marker is ambiguous.",
                file.display(),
                projection.location,
                projection.owner_type,
                projection.rust_field
            );
        }
    }
    Ok(output)
}

fn emit_adapter(
    definitions: &Map<String, Value>,
    projections: &[ProjectionUse],
    functions: &[String],
    schema: &Path,
) -> Result<String> {
    let mut closure = BTreeSet::new();
    for projection in projections {
        closure.extend(projection.closure.iter().cloned());
    }
    let mut output = String::from("#[doc(hidden)]\nmod __reader_projection {\n");
    output.push_str(DUPLICATE_SAFE_VALUE);
    for (index, projection) in projections.iter().enumerate() {
        let rust_type = emitted_name(&projection.target);
        output.push_str(&format!(
            "    pub(super) fn {}<'de, D>(deserializer: D) -> Result<crate::generated::shared::{rust_type}, D::Error>\n    where\n        D: serde::Deserializer<'de>,\n    {{\n        let mut value = serde::de::DeserializeSeed::deserialize(NoDuplicateValue, deserializer)?;\n        prune_{}(&mut value);\n        serde_json::from_value(value).map_err(<D::Error as serde::de::Error>::custom)\n    }}\n\n",
            functions[index],
            rust_field(&projection.target),
        ));
    }
    for name in closure {
        let form = definitions.get(&name).with_context(|| {
            format!(
                "schema {}: projected closure names `{name}` but the resolved definitions omit it",
                schema.display()
            )
        })?;
        let function = format!("prune_{}", rust_field(&name));
        let parameter = if form_needs_pruning(form) {
            "value"
        } else {
            "_value"
        };
        output.push_str(&format!(
            "    fn {function}({parameter}: &mut serde_json::Value) {{\n"
        ));
        emit_pruner(form, parameter, 2, &mut output)?;
        output.push_str("    }\n\n");
    }
    output.push_str("}\n");
    Ok(output)
}

fn emit_pruner(form: &Value, variable: &str, indent: usize, output: &mut String) -> Result<()> {
    let Some(object) = form.as_object() else {
        return Ok(());
    };
    if let Some(reference) = object.get("ref").and_then(Value::as_str) {
        line(
            output,
            indent,
            &format!("prune_{}({variable});", rust_field(reference)),
        );
        return Ok(());
    }
    if let Some(mapping) = object.get("mapping").and_then(Value::as_object) {
        let discriminator = object
            .get("discriminator")
            .and_then(Value::as_str)
            .context("a JTD mapping used by projection has no string discriminator")?;
        line(
            output,
            indent,
            &format!("let Some(object) = {variable}.as_object_mut() else {{ return; }};"),
        );
        line(
            output,
            indent,
            &format!(
                "let Some(tag) = object.get({discriminator:?}).and_then(serde_json::Value::as_str).map(str::to_owned) else {{ return; }};"
            ),
        );
        line(output, indent, "match tag.as_str() {");
        for (tag, arm) in mapping {
            line(output, indent + 1, &format!("{tag:?} => {{"));
            emit_object(arm, Some(discriminator), indent + 2, output)?;
            line(output, indent + 1, "}");
        }
        line(output, indent + 1, "_ => {}");
        line(output, indent, "}");
        return Ok(());
    }
    if object.contains_key("properties") || object.contains_key("optionalProperties") {
        line(
            output,
            indent,
            &format!("let Some(object) = {variable}.as_object_mut() else {{ return; }};"),
        );
        emit_object(form, None, indent, output)?;
        return Ok(());
    }
    if let Some(elements) = object.get("elements") {
        line(
            output,
            indent,
            &format!("if let Some(items) = {variable}.as_array_mut() {{"),
        );
        line(output, indent + 1, "for item in items {");
        emit_pruner(elements, "item", indent + 2, output)?;
        line(output, indent + 1, "}");
        line(output, indent, "}");
        return Ok(());
    }
    if let Some(values) = object.get("values") {
        line(
            output,
            indent,
            &format!("if let Some(items) = {variable}.as_object_mut() {{"),
        );
        line(output, indent + 1, "for item in items.values_mut() {");
        emit_pruner(values, "item", indent + 2, output)?;
        line(output, indent + 1, "}");
        line(output, indent, "}");
    }
    Ok(())
}

fn emit_object(
    form: &Value,
    discriminator: Option<&str>,
    indent: usize,
    output: &mut String,
) -> Result<()> {
    let object = form.as_object().context("object form is not an object")?;
    let mut members: BTreeMap<&str, &Value> = BTreeMap::new();
    for block in ["properties", "optionalProperties"] {
        if let Some(properties) = object.get(block).and_then(Value::as_object) {
            members.extend(
                properties
                    .iter()
                    .map(|(name, child)| (name.as_str(), child)),
            );
        }
    }
    let mut names: Vec<&str> = members.keys().copied().collect();
    if let Some(discriminator) = discriminator {
        names.push(discriminator);
    }
    names.sort_unstable();
    names.dedup();
    if names.is_empty() {
        line(output, indent, "object.clear();");
    } else {
        let patterns = names
            .iter()
            .map(|name| format!("{name:?}"))
            .collect::<Vec<_>>()
            .join(" | ");
        line(
            output,
            indent,
            &format!("object.retain(|key, _| matches!(key.as_str(), {patterns}));"),
        );
    }
    for (name, child) in members {
        if form_needs_pruning(child) {
            line(
                output,
                indent,
                &format!("if let Some(child) = object.get_mut({name:?}) {{"),
            );
            emit_pruner(child, "child", indent + 1, output)?;
            line(output, indent, "}");
        }
    }
    Ok(())
}

fn form_needs_pruning(form: &Value) -> bool {
    form.as_object().is_some_and(|object| {
        object.contains_key("ref")
            || object.contains_key("mapping")
            || object.contains_key("properties")
            || object.contains_key("optionalProperties")
            || object.contains_key("elements")
            || object.contains_key("values")
    })
}

fn line(output: &mut String, indent: usize, text: &str) {
    output.push_str(&"    ".repeat(indent));
    output.push_str(text);
    output.push('\n');
}
