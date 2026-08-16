//! The Rust side of the empty-policy stitch — the emission half split
//! out of `empty_policy.rs` when that file outgrew the 600-line budget,
//! along the seam the pass already had: everything here reads and
//! rewrites the GENERATED TEXT; everything in the parent reads the
//! schema. The two halves meet at `EmptyPolicies` — the parent builds
//! it, this module obeys it.
//!
//! The scanner is an attribute-run machine in `snake_case_fields`'
//! style: a run of attribute lines is buffered until the line after it
//! says what it annotates, a field line (`pub <ident>: <type>,`) of a
//! collection shape is stitched to the schema's policy for its
//! (wire, requiredness) key — requiredness read off the emission's own
//! `Option<Box<…>>` wrapper — and everything else is copied through
//! byte for byte. After the file, the count of collection fields found
//! must meet the schema's site count exactly, the same tally that
//! keeps `open_vocabulary` honest.

use anyhow::{Result, bail};

use super::{EmptyPolicies, Policy, Required};

/// The skip attribute the pinned emission writes over every `Option`
/// field — the anchor this half replaces when it collapses one.
const OPTION_SKIP: &str = r#"#[serde(skip_serializing_if = "Option::is_none")]"#;

/// The collection form of one field's type — whether the emission
/// wrapped it in `Option<Box<…>>` (an `optionalProperties` member), and
/// the outer container, which names the skip predicate's path.
/// Everything else is not this pass's business and passes through.
struct Collection<'a> {
    optional: bool,
    /// `Vec` or `BTreeMap`.
    container: &'static str,
    /// The type with any `Option<Box<…>` wrapper removed — exactly as
    /// the collapsed field line spells it.
    collapsed: &'a str,
}

/// Recognise the four collection shapes this pass rewrites (`Vec<…`,
/// `BTreeMap<…`, `Option<Box<Vec<…`, `Option<Box<BTreeMap<…`); every
/// other type — scalar, `Option<Box<String>>`, a struct reference —
/// yields `None` and the field rides through untouched.
fn collection_type(ty: &str) -> Option<Collection<'_>> {
    if let Some(inner) = ty
        .strip_prefix("Option<Box<")
        .and_then(|rest| rest.strip_suffix(">>"))
    {
        Some(Collection {
            optional: true,
            container: container_of(inner)?,
            collapsed: inner,
        })
    } else {
        Some(Collection {
            optional: false,
            container: container_of(ty)?,
            collapsed: ty,
        })
    }
}

/// The outer container of a collection type, as a predicate path stem.
fn container_of(ty: &str) -> Option<&'static str> {
    if ty.starts_with("Vec<") {
        Some("Vec")
    } else if ty.starts_with("BTreeMap<") {
        Some("BTreeMap")
    } else {
        None
    }
}

