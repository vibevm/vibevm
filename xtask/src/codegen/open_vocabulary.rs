//! Opening vocabularies — the second post-processing pass, enforcing
//! PROP-044 §4.2a: a closed schema-enum becomes an open Rust type,
//! `enum { …known…, Unknown(String) }` with hand-rolled `Serialize` /
//! `Deserialize` (the derive comes off, or it would collide with the
//! manual impls). Compiler-checkable exhaustiveness survives — the
//! `Unknown` arm is mandatory — the original string survives a
//! read/write cycle, and tolerance to future values becomes structural.
//!
//! Which enums open is decided per vocabulary by the schema's
//! `metadata."x-vocabulary": "open"` / `"closed"` annotation and by
//! nothing else: measured, an enum that must open (`PackageKind`) and
//! one that must not (`NamingConvention`) come out of the generator
//! syntactically indistinguishable, so the pass takes its decision from
//! the schema side of the stitch — and a missing annotation is a
//! generation error, not a default, because the one thing this pass may
//! not do is guess.
//!
//! The stitch itself: the schema side collects every enum site (an
//! object with an `"enum"` array of strings) keyed by its set of wire
//! values — not by name, for the generator sorts the variants and mints
//! the type name on its own, while both sides carry the value set
//! verbatim. The Rust side scans the file for the derive line every
//! generated type carries; the line after it decides what the derive
//! labels. A vocabulary enum's wire set is looked up in the map: `open`
//! rewrites, `closed` replays byte for byte, unknown refuses. After the
//! file, the count of vocabulary enums found must meet the schema's
//! site count exactly — the tally that keeps THE UNION SKIP RULE honest
//! rather than silent.
//!
//! The pass never reads its own output: it runs inside
//! `rewrite_generated`, over the boxing pass's result, and the tree is
//! wiped and regenerated before every codegen run.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

/// The derive line every generated type carries by the time this pass
/// reads it, and the anchor the vocabulary scanner keys on.
///
/// jtd-codegen emits the serde pair alone; the trait-floor pass runs
/// directly before this one and widens that line, so the literal lives
/// in `derive_floor` and is borrowed here rather than restated — two
/// copies of it would drift into a scanner that silently finds nothing.
use super::derive_floor::{WITH_FLOOR as DERIVE_LINE, WITHOUT_SERDE};

/// The pass entry the driver calls: read the schema-side policies off
/// the document the generator read (`resolved` — the authored schema
/// when it pulls no vocabularies, the scratch copy with the fragments
/// placed otherwise), then stitch the generated Rust to them. No new
/// input is invented — `generate_into` already holds `resolved` exactly
/// where this pass needs it.
pub(super) fn open_vocabularies(
    src: &str,
    file: &str,
    resolved: &Path,
    schema: &Path,
) -> Result<String> {
    let policies = vocabulary_policies(resolved, schema)?;
    open_with_policies(src, file, &policies)
}

/// The openness policy one enum site carries on the schema side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    Open,
    Closed,
}

impl Policy {
    /// The word the annotation and the refusal texts spell alike.
    fn as_str(self) -> &'static str {
        match self {
            Policy::Open => "open",
            Policy::Closed => "closed",
        }
    }
}

/// What the schema side of the stitch read out of one resolved schema:
/// every enum site's wire-value set keyed to its openness policy, plus
/// the NUMBER OF SITES (not of distinct sets) — the tally the Rust-side
/// scanner must meet exactly, the tripwire that keeps a silently skipped
/// vocabulary from passing for processed.
#[derive(Debug)]
struct VocabularyPolicies {
    map: BTreeMap<BTreeSet<String>, Policy>,
    sites: usize,
}

/// The policies of the document the generator read for one schema.
fn vocabulary_policies(resolved: &Path, schema: &Path) -> Result<VocabularyPolicies> {
    let text = std::fs::read_to_string(resolved)
        .with_context(|| format!("reading the resolved schema {}", resolved.display()))?;
    let doc: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", resolved.display()))?;
    policies_from_doc(&doc, schema)
}

