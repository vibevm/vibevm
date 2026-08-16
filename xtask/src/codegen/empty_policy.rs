//! The empty-collection policy pass — the fourth content edit the
//! generator's emission takes (the order rule lives in `postproc`'s
//! docs: a pass keyed to the emission shape runs while the file is
//! STILL that emission, and this one is keyed to the
//! `#[serde(skip_serializing_if = "Option::is_none")]` attribute over a
//! `pub <field>: Option<Box<Vec<…>>>` / `Option<Box<BTreeMap<…>>>` line,
//! so it runs after arm boxing, field snake_casing and map ordering, and
//! before the vocabularies open).
//!
//! This file is the schema side of the stitch plus the entry; the
//! Rust-side scanner that rewrites the emission lives in the child
//! `empty_policy/emit.rs`, split along that seam when the file
//! outgrew the 600-line budget. The two halves meet at
//! `EmptyPolicies`: this half builds it, the child obeys it.
//!
//! What it enforces: whether an empty collection is WRITTEN is a policy
//! the schema declares per field, `metadata."x-empty"` — `"omit"`: an
//! empty collection is not written; `"emit"`: the collection is written
//! even when empty. A missing annotation on a collection is a
//! generation error, not a default, because the one thing this pass may
//! not do is guess.
//!
//! A SITE is a FIELD, not a node: an `elements` or `values` form
//! standing as the value of a member of a `properties` or
//! `optionalProperties` block. The `elements` INSIDE a `values` node
//! describes the map's value type and has no member of its own to hang
//! an attribute on — the policy is inexpressible there, so it is not a
//! site (a naive node-counting walk says 33 where the fields are 31;
//! the two extras are exactly those inner `elements`).
//!
//! REQUIREDNESS BOUNDS THE POLICY (rule R21, owner ruling P21): a JTD
//! `properties` member is required, so a writer omitting an empty one
//! would produce a document invalid by its own schema — the exact shape
//! of a wrong answer that looks right. Required + `omit` is therefore a
//! generation error of its own (a separate refusal, not a generic
//! annotation complaint), and the only lawful policy for a required
//! collection is `emit`, which changes NOTHING: the emission is already
//! the bare `Vec<…>` / `BTreeMap<…>` with no skip. Optional + `omit`
//! collapses `Option<Box<C>>` to `C` with
//! `#[serde(default, skip_serializing_if = "<C>::is_empty")]`; the
//! optional + `emit` row collapses with `#[serde(default)]` and no skip
//! — a branch with zero sites today, present in the machine and proven
//! by test, because a rule that exists only while its sites do is not a
//! rule.
//!
//! The predicate is chosen by container (`Vec::is_empty`,
//! `BTreeMap::is_empty`) — the exact form the hand-written twins already
//! carry (`vibe-index/src/types/entry/content.rs`).
//!
//! The stitch key is (wire name, requiredness) — one name alone is not
//! enough: `packages` is a required member of `by_name` and an optional
//! member of `conflicts_entry`. Requiredness is visible from both
//! sides: `properties` vs `optionalProperties` in the schema, the
//! `Option<Box<…>>` wrapper vs the bare type in Rust. The wire name in
//! Rust, after the snake_case pass, is the field identifier unless a
//! rename survived it (a camelCase or keyword wire name). Two sites
//! sharing a key with DIFFERENT policies refuse, naming the key and both
//! policies; the same key with the same policy is a legal diamond of
//! the vocabulary substitution.
//!
//! What moves on the wire, said honestly: the collapse removes the
//! state `Some(empty)`, which the projector never produced (empty
//! normalisation is projector work, contract annex B.2) — and the
//! reader still accepts both an absent key and `[]`, because
//! `#[serde(default)]` folds them into the same empty collection.
//!
//! After the file, the count of collection fields found in Rust must
//! meet the schema's site count exactly — the same tally that keeps
//! `open_vocabulary` honest: a collection that slipped past the scanner
//! must refuse the run, not pass for processed.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

mod emit;

use emit::apply_with_policies;

/// The pass entry the driver calls: read the schema-side policies off
/// the document the generator read (`resolved` — the authored schema
/// when it pulls no vocabularies, the scratch copy with the fragments
/// placed otherwise), then stitch the generated Rust to them. No new
/// input is invented — `generate_into` already holds `resolved` exactly
/// where this pass needs it.
pub(super) fn apply_empty_policies(
    src: &str,
    file: &str,
    resolved: &Path,
    schema: &Path,
) -> Result<String> {
    let policies = empty_policies(resolved, schema)?;
    apply_with_policies(src, file, &policies)
}

/// The empty-collection policy one site carries on the schema side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    /// An empty collection is not written.
    Omit,
    /// The collection is written even when empty.
    Emit,
}

impl Policy {
    /// The word the annotation and the refusal texts spell alike.
    fn as_str(self) -> &'static str {
        match self {
            Policy::Omit => "omit",
            Policy::Emit => "emit",
        }
    }
}

/// Which `properties` block a member lives in — the requiredness half
/// of the stitch key, visible from both sides of the stitch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Required {
    /// A member of `properties`: JTD makes it mandatory on the wire.
    Required,
    /// A member of `optionalProperties`: the writer may omit it.
    Optional,
}

impl Required {
    /// The word the refusals spell for this half of the key.
    fn as_str(self) -> &'static str {
        match self {
            Required::Required => "required",
            Required::Optional => "optional",
        }
    }
}

/// What the schema side of the stitch read out of one resolved schema:
/// every field site's policy keyed by (wire name, requiredness), plus
/// the NUMBER OF SITES (not of distinct keys) — the tally the Rust-side
/// scanner must meet exactly, the tripwire that keeps a silently
/// skipped collection from passing for processed.
#[derive(Debug)]
struct EmptyPolicies {
    map: BTreeMap<(String, Required), Policy>,
    sites: usize,
}

