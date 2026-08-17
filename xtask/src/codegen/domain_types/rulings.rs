//! The schema half of the domain-types pass: what the SCHEMA rules.
//!
//! Split off its parent along that seam — the parent keeps the entry
//! point and everything about rewriting the generator's emission, this
//! file holds only the reading that turns `metadata."x-rust-type"` into
//! rulings, plus the two name derivations jtd-codegen applies (a root's
//! stem, a definition's key) so the pass knows what the annotation is
//! talking about. Nothing here touches Rust text; nothing in the parent
//! reads a schema.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// What one definition's naming annotations rule, gathered.
///
/// The two annotations are read INDEPENDENTLY, and that is deliberate:
/// `x-rust-type` answers "what is this type called" and
/// `x-rust-variants` answers "what are its variants called", so a
/// definition may carry either, both, or neither. Requiring the first
/// before reading the second would write today's tree — where the one
/// definition needing variant names happens to name its type too — into
/// the rule.
pub(super) struct Ruling {
    /// The schema-side name for refusals: the `definitions` key, or
    /// `(the root)` for the document's root schema.
    pub(super) definition: String,
    /// The name jtd-codegen derives from the schema's own file stem
    /// (the root) or from the definition's key — PascalCase of either.
    pub(super) emitted: String,
    /// Which half of the declaration `x-rust-type` names, when the
    /// definition carries it.
    pub(super) arm: Option<Arm>,
    /// `x-rust-variants`: wire value → the identifier the schema
    /// chooses for the variant carrying it. Empty when the definition
    /// carries no such annotation.
    ///
    /// Keyed by WIRE VALUE and never by the name the generator minted,
    /// for the reason R16 gives about the whole layer: the minted name
    /// is the generator's business (a PascalCase rule plus a collision
    /// suffix), and a pass that keys on it re-implements the very rule
    /// it exists to be independent of. The wire value is what both
    /// sides carry verbatim.
    pub(super) variants: BTreeMap<String, String>,
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

/// Where a definition is declared, for a refusal to send its reader.
///
/// A definition reaches the generator either from the authored schema
/// or from the one shared vocabulary document, substituted in — and the
/// substitution records no provenance, so this pass cannot tell which.
/// Naming only the schema would send an author to a file the annotation
/// is not in, which is the same defect as a recipe that repairs the
/// wrong thing; naming both is honest, and the definition's own key is
/// the token to grep for.
pub(super) fn declared_in(schema: &Path) -> String {
    format!(
        "{} (or `formats/vocabularies.json`, if the definition is a shared \
         fragment substituted into it)",
        schema.display()
    )
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
    if let Some(ruling) = ruling_for(doc, "(the root)", &pascal_case(root_stem), schema)? {
        rulings.push(ruling);
    }
    let definitions = doc.get("definitions").and_then(Value::as_object);
    for (key, form) in definitions.into_iter().flatten() {
        if let Some(ruling) = ruling_for(form, key, &pascal_case(key), schema)? {
            rulings.push(ruling);
        }
    }
    Ok(rulings)
}

/// Gather one definition's naming annotations, or `Ok(None)` when it
/// carries neither — a definition the pass has no business with at all
/// (`Entry` lives that way).
fn ruling_for(
    form: &Value,
    definition: &str,
    emitted: &str,
    schema: &Path,
) -> Result<Option<Ruling>> {
    let arm = match x_rust_type(form, schema, definition)? {
        Some(annotation) => Some(classify_arm(form, definition, &annotation, schema)?),
        None => None,
    };
    let variants = x_rust_variants(form, schema, definition)?;
    if arm.is_none() && variants.is_empty() {
        return Ok(None);
    }
    Ok(Some(Ruling {
        definition: definition.to_string(),
        emitted: emitted.to_string(),
        arm,
        variants,
    }))
}

/// The `metadata."x-rust-variants"` a node carries, validated against
/// the definition's OWN wire values — a key naming a value the
/// vocabulary does not have is a schema author's typo, and it is
/// catchable here, before any Rust is read.
fn x_rust_variants(
    node: &Value,
    schema: &Path,
    definition: &str,
) -> Result<BTreeMap<String, String>> {
    let Some(found) = node.get("metadata").and_then(|m| m.get("x-rust-variants")) else {
        return Ok(BTreeMap::new());
    };
    let Some(table) = found.as_object() else {
        bail!(
            "schema {}: the definition `{}` carries \
             `metadata.\"x-rust-variants\"` = {found} — the annotation maps \
             each wire value to the Rust identifier its variant should \
             carry, so it must be an object.\n\
             Fix: write it as {{\"<wire value>\": \"<Identifier>\"}}, then \
             run `cargo xtask codegen`.",
            declared_in(schema),
            definition
        );
    };
    let legal = wire_values(node);
    if legal.is_empty() {
        bail!(
            "schema {}: the definition `{}` carries \
             `metadata.\"x-rust-variants\"`, but its form has no variants — \
             only an `enum` (whose values are the variants) or a \
             `discriminator` (whose `mapping` keys are) can name them.\n\
             Fix: drop the annotation, or give the definition a form that \
             has variants, then run `cargo xtask codegen`.",
            declared_in(schema),
            definition
        );
    }
    let mut chosen = BTreeMap::new();
    for (wire, ident) in table {
        if !legal.contains(wire.as_str()) {
            let mut present: Vec<&str> = legal.iter().copied().collect();
            present.sort_unstable();
            bail!(
                "schema {}: the definition `{}` names a Rust identifier for \
                 the wire value `{wire}`, which this definition does not \
                 have. Its values are: {}.\n\
                 The annotation is keyed by WIRE VALUE — never by the name \
                 the generator minted — so a key that is not a value of this \
                 definition can never match anything.\n\
                 Fix: correct the key, then run `cargo xtask codegen`.",
                declared_in(schema),
                definition,
                present.join(", ")
            );
        }
        let Value::String(ident) = ident else {
            bail!(
                "schema {}: the definition `{}` maps the wire value \
                 `{wire}` to {ident} — a variant's Rust identifier is source \
                 text spliced into the generated file, so it must be a \
                 string.\n\
                 Fix: quote the identifier, then run `cargo xtask codegen`.",
                declared_in(schema),
                definition
            );
        };
        chosen.insert(wire.clone(), ident.clone());
    }
    Ok(chosen)
}

/// The wire values a definition's variants carry — an `enum`'s values or
/// a `discriminator`'s mapping keys. Both forms have variants, and a
/// rule that covered one of them would be a rule shaped by whichever
/// form happened to need it first.
fn wire_values(form: &Value) -> BTreeSet<&str> {
    if let Some(values) = form.get("enum").and_then(Value::as_array) {
        return values.iter().filter_map(Value::as_str).collect();
    }
    form.get("discriminator")
        .and(form.get("mapping"))
        .and_then(Value::as_object)
        .map(|mapping| mapping.keys().map(String::as_str).collect())
        .unwrap_or_default()
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
            declared_in(schema),
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
        declared_in(schema),
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
