//! The schema half of the domain-types pass: what the SCHEMA rules.
//!
//! Split off its parent along that seam — the parent keeps the entry
//! point and everything about rewriting the generator's emission, this
//! file holds only the reading that turns `metadata."x-rust-type"` into
//! rulings, plus the two name derivations jtd-codegen applies (a root's
//! stem, a definition's key) so the pass knows what the annotation is
//! talking about. Nothing here touches Rust text; nothing in the parent
//! reads a schema.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// One `x-rust-type` annotation the pass rules on: where it sits on
/// the schema side, the name the generator emits for that definition,
/// and the arm the definition's form decides.
pub(super) struct Ruling {
    /// The schema-side name for refusals: the `definitions` key, or
    /// `(the root)` for the document's root schema.
    pub(super) definition: String,
    /// The name jtd-codegen derives from the schema's own file stem
    /// (the root) or from the definition's key — PascalCase of either.
    pub(super) emitted: String,
    /// Which half of the declaration the annotation names.
    pub(super) arm: Arm,
}

/// The arm a definition's JTD form puts its annotation on.
pub(super) enum Arm {
    /// A `type` (primitive) form, emitted `pub type <Emitted> = …;`:
    /// the annotation is the alias's new right side, and the alias's
    /// own name stays.
    RightSide(String),
    /// An object / enum / discriminator form, emitted `pub struct` /
    /// `pub enum <Emitted>`: the annotation is the type's new name.
    Name(String, Keyword),
}

/// The declaration keyword a name-arm form is emitted under.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Keyword {
    Struct,
    Enum,
}

impl Keyword {
    /// The word the declaration matcher and the refusals spell alike.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
        }
    }
}

/// The rulings of the document the generator read for one schema.
pub(super) fn domain_rulings(resolved: &Path, schema: &Path) -> Result<Vec<Ruling>> {
    let text = std::fs::read_to_string(resolved)
        .with_context(|| format!("reading the resolved schema {}", resolved.display()))?;
    let doc: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", resolved.display()))?;
    let stem = root_stem(resolved)?;
    rulings_from_doc(&doc, &stem, schema)
}

/// The same read over an already-parsed document and a known root stem,
/// so the tests drive the pure half without scratch files.
fn rulings_from_doc(doc: &Value, root_stem: &str, schema: &Path) -> Result<Vec<Ruling>> {
    let mut rulings: Vec<Ruling> = Vec::new();
    if let Some(annotation) = x_rust_type(doc, schema, "(the root)")? {
        let arm = classify_arm(doc, "(the root)", &annotation, schema)?;
        rulings.push(Ruling {
            definition: "(the root)".to_string(),
            emitted: pascal_case(root_stem),
            arm,
        });
    }
    let definitions = doc.get("definitions").and_then(Value::as_object);
    for (key, form) in definitions.into_iter().flatten() {
        if let Some(annotation) = x_rust_type(form, schema, key)? {
            let arm = classify_arm(form, key, &annotation, schema)?;
            rulings.push(Ruling {
                definition: key.clone(),
                emitted: pascal_case(key),
                arm,
            });
        }
    }
    Ok(rulings)
}

/// The `metadata."x-rust-type"` a node carries — `Ok(None)` when it
/// carries none. Anything but a string refuses: the value is Rust
/// source text the pass splices into the generated file, and a number
/// or object there is a schema author's typo, not a policy.
fn x_rust_type(node: &Value, schema: &Path, definition: &str) -> Result<Option<String>> {
    let Some(found) = node.get("metadata").and_then(|m| m.get("x-rust-type")) else {
        return Ok(None);
    };
    match found {
        Value::String(text) => Ok(Some(text.clone())),
        found => bail!(
            "schema {}: the definition `{}` carries \
             `metadata.\"x-rust-type\"` = {found} — the annotation names \
             Rust source text this pass splices into the generated file, so \
             it must be a string.\n\
             Fix: quote the annotation, then run `cargo xtask codegen`.",
            schema.display(),
            definition
        ),
    }
}

/// Decide the arm a definition's form puts its annotation on: a `type`
/// form is emitted as an alias, so the annotation is its right side; an
/// object / enum / discriminator form is emitted as a named type, so
/// the annotation is its name. Anything else under an annotation
/// refuses, naming the schema, the definition and the form.
fn classify_arm(form: &Value, definition: &str, annotation: &str, schema: &Path) -> Result<Arm> {
    if form.get("type").is_some() {
        return Ok(Arm::RightSide(annotation.to_string()));
    }
    if form.get("properties").is_some() || form.get("optionalProperties").is_some() {
        return Ok(Arm::Name(annotation.to_string(), Keyword::Struct));
    }
    if form.get("enum").is_some() || form.get("discriminator").is_some() {
        return Ok(Arm::Name(annotation.to_string(), Keyword::Enum));
    }
    let shape = if form.get("elements").is_some() {
        "an `elements` (array) form"
    } else if form.get("values").is_some() {
        "a `values` (map) form"
    } else if form.get("ref").is_some() {
        "a `ref` form"
    } else {
        "no JTD form at all"
    };
    bail!(
        "schema {}: the definition `{}` carries `x-rust-type` = \
         `{annotation}` but resolves to {shape} — the annotation names \
         either the right side of an emitted alias (a `type` form) or the \
         name of an emitted type (an object, enum or discriminator form), \
         and this form is neither, so the pass has no arm to rule through \
         and refuses to guess.\n\
         Fix: give the definition one of those forms, or drop its \
         annotation, then run `cargo xtask codegen`.",
        schema.display(),
        definition
    );
}

/// The stem the generator derives the root type's name from — the
/// schema's own file name minus the `.jtd.json` tail. The vocabulary
/// substitution's scratch copy keeps the authored file name, so
/// `resolved` spells it either way.
fn root_stem(resolved: &Path) -> Result<String> {
    resolved
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".jtd.json"))
        .map(str::to_string)
        .with_context(|| {
            format!(
                "resolved schema {} is not named `*.jtd.json`, so the root \
                 type's emitted name cannot be derived from it",
                resolved.display()
            )
        })
}

/// `binding_site` → `BindingSite`, `by_purl` → `ByPurl`: the case rule
/// jtd-codegen applies to a definition key (and a schema stem) when it
/// names an emitted type.
fn pascal_case(stem: &str) -> String {
    let mut out = String::with_capacity(stem.len());
    for part in stem.split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}
