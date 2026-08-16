//! Post-processing passes over jtd-codegen output — content edits the
//! generator's emission gets before anything else reads the file.
//!
//! This file is the driver plus the first pass; the other three —
//! renaming field identifiers to snake_case (dropping the identity
//! renames), turning wire maps into ordered `BTreeMap`s, and opening
//! vocabularies per the schema's `x-vocabulary` — live in the sibling
//! `snake_case`, `ordered_maps`, and `open_vocabulary` modules, split
//! along those responsibility seams as the set outgrew the 600-line
//! budget.
//!
//! The passes run in a fixed ORDER, and the order is a rule, not a
//! taste: a pass keyed to the generator's emission shape must run while
//! the file is STILL that emission. Boxing is keyed to the shape
//! (derive, tag attribute, `pub enum`), so it runs first. The snake_case
//! pass is keyed to the shape too (an attribute run resolved against a
//! `pub <ident>: …` field line), so it runs second. The ordered-maps
//! pass is keyed to the shape as well (the `use std::collections::…`
//! import line plus `HashMap<…>` type positions), so it runs third.
//! Opening vocabularies then writes hand-rolled `impl Serialize` /
//! `impl Deserialize` blocks into the file — text the pinned emission
//! shape does not contain — and any shape-keyed pass running after it
//! would be reading a document that is no longer the generator's. The
//! passes also never read their own output: `generate_into` wipes and
//! regenerates the tree before every run, so what they see is always
//! fresh generator emission.
//!
//! Why boxing belongs to the generator and not to the schema or a
//! suppression: the journal schema this phase writes next carries an
//! eleven-variant union whose `published` arm tows the whole catalog
//! record (thirty-three fields), and a measured run on a real fragment
//! of that record makes the generated union trip
//! `clippy::large_enum_variant` — clippy itself names the fix,
//! `Published(Box<…>)`. The panel runs `clippy --workspace --all-targets
//! -D warnings`; generated files are never hand-edited, and this lint has
//! no suppression anywhere in the tree — it has always been answered with
//! a deliberate `Box`, and it caught a real defect in phase 3.
//!
//! The rule is uniform, not sized: EVERY arm of a `#[serde(tag = …)]`
//! union is boxed. A size threshold is a heuristic that moves with the
//! linter's version — a gate whose verdict depends on the tool version is
//! not a gate. A schema annotation would be worse: the schema describes
//! the wire (`x-vocabulary`, `x-empty`, `x-default`, `x-rust-type` all
//! speak about the format), while boxing decides a Rust memory-layout
//! question the compiler owns. The price, named honestly: one allocation
//! per constructed variant — nothing for types that are parsed once and
//! projected once.
//!
//! The wire does not move, and that is checkable, not hoped: serde
//! renders `Box<T>` exactly as `T`, so the wire-parity oracles stay green
//! across the change (Rust changed, bytes did not). The pass is also
//! idempotent — an already-boxed arm is recognised and left alone — so
//! `check-codegen` cannot oscillate between two forms of the same file.
//!
//! Anything unfamiliar inside a recognised union is a loud refusal, not a
//! best-effort rewrite: the emission shape is pinned by the generator's
//! version (PROP-000 §16 pins the toolchain), so a changed shape means
//! the pin moved, and rewriting half a union silently would hide that
//! behind a green run.

use std::path::Path;

use anyhow::{Context, Result, bail};

use super::open_vocabulary::open_vocabularies;
use super::ordered_maps::ordered_maps;
use super::snake_case::snake_case_fields;

/// Read `file`, run all four post-processing passes over it, write the
/// result back. Called in `generate_into` right after the generator
/// succeeds — before the leaf is registered or anything compiles against
/// it, so no consumer — compiler, clippy, oracle — ever sees the
/// unprocessed form.
///
/// Pass order is a rule, not a taste: a pass keyed to the generator's
/// emission shape must run while the file is STILL that emission.
/// Boxing is keyed to the shape, so it runs first; the snake_case field
/// pass is keyed to the shape too, so it runs second; the ordered-maps
/// pass is keyed to the shape as well, so it runs third; opening
/// vocabularies then writes hand-rolled impls into the file, and a
/// shape-keyed pass running after it would be reading a document that is
/// no longer the generator's.
pub(crate) fn rewrite_generated(file: &Path, resolved: &Path, schema: &Path) -> Result<()> {
    let src = std::fs::read_to_string(file)
        .with_context(|| format!("reading generated {}", file.display()))?;
    let name = file.display().to_string();
    let boxed = box_union_arms(&src, &name)?;
    let snaked = snake_case_fields(&boxed, &name)?;
    let ordered = ordered_maps(&snaked, &name)?;
    let opened = open_vocabularies(&ordered, &name, resolved, schema)?;
    std::fs::write(file, opened).with_context(|| format!("writing the post-processed {}", name))?;
    Ok(())
}

