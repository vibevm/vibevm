//! The strictness pass — reader strictness is assigned by the REGISTRY
//! of formats — the sixth content edit the generator's emission takes
//! (the order rule lives in `postproc`'s docs: a pass keyed to the
//! emission shape runs while the file is STILL that emission, and this
//! one is keyed to the `#[derive(Serialize, Deserialize)]` line
//! directly above a `pub struct … {` line — the exact place the
//! attribute is inserted — so it runs after arm boxing, field
//! snake_casing, map ordering, the empty-collection policy and the
//! optional-shape pass, and before the vocabularies open).
//!
//! Effective `unknown_fields` comes from the existing registry loader. Explicit
//! deny/allow overrides parser population; absent legacy policy still defaults
//! to deny for none and allow for ours/many. Struct payloads (including empty
//! discriminator wrappers) own unknown-field refusal; enums are not stamped.
//!
//! One map is built per run. Duplicate owners must agree on effective policy;
//! schema-less formats are silent, future schema paths are named skips, and
//! unregistered schemas refuse. Registry parsing is never duplicated here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use super::format_id::load_format_registry;

/// The line the pinned emission derives every generated type with.
const DERIVE_LINE: &str = "#[derive(Serialize, Deserialize)]";

/// The attribute the pass stamps. Serde reads container attributes
/// next to the derive, and the form — derive line, then this, then the
/// type — is the same one the generator itself uses for
/// `#[serde(tag = …)]` above unions.
const DENY_LINE: &str = "#[serde(deny_unknown_fields)]";

/// The registry read as one map for the whole run: schema path (in the
/// registry's own spelling — repo-relative, forward slashes) → the
/// effective policy that decides generated structs' reader strictness, plus
/// the id of the record that first claimed the path, which the
/// policy-divergence refusal names alongside the challenger. Built once
/// per run beside `Vocabularies::load`; every schema the driver rules
/// on goes through it, never around it.
pub(crate) struct Strictness {
    /// The repo root the record paths resolve against.
    root: PathBuf,
    /// Schema path → (claiming record id, effective unknown_fields policy).
    policies: BTreeMap<String, (String, String)>,
}

impl Strictness {
    /// Read the registry and print the named skips: the map every
    /// generated file is ruled through, loaded once per codegen run.
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let (strictness, skips) = Self::read(root)?;
        for skip in &skips {
            eprintln!("{skip}");
        }
        Ok(strictness)
    }

    /// The pure half of [`Strictness::load`]: parse the registry
    /// through the one loader, skip what has nothing to rule on,
    /// refuse a policy divergence, and hand the skip lines back as data
    /// so the tests assert exactly what the operator reads.
    fn read(root: &Path) -> Result<(Self, Vec<String>)> {
        let entries = load_format_registry(root)?;
        let mut policies: BTreeMap<String, (String, String)> = BTreeMap::new();
        let mut skips: Vec<String> = Vec::new();
        for entry in &entries {
            // `schema = "none"`: an authored format the generator never
            // sees — no output, no structs, no policy. Skipped without
            // a word: there is nothing to name.
            if entry.schema == "none" {
                continue;
            }
            // A record naming a schema no phase has built yet (the
            // plan-assigned paths of later phases): skipped BY NAME,
            // because the schema scanner cannot see the file and
            // silence would read as checked. Not an error — the
            // registry is allowed to pre-register the future.
            if !root.join(&entry.schema).is_file() {
                skips.push(format!(
                    "  - strictness: [format.{}] names schema `{}`, which no \
                     phase has built yet — no reader-strictness policy applied",
                    entry.id, entry.schema
                ));
                continue;
            }
            if let Some((first_id, policy)) = policies.get(&entry.schema) {
                if *policy != entry.unknown_fields {
                    bail!(
                        "formats/REGISTRY.toml: `[format.{first_id}]` \
                         (`unknown_fields = \"{policy}\"`) and `[format.{}]` \
                         (`unknown_fields = \"{}\"`) share the schema `{}` — \
                         reader strictness is a property of the FORMAT, and one \
                         schema claimed by records that disagree has no single \
                         policy. The registry refuses to pick a winner.\n\
                         Fix: give both records the same `unknown_fields` \
                         policy, or point one at a schema of its own, then run \
                         `cargo xtask codegen`.",
                        entry.id,
                        entry.unknown_fields,
                        entry.schema
                    );
                }
                // Same schema, same policy (the `index-entry` /
                // `index-primary` shape): legal and silent.
                continue;
            }
            policies.insert(
                entry.schema.clone(),
                (entry.id.clone(), entry.unknown_fields.clone()),
            );
        }
        Ok((
            Self {
                root: root.to_path_buf(),
                policies,
            },
            skips,
        ))
    }

    /// The effective policy the registry assigns to `schema`, or `None` when no
    /// record claims it. The key is normalised to the registry's own
    /// spelling — repo-relative, forward slashes — so the lookup is the
    /// same act on Windows and POSIX.
    fn policy_for(&self, schema: &Path) -> Option<&str> {
        let rel = schema.strip_prefix(&self.root).unwrap_or(schema);
        let key = rel.display().to_string().replace('\\', "/");
        self.policies.get(&key).map(|(_, policy)| policy.as_str())
    }
}

