//! The Rust side of the optional-shapes stitch — the emission half split
//! out of `optional_shapes.rs` when that file outgrew the 600-line
//! budget, along the seam the pass already had: everything here reads
//! and rewrites the GENERATED TEXT; everything in the parent reads the
//! schema. The two halves meet at `OptionalShapes` — the parent builds
//! it, this module obeys it.
//!
//! The scanner is an attribute-run machine in the sibling passes'
//! style: a run of attribute lines is buffered until the line after it
//! says what it annotates, a field line (`pub <ident>: <type>,`) of the
//! `Option<Box<…>>` shape is classified by its payload (a primitive, or
//! a local `pub type` alias resolving to one, against a `pub struct`
//! the file declares), stitched to the schema's decision for its (wire,
//! class) key, and reshaped; everything else is copied through byte for
//! byte. After the file, the count of `Option<Box<…>>` fields found
//! must meet the schema's site count exactly — the same tally that
//! keeps the sibling passes honest.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};

use super::{Decision, OptionalShapes, ShapeClass};

/// The skip attribute the pinned emission writes over every
/// `optionalProperties` field — the anchor this pass replaces when it
/// reshapes one.
const OPTION_SKIP: &str = r#"#[serde(skip_serializing_if = "Option::is_none")]"#;

/// The attribute the reshaped `Option` field carries — the pinned form's
/// skip, plus the `default` that folds an absent key into `None`.
const OPTION_FORM: &str = r#"#[serde(default, skip_serializing_if = "Option::is_none")]"#;

/// The attribute a false-defaulted bool carries — the exact form the
/// hand-written twin `commands/list.rs` gives `overridden`: an absent
/// key already means `false`, so `false` is the only value never written.
const BOOL_FORM: &str = r#"#[serde(default, skip_serializing_if = "std::ops::Not::not")]"#;

/// The type declarations a generated file carries, as a first sweep
/// reads them: the payload of an `Option<Box<…>>` field is classified
/// against these — a `pub type` alias resolving to a primitive is a
/// scalar, a `pub struct` name is a structure, and anything else (a
/// vocabulary `pub enum`, a stranger) has no rule.
struct TypeDecls<'a> {
    aliases: BTreeMap<&'a str, &'a str>,
    structs: BTreeSet<&'a str>,
}

/// The Rust side of the stitch, over text, so the tests drive exactly
/// what production drives. The run is buffered until the line after it
/// says what it annotates; a field line (`pub <ident>: <type>,`) whose
/// type is `Option<Box<…>>` is classified by its payload, stitched to
/// the schema's decision for its (wire, class) key, and reshaped;
/// everything else is copied through byte for byte. After the file, the
/// count of `Option<Box<…>>` fields found must meet the schema's site
/// count exactly.
pub(super) fn apply_with_shapes(src: &str, file: &str, shapes: &OptionalShapes) -> Result<String> {
    let decls = type_decls(src);
    let mut out = String::with_capacity(src.len());
    // A buffered run of consecutive attribute lines — `(line, chunk,
    // trimmed)` — held until the line after it says what they annotate.
    let mut attrs: Vec<(usize, &str, &str)> = Vec::new();
    // `Option<Box<…>>` fields this file actually carried — must meet the
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
            && let Some(inner) = boxed_payload(ty)
        {
            found += 1;
            // The wire name: the identifier, unless a rename survived
            // the snake_case pass — a camelCase or keyword wire name.
            let wire = attrs
                .iter()
                .find_map(|(_, _, attr)| rename_wire(attr))
                .unwrap_or(ident)
                .to_string();
            let class = classify_payload(inner, &decls, file, line, ident)?;
            let decision = lookup(shapes, &wire, class, file, line, ident, inner)?;
            match decision {
                Decision::OptionValue => {
                    emit_rewritten_run(&attrs, &mut out, file, line, ident, OPTION_FORM)?;
                    emit_type_line(body, chunk, ident, &format!("Option<{inner}>"), &mut out);
                }
                Decision::BoolFalse => {
                    if inner != "bool" {
                        bail!(
                            "{file}:{line}: the schema rules the boolean field \
                             `{ident}` (`{wire}`) false-defaulted, but the \
                             generated type is `Option<Box<{inner}>>` — the \
                             two sides of the stitch disagree about the type \
                             itself, and collapsing a non-bool to `bool` would \
                             hide that behind a green run.\n\
                             Fix: align the schema's `type` with the field, \
                             then run `cargo xtask codegen`."
                        );
                    }
                    emit_rewritten_run(&attrs, &mut out, file, line, ident, BOOL_FORM)?;
                    emit_type_line(body, chunk, ident, "bool", &mut out);
                }
                Decision::RequiredNullable => {
                    replay_run_without_skip(&attrs, &mut out, file, line, ident)?;
                    emit_type_line(body, chunk, ident, &format!("Option<{inner}>"), &mut out);
                }
            }
            attrs.clear();
            continue;
        }
        // Not an optional-shape field — the run and the line are the
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
             `apply_with_shapes` in `xtask/src/codegen/optional_shapes/emit.rs` \
             the new shape, then run `cargo xtask codegen`."
        );
    }
    if found != shapes.sites {
        bail!(
            "{file}: the resolved schema describes {} optional scalar / \
             structure field{} but the generated file carries {} — the counts \
             must meet exactly. The tally is the tripwire that keeps the site \
             definition honest: a field that slipped past the scanner, or a \
             member the walk miscounted, must refuse the run, not pass for \
             processed.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `apply_with_shapes` in `xtask/src/codegen/optional_shapes/emit.rs` \
             the new shape, then run `cargo xtask codegen`.",
            shapes.sites,
            if shapes.sites == 1 { "" } else { "s" },
            found
        );
    }
    Ok(out)
}

