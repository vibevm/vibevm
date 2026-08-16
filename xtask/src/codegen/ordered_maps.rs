//! The ordered-maps pass — the third content edit the generator's
//! emission takes (the order rule lives in `postproc`'s docs: a pass
//! keyed to the emission shape runs while the file is STILL that
//! emission, and this one is keyed to the `use std::collections::…`
//! import line plus the `HashMap<…>` type positions, so it runs after
//! arm boxing and field snake_casing and before the vocabularies open).
//!
//! jtd-codegen materialises a JTD `values` form as `HashMap<…>`. The
//! format contract wants canonical ordering instead (PROP-044 §4.2
//! lists it among what OUR layer emits; §4.3: one state, one byte
//! sequence — sorted keys), so every map on our formats becomes a
//! `BTreeMap`, unconditionally.
//!
//! Why unconditional, with no schema annotation: the annotations this
//! layer already honours (`x-vocabulary`, `x-empty`, `x-default`,
//! `x-rust-type`) all say something ABOUT THE FORMAT — a property of
//! the wire's values. Which Rust map type carries them is not the
//! schema author's question: `BTreeMap` serialises its keys sorted,
//! `HashMap` serialises them in a randomised per-process order, and of
//! "one state, one byte sequence" there is exactly one lawful answer.
//! That is the same argument that boxes union arms without asking the
//! schema — a Rust representation question the layer answers itself —
//! and the hand-written twins already rule this way
//! (`vibe-index/src/types/entry/content.rs`, `types/repomd.rs`): the
//! pass converges the generated side onto what the tree holds correct.
//!
//! The wire VALUE does not move — key order is invisible to
//! `serde_json::Value` equality — so the five wire-parity oracles stay
//! green across this change, and green there is not this pass's proof
//! (those oracles are blind to key order by construction). The proof
//! lives in `crates/vibe-wire/tests/canonical_order.rs`: a generated
//! field compiled against `&BTreeMap<…>` (the compiler refusal is the
//! red form) plus an ascending-keys assertion on the serialised bytes.
//!
//! Like its siblings, the pass never reads its own output: every
//! codegen run wipes the tree and regenerates, so what it sees is
//! always fresh emission. (A second run over its own output is the
//! identity — no `HashMap` is left to see.)

use anyhow::{Result, bail};

/// Rewrite every `HashMap` the pinned emission writes into its ordered
/// twin: a line that is exactly `use std::collections::HashMap;` becomes
/// the `BTreeMap` import, and every `HashMap<` in a non-comment line
/// becomes `BTreeMap<`. Comment lines (`//`, `///`, `//!`) are copied
/// byte for byte — a comment may legitimately say `HashMap` as a word.
/// Two consistency contracts refuse loudly rather than guess: a
/// non-comment `HashMap` word that `<` does not immediately follow (the
/// emission has found a position for the type this pass does not know),
/// and a file whose import line and map usages disagree (the generator
/// emits both together or neither).
pub(crate) fn ordered_maps(src: &str, file: &str) -> Result<String> {
    let mut out = String::with_capacity(src.len());
    let mut saw_import = false;
    let mut saw_usage = false;
    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        let line = index + 1;
        // `chunk` keeps its line ending (`\n`, `\r\n`, or nothing at EOF);
        // `body` is the line without it, `text` the line trimmed.
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();

        // Comment lines are not the generator's code; the pass does not
        // read them, whether or not they say `HashMap`.
        if text.starts_with("//") {
            out.push_str(chunk);
            continue;
        }

        // The one import line, recognised whole so a partial lookalike
        // cannot slip past the stray-word tripwire below.
        if text == "use std::collections::HashMap;" {
            saw_import = true;
            let indent = &body[..body.len() - body.trim_start().len()];
            out.push_str(indent);
            out.push_str("use std::collections::BTreeMap;");
            out.push_str(&chunk[body.len()..]);
            continue;
        }

        // A `HashMap` word that `<` does not immediately follow is a
        // position the pinned emission never writes — refuse rather
        // than rewrite around a shape this pass was not told about.
        if find_stray_hash_map(body).is_some() {
            bail!(
                "{file}:{line}: the word `HashMap` appears without `<` \
                 following it, on this line:\n\
                 `{text}`\n\
                 The pinned jtd-codegen emission writes the type only as \
                 `HashMap<…>` (plus the one import line), so a bare word \
                 means the emission shape has moved, and the pass refuses \
                 to guess which occurrence is a map.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `ordered_maps` in `xtask/src/codegen/ordered_maps.rs` the \
                 new shape, then run `cargo xtask codegen`."
            );
        }

        if body.contains("HashMap<") {
            saw_usage = true;
            out.push_str(&body.replace("HashMap<", "BTreeMap<"));
            out.push_str(&chunk[body.len()..]);
        } else {
            out.push_str(chunk);
        }
    }
    match (saw_import, saw_usage) {
        // No map at all, or the pair rewrote together — the only two
        // states the pinned generator emits.
        (false, false) | (true, true) => Ok(out),
        (false, true) => bail!(
            "{file}: the file writes `HashMap<` but carries no \
             `use std::collections::HashMap;` line — the pinned generator \
             emits the import with the first map type, so import and usage \
             out of step means the emission shape has moved, and the pass \
             refuses to rewrite types it cannot see whole.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `ordered_maps` in `xtask/src/codegen/ordered_maps.rs` the new \
             shape, then run `cargo xtask codegen`."
        ),
        (true, false) => bail!(
            "{file}: the file carries `use std::collections::HashMap;` but \
             never writes `HashMap<` — the pinned generator emits the import \
             only alongside a map type, so an orphaned import means the \
             emission shape has moved, and the pass refuses to leave a dead \
             import behind a guess.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `ordered_maps` in `xtask/src/codegen/ordered_maps.rs` the new \
             shape, then run `cargo xtask codegen`."
        ),
    }
}

/// The offset of the first `HashMap` standing as its own word (not glued
/// into a longer identifier) that `<` does not immediately follow, or
/// `None`. The caller treats a hit as a moved pin: the pinned emission
/// writes `HashMap` only as the map type `HashMap<…>` or the import
/// line, and the import is matched exactly before this runs.
fn find_stray_hash_map(text: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = text[from..].find("HashMap") {
        let at = from + found;
        from = at + "HashMap".len();
        let glued_left = text[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if !glued_left && !text[from..].starts_with('<') {
            return Some(at);
        }
    }
    None
}

#[cfg(test)]
#[path = "ordered_maps/tests.rs"]
mod tests;
