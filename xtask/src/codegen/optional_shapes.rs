//! The optional-shapes pass — the fifth content edit the generator's
//! emission takes (the order rule lives in `postproc`'s docs: a pass
//! keyed to the emission shape runs while the file is STILL that
//! emission, and this one is keyed to the
//! `#[serde(skip_serializing_if = "Option::is_none")]` attribute over a
//! `pub <field>: Option<Box<…>>` line that is not a collection — the
//! collections were collapsed by `empty_policy` one pass earlier — so it
//! runs after arm boxing, field snake_casing, map ordering and the
//! empty-collection policy, and before the vocabularies open).
//!
//! What it enforces: the RUST SHAPE of an optional field is a policy the
//! schema declares, never a guess. After the empty-collection pass two
//! classes of `Option<Box<…>>` remain, and both lose the `Box` — serde
//! renders `Box<T>` exactly as `T`, so the box is invisible on the wire
//! and meaningless in the type. An optional SCALAR is decided by
//! `metadata."x-default"`: `null` keeps the `Option` (an absent key means
//! "no value"), a boolean literal collapses the field to the bare `bool`
//! (PROP-044 §2b's two-part boolean axis: an absent key already means the
//! default, exactly as the hand-written twin `commands/list.rs` carries
//! `overridden`), and a missing key is a generation error — the policy
//! lives on the schema side and is not derivable from the generated Rust.
//! Any OTHER literal — `"stable"`, `1`, and `true` alike — is a loud
//! refusal, not a best effort: emitting it would take a NAMED default
//! function (`#[serde(default = "…")]`) and a NAMED skip predicate, a new
//! surface of names in generated code that no site needs today. An
//! optional STRUCTURE needs no annotation at all and does not read one:
//! the `Option` stays because accepting `{}` from a foreign writer is the
//! type's job, while normalising an empty object is projector work, not
//! a skip predicate's.
//!
//! A third row the tree itself contributed (measured, absent from the
//! phase inventory): a REQUIRED member with `nullable: true` also arrives
//! as `Option<Box<…>>` and carries NO skip attribute. The rule covers it
//! the way the schema states it: the key is required, `null` is a value it
//! carries, so the `Option` stays, the `Box` goes, and no skip predicate
//! is added — `None` serialises as `null`. Reading needs one extra piece
//! that JTD's generated Rust shape does not express on its own: serde's
//! default treatment of an `Option<T>` accepts an ABSENT key as `None`.
//! The pass therefore adds the shared required-nullable deserializer,
//! which accepts a present value or `null` but makes absence a missing-
//! field error. The rule is attached to the decision, not to any format,
//! so every future required-nullable member gets the same strictness.
//!
//! The stitch: the schema side classifies each site by its RESOLVED form
//! (a `ref` follows the document's own `definitions` — the vocabulary
//! substitution has placed every fragment there by now) and keys its
//! decision by (wire name, class); the Rust side classifies each
//! `Option<Box<…>>` field by its payload type (a primitive, or a local
//! `pub type` alias resolving to one, against a `pub struct` the file
//! declares) and cross-checks the class from both sides — a divergence
//! refuses. After the file, the count of `Option<Box<…>>` fields found
//! in Rust must meet the schema's site count exactly — the same tally
//! that keeps `empty_policy` and `open_vocabulary` honest.
//!
//! This file is the schema side of the stitch plus the entry; the
//! Rust-side scanner that rewrites the emission lives in the child
//! `optional_shapes/emit.rs`, split along that seam when the file
//! outgrew the 600-line budget. The two halves meet at
//! `OptionalShapes`: this half builds it, the child obeys it.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

mod emit;

use emit::apply_with_shapes;

