//! The shared module — one home for the vocabulary fragments every
//! schema pulls in, emitted ONCE, with the schema modules re-exporting
//! its types instead of redeclaring them.
//!
//! Why this exists: JTD has no cross-file reference, so the vocabulary
//! layer *substitutes* each fragment into every schema that names it —
//! on the INPUT side. The generator, reading N resolved schemas that
//! share a fragment, emits that fragment's type N times, one per
//! module; for the wire that is harmless, for Rust it is not:
//! `a::VersionEntry` and `b::VersionEntry` are distinct types, and a
//! value of one cannot stand where the other is expected. Measured
//! before this module existed: 102 type declarations across the
//! generated tree against 58 unique names — 44 redundant copies, every
//! copy byte-identical to its siblings. The next campaign step (the
//! hand-written index types becoming re-exports of these) cannot even
//! be *expressed* while one name denotes several types, which is why
//! this step precedes it.
//!
//! The mechanism, in the three phases the driver runs for the host
//! home (the engine home gets none of it — our wire policy has no
//! standing over a vendored package's public Rust API):
//!
//! 1. **The map.** `Vocabularies::resolve` hands back the closure it
//!    placed beside each resolved copy, so the run knows which
//!    fragments each schema's module carries.
//! 2. **The shared module.** A synthetic JTD document whose
//!    `definitions` are every fragment of the home goes through the
//!    same generator (`emit`, below), takes the same post-processing
//!    passes, and loses its one parasitic emission — the root alias a
//!    definitions-only document yields.
//! 3. **The replacement** (`rewire`). Each schema module's copy of a
//!    fragment block — doc comments, attributes, declaration, body,
//!    and the impl blocks the vocabulary-opening pass wrote into it —
//!    is swapped for `pub use crate::generated::shared::<Type>;`, in
//!    place, in declaration order.
//!
//! The stitch is by CONTENT, never by name alone: a block is replaced
//! only after it is proven byte-identical to its same-named block in
//! the shared module. The type's name is the generator's to mint
//! (the layer folds `version_entry` → `VersionEntry` only to route
//! the lookup); a layer that trusted the fold instead of the bytes
//! would be re-inventing the naming rule it is defending against.
//! A mismatch is a loud refusal naming the type, both files and the
//! first diverging line — never a best-effort rewrite.
//!
//! Four guards stand in the machine, not in prose (`rewire` and this
//! file): a closure name with no block in the shared home; a block
//! that is not byte-identical; a fragment whose consumers disagree on
//! reader strictness (the strictness pass rules through the registry
//! per schema, while a fragment is shared across schemas — unanimous
//! `none` consumers make the shared block strict, no `none` consumer
//! leaves it permissive, and a mixture refuses); and the counter — the
//! declarations a module loses must equal the closure its schema
//! pulled, and the tree's unique names must not move.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

mod emit;
mod rewire;

pub(super) use emit::{SHARED_MODULE, emit_shared_module};
pub(super) use rewire::{RewireStats, SharedModule, emitted_name};

use super::format_id::load_format_registry;
use super::vocabulary::Resolved;

/// The one reader policy a resolved shared fragment can carry.
///
/// This is deliberately not a pair of booleans: a fragment is emitted
/// under exactly one policy after all of its registered consumers have
/// been considered. Enums and aliases carry the same classification,
/// although only a generated struct has fields for serde to deny.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FragmentReaderPolicy {
    Permissive,
    Strict,
}

/// The computed policy of every shared fragment with at least one
/// registered consumer. A fragment seen only from an unregistered
/// schema is absent and therefore retains the old permissive shared
/// emission; the schema-local strictness pass later issues the existing
/// unregistered-format refusal.
#[derive(Debug)]
pub(super) struct SharedStrictness {
    fragments: BTreeMap<String, FragmentReaderPolicy>,
}

impl SharedStrictness {
    fn has_strict_fragment(&self) -> bool {
        self.fragments
            .values()
            .any(|policy| *policy == FragmentReaderPolicy::Strict)
    }
}

