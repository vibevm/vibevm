//! Schema-declared reader projections for one shared strict fragment.
//!
//! A permissive format may place `"x-reader-projection": "permissive"`
//! in the metadata of an object-member `ref`. The shared fragment stays the
//! one strict generated type. This layer emits a consumer-local serde adapter
//! from the resolved JTD closure: it removes unknown object members at every
//! object form, then deserializes the resulting value into the canonical
//! shared type. Required members, scalar types, discriminator tags and closed
//! enum values are still enforced by that canonical deserializer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

use super::format_id::load_format_registry;
use super::shared_module::emitted_name;
use super::vocabulary::Resolved;

mod emit;
pub(crate) use emit::rewrite_consumer;

const MARKER: &str = "x-reader-projection";
const PERMISSIVE: &str = "permissive";

/// One marker as authored in a schema and the exact generated field it owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectionUse {
    pub(crate) target: String,
    pub(crate) owner_type: String,
    pub(crate) rust_field: String,
    pub(crate) location: String,
    pub(crate) closure: BTreeSet<String>,
}

/// The vocabulary refs a schema reaches with and without projection.
#[derive(Debug, Default)]
pub(crate) struct ProjectionScan {
    pub(crate) uses: Vec<ProjectionUse>,
    pub(crate) ordinary_roots: BTreeSet<String>,
}

struct ScanContext<'a> {
    fragment_names: &'a BTreeSet<String>,
    schema: &'a Path,
    scan: ProjectionScan,
    consumed: BTreeMap<String, usize>,
}

/// Reject projection metadata in the canonical vocabulary home. Projection is
/// a consumer-site reading policy, not a property of the shared declaration.
pub(crate) fn reject_vocabulary_markers(fragments: &Map<String, Value>, home: &Path) -> Result<()> {
    for (name, fragment) in fragments {
        if contains_marker(fragment) {
            bail!(
                "{}: vocabulary `{name}` contains `{MARKER}`. Reader projection is legal only on a consumer schema's object-member `ref`; putting it in the canonical fragment would weaken every consumer and would not be consumed exactly once.\nFix: move the marker to the permissive consumer's reference site, then run `cargo xtask codegen`.",
                home.display()
            );
        }
    }
    Ok(())
}

fn contains_marker(value: &Value) -> bool {
    match value {
        Value::Object(object) => object
            .iter()
            .any(|(key, child)| key == MARKER || contains_marker(child)),
        Value::Array(items) => items.iter().any(contains_marker),
        _ => false,
    }
}

/// Find every marker in the authored schema, validate its local shape, and
/// record the unprojected vocabulary roots used to compute ordinary shared
/// reader policy. Markers on array elements, map values, roots, or metadata
/// data are refused: the declared mechanism is a projected request FIELD.
pub(crate) fn scan_schema(
    doc: &Value,
    schema: &Path,
    fragment_names: &BTreeSet<String>,
) -> Result<ProjectionScan> {
    let root_type = schema_root_type(schema)?;
    let mut discovered = Vec::new();
    discover_markers(doc, "$", &mut discovered);
    let mut context = ScanContext {
        fragment_names,
        schema,
        scan: ProjectionScan::default(),
        consumed: BTreeMap::new(),
    };
    scan_form(doc, &root_type, None, "$", &mut context)?;
    for location in discovered {
        match context.consumed.get(&location).copied().unwrap_or(0) {
            1 => {}
            0 => bail!(
                "schema {} at {location}: `{MARKER}` is not the direct metadata member of an object-member `ref`, so it was not consumed.",
                schema.display()
            ),
            count => bail!(
                "schema {} at {location}: `{MARKER}` was consumed {count} times (expected exactly once).",
                schema.display()
            ),
        }
    }
    let mut fields: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    for usage in &context.scan.uses {
        if let Some(first) = fields.insert(
            (&usage.owner_type, &usage.rust_field),
            usage.location.as_str(),
        ) {
            bail!(
                "schema {}: projection markers at {first} and {} both resolve to generated field `{}::{}`; a marker must be consumed by exactly one field.",
                schema.display(),
                usage.location,
                usage.owner_type,
                usage.rust_field
            );
        }
    }
    Ok(context.scan)
}

fn discover_markers(value: &Value, location: &str, found: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_location = format!("{location}.{key}");
                if key == MARKER {
                    found.push(child_location.clone());
                }
                discover_markers(child, &child_location, found);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                discover_markers(child, &format!("{location}[{index}]"), found);
            }
        }
        _ => {}
    }
}

