//! The snake_case field pass — the second content edit the generator's
//! emission takes (the order rule lives in `postproc`'s docs: a pass
//! keyed to the emission shape runs while the file is STILL that
//! emission, and this one is keyed to the attribute-run-plus-field-line
//! shape, so it runs after arm boxing and before the vocabularies open).
//!
//! jtd-codegen turns a JTD property `content_hash` into a Rust field
//! `contentHash` and papers over the gap with
//! `#[serde(rename = "content_hash")]`. The Rust side is ours to name,
//! and the house naming is snake_case — so the pass renames every
//! struct field identifier to its snake_case form, then removes the
//! `#[serde(rename = …)]` that would merely repeat the new identifier's
//! identity. Only that one: a rename whose wire string DIFFERS from
//! `snake_case(identifier)` carries information — the schema declared a
//! camelCase property ON THE WIRE — and it stays.
//!
//! The invariant, stated in full because it is what makes the pass safe
//! by construction: **the wire does not move in either branch.** Either
//! the identifier equals the wire string and serde takes it by default,
//! or the rename is kept. There is no third branch.
//!
//! The second branch is not hypothetical, and the way that was learned
//! is worth the four lines. The measurement this pass was cut from said
//! the branch had zero sites — all 309 field renames repeat their
//! identifier — and the script behind that claim could not fire at all
//! (it anchored the end of the line right after the field's colon, which
//! no emitted field has). One site exists: `registry_sync_report`
//! declares a property named `ref`, a Rust keyword, so the generator
//! escapes the identifier to `ref_`, `snake_case("ref_")` is `"ref_"`,
//! and the rename is the only thing carrying `"ref"` to the wire.
//! Dropping it would have moved that format's bytes silently — it has no
//! oracle. A schema property that is a Rust keyword is a permanent
//! class, not an accident of this tree; so is a property spelled
//! camelCase ON THE WIRE. The rule is written for the schema's rights,
//! never for a slice of today, and here that is what saved the pass from
//! its own measurement.
//!
//! Enum VARIANTS are skipped, and the skip is a named rule, not an
//! accident: a variant identifier is PascalCase and correct as it is
//! (the `non_snake_case` lint does not apply to variants), and its wire
//! string (`kind-name`, `lazy-pull`) is not derivable from any case
//! rule. The pass tells the two apart by the form of the line that
//! follows the attribute run: a field line starts with `pub `, a
//! variant line does not — the same emission-form key the whole layer
//! is pinned to.
//!
//! Like its siblings, the pass never reads its own output: every codegen
//! run wipes the tree and regenerates, so what it sees is always fresh
//! emission. (A second run over its own output would refuse loudly on
//! the now-rename-less fields — the honest signal, not a defect: that
//! text is no longer the generator's.)

use anyhow::{Result, bail};