/// The pass entry the driver calls: read the schema-side decisions off
/// the document the generator read (`resolved` — the authored schema
/// when it pulls no vocabularies, the scratch copy with the fragments
/// placed otherwise), then stitch the generated Rust to them. No new
/// input is invented — `generate_into` already holds `resolved` exactly
/// where this pass needs it.
pub(super) fn apply_optional_shapes(
    src: &str,
    file: &str,
    resolved: &Path,
    schema: &Path,
) -> Result<String> {
    let shapes = shape_decisions(resolved, schema)?;
    apply_with_shapes(src, file, &shapes)
}

/// The class of one site's resolved form — the half of the stitch key
/// both sides carry, and the thing a scalar's policy is read against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ShapeClass {
    /// A `type` form: string, boolean, timestamp, a number.
    Scalar,
    /// An `enum` form: one closed vocabulary value.
    Vocabulary,
    /// A `properties` / `optionalProperties` form: a generated struct.
    Structure,
    /// An `elements` / `values` form reached through a named alias. Direct
    /// collection fields remain owned by the empty-policy pass.
    Collection,
}

impl ShapeClass {
    /// The word the refusals spell for this class.
    fn as_str(self) -> &'static str {
        match self {
            ShapeClass::Scalar => "scalar",
            ShapeClass::Vocabulary => "vocabulary",
            ShapeClass::Structure => "structure",
            ShapeClass::Collection => "collection",
        }
    }
}

/// The emission one site's field takes — what the two sides agreed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    /// `Option<Box<T>>` → `Option<T>` with
    /// `#[serde(default, skip_serializing_if = "Option::is_none")]` —
    /// the `x-default: null` scalar and every optional structure.
    OptionValue,
    /// `Option<Box<bool>>` → `bool` with
    /// `#[serde(default, skip_serializing_if = "std::ops::Not::not")]` —
    /// the `x-default: false` boolean (an absent key means `false`).
    BoolFalse,
    /// A required `nullable: true` member: `Option<Box<T>>` → `Option<T>`
    /// with the shared strict deserializer — `None` writes `null`, while
    /// an absent key is a parse refusal.
    RequiredNullable,
}

impl Decision {
    /// The phrase the conflict refusal names each decision by.
    fn as_str(self) -> &'static str {
        match self {
            Decision::OptionValue => "an Option value",
            Decision::BoolFalse => "a false-defaulted bool",
            Decision::RequiredNullable => "a required nullable",
        }
    }
}

/// Which `properties` block a member lives in — visible from both sides
/// of the stitch, and the difference between an absent key the writer
/// may omit and a null it must write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Required {
    /// A member of `properties`: JTD makes it mandatory on the wire.
    Required,
    /// A member of `optionalProperties`: the writer may omit it.
    Optional,
}

/// What the schema side of the stitch read out of one resolved schema:
/// every site's decision keyed by (wire name, class), plus the NUMBER OF
/// SITES (not of distinct keys) the pass rules on — the tally the
/// Rust-side scanner must meet exactly, the tripwire that keeps a
/// silently skipped field from passing for processed.
#[derive(Debug)]
struct OptionalShapes {
    map: BTreeMap<(String, ShapeClass), Decision>,
    sites: usize,
}

/// One member site the walk found: the member's wire name, the
/// `properties` block it lives in, the member node itself (its
/// annotation and `ref` are read at the site), and the resolved-document
/// path the refusals name.
struct Site<'a> {
    wire: String,
    required: Required,
    node: &'a Map<String, Value>,
    path: String,
}

/// The decisions of the document the generator read for one schema.
fn shape_decisions(resolved: &Path, schema: &Path) -> Result<OptionalShapes> {
    let text = std::fs::read_to_string(resolved)
        .with_context(|| format!("reading the resolved schema {}", resolved.display()))?;
    let doc: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", resolved.display()))?;
    decisions_from_doc(&doc, schema)
}

