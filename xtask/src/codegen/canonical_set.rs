//! Schema-declared canonical non-empty sets.
//!
//! JTD can describe an array but cannot say that it is non-empty, contains no
//! duplicate, or is written in the declaration order of a closed vocabulary.
//! A root carrying
//! `metadata."x-canonical-set" = "nonempty-declaration-order"` opts into that
//! stronger contract. The root must be `elements: { ref: ... }`, and the
//! referenced definition must be a closed enum. The pass turns jtd-codegen's
//! `Vec<Enum>` alias into a validated newtype and restores the schema enum's
//! authored order (the pinned generator sorts variants lexically).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

const POLICY: &str = "nonempty-declaration-order";

pub(super) fn apply_canonical_set(
    src: &str,
    file: &str,
    resolved: &Path,
    schema: &Path,
) -> Result<String> {
    let text = std::fs::read_to_string(resolved)
        .with_context(|| format!("reading resolved schema {}", resolved.display()))?;
    let doc: Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing resolved schema {}", resolved.display()))?;
    apply_from_doc(src, file, &doc, schema)
}

fn apply_from_doc(src: &str, file: &str, doc: &Value, schema: &Path) -> Result<String> {
    let Some(annotation) = doc
        .get("metadata")
        .and_then(|metadata| metadata.get("x-canonical-set"))
    else {
        return Ok(src.to_owned());
    };
    if annotation.as_str() != Some(POLICY) {
        bail!(
            "schema {}: metadata.\"x-canonical-set\" must be the string \
             \"{POLICY}\", found {annotation}",
            schema.display()
        );
    }

    let definition = doc
        .get("elements")
        .and_then(|elements| elements.get("ref"))
        .and_then(Value::as_str)
        .with_context(|| {
            format!(
                "schema {}: `{POLICY}` requires the root form `elements: {{ ref: ... }}`",
                schema.display()
            )
        })?;
    let vocabulary = doc
        .get("definitions")
        .and_then(|definitions| definitions.get(definition))
        .with_context(|| {
            format!(
                "schema {}: canonical-set root refers to missing definition `{definition}`",
                schema.display()
            )
        })?;
    if vocabulary
        .get("metadata")
        .and_then(|metadata| metadata.get("x-vocabulary"))
        .and_then(Value::as_str)
        != Some("closed")
    {
        bail!(
            "schema {}: canonical-set element definition `{definition}` must carry \
             metadata.\"x-vocabulary\" = \"closed\"",
            schema.display()
        );
    }
    let declared = vocabulary
        .get("enum")
        .and_then(Value::as_array)
        .with_context(|| {
            format!(
                "schema {}: canonical-set element definition `{definition}` must be an enum",
                schema.display()
            )
        })?;
    if declared.is_empty() {
        bail!(
            "schema {}: canonical-set vocabulary `{definition}` must not be empty",
            schema.display()
        );
    }
    let mut values = Vec::with_capacity(declared.len());
    let mut seen = BTreeSet::new();
    for value in declared {
        let value = value.as_str().with_context(|| {
            format!(
                "schema {}: canonical-set vocabulary `{definition}` contains a non-string value",
                schema.display()
            )
        })?;
        if !seen.insert(value) {
            bail!(
                "schema {}: canonical-set vocabulary `{definition}` repeats `{value}`",
                schema.display()
            );
        }
        values.push(value);
    }

    let (root_name, enum_name) = root_alias(src, file)?;
    let (ordered, variants) = reorder_enum(src, file, &enum_name, &values)?;
    replace_alias_and_append(&ordered, file, &root_name, &enum_name, &values, &variants)
}

fn root_alias(src: &str, file: &str) -> Result<(String, String)> {
    let mut found = Vec::new();
    for line in src.lines() {
        let text = line.trim();
        let Some(rest) = text.strip_prefix("pub type ") else {
            continue;
        };
        let Some((name, rhs)) = rest.split_once(" = Vec<") else {
            continue;
        };
        let Some(element) = rhs.strip_suffix(">;") else {
            continue;
        };
        found.push((name.to_owned(), element.to_owned()));
    }
    match found.as_slice() {
        [(root, element)] => Ok((root.clone(), element.clone())),
        _ => bail!(
            "{file}: a canonical-set schema must generate exactly one root \
             `pub type <Root> = Vec<<Enum>>;` alias, found {}",
            found.len()
        ),
    }
}

