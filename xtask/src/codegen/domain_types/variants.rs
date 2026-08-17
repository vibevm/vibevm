//! Variant naming — the third arm of the domain-types pass, split off
//! its parent along the question it answers: which identifier did the
//! generator mint for a given wire value, and which one does the schema
//! want instead.
//!
//! The parent keeps the rewriting of declarations, aliases and imports;
//! `rulings` keeps what the schema declares; this file is the join
//! between a wire value and the identifier standing on it. The lookup
//! runs one way only, and that direction is R16's law rather than a
//! convenience: the schema says `"kind/name"` should be
//! `KindSlashName`, the emission says `"kind/name"` is currently a
//! collision-suffixed name, and the two are joined on the value both
//! carry verbatim. Reasoning from the minted identifier would mean
//! re-deriving a PascalCase rule plus a suffix — both of them the
//! generator's business, and precisely what this layer exists to stay
//! independent of.
//!
//! Why the rewrite is scoped to the declaration, unlike the type-name
//! arm which is file-wide. A TYPE's identifier is unique in its file by
//! construction, so every whole-token occurrence of it means that type.
//! A VARIANT's identifier is scoped to its enum: the same token
//! elsewhere in the file is a different thing entirely — another type,
//! another enum's variant — and renaming it would be the pass silently
//! editing something nobody asked about. So this arm walks the
//! declaration's own body and stops at its closing brace.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};

use super::is_ident;
use super::rulings::{Keyword, Ruling, declared_in};

/// Apply every ruling's `x-rust-variants` inside its own declaration,
/// returning the rewritten source.
///
/// Line count is preserved — each rewrite swaps an identifier within
/// one line — so the line indices the caller collected before this runs
/// stay valid after it.
pub(super) fn rename_variants(
    src: &str,
    file: &str,
    schema: &Path,
    rulings: &[Ruling],
    decls: &BTreeMap<&str, (usize, Keyword)>,
) -> Result<String> {
    let mut renames: BTreeMap<usize, BTreeMap<String, String>> = BTreeMap::new();
    for ruling in rulings {
        if ruling.variants.is_empty() {
            continue;
        }
        let Some(&(line, _)) = decls.get(ruling.emitted.as_str()) else {
            bail!(
                "schema {}: the definition `{}` names identifiers for its \
                 variants, but the generated file declares no `{}` for them \
                 to belong to. The emission shape of jtd-codegen this pass \
                 is pinned to has moved.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `domain_types/variants.rs` the new shape, then run \
                 `cargo xtask codegen`.",
                declared_in(schema),
                ruling.definition,
                ruling.emitted
            );
        };
        let minted = minted_variants(src, line);
        let mut chosen: BTreeMap<String, String> = BTreeMap::new();
        for (wire, want) in &ruling.variants {
            let Some(current) = minted.get(wire.as_str()) else {
                let mut carried: Vec<&str> = minted.keys().copied().collect();
                carried.sort_unstable();
                bail!(
                    "{file}: `{}` carries no variant for the wire value \
                     `{wire}`, which schema {} names `{want}`. The variants \
                     it does carry: {}.\n\
                     The schema side already checked that this value belongs \
                     to the definition, so the two documents disagree about \
                     the EMISSION rather than about the vocabulary.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `domain_types/variants.rs` the new shape, then run \
                     `cargo xtask codegen`.",
                    ruling.emitted,
                    declared_in(schema),
                    carried.join(", ")
                );
            };
            if current == want {
                // The generator already minted the chosen name: a no-op,
                // and the file must not move by a byte because a schema
                // agreed with it.
                continue;
            }
            chosen.insert((*current).to_string(), want.clone());
        }
        if !chosen.is_empty() {
            renames.insert(line, chosen);
        }
    }
    if renames.is_empty() {
        return Ok(src.to_string());
    }
    Ok(rewrite_bodies(src, &renames))
}

/// Walk the file once, swapping variant identifiers inside the bodies
/// the caller mapped and copying every other byte through.
fn rewrite_bodies(src: &str, renames: &BTreeMap<usize, BTreeMap<String, String>>) -> String {
    let mut out = String::with_capacity(src.len() + src.len() / 16);
    // The rename table of the declaration currently open, if any.
    let mut inside: Option<&BTreeMap<String, String>> = None;
    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        if let Some(table) = renames.get(&index) {
            inside = Some(table);
            out.push_str(chunk);
            continue;
        }
        let Some(table) = inside else {
            out.push_str(chunk);
            continue;
        };
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();
        if text == "}" {
            inside = None;
            out.push_str(chunk);
            continue;
        }
        // Only a variant line carries an identifier this arm may touch;
        // an attribute line holds the wire string, which never moves.
        let ident = text
            .split_once('(')
            .map(|(head, _)| head)
            .unwrap_or_else(|| text.trim_end_matches(','));
        match table.get(ident) {
            Some(want) if is_ident(ident) => {
                let indent = &body[..body.len() - text.len()];
                out.push_str(indent);
                out.push_str(want);
                out.push_str(&text[ident.len()..]);
                out.push_str(&chunk[body.len()..]);
            }
            _ => out.push_str(chunk),
        }
    }
    out
}

/// Read `wire value → minted identifier` out of the declaration opened
/// at `line`, walking to its closing brace.
///
/// Both variant shapes the pinned emission writes are read the same
/// way, because both wear their rename on the line above: a unit
/// variant (`Ident,`) and a boxed union arm (`Ident(Box<Payload>),`).
fn minted_variants(src: &str, line: usize) -> BTreeMap<&str, &str> {
    let mut found: BTreeMap<&str, &str> = BTreeMap::new();
    let mut wire: Option<&str> = None;
    for chunk in src.split_inclusive('\n').skip(line + 1) {
        let text = chunk.trim_end_matches(['\r', '\n']).trim();
        if text == "}" {
            break;
        }
        if let Some(rest) = text.strip_prefix("#[serde(rename = \"") {
            wire = rest.strip_suffix("\")]");
            continue;
        }
        if let Some(value) = wire.take() {
            let ident = text
                .split_once('(')
                .map(|(head, _)| head)
                .unwrap_or_else(|| text.trim_end_matches(','));
            if is_ident(ident) {
                found.insert(value, ident);
            }
        }
    }
    found
}

#[cfg(test)]
#[path = "variants/tests.rs"]
mod tests;