/// The same read over an already-parsed document, so the tests drive the
/// pure half without scratch files.
fn policies_from_doc(doc: &Value, schema: &Path) -> Result<VocabularyPolicies> {
    let mut sites: Vec<&Map<String, Value>> = Vec::new();
    collect_enum_sites(doc, &mut sites);
    let mut map: BTreeMap<BTreeSet<String>, Policy> = BTreeMap::new();
    for site in &sites {
        let values: BTreeSet<String> = site
            .get("enum")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| item.as_str())
            .map(str::to_string)
            .collect();
        let policy = site_policy(site, schema, &values)?;
        if let Some(existing) = map.get(&values) {
            if *existing != policy {
                bail!(
                    "schema {}: two enum definitions carry the same wire \
                     values ({}) with different `x-vocabulary` policies — \
                     `{}` and `{}`. One set of values is one vocabulary; it \
                     cannot be open and closed at once.\n\
                     Fix: make the `metadata.\"x-vocabulary\"` of both \
                     definitions agree, then run `cargo xtask codegen`.",
                    schema.display(),
                    quote_set(&values),
                    existing.as_str(),
                    policy.as_str()
                );
            }
            // Same set, same policy: a legal diamond of the vocabulary
            // substitution — counted as its own site all the same.
        } else {
            map.insert(values, policy);
        }
    }
    Ok(VocabularyPolicies {
        map,
        sites: sites.len(),
    })
}

/// Walk a resolved schema document collecting every enum site: an object
/// with an `"enum"` key whose value is an array of strings. `metadata`
/// blocks are skipped on the way down — they are annotation data the JTD
/// machinery never reads, so an `"enum"`-shaped key inside one is data,
/// not a site (the same cut `find_dangling_ref` in `vocabulary.rs`
/// already makes for `ref`). The site's OWN `metadata` is read at the
/// site, by `site_policy` below.
fn collect_enum_sites<'a>(value: &'a Value, sites: &mut Vec<&'a Map<String, Value>>) {
    match value {
        Value::Object(fields) => {
            if is_string_array(fields.get("enum")) {
                sites.push(fields);
            }
            for (key, field) in fields {
                if key != "metadata" {
                    collect_enum_sites(field, sites);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_enum_sites(item, sites);
            }
        }
        _ => {}
    }
}

/// An `"enum"` value that is an array of strings — the JTD enum form, and
/// the only one this pass counts as a site. A differently-shaped `"enum"`
/// never reaches here: the generator has already accepted the document by
/// the time the post-processor reads it, and JTD admits no other shape.
fn is_string_array(value: Option<&Value>) -> bool {
    value.is_some_and(|v| {
        v.as_array()
            .is_some_and(|items| items.iter().all(|item| item.as_str().is_some()))
    })
}

/// Read one site's `metadata."x-vocabulary"`: `"open"` or `"closed"` is
/// the policy; a missing key, a non-string, or a stranger word is a
/// generation error naming the schema, the vocabulary's values and the
/// recipe — the policy is decided per vocabulary on the schema side and
/// is not derivable from the generated Rust, so the pass refuses rather
/// than guess.
fn site_policy(
    site: &Map<String, Value>,
    schema: &Path,
    values: &BTreeSet<String>,
) -> Result<Policy> {
    let Some(annotation) = site.get("metadata").and_then(|m| m.get("x-vocabulary")) else {
        bail!(
            "schema {}: the enum definition with the values ({}) carries no \
             `metadata.\"x-vocabulary\"` — whether a vocabulary is open or \
             closed is decided per vocabulary on the schema side (PROP-044 \
             §4.2a) and is not derivable from the generated Rust.\n\
             Fix: add `\"x-vocabulary\": \"open\"` or `\"closed\"` to the \
             `metadata` of this definition, then run `cargo xtask codegen`.",
            schema.display(),
            quote_set(values)
        );
    };
    match annotation.as_str() {
        Some("open") => Ok(Policy::Open),
        Some("closed") => Ok(Policy::Closed),
        _ => {
            let found = annotation.to_string();
            bail!(
                "schema {}: the enum definition with the values ({}) carries \
                 `metadata.\"x-vocabulary\"` = `{found}` — expected the string \
                 `\"open\"` or `\"closed\"`.\n\
                 Fix: set the annotation to `\"open\"` or `\"closed\"`, then \
                 run `cargo xtask codegen`.",
                schema.display(),
                quote_set(values)
            );
        }
    }
}