/// The decision the schema holds for one found field, or a refusal: a
/// wire name no member describes, or one described under a DIFFERENT
/// class than the payload carries — the two sides disagreeing about what
/// the field even is.
fn lookup(
    shapes: &OptionalShapes,
    wire: &str,
    class: ShapeClass,
    file: &str,
    line: usize,
    ident: &str,
    inner: &str,
) -> Result<Decision> {
    if let Some(decision) = shapes.map.get(&(wire.to_string(), class)).copied() {
        return Ok(decision);
    }
    if let Some((_, other)) = shapes.map.keys().find(|(name, _)| name == wire) {
        bail!(
            "{file}:{line}: the generated field `{ident}` carries the payload \
             `{inner}` (a {}), but the schema keys `{wire}` as a {} — the \
             class disagrees between the sides, and the pass refuses to guess \
             which one moved.\n\
             Fix: align the schema's form for `{wire}` with the generated \
             payload, then run `cargo xtask codegen`.",
            class.as_str(),
            other.as_str()
        );
    }
    bail!(
        "{file}:{line}: the generated field `{ident}` is an optional shape \
         keyed `{wire}`, which no member of the schema describes.\n\
         The pass keys every `Option<Box<…>>` field by (wire name, class) — \
         both sides carry both halves — so a field the schema does not key \
         means the emission and the schema have parted ways, and the pass \
         refuses to guess a shape for it.\n\
         Fix: restore the pinned jtd-codegen version, or teach \
         `apply_with_shapes` in `xtask/src/codegen/optional_shapes/emit.rs` the \
         new shape, then run `cargo xtask codegen`.",
    );
}

/// Read a generated file's type declarations — every `pub type <Name> =
/// <Target>;` alias and `pub struct <Name>` — in one sweep, so field
/// payloads are classified against what the file itself declares.
fn type_decls(src: &str) -> TypeDecls<'_> {
    let mut decls = TypeDecls {
        aliases: BTreeMap::new(),
        structs: BTreeSet::new(),
    };
    for chunk in src.split_inclusive('\n') {
        let text = chunk.trim_end_matches(['\r', '\n']).trim();
        if let Some(rest) = text.strip_prefix("pub type ")
            && let Some((name, target)) = rest.split_once('=')
        {
            let name = name.trim();
            let target = target.trim().strip_suffix(';').unwrap_or(target);
            if is_ascii_ident(name) && !target.is_empty() {
                decls.aliases.insert(name, target);
            }
        } else if let Some(rest) = text.strip_prefix("pub struct ")
            && let Some(name) = rest.strip_suffix(" {")
        {
            decls.structs.insert(name);
        }
    }
    decls
}