/// The Rust side of the stitch, over text, so the tests drive exactly
/// what production drives. The run is buffered until the line after
/// it says what it annotates; a field line (`pub <ident>: <type>,`) of
/// a collection shape is stitched to the schema's policy for its
/// (wire, requiredness) key, everything else is copied through. After
/// the file, the count of collection fields found must meet the
/// schema's site count exactly.
pub(super) fn apply_with_policies(
    src: &str,
    file: &str,
    policies: &EmptyPolicies,
) -> Result<String> {
    let mut out = String::with_capacity(src.len());
    // A buffered run of consecutive attribute lines — `(line, chunk,
    // trimmed)` — held until the line after it says what they annotate.
    let mut attrs: Vec<(usize, &str, &str)> = Vec::new();
    // Collection fields this file actually carried — must meet the
    // schema's site count exactly once the file ends.
    let mut found: usize = 0;

    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        let line = index + 1;
        // `chunk` keeps its line ending (`\n`, `\r\n`, or nothing at EOF);
        // `body` is the line without it, `text` the line trimmed.
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();

        if text.starts_with("#[") && text.ends_with(']') {
            attrs.push((line, chunk, text));
            continue;
        }

        if let Some((ident, ty)) = field_parts(text)
            && let Some(collection) = collection_type(ty)
        {
            found += 1;
            // The wire name: the identifier, unless a rename survived
            // the snake_case pass — a camelCase or keyword wire name.
            let wire = attrs
                .iter()
                .find_map(|(_, _, attr)| rename_wire(attr))
                .unwrap_or(ident)
                .to_string();
            let required = if collection.optional {
                Required::Optional
            } else {
                Required::Required
            };
            let Some(policy) = policies.map.get(&(wire.clone(), required)).copied() else {
                bail!(
                    "{file}:{line}: the generated field `{ident}` is a {} \
                     collection keyed `{wire}`, which no collection member \
                     of the schema describes.\n\
                     The pass keys every collection field by (wire name, \
                     requiredness) — both sides carry both halves — so a \
                     field the schema does not key means the emission and \
                     the schema have parted ways, and the pass refuses to \
                     guess a policy for it.\n\
                     Fix: restore the pinned jtd-codegen version, or \
                     teach `apply_with_policies` in \
                     `xtask/src/codegen/empty_policy/emit.rs` the new shape, \
                     then run `cargo xtask codegen`.",
                    required.as_str()
                );
            };
            match (required, policy) {
                // The emission is already the bare collection with no
                // skip — byte for byte, not one comma moved.
                (Required::Required, Policy::Emit) => {
                    replay_attrs(&attrs, &mut out);
                    out.push_str(chunk);
                }
                // Unreachable through the schema side — R21 refuses the
                // site while the map is built — and named here rather
                // than rewritten, so the two sides cannot part silently.
                (Required::Required, Policy::Omit) => {
                    bail!(
                        "{file}:{line}: the collection field `{ident}` is \
                         bare (a required member) yet keyed `omit` — rule \
                         R21 forbids omitting a required collection.\n\
                         Fix: align the schema's `metadata.\"x-empty\"` \
                         with the field's requiredness, then run `cargo \
                         xtask codegen`."
                    );
                }
                (Required::Optional, policy) => {
                    emit_collapsed(&attrs, &mut out, file, line, ident, &collection, policy)?;
                    emit_collapsed_line(body, chunk, ident, &collection, &mut out);
                }
            }
            attrs.clear();
            continue;
        }
        // Not a collection field — the run and the line are the
        // generator's, not ours.
        replay_attrs(&attrs, &mut out);
        out.push_str(chunk);
        attrs.clear();
    }
    if let Some((opened_at, _, _)) = attrs.first() {
        bail!(
            "{file}: the file ends inside an attribute run opened at line \
             {opened_at} — jtd-codegen never ends a file mid-attribute, so the \
             file this pass read is not the shape it is pinned to.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `apply_with_policies` in `xtask/src/codegen/empty_policy/emit.rs` \
             the new shape, then run `cargo xtask codegen`."
        );
    }
    if found != policies.sites {
        bail!(
            "{file}: the resolved schema describes {} collection field{} but \
             the generated file carries {} — the counts must meet exactly. \
             The tally is the tripwire that keeps the site definition honest: \
             a collection that slipped past the scanner, or an inner \
             `elements` under a `values` counted as a site, must refuse the \
             run, not pass for processed.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `apply_with_policies` in `xtask/src/codegen/empty_policy/emit.rs` \
             the new shape, then run `cargo xtask codegen`.",
            policies.sites,
            if policies.sites == 1 { "" } else { "s" },
            found
        );
    }
    Ok(out)
}

/// Replay a buffered attribute run byte for byte.
fn replay_attrs(attrs: &[(usize, &str, &str)], out: &mut String) {
    for (_, attr_chunk, _) in attrs {
        out.push_str(attr_chunk);
    }
}