/// The wire values of a site, as a refusal names them: `"feat", "flow"`.
fn quote_set(values: &BTreeSet<String>) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", value))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The Rust side of the stitch, over text, so the tests drive exactly
/// what production drives — a line scanner in `box_union_arms`'s style.
/// The anchor is the derive line every generated type carries; the line
/// AFTER it decides what the derive labels: a vocabulary enum
/// (`pub enum <Ident> {`) is stitched to the schema's policy, everything
/// else is copied through. Inside a vocabulary enum only three line
/// shapes are legal — blank, per-variant `#[serde(rename = "…")]`,
/// `<Ident>,` — and each variant must carry its rename: the generator
/// always emits one (measured), and guessing a wire string is the one
/// thing this pass may not do. A `closed` vocabulary is replayed byte
/// for byte; an `open` one takes the PROP-044 §4.2a form
/// (`emit_open_enum`). After the file, the count of vocabulary enums
/// found must meet the schema's site count exactly.
fn open_with_policies(src: &str, file: &str, policies: &VocabularyPolicies) -> Result<String> {
    let mut out = String::with_capacity(src.len() + src.len() / 4);
    // `Some((line, chunk) of the derive attribute)` — the next line
    // decides what the derive labels. The chunk is held, not pushed: a
    // `closed` policy replays it byte for byte, an `open` one drops it
    // (a derive and the hand-rolled impls would collide).
    let mut pending_derive: Option<(usize, &str)> = None;
    // Inside the braces of a recognised vocabulary enum: (name, line of
    // the derive), for the emissions and refusals below.
    let mut in_vocab: Option<(String, usize)> = None;
    // The replay buffer and the parse of the body — meaningful only while
    // `in_vocab` is `Some`.
    let mut raw: Vec<&str> = Vec::new();
    let mut variants: Vec<(String, String)> = Vec::new();
    let mut pending_rename: Option<String> = None;
    // Vocabulary enums this file actually carried — must meet the
    // schema's site count exactly once the file ends.
    let mut found: usize = 0;

    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        let line = index + 1;
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();

        // The line after a derive decides what the derive labels.
        if let Some((derive_line, derive_chunk)) = pending_derive.take() {
            if labels_a_discriminator_union(text) {
                // THE UNION SKIP RULE — a discriminator union is copied
                // through verbatim and counted as no vocabulary.
                out.push_str(derive_chunk);
                out.push_str(chunk);
            } else if let Some(name) = vocabulary_enum_name(text) {
                raw.clear();
                variants.clear();
                pending_rename = None;
                raw.push(derive_chunk);
                raw.push(chunk);
                in_vocab = Some((name.to_string(), derive_line));
            } else {
                // A struct or a type alias: the pass is a copy for
                // everything that is not a vocabulary enum.
                out.push_str(derive_chunk);
                out.push_str(chunk);
            }
            continue;
        }

        // Closing a recognised vocabulary enum: stitch it to the schema
        // here, and emit (open) or replay (closed).
        if in_vocab.is_some() && text == "}" {
            let Some((name, derive_line)) = in_vocab.take() else {
                continue;
            };
            if pending_rename.is_some() {
                bail!(
                    "{file}:{line}: the vocabulary enum opened at line \
                     {derive_line} ends with a `#[serde(rename = …)]` no \
                     variant consumed — jtd-codegen emits a rename directly \
                     above its variant (the emission shape is pinned by the \
                     generator's version), so this file is not that shape.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `open_with_policies` in \
                     `xtask/src/codegen/open_vocabulary.rs` the new shape, \
                     then run `cargo xtask codegen`."
                );
            }
            found += 1;
            let wires: BTreeSet<String> = variants.iter().map(|(_, wire)| wire.clone()).collect();
            match policies.map.get(&wires) {
                None => bail!(
                    "{file}:{line}: the generated enum `{name}` carries the \
                     values ({}), which no enum definition of the schema \
                     describes.\n\
                     The pass opens or closes a vocabulary by matching the \
                     generated wire values against the schema's enum \
                     definitions; an enum the schema does not describe means \
                     the emission shape moved, and the pass refuses to guess \
                     a policy for it.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `open_with_policies` in \
                     `xtask/src/codegen/open_vocabulary.rs` the new shape, \
                     then run `cargo xtask codegen`.",
                    quote_set(&wires)
                ),
                Some(Policy::Closed) => {
                    for replay in &raw {
                        out.push_str(replay);
                    }
                    out.push_str(chunk);
                }
                Some(Policy::Open) => emit_open_enum(&name, &variants, &mut out, file, line)?,
            }
            continue;
        }

        // Inside a vocabulary enum: three line shapes, no fourth.
        if in_vocab.is_some() {
            let Some((_, derive_line)) = in_vocab.as_ref() else {
                continue;
            };
            if text.is_empty() {
                // Layout is the generator's, not ours — kept only for the
                // `closed` replay.
                raw.push(chunk);
                continue;
            }
            if let Some(wire) = rename_wire(text) {
                if pending_rename.is_some() {
                    bail!(
                        "{file}:{line}: two `#[serde(rename = …)]` lines in \
                         a row inside the vocabulary enum opened at line \
                         {derive_line} — jtd-codegen emits one rename per \
                         variant (the emission shape is pinned by the \
                         generator's version), so this file is not that \
                         shape.\n\
                         Fix: restore the pinned jtd-codegen version, or \
                         teach `open_with_policies` in \
                         `xtask/src/codegen/open_vocabulary.rs` the new \
                         shape, then run `cargo xtask codegen`."
                    );
                }
                pending_rename = Some(wire.to_string());
                raw.push(chunk);
                continue;
            }
            if let Some(ident) = text.strip_suffix(',').filter(|ident| is_ident(ident)) {
                let Some(wire) = pending_rename.take() else {
                    bail!(
                        "{file}:{line}: the variant `{ident}` of a vocabulary \
                         enum carries no `#[serde(rename = …)]` — jtd-codegen \
                         always emits one (the emission shape is pinned by \
                         the generator's version), and the pass may not guess \
                         a wire string.\n\
                         Fix: restore the pinned jtd-codegen version, or \
                         teach `open_with_policies` in \
                         `xtask/src/codegen/open_vocabulary.rs` the new \
                         shape, then run `cargo xtask codegen`."
                    );
                };
                variants.push((ident.to_string(), wire));
                raw.push(chunk);
                continue;
            }
            bail!(
                "{file}:{line}: the vocabulary enum opened at line \
                 {derive_line} holds a line this pass does not recognise:\n\
                 `{text}`\n\
                 The pass opens or closes whole vocabulary enums and refuses \
                 to guess past an unfamiliar line — the emission shape is \
                 pinned by the generator's version, and rewriting half an \
                 enum silently would hide a moved pin behind a green run.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `open_with_policies` in \
                 `xtask/src/codegen/open_vocabulary.rs` the new shape, then \
                 run `cargo xtask codegen`."
            );
        }

        // Outside any enum the pass is a copy — one tripwire: the derive
        // line every generated type carries opens the decision on the
        // next line. A doc comment starts with `///`, never `#[`.
        if text == DERIVE_LINE {
            pending_derive = Some((line, chunk));
            continue;
        }
        out.push_str(chunk);
    }
    if pending_derive.is_some() || in_vocab.is_some() {
        bail!(
            "{file}: a vocabulary enum opens but never closes before end of \
             file — jtd-codegen always closes every enum it emits, so the \
             file this pass read is not the shape it is pinned to.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `open_with_policies` in `xtask/src/codegen/open_vocabulary.rs` \
             the new shape, then run `cargo xtask codegen`."
        );
    }
    if found != policies.sites {
        bail!(
            "{file}: the resolved schema describes {} enum definition{} but \
             the generated file carries {} vocabulary enum{} — the counts \
             must meet exactly. The tally is the tripwire that keeps the \
             union skip rule honest: a vocabulary that slipped past the \
             scanner must refuse the run, not pass for processed.\n\
             Fix: restore the pinned jtd-codegen version, or teach \
             `open_with_policies` in `xtask/src/codegen/open_vocabulary.rs` \
             the new shape, then run `cargo xtask codegen`.",
            policies.sites,
            if policies.sites == 1 { "" } else { "s" },
            found,
            if found == 1 { "" } else { "s" }
        );
    }
    Ok(out)
}