fn scan_form(
    form: &Value,
    owner_type: &str,
    member: Option<&str>,
    location: &str,
    context: &mut ScanContext<'_>,
) -> Result<()> {
    let Some(object) = form.as_object() else {
        return Ok(());
    };
    let marker = object
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get(MARKER));
    if let Some(value) = marker {
        let Some(value) = value.as_str() else {
            bail!(
                "schema {} at {location}: `{MARKER}` must be the string `permissive`, got {}.",
                context.schema.display(),
                json_kind(value)
            );
        };
        if value != PERMISSIVE {
            bail!(
                "schema {} at {location}: unknown `{MARKER}` value `{value}`; the only admitted value is `permissive`.",
                context.schema.display()
            );
        }
        let Some(target) = object.get("ref").and_then(Value::as_str) else {
            bail!(
                "schema {} at {location}: `{MARKER}` is legal only on a JTD `ref` site.",
                context.schema.display()
            );
        };
        let Some(field) = member else {
            bail!(
                "schema {} at {location}: projected `ref` `{target}` is not an object-member field; reader projection is a field adapter and every marker must be consumed exactly once.",
                context.schema.display()
            );
        };
        if !context.fragment_names.contains(target) {
            bail!(
                "schema {} at {location}: projected `ref` `{target}` is not a shared fragment in `formats/vocabularies.json`.",
                context.schema.display()
            );
        }
        let marker_location = format!("{location}.metadata.{MARKER}");
        *context.consumed.entry(marker_location).or_default() += 1;
        context.scan.uses.push(ProjectionUse {
            target: target.to_string(),
            owner_type: owner_type.to_string(),
            rust_field: rust_field(field),
            location: location.to_string(),
            closure: BTreeSet::new(),
        });
    } else if let Some(target) = object.get("ref").and_then(Value::as_str)
        && context.fragment_names.contains(target)
    {
        context.scan.ordinary_roots.insert(target.to_string());
    }

    let nested_owner;
    let child_owner = if let Some(field) = member
        && object.get("ref").is_none()
        && (object.contains_key("properties")
            || object.contains_key("optionalProperties")
            || object.contains_key("mapping"))
    {
        nested_owner = format!("{owner_type}{}", pascal_wire(field));
        nested_owner.as_str()
    } else {
        owner_type
    };

    if let Some(definitions) = object.get("definitions").and_then(Value::as_object) {
        for (name, definition) in definitions {
            scan_form(
                definition,
                &emitted_name(name),
                None,
                &format!("{location}.definitions.{name}"),
                context,
            )?;
        }
    }
    if let Some(mapping) = object.get("mapping").and_then(Value::as_object) {
        for (tag, arm) in mapping {
            scan_form(
                arm,
                &format!("{child_owner}{}", pascal_wire(tag)),
                None,
                &format!("{location}.mapping.{tag}"),
                context,
            )?;
        }
    }
    for block in ["properties", "optionalProperties"] {
        if let Some(properties) = object.get(block).and_then(Value::as_object) {
            for (field, child) in properties {
                let child_location = format!("{location}.{block}.{field}");
                scan_form(child, child_owner, Some(field), &child_location, context)?;
            }
        }
    }
    for wrapper in ["elements", "values"] {
        if let Some(child) = object.get(wrapper) {
            scan_form(
                child,
                child_owner,
                None,
                &format!("{location}.{wrapper}"),
                context,
            )?;
        }
    }
    Ok(())
}

/// Projection does not turn an ordinary permissive consumer into a legal
/// mixed-policy consumer. It is admitted only when the marked target has an
/// unprojected registered strict owner.
pub(crate) fn validate_policies(root: &Path, resolved: &[(PathBuf, Resolved)]) -> Result<()> {
    let entries = load_format_registry(root)?;
    let role_of = |schema: &Path| -> Option<&str> {
        let rel = schema.strip_prefix(root).unwrap_or(schema);
        let key = rel.display().to_string().replace('\\', "/");
        entries
            .iter()
            .find(|entry| entry.schema == key)
            .map(|entry| entry.foreign_parsers.as_str())
    };

    for (schema, resolution) in resolved {
        if resolution.projections.is_empty() {
            continue;
        }
        let Some(role) = role_of(schema) else {
            bail!(
                "schema {} declares `{MARKER}` but no registered format owns it; projection policy cannot be computed.",
                schema.display()
            );
        };
        if role == "none" {
            bail!(
                "schema {} declares `{MARKER}` while its registered consumer is strict (`foreign_parsers = \"none\"`); projection is legal only in a registry-permissive consumer.",
                schema.display()
            );
        }
        for projection in &resolution.projections {
            let owner = resolved.iter().any(|(candidate, candidate_resolution)| {
                role_of(candidate) == Some("none")
                    && candidate_resolution
                        .ordinary_vocabularies
                        .contains(&projection.target)
            });
            if !owner {
                bail!(
                    "schema {} at {} projects shared fragment `{}` permissively, but no registered strict consumer owns that fragment through an unprojected reference. Projection may adapt a strict canonical owner; it may not create one or weaken an ownerless fragment.",
                    schema.display(),
                    projection.location,
                    projection.target
                );
            }
        }
    }
    Ok(())
}