/// Compute one policy per fragment from the registry roles of every
/// schema in the resolved-closure map. A unanimous `none` set is
/// strict, a set with no `none` member is permissive, and a mixture is
/// refused with both consumer sets named.
///
/// Fed by the run's schema → closure map and the registry; the roles
/// are read through the one registry loader (`format_id`), the same
/// loader `Strictness` builds its map from, so the two can never
/// disagree. A schema no record claims is not refused here — its own
/// strictness pass refuses it when its module is processed.
pub(super) fn guard_shared_strictness(
    root: &Path,
    resolved: &[(PathBuf, Resolved)],
) -> Result<SharedStrictness> {
    let entries = load_format_registry(root)?;
    // Registry spelling of a schema path — repo-relative, forward
    // slashes — the same normalisation `Strictness` keys its map by.
    let role_of = |schema: &Path| -> Option<String> {
        let rel = schema.strip_prefix(root).unwrap_or(schema);
        let key = rel.display().to_string().replace('\\', "/");
        entries
            .iter()
            .find(|entry| entry.schema == key)
            .map(|entry| entry.foreign_parsers.clone())
    };
    policies_from_resolutions(
        resolved
            .iter()
            .map(|(schema, resolution)| (schema.as_path(), &resolution.ordinary_vocabularies)),
        role_of,
    )
}

/// Pure join of the two inputs production already owns: resolved
/// schema closures and the registry's role lookup. Keeping it separate
/// makes the unanimous and mixed laws testable without a second
/// registry parser or a hand-maintained fragment list.
fn policies_from_resolutions<'a, I, F>(resolved: I, mut role_of: F) -> Result<SharedStrictness>
where
    I: IntoIterator<Item = (&'a Path, &'a BTreeSet<String>)>,
    F: FnMut(&Path) -> Option<String>,
{
    // fragment → (schema, role) for every registered consumer, in walk
    // order; a divergence between records sharing one schema was
    // already refused by `Strictness::load`, which runs before this
    // guard in production.
    let mut consumers: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (schema, closure) in resolved {
        let Some(role) = role_of(schema) else {
            continue;
        };
        for fragment in closure {
            consumers
                .entry(fragment.clone())
                .or_default()
                .push((schema.display().to_string(), role.clone()));
        }
    }
    let mut fragments = BTreeMap::new();
    for (fragment, schemas) in &consumers {
        fragments.insert(fragment.clone(), classify_fragment(fragment, schemas)?);
    }
    Ok(SharedStrictness { fragments })
}

/// Classify one fragment's registered consumers, refusing exactly the
/// strict/permissive mixture that has no single set of shared bytes.
fn classify_fragment(
    fragment: &str,
    consumers: &[(String, String)],
) -> Result<FragmentReaderPolicy> {
    let strict: Vec<&str> = consumers
        .iter()
        .filter(|(_, role)| role == "none")
        .map(|(schema, _)| schema.as_str())
        .collect();
    let permissive: Vec<&str> = consumers
        .iter()
        .filter(|(_, role)| role != "none")
        .map(|(schema, _)| schema.as_str())
        .collect();
    if strict.is_empty() {
        return Ok(FragmentReaderPolicy::Permissive);
    }
    if permissive.is_empty() {
        return Ok(FragmentReaderPolicy::Strict);
    }
    bail!(
        "vocabulary `{fragment}` has mixed registered reader policy, so \
         its one shared block cannot be both strict and permissive.\n\
         Strict consumers (`foreign_parsers = \"none\"`): {}\n\
         Permissive consumers (`foreign_parsers = \"ours\"` or \
         `\"many\"`): {}\n\
         Fix: give every registered consumer of `{fragment}` the same \
         strictness class, or stop sharing the fragment across those \
         formats, then run `cargo xtask codegen`.",
        strict.join(", "),
        permissive.join(", "),
    )
}