/// Rewrite every struct field identifier in `src` to snake_case, dropping
/// the per-field `#[serde(rename = …)]` exactly when it would repeat the
/// new identifier (the full contract is the module's docs). The pass
/// buffers each run of consecutive attribute lines and decides by the
/// line that follows it: a field line is rewritten, anything else — a
/// variant, `pub struct` / `pub enum`, a blank, a doc comment — is copied
/// through byte for byte together with its run. A line that looks like
/// the pinned shape but is not (a field without exactly one rename, a
/// non-ASCII identifier, a file ending mid-run) refuses, naming the file
/// and the line, rather than being rewritten on a guess.
pub(crate) fn snake_case_fields(src: &str, file: &str) -> Result<String> {
    let mut out = String::with_capacity(src.len() + src.len() / 8);
    // A buffered run of consecutive attribute lines — `(line, chunk,
    // trimmed)` — held until the line after it says what they annotate.
    let mut attrs: Vec<(usize, &str, &str)> = Vec::new();
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

        match line_form(text) {
            LineForm::Field(ident) => {
                // Exactly one rename per field is the pinned shape; zero
                // or two is a moved pin, and each refuses with its own
                // recipe.
                let wires: Vec<&str> = attrs
                    .iter()
                    .filter_map(|(_, _, t)| rename_wire(t))
                    .collect();
                let wire = match wires.as_slice() {
                    [wire] => *wire,
                    [] => bail!(
                        "{file}:{line}: the field `{ident}` carries no \
                         `#[serde(rename = …)]` in its attribute run — every \
                         field of the pinned jtd-codegen emission has exactly \
                         one, so a field without one means the emission shape \
                         has moved, and the pass refuses to guess which name \
                         is the wire's.\n\
                         Fix: restore the pinned jtd-codegen version, or \
                         teach `snake_case_fields` in \
                         `xtask/src/codegen/snake_case.rs` the new shape, \
                         then run `cargo xtask codegen`."
                    ),
                    _ => bail!(
                        "{file}:{line}: the field `{ident}` carries {} \
                         `#[serde(rename = …)]` attributes in one run — the \
                         pinned emission writes exactly one per field, and \
                         two names cannot both be the wire.\n\
                         Fix: restore the pinned jtd-codegen version, or \
                         teach `snake_case_fields` in \
                         `xtask/src/codegen/snake_case.rs` the new shape, \
                         then run `cargo xtask codegen`.",
                        wires.len()
                    ),
                };
                // The invariant's two branches (module docs): drop the
                // rename only where it would repeat the identifier.
                let snake = to_snake_case(ident);
                let rename_repeats_identity = snake == wire;
                for (_, attr_chunk, attr_text) in &attrs {
                    if rename_repeats_identity && rename_wire(attr_text).is_some() {
                        continue;
                    }
                    out.push_str(attr_chunk);
                }
                // Rebuild the field line: the indent and everything after
                // the identifier are the generator's, byte for byte.
                let indent = &body[..body.len() - body.trim_start().len()];
                let tail = &text["pub ".len() + ident.len()..];
                out.push_str(indent);
                out.push_str("pub ");
                out.push_str(&snake);
                out.push_str(tail);
                out.push_str(&chunk[body.len()..]);
            }
            LineForm::MalformedField(ident) => {
                bail!(
                    "{file}:{line}: the field identifier `{ident}` is not an \
                     ASCII identifier — the pinned jtd-codegen emission only \
                     emits ASCII field names, so this is not the shape the \
                     pass is pinned to.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `snake_case_fields` in `xtask/src/codegen/snake_case.rs` \
                     the new shape, then run `cargo xtask codegen`."
                );
            }
            LineForm::Other => {
                // Not a field line — the run and the line are the
                // generator's, not ours.
                for (_, attr_chunk, _) in &attrs {
                    out.push_str(attr_chunk);
                }
                out.push_str(chunk);
            }
        }
        attrs.clear();
    }
    if let Some((opened_at, _, _)) = attrs.first() {
        bail!(
            "{file}: the file ends inside an attribute run opened at line \
             {opened_at} — jtd-codegen never ends a file mid-attribute, so \
             the file this pass read is not the shape it is pinned to.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `snake_case_fields` in `xtask/src/codegen/snake_case.rs` the \
             new shape, then run `cargo xtask codegen`."
        );
    }
    Ok(out)
}

/// What a decision line is, as far as this pass's decisions go.
enum LineForm<'a> {
    /// `pub <ident>: <type>,` with a well-formed ASCII identifier.
    Field(&'a str),
    /// Starts like a field line (`pub …: …,`) but the identifier slot is
    /// not a single ASCII identifier — a moved pin, not a pass-through.
    MalformedField(&'a str),
    /// Anything else — the pass has no decision to make about it.
    Other,
}

/// Classify a line by the emission forms the pass is pinned to. A field
/// line carries its colon right after one identifier and ends the line
/// with the comma of a struct member; `pub struct` / `pub enum` /
/// `pub type` never match that shape.
fn line_form(text: &str) -> LineForm<'_> {
    let Some(rest) = text.strip_prefix("pub ") else {
        return LineForm::Other;
    };
    let Some(colon) = rest.find(':') else {
        return LineForm::Other;
    };
    if !text.ends_with(',') {
        return LineForm::Other;
    }
    let ident = &rest[..colon];
    if is_ascii_ident(ident) {
        LineForm::Field(ident)
    } else {
        LineForm::MalformedField(ident)
    }
}

/// The wire string of a `#[serde(rename = "…")]` attribute line, or
/// `None` for any other attribute. Anchoring the match on
/// `#[serde(rename = "` and the value's closing `")]` keeps
/// `rename_all` (a different attribute the emission never writes for a
/// field) and mangled spacing out of the recognition.
fn rename_wire(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("#[serde(rename = \"")?;
    let end = rest.find("\")]")?;
    Some(&rest[..end])
}

/// `contentHash` → `content_hash`, `filesCount` → `files_count`, `oneOf`
/// → `one_of`, `path` → `path`: a `_` before every capital except the
/// first, everything lowercased. The caller has already checked the
/// identifier is ASCII, so the fold is exact.
fn to_snake_case(ident: &str) -> String {
    let mut out = String::with_capacity(ident.len() + ident.len() / 4);
    for (index, c) in ident.chars().enumerate() {
        if index > 0 && c.is_ascii_uppercase() {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// ASCII identifier shape — the twin of `is_ident` in `postproc.rs`,
/// local here because it polices this pass's own surface (a field
/// identifier the pass is about to rewrite around).
fn is_ascii_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
#[path = "snake_case/tests.rs"]
mod tests;
