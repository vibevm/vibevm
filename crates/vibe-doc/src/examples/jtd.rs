//! A JSON Type Definition validator, brought along on purpose
//! (PROP-057 `##PIPE-EXAMPLE-RUNNER`).
//!
//! `vibe-wire` has none, and that is not an oversight to route around:
//! the schemas under `schemas/` are CODEGEN INPUT, and the runtime
//! contract is serde deserialisation into the generated type — which is
//! strictly weaker than the schema, because no generated type denies
//! unknown fields. So a `--json` document can satisfy every consumer in
//! the tree and still not satisfy the schema that document is published
//! under. Checking a documented `--json` example against its schema is the
//! only place that difference becomes visible.
//!
//! RFC 8927 has eight forms and they all fit here. A document whose
//! `command` the fixture does not map to a schema is reported as
//! UNCHECKED — never as passed, which is how an unchecked document
//! quietly becomes a checked one in a reader's mind.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER");

use serde_json::{Map, Value};

/// One schema violation: where it is, and what is wrong there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// A JSON pointer into the instance.
    pub at: String,
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.at, self.message)
    }
}

/// Validate `instance` against a JTD `schema`.
///
/// ```
/// let schema: serde_json::Value = serde_json::json!({
///     "properties": { "ok": { "type": "boolean" } },
///     "optionalProperties": { "count": { "type": "uint32" } }
/// });
/// let good = serde_json::json!({ "ok": true, "count": 1 });
/// assert!(vibe_doc::examples::jtd::validate(&schema, &good).is_empty());
///
/// let bad = serde_json::json!({ "ok": true, "extra": 1 });
/// let found = vibe_doc::examples::jtd::validate(&schema, &bad);
/// assert_eq!(found.len(), 1);
/// assert!(found[0].message.contains("unexpected member `extra`"));
/// ```
pub fn validate(schema: &Value, instance: &Value) -> Vec<Violation> {
    let root = schema.clone();
    let mut found = Vec::new();
    check(&root, schema, instance, "", &mut found);
    found
}

/// Fold the project's SHARED definitions into one schema's own.
///
/// A schema under `schemas/` refers to a name it does not define when the
/// name is a shared vocabulary — `package_kind` and its neighbours live in
/// one file so that every format spells the same register the same way
/// (the schema-vocabulary home `specmap.toml` names). Without this the
/// validator would report «undefined definition» for a document that is
/// perfectly valid, which is worse than not checking it at all.
///
/// A schema's own definition WINS: the shared file is a fallback, never an
/// override.
///
/// ```
/// let schema = serde_json::json!({ "properties": { "k": { "ref": "kind" } } });
/// let shared = serde_json::json!({ "kind": { "enum": ["flow", "doc"] } });
/// let merged = vibe_doc::examples::jtd::with_vocabulary(schema, &shared);
/// assert!(vibe_doc::examples::jtd::validate(&merged, &serde_json::json!({ "k": "doc" })).is_empty());
/// assert_eq!(vibe_doc::examples::jtd::validate(&merged, &serde_json::json!({ "k": "nope" })).len(), 1);
/// ```
pub fn with_vocabulary(mut schema: Value, vocabulary: &Value) -> Value {
    let Some(shared) = vocabulary.as_object() else {
        return schema;
    };
    let Some(root) = schema.as_object_mut() else {
        return schema;
    };
    let own = root
        .entry("definitions")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(own) = own.as_object_mut() else {
        return schema;
    };
    for (name, definition) in shared {
        own.entry(name.clone())
            .or_insert_with(|| definition.clone());
    }
    schema
}

fn check(root: &Value, schema: &Value, value: &Value, at: &str, out: &mut Vec<Violation>) {
    let Some(map) = schema.as_object() else {
        return;
    };
    if map.get("nullable").and_then(Value::as_bool) == Some(true) && value.is_null() {
        return;
    }
    if let Some(name) = map.get("ref").and_then(Value::as_str) {
        match root
            .get("definitions")
            .and_then(Value::as_object)
            .and_then(|d| d.get(name))
        {
            Some(target) => check(root, target, value, at, out),
            None => out.push(Violation {
                at: pointer(at),
                message: format!("schema refers to an undefined definition `{name}`"),
            }),
        }
        return;
    }
    if let Some(kind) = map.get("type").and_then(Value::as_str) {
        check_type(kind, value, at, out);
        return;
    }
    if let Some(values) = map.get("enum").and_then(Value::as_array) {
        check_enum(values, value, at, out);
        return;
    }
    if let Some(elements) = map.get("elements") {
        match value.as_array() {
            Some(items) => {
                for (index, item) in items.iter().enumerate() {
                    check(root, elements, item, &format!("{at}/{index}"), out);
                }
            }
            None => wrong(at, "an array", value, out),
        }
        return;
    }
    if let Some(values) = map.get("values") {
        match value.as_object() {
            Some(members) => {
                for (key, member) in members {
                    check(root, values, member, &format!("{at}/{key}"), out);
                }
            }
            None => wrong(at, "an object", value, out),
        }
        return;
    }
    if let Some(tag) = map.get("discriminator").and_then(Value::as_str) {
        check_discriminator(root, map, tag, value, at, out);
        return;
    }
    if map.contains_key("properties") || map.contains_key("optionalProperties") {
        check_properties(root, map, value, at, out);
    }
}