/// One collection site the walk found: the member's wire name, the
/// `properties` block it lives in, the node itself (its annotation is
/// read at the site), and the resolved-document path the refusals name.
struct Site<'a> {
    wire: String,
    required: Required,
    node: &'a Map<String, Value>,
    path: String,
}

/// The policies of the document the generator read for one schema.
fn empty_policies(resolved: &Path, schema: &Path) -> Result<EmptyPolicies> {
    let text = std::fs::read_to_string(resolved)
        .with_context(|| format!("reading the resolved schema {}", resolved.display()))?;
    let doc: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", resolved.display()))?;
    policies_from_doc(&doc, schema)
}

/// The same read over an already-parsed document, so the tests drive the
/// pure half without scratch files.
fn policies_from_doc(doc: &Value, schema: &Path) -> Result<EmptyPolicies> {
    let mut sites: Vec<Site<'_>> = Vec::new();
    collect_sites(doc, "", &mut sites);
    let mut map: BTreeMap<(String, Required), Policy> = BTreeMap::new();
    for site in &sites {
        let policy = site_policy(site, schema)?;
        let key = (site.wire.clone(), site.required);
        if let Some(existing) = map.get(&key) {
            if *existing != policy {
                bail!(
                    "schema {}: two collection fields share the key (`{}`, \
                     {}) with different `x-empty` policies — `{}` and `{}`. \
                     One field on one side of the requiredness line carries \
                     one policy; it cannot omit and emit at once.\n\
                     Fix: make the `metadata.\"x-empty\"` of both definitions \
                     agree, then run `cargo xtask codegen`.",
                    schema.display(),
                    site.wire,
                    site.required.as_str(),
                    existing.as_str(),
                    policy.as_str()
                );
            }
            // Same key, same policy: a legal diamond of the vocabulary
            // substitution — counted as its own site all the same.
        } else {
            map.insert(key, policy);
        }
    }
    Ok(EmptyPolicies {
        map,
        sites: sites.len(),
    })
}

/// Walk a resolved schema collecting every collection FIELD site: an
/// `elements` or `values` node standing as the value of a member of a
/// `properties` / `optionalProperties` block — the one place a skip
/// policy is expressible. The `elements` inside a `values` node (the
/// map's value type) is walked past without counting: it is not a
/// member of any `properties` block, so no policy could be read off it
/// however the walk ran. `metadata` blocks are skipped on the way down
/// — annotation data the JTD machinery never reads, so an
/// `elements`-shaped key inside one is data, not a site (the same cut
/// `collect_enum_sites` in `open_vocabulary.rs` makes for `enum`).
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
            "properties" => Required::Required,
            "optionalProperties" => Required::Optional,
            _ => {
                collect_sites(field, &path, sites);
                continue;
            }
        };
        let Some(members) = field.as_object() else {
            continue;
        };
        for (wire, member) in members {
            let member_path = join_trail(&path, wire);
            let Some(node) = member.as_object() else {
                continue;
            };
            if node.contains_key("elements") || node.contains_key("values") {
                sites.push(Site {
                    wire: wire.clone(),
                    required,
                    node,
                    path: member_path,
                });
                // An `elements` / `values` form admits no nested
                // properties (JTD forms are exclusive), so there is
                // nothing below a site left to walk.
                continue;
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

/// Read one site's `metadata."x-empty"` and rule on it: `"omit"` or
/// `"emit"` is the policy; a missing key, a non-string, or a stranger
/// word is a generation error naming the schema, the site's path in the
/// resolved document and the recipe. A REQUIRED member carrying `omit`
/// refuses separately — rule R21: a `properties` member is required, and
/// a writer omitting an empty one would produce a document invalid by
/// this same schema.
fn site_policy(site: &Site<'_>, schema: &Path) -> Result<Policy> {
    let Some(annotation) = site.node.get("metadata").and_then(|m| m.get("x-empty")) else {
        bail!(
            "schema {}: the collection field `{}` carries no \
             `metadata.\"x-empty\"` — whether an empty collection is written \
             or omitted is decided per field on the schema side and is not \
             derivable from the generated Rust.\n\
             Fix: add `\"x-empty\": \"omit\"` or `\"emit\"` to this member's \
             `metadata` (in {} itself, or in the vocabulary fragment it pulls \
             from formats/vocabularies.json), then run `cargo xtask codegen`.",
            schema.display(),
            site.path,
            schema.display()
        );
    };
    let policy = match annotation.as_str() {
        Some("omit") => Policy::Omit,
        Some("emit") => Policy::Emit,
        _ => {
            let found = annotation.to_string();
            bail!(
                "schema {}: the collection field `{}` carries \
                 `metadata.\"x-empty\"` = {found} — expected the string \
                 `\"omit\"` or `\"emit\"`.\n\
                 Fix: set the annotation to `\"omit\"` or `\"emit\"`, then run \
                 `cargo xtask codegen`.",
                schema.display(),
                site.path
            );
        }
    };
    if site.required == Required::Required && policy == Policy::Omit {
        bail!(
            "schema {}: the collection field `{}` carries \
             `metadata.\"x-empty\"` = \"omit\", but it is a member of \
             `properties` — rule R21: a `properties` member is required; a \
             writer omitting an empty one would produce a document invalid by \
             this same schema.\n\
             Fix: set the annotation to \"emit\" (the only lawful policy for a \
             required collection), then run `cargo xtask codegen`.",
            schema.display(),
            site.path
        );
    }
    Ok(policy)
}

#[cfg(test)]
#[path = "empty_policy/tests.rs"]
mod tests;