/// The same read over an already-parsed document, so the tests drive the
/// pure half without scratch files.
fn decisions_from_doc(doc: &Value, schema: &Path) -> Result<OptionalShapes> {
    let definitions = doc.get("definitions").and_then(Value::as_object);
    let mut sites: Vec<Site<'_>> = Vec::new();
    collect_sites(doc, "", &mut sites);
    let mut map: BTreeMap<(String, ShapeClass), Decision> = BTreeMap::new();
    // The tally counts the sites this pass RULES ON — a collection
    // member is a site of the walk but not of this pass, and the Rust
    // side's counter never sees it (the empty-policy pass collapsed it),
    // so counting it here would part the two numbers for no reason.
    let mut counted: usize = 0;
    for site in &sites {
        let Some((class, decision)) = site_decision(site, definitions, schema)? else {
            // A collection site — `empty_policy`'s half, already collapsed
            // one pass earlier, so the Rust side will not count it either.
            continue;
        };
        counted += 1;
        let key = (site.wire.clone(), class);
        if let Some(existing) = map.get(&key) {
            if *existing != decision {
                bail!(
                    "schema {}: two fields share the key (`{}`, {}) with \
                     different shape policies — {} and {}. One field on one \
                     side of the requiredness line carries one shape; it cannot \
                     be two at once.\n\
                     Fix: make the `metadata.\"x-default\"` of both definitions \
                     agree, then run `cargo xtask codegen`.",
                    schema.display(),
                    site.wire,
                    class.as_str(),
                    existing.as_str(),
                    decision.as_str()
                );
            }
            // Same key, same decision: a legal diamond of the vocabulary
            // substitution — counted as its own site all the same.
        } else {
            map.insert(key, decision);
        }
    }
    Ok(OptionalShapes {
        map,
        sites: counted,
    })
}

/// Walk a resolved schema collecting every member site this pass rules
/// on: EVERY member of an `optionalProperties` block (the writer may
/// omit it — that is the state the shape decides), and a `properties`
/// member only when it is `nullable` (the one required form the
/// generator also wraps in `Option<Box<…>>`). `metadata` blocks are
/// skipped on the way down — annotation data the JTD machinery never
/// reads, so a form-shaped key inside one is data, not a site (the same
/// cut the sibling passes make). Members are walked INTO as well: an
/// inline structure's own members are sites of their own, and a `ref`
/// member's target is reached through `definitions`, never by inlining.
fn collect_sites<'a>(value: &'a Value, trail: &str, sites: &mut Vec<Site<'a>>) {
    let Some(fields) = value.as_object() else {
        return;
    };
    for (key, field) in fields {
        if key == "metadata" {
            continue;
        }
        let path = join_trail(trail, key);
        let required = match key.as_str() {
            "properties" => Some(Required::Required),
            "optionalProperties" => Some(Required::Optional),
            _ => None,
        };
        let Some(required) = required else {
            collect_sites(field, &path, sites);
            continue;
        };
        let Some(members) = field.as_object() else {
            continue;
        };
        for (wire, member) in members {
            let member_path = join_trail(&path, wire);
            let Some(node) = member.as_object() else {
                continue;
            };
            let nullable = node.get("nullable") == Some(&Value::Bool(true));
            if required == Required::Optional || nullable {
                sites.push(Site {
                    wire: wire.clone(),
                    required,
                    node,
                    path: member_path.clone(),
                });
            }
            collect_sites(member, &member_path, sites);
        }
    }
}

/// `trail.key`, with the root spelled as just `key`.
fn join_trail(trail: &str, key: &str) -> String {
    if trail.is_empty() {
        key.to_string()
    } else {
        format!("{trail}.{key}")
    }
}