/// Apply the computed per-fragment policy at the strictness slot of
/// the shared module's normal post-processing pipeline. The routing
/// fold is the same one the replacement uses; policy itself was
/// computed solely from registry roles and resolved closures. A strict
/// enum or alias is recognised but copied, because the serde container
/// attribute is valid only on generated structs.
pub(super) fn apply_shared_strictness(
    src: &str,
    file: &str,
    strictness: &SharedStrictness,
) -> Result<String> {
    if !strictness.has_strict_fragment() {
        return Ok(src.to_string());
    }

    let mut type_owners: BTreeMap<String, (&str, FragmentReaderPolicy)> = BTreeMap::new();
    for (fragment, policy) in &strictness.fragments {
        let emitted = rewire::emitted_name(fragment);
        if let Some((first, _)) = type_owners.insert(emitted.clone(), (fragment, *policy)) {
            bail!(
                "the shared vocabularies `{first}` and `{fragment}` \
                 both fold to the emitted type `{emitted}` — the generator \
                 naming route is ambiguous.\n\
                 Fix: rename the colliding entries in \
                 `formats/vocabularies.json`, then run `cargo xtask codegen`."
            );
        }
    }

    const DERIVE_LINE: &str = "#[derive(Serialize, Deserialize)]";
    const DENY_LINE: &str = "#[serde(deny_unknown_fields)]";
    let mut out = String::with_capacity(src.len() + type_owners.len() * (DENY_LINE.len() + 1));
    let mut previous: Option<(&str, &str, &str)> = None;
    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();
        if let Some(name) = struct_name(text)
            && let Some((fragment, policy)) = owner_for_type(name, &type_owners)
            && policy == FragmentReaderPolicy::Strict
        {
            let Some((previous_text, indent, ending)) = previous else {
                bail!(
                    "{file}:{}: strict shared vocabulary `{fragment}` emits \
                     struct `{name}` with no `{DERIVE_LINE}` line directly \
                     above — the pinned jtd-codegen shape moved, so the \
                     shared strictness pass refuses to guess.",
                    index + 1
                );
            };
            if previous_text != DERIVE_LINE && previous_text != DENY_LINE {
                bail!(
                    "{file}:{}: strict shared vocabulary `{fragment}` emits \
                     struct `{name}` with no `{DERIVE_LINE}` line directly \
                     above — the pinned jtd-codegen shape moved, so the \
                     shared strictness pass refuses to guess.",
                    index + 1
                );
            }
            if previous_text == DERIVE_LINE {
                out.push_str(indent);
                out.push_str(DENY_LINE);
                out.push_str(ending);
            }
        }
        out.push_str(chunk);
        let indent = &body[..body.len() - body.trim_start().len()];
        let ending = &chunk[body.len()..];
        previous = Some((text, indent, ending));
    }
    Ok(out)
}

/// Attribute a subsidiary to its longest vocabulary type prefix, so
/// `Artifact` cannot steal `ArtifactFrameStaticLane` from `ArtifactFrame`.
fn owner_for_type<'a>(
    name: &str,
    owners: &'a BTreeMap<String, (&'a str, FragmentReaderPolicy)>,
) -> Option<(&'a str, FragmentReaderPolicy)> {
    owners
        .iter()
        .filter(|(main, _)| name == main.as_str() || name.starts_with(main.as_str()))
        .max_by_key(|(main, _)| main.len())
        .map(|(_, owner)| *owner)
}

/// Stamp projected local copies strict; only the consumer root is permissive.
pub(super) fn apply_projected_copy_strictness(
    src: &str,
    file: &str,
    fragments: &BTreeSet<String>,
) -> Result<String> {
    if fragments.is_empty() {
        return Ok(src.to_string());
    }
    let strictness = SharedStrictness {
        fragments: fragments
            .iter()
            .map(|fragment| (fragment.clone(), FragmentReaderPolicy::Strict))
            .collect(),
    };
    apply_shared_strictness(src, file, &strictness)
}

/// The emitted struct name on a declaration line of the pinned shape.
fn struct_name(text: &str) -> Option<&str> {
    let tail = text.strip_prefix("pub struct ")?;
    let name = tail
        .strip_suffix("{}")
        .or_else(|| tail.strip_suffix('{'))?
        .trim_end();
    is_ident(name).then_some(name)
}