/// The pass proper, over text, so the tests drive exactly what production
/// drives. Everything outside a recognised union is copied byte for byte:
/// structs, vocabulary enums (unit variants), enums without
/// `#[serde(tag = …)]`, type aliases, comments. A union is recognised by
/// its `#[serde(tag = …)]` attribute immediately followed by
/// `pub enum <Name> {`; inside, only three line shapes are legal — a
/// blank, a per-arm `#[serde(rename = …)]`, or an arm
/// `<Ident>(<Type>),` rewritten to `<Ident>(Box<<Type>>),` unless the
/// payload is already boxed. Anything else refuses, naming the file and
/// the line.
pub(crate) fn box_union_arms(src: &str, file: &str) -> Result<String> {
    let mut out = String::with_capacity(src.len() + src.len() / 8);
    // `Some(line of the tag attribute)` — the next line must open the enum.
    let mut pending_enum: Option<usize> = None;
    // Inside the braces of a recognised union; carries the line the enum
    // opened at, for the never-closed refusal.
    let mut in_union: Option<usize> = None;
    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        let line = index + 1;
        // `chunk` keeps its line ending (`\n`, `\r\n`, or nothing at EOF);
        // `body` is the line without it, `text` the line trimmed.
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();

        if let Some(opened_at) = pending_enum.take() {
            if !text.starts_with("pub enum ") || !text.ends_with('{') {
                bail!(
                    "{file}:{line}: a `#[serde(tag = …)]` attribute (line \
                     {opened_at}) is not followed by `pub enum … {{` — the \
                     emission shape of jtd-codegen this pass is pinned to has \
                     moved, and the pass refuses to guess which union is \
                     which.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `box_union_arms` in `xtask/src/codegen/postproc.rs` the \
                     new shape, then run `cargo xtask codegen`."
                );
            }
            in_union = Some(line);
            out.push_str(chunk);
            continue;
        }

        if let Some(opened_at) = in_union {
            if text.is_empty() || (text.starts_with("#[serde(rename") && text.ends_with(")]")) {
                // Layout and per-arm renames are the generator's, not ours.
                out.push_str(chunk);
                continue;
            }
            if text == "}" {
                in_union = None;
                out.push_str(chunk);
                continue;
            }
            if let Some((ident, payload)) = split_arm(text) {
                if is_boxed(payload) {
                    // Already boxed: the second run over this text is the
                    // identity, which is what keeps `check-codegen` stable.
                    out.push_str(chunk);
                } else {
                    let indent = &body[..body.len() - body.trim_start().len()];
                    out.push_str(indent);
                    out.push_str(ident);
                    out.push_str("(Box<");
                    out.push_str(payload);
                    out.push_str(">),");
                    out.push_str(&chunk[body.len()..]);
                }
                continue;
            }
            bail!(
                "{file}:{line}: the discriminator union opened at line \
                 {opened_at} holds a line this pass does not recognise:\n\
                 `{text}`\n\
                 The pass boxes every arm of a `#[serde(tag = …)]` union and \
                 refuses to guess past an unfamiliar line — the emission shape \
                 is pinned by the generator's version, and rewriting half a \
                 union silently would hide a moved pin behind a green run.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `box_union_arms` in `xtask/src/codegen/postproc.rs` the new \
                 shape, then run `cargo xtask codegen`."
            );
        }

        // Outside any union the pass is a copy — one tripwire: the tag
        // attribute opens a union on the next line. A doc comment that
        // merely QUOTES the attribute (as the repomd schema's does) starts
        // with `///`, not `#[`, so it never fires this.
        if text.starts_with("#[serde(tag") {
            pending_enum = Some(line);
        }
        out.push_str(chunk);
    }
    if pending_enum.is_some() || in_union.is_some() {
        bail!(
            "{file}: a discriminator union opens but never closes before \
             end of file — jtd-codegen always closes every enum it emits, so \
             the file this pass read is not the shape it is pinned to.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `box_union_arms` in `xtask/src/codegen/postproc.rs` the new \
             shape, then run `cargo xtask codegen`."
        );
    }
    Ok(out)
}

/// Split an arm of the pinned shape — `<Ident>(<Type>),` — into its
/// variant identifier and payload type, or `None` when the line is not
/// that shape (the caller refuses, loudly, rather than guess).
fn split_arm(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_suffix(',')?;
    let text = text.strip_suffix(')')?;
    let (ident, payload) = text.split_once('(')?;
    if !is_ident(ident) || payload.is_empty() {
        return None;
    }
    Some((ident, payload))
}

/// A payload already boxed — `Box<…>` — so rewriting it would nest boxes
/// and break the pass's own idempotency contract.
fn is_boxed(payload: &str) -> bool {
    payload.starts_with("Box<") && payload.ends_with('>')
}

/// ASCII identifier shape — the same contract `check_module_ident`
/// enforces for module names, local here because it polices a different
/// surface: a variant identifier the pass is about to rewrite around.
fn is_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
#[path = "postproc/tests.rs"]
mod tests;