/// Rule on one site: resolve its `ref` through the document's
/// `definitions`, classify the resolved form, and read the policy the
/// class calls for. `Ok(None)` is the honest skip — a collection site,
/// `empty_policy`'s half of the stitch, already collapsed one pass
/// earlier.
fn site_decision(
    site: &Site<'_>,
    definitions: Option<&Map<String, Value>>,
    schema: &Path,
) -> Result<Option<(ShapeClass, Decision)>> {
    let form = resolve_form(site.node, definitions, &site.path, schema)?;
    if form.contains_key("elements") || form.contains_key("values") {
        if site.required == Required::Required
            && site.node.get("nullable") == Some(&Value::Bool(true))
            && site.node.contains_key("ref")
        {
            return Ok(Some((ShapeClass::Collection, Decision::RequiredNullable)));
        }
        return Ok(None);
    }
    let class = classify_form(form, &site.path, schema)?;
    if site.required == Required::Required {
        // The walk sent this member here only because it is nullable.
        return Ok(Some((class, Decision::RequiredNullable)));
    }
    if site.node.contains_key("nullable") {
        bail!(
            "schema {}: the optional field `{}` also carries `nullable` — \
             the pass has no rule for a member that may be absent AND null at \
             once (no site of today's tree does), and guessing the emission \
             would write a shape nothing pins.\n\
             Fix: drop `nullable` from this member, or teach \
             `optional_shapes.rs` the combination, then run `cargo xtask \
             codegen`.",
            schema.display(),
            site.path
        );
    }
    let decision = match class {
        // A structure reads no annotation: the Option is the point, the
        // Box is the noise, and the decision is already made.
        ShapeClass::Structure => Decision::OptionValue,
        ShapeClass::Collection => unreachable!("required collection refs returned above"),
        ShapeClass::Scalar => scalar_decision(site, schema)?,
        ShapeClass::Vocabulary => vocabulary_decision(site, schema)?,
    };
    Ok(Some((class, decision)))
}

/// Follow a member's `ref` chain through the document's own
/// `definitions` — the vocabulary substitution has placed every
/// fragment there — and return the node the reference lands on. A
/// dangling name or a cycle refuses, naming the site and the route.
fn resolve_form<'a>(
    node: &'a Map<String, Value>,
    definitions: Option<&'a Map<String, Value>>,
    path: &str,
    schema: &Path,
) -> Result<&'a Map<String, Value>> {
    let mut form = node;
    let mut route: Vec<&str> = Vec::new();
    while let Some(name) = form.get("ref").and_then(Value::as_str) {
        if route.contains(&name) {
            bail!(
                "schema {}: the `ref` chain at `{}` ({}) loops back to \
                 `{name}` — a form cannot resolve through itself.\n\
                 Fix: break the cycle in `definitions`, then run `cargo xtask \
                 codegen`.",
                schema.display(),
                path,
                route.join(" -> ")
            );
        }
        route.push(name);
        let Some(next) = definitions
            .and_then(|defs| defs.get(name))
            .and_then(Value::as_object)
        else {
            bail!(
                "schema {}: `{{\"ref\": \"{name}\"}}` at `{}` does not resolve \
                 — `{name}` is not in this document's `definitions`.\n\
                 Fix: declare `{name}` in `definitions` (or pull it in through \
                 `metadata.x-vocabularies`), then run `cargo xtask codegen`.",
                schema.display(),
                path
            );
        };
        form = next;
    }
    Ok(form)
}

/// Classify a resolved form by its JTD shape: a `type` form is a scalar,
/// an `enum` is a vocabulary, and a `properties` / `optionalProperties`
/// form is a structure. Anything else — a `discriminator` union or empty
/// form — has no rule in this pass and refuses rather than passes for
/// processed: the tally would otherwise catch it as a bare count, which
/// names nothing.
fn classify_form(form: &Map<String, Value>, path: &str, schema: &Path) -> Result<ShapeClass> {
    if form.contains_key("type") {
        return Ok(ShapeClass::Scalar);
    }
    if form.contains_key("enum") {
        return Ok(ShapeClass::Vocabulary);
    }
    if form.contains_key("properties") || form.contains_key("optionalProperties") {
        return Ok(ShapeClass::Structure);
    }
    let shape = if form.contains_key("discriminator") {
        "a discriminator union"
    } else {
        "no JTD form at all"
    };
    bail!(
        "schema {}: the optional field `{}` resolves to {shape} — the pass \
         has no shape rule for it (an optional vocabulary, a tagged union or \
         an empty form is neither the scalar `x-default` decides nor the \
         structure that loses its `Box`), and it refuses to guess.\n\
         Fix: give the member a scalar or structure form, or teach \
         `optional_shapes.rs` this one, then run `cargo xtask codegen`.",
        schema.display(),
        path
    );
}