/// The pass entry the driver calls: rule on the schema through the
/// registry, then stamp or copy. Returning `src` unchanged for a policy
/// other than `deny` is the contract, not a shortcut — those bytes do
/// not move.
pub(super) fn apply_strictness(
    src: &str,
    file: &str,
    schema: &Path,
    strictness: &Strictness,
) -> Result<String> {
    let Some(policy) = strictness.policy_for(schema) else {
        bail!(
            "schema {}: no `[format.*]` record in formats/REGISTRY.toml \
             names it, so its generated output has no effective \
             unknown-field policy and no reader strictness can be computed. An \
             unregistered format is inexpressible by design (PROP-044 §4.1 \
             `##M-FORMAT-REGISTRY`); skipping it silently would be a hole \
             exactly where the registry exists.\n\
             Fix: add a `[format.<id>]` record naming this schema, then run \
             `cargo xtask codegen`.",
            schema.display()
        );
    };
    if policy != "deny" {
        return Ok(src.to_string());
    }
    stamp_deny_unknown_fields(src, file)
}

/// Stamp `#[serde(deny_unknown_fields)]` between the derive line and
/// every `pub struct … {` of the file. Enums are not touched, and
/// everything outside the derive/struct pair is copied byte for byte,
/// layout and line endings included. A struct with no derive line
/// directly above refuses loudly: the emission shape is pinned by the
/// generator's version, and stamping a struct the pass cannot explain
/// would hide a moved pin behind a green run.
fn stamp_deny_unknown_fields(src: &str, file: &str) -> Result<String> {
    let mut out = String::with_capacity(src.len() + src.len() / 16);
    // What the line just copied was: a bare derive line waiting for the
    // type it derives (carrying the indent and line ending the stamp
    // reuses), or a derive line already wearing this pass's attribute
    // (the second run over the pass's own output — the identity that
    // keeps `check-codegen` stable).
    let mut pending: Option<Pending> = None;
    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        let line = index + 1;
        // `chunk` keeps its line ending (`\n`, `\r\n`, or nothing at
        // EOF); `body` is the line without it, `text` the line trimmed.
        let body = chunk.trim_end_matches(['\r', '\n']);
        let text = body.trim();

        if let Some(state) = pending.take() {
            match state {
                Pending::Stamped => {
                    // The attribute is already there; the struct it
                    // belongs to — or an enum, or whatever follows — is
                    // copied without a second stamp.
                    out.push_str(chunk);
                    continue;
                }
                Pending::Derived(indent, ending) => {
                    if text == DENY_LINE {
                        pending = Some(Pending::Stamped);
                        out.push_str(chunk);
                        continue;
                    }
                    if is_struct_line(text) {
                        out.push_str(&indent);
                        out.push_str(DENY_LINE);
                        out.push_str(&ending);
                        out.push_str(chunk);
                        continue;
                    }
                    // The derive line belongs to an enum (or whatever
                    // follows): not this pass's business — enums carry
                    // no deny attribute.
                    out.push_str(chunk);
                    continue;
                }
            }
        }

        if is_struct_line(text) {
            bail!(
                "{file}:{line}: `pub struct` with no `{DERIVE_LINE}` line \
                 directly above — the strictness pass stamps every struct of \
                 a `unknown_fields = \"deny\"` format by inserting \
                 `{DENY_LINE}` right after the derive line, and the emission \
                 shape of jtd-codegen this pass is pinned to has moved, so \
                 the pass refuses to guess which struct is which.\n\
                 Fix: restore the pinned jtd-codegen version, or teach \
                 `strictness.rs` the new shape, then run `cargo xtask \
                 codegen`."
            );
        }
        if text == DERIVE_LINE {
            let indent = body[..body.len() - body.trim_start().len()].to_string();
            let ending = chunk[body.len()..].to_string();
            pending = Some(Pending::Derived(indent, ending));
        }
        out.push_str(chunk);
    }
    Ok(out)
}

/// Where the stamping walker stands relative to the derive line it may
/// stamp under.
enum Pending {
    /// The derive line was just copied; the next line decides whether a
    /// struct opens here — carrying the derive line's indent and line
    /// ending for the stamp.
    Derived(String, String),
    /// The derive line AND this pass's attribute are already in place,
    /// so the struct that follows is done — insert nothing.
    Stamped,
}

/// A line opening a generated struct — in the pinned emission,
/// `pub struct <Ident> {`. Matched by prefix so a drifted shape lands
/// in the refusal rather than slipping through unstamped.
fn is_struct_line(text: &str) -> bool {
    text.starts_with("pub struct ")
}

#[cfg(test)]
#[path = "strictness/tests.rs"]
mod tests;
