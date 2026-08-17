//! The trait-floor pass — the ninth content edit the generator's
//! emission takes, and the one whose output another pass has to read.
//!
//! What it enforces: every generated type carries the same floor of
//! derived traits, `Debug, Clone, PartialEq, Eq`, beside the
//! `Serialize, Deserialize` jtd-codegen already writes. The generator's
//! own emission carries the serde pair and nothing else — 74 sites out
//! of 74, measured — while the hand-written types it is about to
//! replace carry the floor everywhere. A generated type without it
//! cannot appear in a failure message, cannot be cloned, and cannot be
//! compared, so a re-export of it is a downgrade wearing automation:
//! the same defect O3 caught one storey down, where the FIELD types
//! would have been lowered to strings.
//!
//! Why it is unconditional rather than a tenth annotation. The
//! discriminating question the campaign uses is whether the thing says
//! anything about the FORMAT. `Debug`, `Clone`, `PartialEq` and `Eq`
//! are properties of the Rust representation; the wire knows nothing of
//! them and no reader's behaviour turns on them. That puts the floor in
//! the same class as boxing a union's arms and ordering a map's keys —
//! one lawful answer, so the layer emits it rather than asking a schema
//! author to repeat it once per definition.
//!
//! Why `Default` is NOT in the floor, which is the interesting half.
//! "Does this type have a meaningful empty value" is a judgement about
//! the type rather than a fact about its form: an empty `ProvidesEntry`
//! means it provides nothing, an empty `VersionEntry` means nothing at
//! all — it has twenty-odd required members. The generator emits form
//! and never behaviour (the rule that kept `is_empty` predicates out of
//! it), so `Default` belongs to the hand-written impls beside this
//! tree.
//!
//! Why `Eq` is lawful today, said with its expiry rather than assumed:
//! every schema in the tree carries zero float types, so every field
//! bottoms out in `String`, `bool`, an integer, a collection, or one of
//! the three domain types — each of which is `Eq`. The first float in
//! any schema takes that away, and the step that introduces one decides
//! what to do; building reachability analysis for a case the tree
//! deliberately avoids would be machinery for an absent consumer.
//!
//! Where it sits in the order, and why the slot is forced rather than
//! chosen. Two constraints pin it. The strictness pass anchors on the
//! PRISTINE derive line to place its attribute, so the floor must land
//! after it. The vocabulary pass reads that same line and decides what
//! to keep when it takes the serde derive off an opened enum, so the
//! floor must land before it. Between those two there is exactly one
//! slot, and this pass occupies it.

use anyhow::{Result, bail};

/// The line the pinned emission derives every generated type with —
/// the same literal the strictness and vocabulary passes anchor on,
/// stated here because this pass is the one that replaces it.
const PRISTINE: &str = "#[derive(Serialize, Deserialize)]";

/// The floor, spelled in the house order the hand-written types use
/// (`crates/vibe-index/src/types/**`), so a reader moving between the
/// two halves of the tree does not have to notice a difference that
/// carries no meaning.
///
/// `pub(super)` because the vocabulary pass runs directly after this one
/// and anchors on the line this pass leaves behind — one home for the
/// literal, so the two cannot drift apart into a scanner that finds
/// nothing.
pub(super) const WITH_FLOOR: &str =
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]";

/// The floor without the serde pair — what a type keeps when a later
/// pass has to take the derived `Serialize`/`Deserialize` away and
/// hand-roll them instead (an opened vocabulary is the only case).
///
/// It exists so that taking the serde derive off does not take the
/// whole floor with it: a pass removes exactly what it must, the same
/// rule the domain-types pass follows when a substitution orphans an
/// import item.
pub(super) const WITHOUT_SERDE: &str = "#[derive(Debug, Clone, PartialEq, Eq)]";

/// Rewrite every derive line of the emission to carry the floor.
///
/// Everything else is copied byte for byte — declarations, attributes,
/// bodies, layout, line endings. The pass is idempotent: a line already
/// wearing the floor is left alone, so `check-codegen` cannot oscillate
/// between two forms of one file.
pub(super) fn apply_derive_floor(src: &str, file: &str) -> Result<String> {
    let mut out = String::with_capacity(src.len() + src.len() / 16);
    for chunk in src.split_inclusive('\n') {
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();
        if text == PRISTINE {
            let indent = &body[..body.len() - text.len()];
            out.push_str(indent);
            out.push_str(WITH_FLOOR);
            out.push_str(&chunk[body.len()..]);
            continue;
        }
        out.push_str(chunk);
    }

    // The post-condition is the tripwire: a derive line the walk did not
    // reach would leave a type without the floor, and the re-export that
    // consumes it would fail somewhere else entirely, on a symptom.
    if out.contains(PRISTINE) {
        bail!(
            "{file}: a `{PRISTINE}` line survived the trait-floor pass, so \
             at least one generated type would reach the re-export without \
             `Debug`/`Clone`/`PartialEq`/`Eq`.\n\
             The pass rewrites that exact line and nothing else, so a \
             survivor means the emission wrote it in a shape this pass does \
             not match — indentation inside a nested item, a differently \
             spelled derive list — and the pinned emission shape has moved.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `apply_derive_floor` in `xtask/src/codegen/derive_floor.rs` the \
             new shape, then run `cargo xtask codegen`."
        );
    }
    Ok(out)
}

#[cfg(test)]
#[path = "derive_floor/tests.rs"]
mod tests;