/// THE UNION SKIP RULE. A `#[derive(Serialize, Deserialize)]` followed by
/// a `#[serde(tag = …)]` attribute labels a discriminator union, and a
/// union is not a vocabulary: no `enum` form in the schema stands behind
/// it, and "opening" one would mean silently accepting an unknown tag —
/// the opposite of the compiler-checked exhaustiveness a tagged union
/// exists to provide. Such an enum is copied through verbatim and counts
/// as no vocabulary; the site-count tripwire at the end of the pass is
/// what keeps the skip honest rather than silent — if the emission shape
/// ever shifts and a real dictionary slips past the scanner, the counts
/// part and the run refuses.
fn labels_a_discriminator_union(text: &str) -> bool {
    text.starts_with("#[serde(tag")
}

/// `pub enum <Ident> {` — the vocabulary-enum opening shape — yields the
/// identifier; everything else (a tag attribute, a struct, a type alias)
/// yields `None` and is copied through.
fn vocabulary_enum_name(text: &str) -> Option<&str> {
    text.strip_prefix("pub enum ")
        .and_then(|rest| rest.strip_suffix(" {"))
        .filter(|name| is_ident(name))
}

/// The wire string of a `#[serde(rename = "…")]` line — strictly that
/// shape (`rename`, ` = `, one quoted string with no embedded quote): a
/// sibling attribute the pinned emission does not carry here (say
/// `rename_all`) returns `None`, and the caller refuses rather than
/// guesses.
fn rename_wire(text: &str) -> Option<&str> {
    let wire = text
        .strip_prefix("#[serde(rename = \"")?
        .strip_suffix("\")]")?;
    if wire.contains('"') {
        return None;
    }
    Some(wire)
}