/// Classify an `Option<Box<…>>` payload the way the schema classifies
/// its form: a primitive (or a local `pub type` alias resolving to one)
/// is a scalar, a `pub struct` the file declares is a structure, and
/// anything else refuses — an optional vocabulary, an undeclared name or
/// a stranger is none of the pass's shapes.
fn classify_payload(
    ty: &str,
    decls: &TypeDecls<'_>,
    file: &str,
    line: usize,
    ident: &str,
) -> Result<ShapeClass> {
    if is_primitive(ty) {
        return Ok(ShapeClass::Scalar);
    }
    if !is_ascii_ident(ty) {
        bail!(
            "{file}:{line}: the optional field `{ident}` carries the payload \
             type `{ty}` — neither a scalar nor a declared name, so the pass \
             has no class for it and refuses to guess.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `apply_with_shapes` in `xtask/src/codegen/optional_shapes/emit.rs` \
             the new shape, then run `cargo xtask codegen`."
        );
    }
    if let Some(mut target) = decls.aliases.get(ty).copied() {
        let mut route: Vec<&str> = vec![ty];
        loop {
            if is_primitive(target) {
                return Ok(ShapeClass::Scalar);
            }
            let Some(next) = decls.aliases.get(target).copied() else {
                bail!(
                    "{file}:{line}: the alias `{ty}` resolves to `{target}`, \
                     which is no primitive — the pass has no class for it and \
                     refuses to guess.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `apply_with_shapes` in \
                     `xtask/src/codegen/optional_shapes/emit.rs` the new \
                     shape, then run `cargo xtask codegen`."
                );
            };
            if route.contains(&target) {
                bail!(
                    "{file}:{line}: the type aliases at `{ty}` loop back \
                     through `{target}` — no alias chain may resolve through \
                     itself.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `apply_with_shapes` in \
                     `xtask/src/codegen/optional_shapes/emit.rs` the new \
                     shape, then run `cargo xtask codegen`."
                );
            }
            route.push(target);
            target = next;
        }
    }
    if decls.structs.contains(ty) {
        return Ok(ShapeClass::Structure);
    }
    bail!(
        "{file}:{line}: the optional field `{ident}` carries the payload \
         type `{ty}`, which this file declares as neither a `pub type` alias \
         nor a `pub struct` — an optional vocabulary or a stranger, and the \
         pass has no shape rule for either.\n\
         Fix: give the member a scalar or structure form, or teach \
         `optional_shapes.rs` this one, then run `cargo xtask codegen`."
    );
}

/// The primitive payload types the pinned emission spells — the JTD
/// `type` forms as jtd-codegen renders them.
///
/// The list is meant to be COMPLETE over those forms, and it had
/// exactly one hole: JTD's `timestamp` renders as
/// `DateTime<FixedOffset>`, and until a schema needed an OPTIONAL date
/// no site ever asked. The two halves of this pass disagreed precisely
/// there — the schema side classifies a `timestamp` member as a scalar
/// (`ShapeClass::Scalar`), while this side had no class for its Rust
/// spelling and refused rather than guess. A required date never
/// reached here, because only an optional payload is reshaped.
/// `chrono` is the pinned emission's spelling, not a choice of ours:
/// the domain-types pass rewrites the alias afterwards (pass order is
/// the law recorded in `postproc.rs`), so what this pass sees is always
/// the raw `DateTime<FixedOffset>`.
fn is_primitive(ty: &str) -> bool {
    matches!(
        ty,
        "String"
            | "bool"
            | "char"
            | "DateTime<FixedOffset>"
            | "f32"
            | "f64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "isize"
            | "usize"
    )
}

/// The payload inside an `Option<Box<…>>` type — exactly as the
/// reshaped field line spells it. Every other type yields `None` and
/// rides through untouched.
fn boxed_payload(ty: &str) -> Option<&str> {
    ty.strip_prefix("Option<Box<")
        .and_then(|rest| rest.strip_suffix(">>"))
}