fn reorder_enum(
    src: &str,
    file: &str,
    enum_name: &str,
    declared: &[&str],
) -> Result<(String, BTreeMap<String, String>)> {
    let opener = format!("pub enum {enum_name} {{\n");
    let start = src
        .find(&opener)
        .map(|at| at + opener.len())
        .with_context(|| format!("{file}: generated enum `{enum_name}` is absent"))?;
    let relative_end = src[start..]
        .find("\n}\n")
        .with_context(|| format!("{file}: generated enum `{enum_name}` never closes"))?;
    let end = start + relative_end;
    let body = &src[start..end];

    let mut blocks = BTreeMap::<String, String>::new();
    let mut variants = BTreeMap::<String, String>::new();
    for block in body.split("\n\n").filter(|block| !block.trim().is_empty()) {
        let lines = block.lines().map(str::trim).collect::<Vec<_>>();
        if lines.len() != 2 {
            bail!("{file}: unfamiliar generated variant block in `{enum_name}`: `{block}`");
        }
        let wire = lines[0]
            .strip_prefix("#[serde(rename = \"")
            .and_then(|rest| {
                let suffix = "\")]";
                rest.strip_suffix(suffix)
            })
            .with_context(|| format!("{file}: enum `{enum_name}` variant has no serde rename"))?;
        let variant = lines[1]
            .strip_suffix(',')
            .with_context(|| format!("{file}: enum `{enum_name}` variant is malformed"))?;
        if blocks.insert(wire.to_owned(), block.to_owned()).is_some() {
            bail!("{file}: enum `{enum_name}` repeats wire value `{wire}`");
        }
        variants.insert(wire.to_owned(), variant.to_owned());
    }
    if blocks.len() != declared.len() || declared.iter().any(|value| !blocks.contains_key(*value)) {
        bail!(
            "{file}: generated enum `{enum_name}` does not match the schema's canonical vocabulary"
        );
    }
    let ordered_body = declared
        .iter()
        .map(|value| blocks[*value].as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut out = String::with_capacity(src.len());
    out.push_str(&src[..start]);
    out.push_str(&ordered_body);
    out.push_str(&src[end..]);
    Ok((out, variants))
}

fn replace_alias_and_append(
    src: &str,
    file: &str,
    root: &str,
    element: &str,
    declared: &[&str],
    variants: &BTreeMap<String, String>,
) -> Result<String> {
    let alias = format!("pub type {root} = Vec<{element}>;");
    if src.matches(&alias).count() != 1 {
        bail!("{file}: expected exactly one `{alias}` line");
    }
    let declaration =
        format!("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {root}(Vec<{element}>);");
    let mut out = src.replacen(&alias, &declaration, 1);

    let enum_derive = "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]";
    let enum_opener = format!("{enum_derive}\npub enum {element} {{");
    let widened = "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]";
    if out.matches(&enum_opener).count() != 1 {
        bail!("{file}: canonical-set element `{element}` has an unfamiliar derive line");
    }
    out = out.replacen(
        &enum_opener,
        &format!("{widened}\npub enum {element} {{"),
        1,
    );

    out.push_str(&format!("\nimpl {element} {{\n"));
    out.push_str(&format!(
        "    pub const ALL: [{element}; {}] = [\n",
        declared.len()
    ));
    for value in declared {
        out.push_str(&format!("        {element}::{},\n", variants[*value]));
    }
    out.push_str(
        "    ];\n\n    pub const fn as_str(self) -> &'static str {\n        match self {\n",
    );
    for value in declared {
        out.push_str(&format!(
            "            {element}::{} => \"{value}\",\n",
            variants[*value]
        ));
    }
    out.push_str("        }\n    }\n\n");
    out.push_str("    pub fn parse(value: &str) -> Option<Self> {\n");
    out.push_str("        Self::ALL.into_iter().find(|kind| kind.as_str() == value)\n    }\n");
    out.push_str("}\n\n");

    out.push_str(&format!("impl {root} {{\n"));
    out.push_str(&format!(
        "    pub(crate) fn from_vec(mut values: Vec<{element}>) -> Result<Self, String> {{\n"
    ));
    out.push_str("        if values.is_empty() {\n            return Err(\"artifact requirements must not be empty\".into());\n        }\n");
    out.push_str("        values.sort();\n");
    out.push_str(
        "        if let Some(pair) = values.windows(2).find(|pair| pair[0] == pair[1]) {\n",
    );
    out.push_str(concat!(
        "            return Err(format!(\n",
        "                \"duplicate required artifact kind `{}`\",\n",
        "                pair[0].as_str()\n",
        "            ));\n",
        "        }\n",
    ));
    out.push_str("        Ok(Self(values))\n    }\n\n");
    out.push_str(&format!(
        "    pub(crate) fn as_slice(&self) -> &[{element}] {{\n        &self.0\n    }}\n"
    ));
    out.push_str("}\n\n");

    out.push_str(&format!("impl Serialize for {root} {{\n"));
    out.push_str("    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>\n    where\n        S: serde::Serializer,\n    {\n        self.0.serialize(serializer)\n    }\n}\n\n");
    out.push_str(&format!("impl<'de> Deserialize<'de> for {root} {{\n"));
    out.push_str(&format!(
        "    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>\n    where\n        D: serde::Deserializer<'de>,\n    {{\n        let values = Vec::<{element}>::deserialize(deserializer)?;\n        Self::from_vec(values).map_err(serde::de::Error::custom)\n    }}\n}}\n"
    ));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const GENERATED: &str = r#"use serde::{Deserialize, Serialize};

pub type RequirementSet = Vec<Kind>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    #[serde(rename = "alpha")]
    Alpha,

    #[serde(rename = "zeta")]
    Zeta,
}
"#;

    #[test]
    fn declaration_order_drives_the_generated_set() {
        let doc = json!({
            "metadata": {"x-canonical-set": "nonempty-declaration-order"},
            "elements": {"ref": "kind"},
            "definitions": {
                "kind": {
                    "enum": ["zeta", "alpha"],
                    "metadata": {"x-vocabulary": "closed"}
                }
            }
        });
        let out = apply_from_doc(
            GENERATED,
            "generated.rs",
            &doc,
            Path::new("schema.jtd.json"),
        )
        .unwrap();
        assert!(out.contains("pub struct RequirementSet(Vec<Kind>);"));
        assert!(out.find("Zeta,").unwrap() < out.find("Alpha,").unwrap());
        assert!(out.contains("values.sort();"));
        assert!(out.contains("artifact requirements must not be empty"));
        assert!(out.contains("duplicate required artifact kind"));
    }

    #[test]
    fn absent_annotation_is_byte_identical() {
        assert_eq!(
            apply_from_doc(
                GENERATED,
                "generated.rs",
                &json!({}),
                Path::new("schema.jtd.json")
            )
            .unwrap(),
            GENERATED
        );
    }
}