fn check_properties(
    root: &Value,
    map: &Map<String, Value>,
    value: &Value,
    at: &str,
    out: &mut Vec<Violation>,
) {
    let Some(members) = value.as_object() else {
        wrong(at, "an object", value, out);
        return;
    };
    let required = map.get("properties").and_then(Value::as_object);
    let optional = map.get("optionalProperties").and_then(Value::as_object);
    if let Some(required) = required {
        for (key, sub) in required {
            match members.get(key) {
                Some(member) => check(root, sub, member, &format!("{at}/{key}"), out),
                None => out.push(Violation {
                    at: pointer(at),
                    message: format!("missing required member `{key}`"),
                }),
            }
        }
    }
    if let Some(optional) = optional {
        for (key, sub) in optional {
            if let Some(member) = members.get(key) {
                check(root, sub, member, &format!("{at}/{key}"), out);
            }
        }
    }
    if map.get("additionalProperties").and_then(Value::as_bool) == Some(true) {
        return;
    }
    for key in members.keys() {
        let known = required.is_some_and(|r| r.contains_key(key))
            || optional.is_some_and(|o| o.contains_key(key));
        if !known {
            out.push(Violation {
                at: pointer(at),
                message: format!("unexpected member `{key}`"),
            });
        }
    }
}

fn check_discriminator(
    root: &Value,
    map: &Map<String, Value>,
    tag: &str,
    value: &Value,
    at: &str,
    out: &mut Vec<Violation>,
) {
    let Some(members) = value.as_object() else {
        wrong(at, "an object", value, out);
        return;
    };
    let Some(label) = members.get(tag).and_then(Value::as_str) else {
        out.push(Violation {
            at: pointer(at),
            message: format!("missing or non-string discriminator `{tag}`"),
        });
        return;
    };
    let mapping = map.get("mapping").and_then(Value::as_object);
    let Some(sub) = mapping.and_then(|m| m.get(label)) else {
        out.push(Violation {
            at: pointer(at),
            message: format!("discriminator `{tag}` = `{label}` is in no mapping"),
        });
        return;
    };
    // The tag itself is not described by the mapped form; hide it for the
    // member check and put it back afterwards (RFC 8927 §2.2.8).
    let mut without = members.clone();
    without.remove(tag);
    check(root, sub, &Value::Object(without), at, out);
}

fn check_enum(values: &[Value], value: &Value, at: &str, out: &mut Vec<Violation>) {
    let allowed: Vec<&str> = values.iter().filter_map(Value::as_str).collect();
    match value.as_str() {
        Some(text) if allowed.contains(&text) => {}
        _ => out.push(Violation {
            at: pointer(at),
            message: format!("{} is not one of {}", render(value), allowed.join(" | ")),
        }),
    }
}

fn check_type(kind: &str, value: &Value, at: &str, out: &mut Vec<Violation>) {
    let ok = match kind {
        "boolean" => value.is_boolean(),
        "string" => value.is_string(),
        "timestamp" => value.as_str().is_some_and(is_rfc3339),
        "float32" | "float64" => value.is_f64() || value.is_i64() || value.is_u64(),
        "int8" => in_range(value, -128, 127),
        "uint8" => in_range(value, 0, 255),
        "int16" => in_range(value, -32_768, 32_767),
        "uint16" => in_range(value, 0, 65_535),
        "int32" => in_range(value, -2_147_483_648, 2_147_483_647),
        "uint32" => in_range(value, 0, 4_294_967_295),
        other => {
            out.push(Violation {
                at: pointer(at),
                message: format!("schema names an unknown type `{other}`"),
            });
            return;
        }
    };
    if !ok {
        wrong(at, kind, value, out);
    }
}

fn in_range(value: &Value, low: i64, high: i64) -> bool {
    value.as_i64().is_some_and(|n| n >= low && n <= high)
}

/// RFC 3339 as far as a schema check needs it: the shape, not the
/// calendar. A wrong-shaped timestamp is what a report gets wrong; an
/// impossible date is a different bug and a different check.
fn is_rfc3339(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 20
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[10] == b'T'
        && (text.ends_with('Z') || text.contains('+') || text[11..].contains('-'))
}

fn wrong(at: &str, expected: &str, value: &Value, out: &mut Vec<Violation>) {
    out.push(Violation {
        at: pointer(at),
        message: format!("expected {expected}, found {}", render(value)),
    });
}

fn render(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(_) => "a boolean".into(),
        Value::Number(_) => "a number".into(),
        Value::String(_) => "a string".into(),
        Value::Array(_) => "an array".into(),
        Value::Object(_) => "an object".into(),
    }
}

fn pointer(at: &str) -> String {
    if at.is_empty() { "/".into() } else { at.into() }
}

/// Split a stream of concatenated JSON documents — `vibe install --json`
/// prints three — into its documents. A single `from_str` over the whole
/// stream fails at the second one, and reporting that as «invalid JSON»
/// would be a lie about the product.
///
/// ```
/// let docs = vibe_doc::examples::jtd::split_documents("{\"a\":1}\n{\"b\":2}\n");
/// assert_eq!(docs.len(), 2);
/// ```
pub fn split_documents(stream: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut reader = serde_json::Deserializer::from_str(stream).into_iter::<Value>();
    for item in reader.by_ref() {
        match item {
            Ok(value) => out.push(value),
            Err(_) => break,
        }
    }
    out
}

#[cfg(test)]
mod tests;