/// Emit one reshaped field's attribute run: the `Option::is_none` skip
/// attribute becomes the given form, and any surviving rename rides
/// along untouched. The pinned emission writes exactly one skip
/// attribute over every `Option` field; a run without it, or carrying an
/// attribute that is neither it nor a rename, is a moved pin and refuses.
fn emit_rewritten_run(
    attrs: &[(usize, &str, &str)],
    out: &mut String,
    file: &str,
    line: usize,
    ident: &str,
    form: &str,
) -> Result<()> {
    let mut saw_skip = false;
    for (_, attr_chunk, attr_text) in attrs {
        let attr_body = attr_chunk.trim_end_matches(['\r', '\n']);
        let indent = &attr_body[..attr_body.len() - attr_body.trim_start().len()];
        let ending = &attr_chunk[attr_body.len()..];
        if *attr_text == OPTION_SKIP {
            saw_skip = true;
            out.push_str(indent);
            out.push_str(form);
            out.push_str(ending);
        } else if rename_wire(attr_text).is_some() {
            out.push_str(attr_chunk);
        } else {
            bail!(
                "{file}:{line}: the optional field `{ident}` carries an \
                 attribute this pass does not recognise:\n\
                 `{attr_text}`\n\
                 The pinned jtd-codegen emission writes only the \
                 `Option::is_none` skip and a `rename` over a field, so an \
                 unfamiliar attribute means the emission shape has moved, and \
                 the pass refuses to guess what it labels.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `optional_shapes/emit.rs` the new shape, then run `cargo \
                 xtask codegen`."
            );
        }
    }
    if !saw_skip {
        bail!(
            "{file}:{line}: the optional field `{ident}` carries no \
             `{OPTION_SKIP}` — jtd-codegen writes it over every `Option` field \
             (the emission shape is pinned by the generator's version), so \
             this file is not that shape.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `optional_shapes/emit.rs` the new shape, then run `cargo xtask \
             codegen`."
        );
    }
    Ok(())
}

/// Replay a required-nullable field's attribute run byte for byte — the
/// pinned emission writes no skip over it (`None` serialises as `null`,
/// which IS the wire), so a run that carries one, or any attribute but a
/// rename, is a moved pin and refuses.
fn replay_run_without_skip(
    attrs: &[(usize, &str, &str)],
    out: &mut String,
    file: &str,
    line: usize,
    ident: &str,
) -> Result<()> {
    for (_, attr_chunk, attr_text) in attrs {
        if *attr_text == OPTION_SKIP {
            bail!(
                "{file}:{line}: the required-nullable field `{ident}` carries \
                 a `{OPTION_SKIP}` the pinned emission does not write over it \
                 — the required form's wire writes `null`, never an absent \
                 key, and a skip attribute would silently change that.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `optional_shapes/emit.rs` the new shape, then run `cargo \
                 xtask codegen`."
            );
        }
        if rename_wire(attr_text).is_some() {
            out.push_str(attr_chunk);
        } else {
            bail!(
                "{file}:{line}: the required-nullable field `{ident}` carries \
                 an attribute this pass does not recognise:\n\
                 `{attr_text}`\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `optional_shapes/emit.rs` the new shape, then run `cargo \
                 xtask codegen`."
            );
        }
    }
    Ok(())
}

/// Replay a buffered attribute run byte for byte.
fn replay_attrs(attrs: &[(usize, &str, &str)], out: &mut String) {
    for (_, attr_chunk, _) in attrs {
        out.push_str(attr_chunk);
    }
}

/// Emit one reshaped field line: the indent and the identifier are the
/// generator's, byte for byte; only the type moves.
fn emit_type_line(body: &str, chunk: &str, ident: &str, ty: &str, out: &mut String) {
    let indent = &body[..body.len() - body.trim_start().len()];
    let ending = &chunk[body.len()..];
    out.push_str(indent);
    out.push_str("pub ");
    out.push_str(ident);
    out.push_str(": ");
    out.push_str(ty);
    out.push(',');
    out.push_str(ending);
}

/// Split a `pub <ident>: <type>,` field line into its identifier and
/// type — the same pinned emission shape the sibling passes key on,
/// which has already refused anything malformed by the time this pass
/// runs. Anything else (a `pub struct` / `pub enum` / `pub type` line, a
/// variant, a brace, a comment) yields `None`.
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
/// `None` for any other attribute — the twin of `rename_wire` in the
/// sibling passes, local here because it polices this pass's own surface
/// (the wire half of the stitch key).
fn rename_wire(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("#[serde(rename = \"")?;
    let end = rest.find("\"]")?;
    Some(&rest[..end])
}

/// ASCII identifier shape — the same contract the sibling passes
/// enforce, local here because it polices this pass's own surface (a
/// field identifier or type name the pass is about to rewrite around).
fn is_ascii_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