/// Emit one collapsed optional-collection field's attribute run: the
/// `Option::is_none` skip attribute becomes the container's emptiness
/// predicate (`omit`) or a plain `#[serde(default)]` (`emit`), and any
/// surviving rename rides along untouched. The pinned emission writes
/// exactly one skip attribute over every `Option` field; a run without
/// it, or carrying an attribute that is neither it nor a rename, is a
/// moved pin and refuses.
fn emit_collapsed(
    attrs: &[(usize, &str, &str)],
    out: &mut String,
    file: &str,
    line: usize,
    ident: &str,
    collection: &Collection<'_>,
    policy: Policy,
) -> Result<()> {
    let mut saw_skip = false;
    for (_, attr_chunk, attr_text) in attrs {
        let attr_body = attr_chunk.trim_end_matches(['\r', '\n']);
        let indent = &attr_body[..attr_body.len() - attr_body.trim_start().len()];
        let ending = &attr_chunk[attr_body.len()..];
        if *attr_text == OPTION_SKIP {
            saw_skip = true;
            out.push_str(indent);
            match policy {
                Policy::Omit => out.push_str(&format!(
                    "#[serde(default, skip_serializing_if = \"{}::is_empty\")]",
                    collection.container
                )),
                Policy::Emit => out.push_str("#[serde(default)]"),
            }
            out.push_str(ending);
        } else if rename_wire(attr_text).is_some() {
            out.push_str(attr_chunk);
        } else {
            bail!(
                "{file}:{line}: the optional collection field `{ident}` \
                 carries an attribute this pass does not recognise:\n\
                 `{attr_text}`\n\
                 The pinned jtd-codegen emission writes only the \
                 `Option::is_none` skip and a `rename` over a field, so an \
                 unfamiliar attribute means the emission shape has moved, and \
                 the pass refuses to guess what it labels.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `emit_collapsed` in `xtask/src/codegen/empty_policy/emit.rs` \
                 the new shape, then run `cargo xtask codegen`."
            );
        }
    }
    if !saw_skip {
        bail!(
            "{file}:{line}: the optional collection field `{ident}` carries \
             no `{OPTION_SKIP}` — jtd-codegen writes it over every `Option` \
             field (the emission shape is pinned by the generator's version), \
             so this file is not that shape.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `emit_collapsed` in `xtask/src/codegen/empty_policy/emit.rs` the \
             new shape, then run `cargo xtask codegen`."
        );
    }
    Ok(())
}

/// Emit the collapsed field line itself: the indent and the identifier
/// are the generator's, byte for byte; only the type loses its
/// `Option<Box<…>` wrapper.
fn emit_collapsed_line(
    body: &str,
    chunk: &str,
    ident: &str,
    collection: &Collection<'_>,
    out: &mut String,
) {
    let indent = &body[..body.len() - body.trim_start().len()];
    let ending = &chunk[body.len()..];
    out.push_str(indent);
    out.push_str("pub ");
    out.push_str(ident);
    out.push_str(": ");
    out.push_str(collection.collapsed);
    out.push(',');
    out.push_str(ending);
}

/// Split a `pub <ident>: <type>,` field line into its identifier and
/// type — the same pinned emission shape the snake_case pass keys on,
/// which has already refused anything malformed by the time this pass
/// runs. Anything else (a `pub struct` / `pub enum` / `pub type` line,
/// a variant, a brace, a comment) yields `None`.
fn field_parts(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("pub ")?;
    let colon = rest.find(':')?;
    let ty = rest[colon + 1..].trim_start().strip_suffix(',')?;
    let ident = &rest[..colon];
    if !is_ascii_ident(ident) {
        return None;
    }
    Some((ident, ty))
}

/// The wire string of a `#[serde(rename = "…")]` attribute line, or
/// `None` for any other attribute — the twin of `rename_wire` in
/// `snake_case.rs`, local here because it polices this pass's own
/// surface (the wire half of the stitch key).
fn rename_wire(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("#[serde(rename = \"")?;
    let end = rest.find("\"]")?;
    Some(&rest[..end])
}

/// ASCII identifier shape — the same contract the sibling passes
/// enforce, local here because it polices this pass's own surface (a
/// field identifier the pass is about to rewrite around).
fn is_ascii_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