/// ASCII identifier shape — the same contract `check_module_ident`
/// enforces for module names, local here because it polices a different
/// surface: an enum or variant identifier the pass is about to rewrite
/// around.
fn is_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Emit the open form of one vocabulary enum — PROP-044 §4.2a's
/// contract: the known variants in their file order, then
/// `Unknown(String)`, then hand-rolled `Serialize` / `Deserialize` (the
/// derive comes off, or it would collide with the manual impls). Every
/// known wire string lands verbatim on both sides, so the bytes of a
/// known value never move; an unknown one is carried as the string
/// itself.
///
/// The emission is LF, stated rather than assumed: `.gitattributes`
/// pins the whole tree to `eol=lf` (for `content_hash` stability) and
/// the generator emits LF (measured — no `\r` in any generated file), so
/// this pass writes the endings it will be checked against. The boxing
/// pass, which rewrites a line in place rather than emitting new ones,
/// preserves whatever ending it found; the asymmetry is real and this is
/// why it is harmless.
fn emit_open_enum(
    name: &str,
    variants: &[(String, String)],
    out: &mut String,
    file: &str,
    line: usize,
) -> Result<()> {
    if variants.iter().any(|(ident, _)| ident == "Unknown") {
        bail!(
            "{file}:{line}: the vocabulary enum `{name}` already has a \
             variant named `Unknown` — the open form adds its own \
             `Unknown(String)`, and the collision would corrupt the type \
             instead of refusing.\n\
             Fix: rename the schema's enum value so the generated \
             identifier is not `Unknown`, then run `cargo xtask codegen`."
        );
    }
    // The serde derive comes off — it would collide with the hand-rolled
    // impls below — but nothing else does: the rest of the floor is what
    // every other generated type keeps, and an opened vocabulary has no
    // reason to be the one type that cannot be printed or compared.
    out.push_str(WITHOUT_SERDE);
    out.push('\n');
    out.push_str("pub enum ");
    out.push_str(name);
    out.push_str(" {\n");
    for (ident, _) in variants {
        out.push_str("    ");
        out.push_str(ident);
        out.push_str(",\n");
    }
    out.push_str("    /// A value this build does not know. The string is preserved\n");
    out.push_str("    /// verbatim across a read/write cycle, so an older reader never\n");
    out.push_str("    /// silently drops or rewrites a newer writer's vocabulary\n");
    out.push_str("    /// (PROP-044 §4.2a).\n");
    out.push_str("    Unknown(String),\n");
    out.push_str("}\n\n");
    out.push_str("impl Serialize for ");
    out.push_str(name);
    out.push_str(" {\n");
    out.push_str("    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>\n");
    out.push_str("    where\n");
    out.push_str("        S: serde::Serializer,\n");
    out.push_str("    {\n");
    out.push_str("        let wire: &str = match self {\n");
    for (ident, wire) in variants {
        out.push_str("            ");
        out.push_str(name);
        out.push_str("::");
        out.push_str(ident);
        out.push_str(" => \"");
        out.push_str(wire);
        out.push_str("\",\n");
    }
    out.push_str("            ");
    out.push_str(name);
    out.push_str("::Unknown(value) => value.as_str(),\n");
    out.push_str("        };\n");
    out.push_str("        serializer.serialize_str(wire)\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("impl<'de> Deserialize<'de> for ");
    out.push_str(name);
    out.push_str(" {\n");
    out.push_str("    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>\n");
    out.push_str("    where\n");
    out.push_str("        D: serde::Deserializer<'de>,\n");
    out.push_str("    {\n");
    out.push_str("        let wire = String::deserialize(deserializer)?;\n");
    out.push_str("        Ok(match wire.as_str() {\n");
    for (ident, wire) in variants {
        out.push_str("            \"");
        out.push_str(wire);
        out.push_str("\" => ");
        out.push_str(name);
        out.push_str("::");
        out.push_str(ident);
        out.push_str(",\n");
    }
    out.push_str("            _ => ");
    out.push_str(name);
    out.push_str("::Unknown(wire),\n");
    out.push_str("        })\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    Ok(())
}

#[cfg(test)]
#[path = "open_vocabulary/tests.rs"]
mod tests;