/// A thin schema whose root is a same-named shared fragment makes the pinned
/// generator emit a reflexive alias beside the fragment declaration
/// (`pub type Ir = Ir;`). Remove that parasitic root before the normal passes;
/// rewiring then replaces the fragment declaration with the canonical shared
/// re-export, leaving the legacy module path as an exact re-export surface.
pub(crate) fn strip_reflexive_root_alias(
    file: &Path,
    resolved_doc: &Path,
    schema: &Path,
) -> Result<()> {
    let document: Value = serde_json::from_str(
        &std::fs::read_to_string(resolved_doc)
            .with_context(|| format!("reading resolved schema {}", resolved_doc.display()))?,
    )
    .with_context(|| format!("parsing resolved schema {}", resolved_doc.display()))?;
    let Some(target) = document.get("ref").and_then(Value::as_str) else {
        return Ok(());
    };
    let root = schema_root_type(schema)?;
    if emitted_name(target) != root {
        return Ok(());
    }
    let source = std::fs::read_to_string(file)
        .with_context(|| format!("reading generated root projection {}", file.display()))?;
    let prefix = format!("pub type {root} = ");
    let aliases = source
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix(&prefix)
                .and_then(|tail| tail.strip_suffix(';'))
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    if aliases.len() != 1 {
        bail!(
            "{}: same-named shared root `{target}` requires exactly one generated root alias `{prefix}<definition>;`, found {}. The pinned generator shape moved, so the projection refuses to guess.",
            file.display(),
            aliases.len()
        );
    }
    let collision = &aliases[0];
    if collision != &root
        && !(collision.starts_with(&root)
            && collision[root.len()..]
                .chars()
                .all(|character| character.is_ascii_digit()))
    {
        bail!(
            "{}: same-named shared root `{target}` was emitted through unexpected local definition `{collision}`; expected `{root}` plus only the generator's numeric collision suffix.",
            file.display()
        );
    }
    let alias = format!("pub type {root} = {collision};");
    let mut output = String::with_capacity(source.len());
    let mut squash_next_blank = false;
    for chunk in source.split_inclusive('\n') {
        let text = chunk.trim_end_matches(['\r', '\n']).trim();
        if text == alias {
            squash_next_blank = true;
            continue;
        }
        if squash_next_blank && text.is_empty() {
            squash_next_blank = false;
            continue;
        }
        squash_next_blank = false;
        if collision == &root {
            output.push_str(chunk);
        } else {
            output.push_str(&chunk.replace(collision, &root));
        }
    }
    super::write::write_generated(file, &output)
}

/// Make only the soon-to-be-rewired local copies of projected fragments match
/// their strict shared owner. The permissive consumer root remains permissive;
/// its generated field adapter is the sole projection boundary.
pub(crate) fn apply_projected_copy_strictness(
    source: &str,
    file: &str,
    projections: &[ProjectionUse],
) -> Result<String> {
    let fragments = projections
        .iter()
        .flat_map(|projection| projection.closure.iter().cloned())
        .collect::<BTreeSet<_>>();
    super::shared_module::apply_projected_copy_strictness(source, file, &fragments)
}

/// Add one `deserialize_with` attribute per marker and append the adapter code
/// derived from that marker's resolved transitive fragment closure.
fn schema_root_type(schema: &Path) -> Result<String> {
    let file = schema
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("schema {} has no UTF-8 file name", schema.display()))?;
    let stem = file
        .strip_suffix(".jtd.json")
        .with_context(|| format!("schema {} does not end in `.jtd.json`", schema.display()))?;
    Ok(pascal_wire(stem))
}

fn pascal_wire(value: &str) -> String {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut characters = segment.chars();
            let Some(first) = characters.next() else {
                return String::new();
            };
            first.to_ascii_uppercase().to_string() + characters.as_str()
        })
        .collect()
}

fn rust_field(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn struct_name(text: &str) -> Option<&str> {
    let name = text
        .strip_prefix("pub struct ")?
        .strip_suffix('{')?
        .trim_end();
    (!name.is_empty()).then_some(name)
}

fn field_name(text: &str) -> Option<&str> {
    text.strip_prefix("pub ")?
        .split_once(':')
        .map(|(name, _)| name)
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
#[path = "reader_projection/tests.rs"]
mod tests;