/// An optional vocabulary has one supported absent-key policy: `null` keeps
/// it as `Option<Enum>`. Unlike a boolean scalar, an enum has no zero value to
/// collapse to; any non-null default would require a named generator-owned
/// function and is refused until a real format needs one.
fn vocabulary_decision(site: &Site<'_>, schema: &Path) -> Result<Decision> {
    let Some(annotation) = site.node.get("metadata").and_then(|m| m.get("x-default")) else {
        bail!(
            "schema {}: the optional vocabulary field `{}` carries no \
             `metadata.\"x-default\"` — an absent key must be declared as \
             `null` before the generator may emit `Option<Enum>`.\n\
             Fix: add `\"x-default\": null`, then run `cargo xtask codegen`.",
            schema.display(),
            site.path
        );
    };
    if annotation == &Value::Null {
        return Ok(Decision::OptionValue);
    }
    bail!(
        "schema {}: the optional vocabulary field `{}` carries \
         `metadata.\"x-default\"` = {} — only `null` is supported for a \
         vocabulary; any named default needs a new generator-owned function.\n\
         Fix: set the annotation to `null`, then run `cargo xtask codegen`.",
        schema.display(),
        site.path,
        annotation
    )
}

/// Read one scalar site's `metadata."x-default"` and rule on it: `null`
/// keeps the `Option`, `false` collapses to the bare bool, and anything
/// else — a non-boolean literal, or `true` — is a loud refusal naming
/// exactly what emitting it would take: a NAMED default function
/// (`#[serde(default = "…")]`) and a NAMED skip predicate, a new surface
/// of names in generated code. Zero sites need it today; the limit is
/// named, not silently skipped.
fn scalar_decision(site: &Site<'_>, schema: &Path) -> Result<Decision> {
    let Some(annotation) = site.node.get("metadata").and_then(|m| m.get("x-default")) else {
        bail!(
            "schema {}: the optional scalar field `{}` carries no \
             `metadata.\"x-default\"` — whether an absent key means \"no \
             value\" (`null`), a collapsed default (`false`) or a refusal is \
             decided per field on the schema side and is not derivable from \
             the generated Rust.\n\
             Fix: add `\"x-default\": null` (or a boolean literal) to this \
             member's `metadata` (in {} itself, or in the vocabulary fragment \
             it pulls from formats/vocabularies.json), then run `cargo xtask \
             codegen`.",
            schema.display(),
            site.path,
            schema.display()
        );
    };
    match annotation {
        Value::Null => Ok(Decision::OptionValue),
        Value::Bool(false) => Ok(Decision::BoolFalse),
        found => {
            let found = found.to_string();
            bail!(
                "schema {}: the optional scalar field `{}` carries \
                 `metadata.\"x-default\"` = {found} — the pass builds no \
                 literal but the boolean `false`: any other default (a \
                 string, a number, even `true`) would take a NAMED default \
                 function (`#[serde(default = \"…\")]`) and a NAMED skip \
                 predicate — a new surface of names in generated code no site \
                 of today's tree needs.\n\
                 Fix: set the annotation to `null` or `false`, or teach \
                 `optional_shapes.rs` the named-function emission, then run \
                 `cargo xtask codegen`.",
                schema.display(),
                site.path
            );
        }
    }
}

#[cfg(test)]
#[path = "optional_shapes/tests.rs"]
mod tests;