/// The run-level counter — the fourth guard. The per-module half
/// (`rewire`) has already refused a module whose declarations did not
/// drop by exactly its closure's size; this half crosses modules: the
/// TOTAL drop over the run must equal the total number of replaced
/// copies, and the tree's set of unique type names must not move —
/// names may vanish from a schema module only by existing in the
/// shared home, and the shared home may introduce a name only by
/// replacing someone's copy of it (a fragment no schema pulls would
/// otherwise mint a type nobody asked for, and the counter says so
/// rather than letting it slip in).
pub(super) fn check_counter(
    per_module: &[RewireStats],
    shared_names: &BTreeSet<String>,
) -> Result<()> {
    let before: usize = per_module.iter().map(|stats| stats.before).sum();
    let after: usize = per_module.iter().map(|stats| stats.after).sum();
    let replaced: usize = per_module.iter().map(|stats| stats.replaced).sum();
    if before - after != replaced {
        bail!(
            "the shared-module replacement left the declaration count \
             inconsistent: {} declarations before, {} after, {} copies \
             replaced — the drop and the replacements must agree \
             exactly.\n\
             Fix: this is a defect in the replacement pass \
             (`xtask/src/codegen/shared_module/rewire.rs`), not in the \
             schemas; the run refuses to write a half-replaced tree.",
            before,
            after,
            replaced
        );
    }
    let mut names_before: BTreeSet<String> = BTreeSet::new();
    let mut names_after: BTreeSet<String> = shared_names.clone();
    let mut copied_after: BTreeSet<String> = BTreeSet::new();
    for stats in per_module {
        names_before.extend(stats.names_before.iter().cloned());
        names_after.extend(stats.names_after.iter().cloned());
        copied_after.extend(stats.copies_after.iter().cloned());
    }
    let duplicated: Vec<String> = copied_after.into_iter().collect();
    if !duplicated.is_empty() {
        bail!(
            "the shared-module replacement left canonical type declarations \
             copied in schema modules: [{}]. A shared name must have exactly \
             one physical declaration, in `generated::shared`; every schema \
             path is a re-export.\n\
             Fix: restore the missing fragment(s) to the schema's resolved \
             shared closure or repair `shared_module/rewire.rs`, then run \
             `cargo xtask codegen`.",
            duplicated.join(", ")
        );
    }
    let appeared: Vec<String> = names_after.difference(&names_before).cloned().collect();
    let vanished: Vec<String> = names_before.difference(&names_after).cloned().collect();
    if !appeared.is_empty() || !vanished.is_empty() {
        bail!(
            "the shared-module replacement moved the tree's set of type \
             names: appeared [{}], vanished [{}]. A schema module may \
             lose a name only to the shared home, and the shared home \
             may carry a name only some schema pulls — a name that only \
             appeared points at a fragment in `formats/vocabularies.json` \
             no schema names, a name that only vanished at a replacement \
             that took more than its own block.\n\
             Fix: pull the unconsumed fragment into a schema, or drop it \
             from `formats/vocabularies.json` (for an appearance); for a \
             vanishment this is a defect in the replacement pass \
             (`xtask/src/codegen/shared_module/rewire.rs`) — the run \
             refuses to write a half-replaced tree either way.",
            appeared.join(", "),
            vanished.join(", ")
        );
    }
    eprintln!(
        "xtask codegen: shared module — {} cop{} replaced by re-exports \
         across {} module{}; declarations {} → {} in the schema modules \
         (the shared home adds {}), unique names {} unchanged.",
        replaced,
        if replaced == 1 { "y" } else { "ies" },
        per_module.len(),
        if per_module.len() == 1 { "" } else { "s" },
        before,
        after,
        shared_names.len(),
        names_before.len(),
    );
    Ok(())
}

/// Take away exactly the import items nothing uses any more — the rule
/// `domain_types` follows when a substitution orphans an item, applied
/// here to the two shapes that orphan imports on this floor: the
/// shared module losing its parasitic root (`emit`), and a schema
/// module losing the blocks that were an import's last users
/// (`rewire`). An item is orphaned when its whole token appears
/// nowhere outside the `use` lines themselves and never as a
/// `::`-qualified path segment (`chrono::DateTime` names the path, not
/// the import); an emptied line goes entirely; a lone survivor sheds
/// its braces (a shape rustfmt rewrites, and a generated file is
/// never hand-formatted); an import with a user left stands byte for
/// byte. A `use` line in neither of the two pinned shapes refuses
/// loudly — the generator writes only those, and anything else means
/// the emission this layer is pinned to has moved.
pub(super) fn prune_orphan_imports(src: &str, file: &str) -> Result<String> {
    let lines: Vec<&str> = src.split_inclusive('\n').collect();
    // Parse every use line first; a refusal fires before anything is
    // rewritten, so the pass never leaves a half-pruned file behind.
    let mut uses: BTreeMap<usize, UseLine> = BTreeMap::new();
    for (index, chunk) in lines.iter().enumerate() {
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();
        if !text.starts_with("use ") {
            continue;
        }
        let Some(rest) = text
            .strip_prefix("use ")
            .and_then(|rest| rest.strip_suffix(';'))
        else {
            bail!(
                "{file}:{}: a `use` line this pass cannot parse: `{text}`",
                index + 1
            );
        };
        let form = if let Some((head, braced)) = rest.split_once("::{") {
            let Some(items) = braced.strip_suffix('}') else {
                bail!(
                    "{file}:{}: a `use` line this pass cannot parse: `{text}`",
                    index + 1
                );
            };
            if head.is_empty() || head.split("::").any(|segment| !is_ident(segment)) {
                bail!(
                    "{file}:{}: a `use` line this pass cannot parse: `{text}`",
                    index + 1
                );
            }
            let items: Vec<&str> = items.split(", ").collect();
            if items.iter().any(|item| !is_ident(item)) {
                bail!(
                    "{file}:{}: a `use` line this pass cannot parse: `{text}`",
                    index + 1
                );
            }
            UseLine::Braced { head, items }
        } else if rest.contains("::")
            && rest.split("::").all(is_ident)
            && let Some(last) = rest.rsplit("::").next()
        {
            UseLine::Plain { last }
        } else {
            bail!(
                "{file}:{}: a `use` line this pass cannot parse:\n\
                 `{text}`\n\
                 The pinned jtd-codegen emission writes imports only as \
                 `use <path>::{{A, B}};` or `use <path>;`, and the \
                 orphan pruning removes exactly the items that lost their \
                 last user — reasoning about any other shape would hide a \
                 moved pin behind a green run.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `shared_module.rs` the new shape, then run `cargo xtask \
                 codegen`.",
                index + 1
            );
        };
        uses.insert(index, form);
    }
    // Count, per import item, the references that keep it alive: whole
    // tokens outside the `use` lines, never `::`-qualified on the left
    // (`chrono::DateTime` names the path, not the import).
    let mut alive: BTreeSet<String> = BTreeSet::new();
    for (index, chunk) in lines.iter().enumerate() {
        if uses.contains_key(&index) {
            continue;
        }
        let mut run = String::new();
        let mut run_start: Option<usize> = None;
        for (offset, character) in chunk.char_indices() {
            if character.is_ascii_alphanumeric() || character == '_' {
                run_start.get_or_insert(offset);
                run.push(character);
                continue;
            }
            if let Some(start) = run_start.take()
                && !chunk[..start].ends_with(':')
            {
                alive.insert(run.clone());
            }
            run.clear();
        }
        if let Some(start) = run_start.take()
            && !chunk[..start].ends_with(':')
        {
            alive.insert(run.clone());
        }
    }
    // Rebuild: drop orphaned items, whole lines when every item is
    // orphaned, and the blank a removal strands between two blanks.
    let mut out = String::with_capacity(src.len());
    let mut previous_blank = false;
    let mut squash_next_blank = false;
    for (index, chunk) in lines.iter().enumerate() {
        let blank = chunk.trim().is_empty();
        if squash_next_blank {
            squash_next_blank = false;
            if blank {
                continue;
            }
        }
        let Some(form) = uses.get(&index) else {
            out.push_str(chunk);
            previous_blank = blank;
            continue;
        };
        match form {
            UseLine::Braced { head, items } => {
                let kept: Vec<&str> = items
                    .iter()
                    .copied()
                    .filter(|item| alive.contains(*item))
                    .collect();
                if kept.is_empty() {
                    squash_next_blank = previous_blank;
                    continue;
                }
                if kept.len() == items.len() {
                    out.push_str(chunk);
                    previous_blank = false;
                    continue;
                }
                let body = chunk.trim_end_matches(['\r', '\n']);
                let text = body.trim();
                let indent = &body[..body.len() - text.len()];
                out.push_str(indent);
                out.push_str("use ");
                out.push_str(head);
                out.push_str("::");
                if let [single] = kept[..] {
                    // A lone survivor wears no braces: `use a::{b};` is
                    // a shape rustfmt rewrites, and a generated file is
                    // never hand-formatted, so the pass emits the form
                    // the panel's first gate already accepts.
                    out.push_str(single);
                } else {
                    out.push_str(&format!("{{{}}}", kept.join(", ")));
                }
                out.push(';');
                out.push_str(&chunk[body.len()..]);
                previous_blank = false;
            }
            UseLine::Plain { last } => {
                if !alive.contains(*last) {
                    squash_next_blank = previous_blank;
                    continue;
                }
                out.push_str(chunk);
                previous_blank = false;
            }
        }
    }
    Ok(out)
}

/// One `use` line of the pinned emission — the two shapes the
/// generator writes, the same pair `domain_types` parses.
enum UseLine<'a> {
    /// `use <path>::{A, B};` — every braced name binds an item.
    Braced { head: &'a str, items: Vec<&'a str> },
    /// `use <path>;` — the last path segment binds the item.
    Plain { last: &'a str },
}

/// ASCII identifier shape — the same contract the sibling passes'
/// matchers enforce around the identifiers they rewrite.
fn is_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
#[path = "shared_module/tests.rs"]
mod tests;
