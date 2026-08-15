//! Shared JTD vocabularies — one home, substituted by our layer.
//!
//! JTD cannot express a definition shared across schemas: `ref` resolves
//! only against `definitions` of the same document, and the language has
//! no cross-file reference (no `$id`, no URI, nothing). A vocabulary two
//! schemas need — `package_kind` today — would otherwise be transcribed
//! verbatim into each, which is exactly the duplication PROP-044 §8 (G9)
//! forbids. So the split is ours to make, per PROP-044 §4.2: what the
//! schema language cannot express, our generator emits.
//!
//! The shape: vocabularies live once in `formats/vocabularies.json`
//! (name → the JTD fragment that becomes its `definitions` entry); a
//! schema declares what it pulls in via `metadata.x-vocabularies`;
//! [`Vocabularies::resolve`] materialises the document the generator
//! sees — the schema's own definitions plus the named fragments — as a
//! scratch copy, leaving the authored schema untouched. The same pass
//! refuses, with a recipe, every input that would otherwise reach
//! jtd-codegen as a panic: a `{"ref": "x"}` with no matching definition
//! dies inside the binary with `no entry found for key`, naming neither
//! the schema nor the name.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

/// The vocabulary home, relative to the repo root. `formats/` is the
/// house of data about formats (`REGISTRY.toml`, `hash_recipes/`,
/// `breaks/`); the schema scanner collects `*.jtd.json` under the schema
/// homes only, so a plain `.json` here is vocabulary data, never a
/// schema the generator would try to build as a format of its own.
pub(crate) fn vocabularies_path(root: &Path) -> PathBuf {
    root.join("formats").join("vocabularies.json")
}

/// `formats/vocabularies.json` parsed once per codegen run, plus the
/// scratch area holding resolved schema copies for the generator. The
/// scratch lives exactly as long as the struct — dropping it mid-run
/// would delete the copy a spawned jtd-codegen is reading.
pub(crate) struct Vocabularies {
    /// Where the fragments came from — named in refusals so the fix
    /// points at the file to edit, not just at an abstract home.
    home: PathBuf,
    /// Vocabulary name → the JTD fragment that becomes its
    /// `definitions` entry.
    fragments: Map<String, Value>,
    /// Holds every resolved copy `resolve` has issued.
    scratch: tempfile::TempDir,
    /// Copies issued so far. Each gets its own numbered directory, so
    /// equally named schemas from different homes cannot overwrite each
    /// other's copy.
    issued: usize,
}

impl Vocabularies {
    /// Parse the vocabulary home and prepare the scratch area. The home
    /// is committed state: a missing file is a broken checkout, not an
    /// empty vocabulary — the doctrine the schema homes already follow.
    pub(crate) fn load(home: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(home)
            .with_context(|| format!("reading shared vocabularies at {}", home.display()))?;
        let parsed: Value =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", home.display()))?;
        let fragments = match parsed {
            Value::Object(fragments) => fragments,
            _ => bail!(
                "{}: the vocabulary home must be a JSON object mapping a \
                 vocabulary name to the JTD fragment that becomes its \
                 `definitions` entry, e.g. \
                 `{{\"package_kind\": {{\"enum\": [\"feat\"]}}}}` — found {}.",
                home.display(),
                json_kind(&parsed)
            ),
        };
        Ok(Self {
            home: home.to_path_buf(),
            fragments,
            scratch: tempfile::tempdir().context("creating the codegen scratch dir")?,
            issued: 0,
        })
    }

    /// Resolve one schema to the document the generator should read: the
    /// schema's own path when it declares no vocabularies, otherwise a
    /// scratch copy whose `definitions` carry the fragments named in
    /// `metadata.x-vocabularies`. The schema on disk is never rewritten.
    ///
    /// Every schema passes the dangling-`ref` check, annotated or not —
    /// an unresolved reference is fatal inside the binary either way,
    /// and this is the only place positioned to say which file and which
    /// name, instead of letting a panic say nothing.
    pub(crate) fn resolve(&mut self, schema: &Path) -> Result<PathBuf> {
        let text = std::fs::read_to_string(schema)
            .with_context(|| format!("reading schema {}", schema.display()))?;
        let mut doc: Value =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", schema.display()))?;

        let Some(annotation) = doc
            .get("metadata")
            .and_then(|metadata| metadata.get("x-vocabularies"))
            .cloned()
        else {
            check_dangling_refs(&doc, schema)?;
            return Ok(schema.to_path_buf());
        };
        let names = expect_name_array(&annotation, schema)?;

        // Place the named fragments. `definitions` may be absent — a
        // vocabulary-only schema is legal — in which case it is created;
        // a pre-existing non-object one is invalid JTD with nowhere to
        // put fragments, refused rather than clobbered. (`doc` itself is
        // an object here: `metadata` was just read out of it, so the
        // `None` arm can only be that broken-`definitions` case.)
        let Some(definitions) = doc
            .as_object_mut()
            .map(|root| {
                root.entry("definitions")
                    .or_insert_with(|| Value::Object(Map::new()))
            })
            .and_then(Value::as_object_mut)
        else {
            bail!(
                "schema {}: `definitions` is not an object, so the \
                 vocabularies named in `metadata.x-vocabularies` have nowhere \
                 to be placed.\n\
                 Fix: make `definitions` an object of JTD definitions, then \
                 run `cargo xtask codegen`.",
                schema.display()
            );
        };
        for name in &names {
            let Some(fragment) = self.fragments.get(name) else {
                bail!(
                    "schema {}: `metadata.x-vocabularies` names `{name}`, but \
                     the vocabulary home {} has no `{name}`.\n\
                     Fix: add a `{name}` entry to {} (or drop `{name}` from \
                     the schema's `metadata.x-vocabularies`), then run \
                     `cargo xtask codegen`.",
                    schema.display(),
                    self.home.display(),
                    self.home.display()
                );
            };
            if definitions.contains_key(name) {
                bail!(
                    "schema {}: vocabulary `{name}` collides with the \
                     schema's own `definitions.{name}` — a substitution must \
                     not silently overwrite a definition the schema carries.\n\
                     Fix: rename the definition or remove `{name}` from \
                     `metadata.x-vocabularies`, then run `cargo xtask codegen`.",
                    schema.display()
                );
            }
            definitions.insert(name.clone(), fragment.clone());
        }

        check_dangling_refs(&doc, schema)?;

        let copy_dir = self.scratch.path().join(format!("{:04}", self.issued));
        self.issued += 1;
        std::fs::create_dir_all(&copy_dir)
            .with_context(|| format!("creating {}", copy_dir.display()))?;
        // Keep the schema's own file name (`.jtd.json` tail included) so
        // the copy is indistinguishable from an authored schema to
        // anything that inspects the path.
        let file_name = schema
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "schema.jtd.json".to_string());
        let copy = copy_dir.join(file_name);
        let rendered = serde_json::to_string_pretty(&doc)
            .with_context(|| format!("rendering the resolved copy of {}", schema.display()))?;
        std::fs::write(&copy, rendered)
            .with_context(|| format!("writing the resolved copy {}", copy.display()))?;
        Ok(copy)
    }
}

/// `metadata.x-vocabularies` must be an array of vocabulary names —
/// anything else is a broken annotation, and tolerating it (say,
/// accepting a bare string) would only move the failure somewhere less
/// legible.
fn expect_name_array(annotation: &Value, schema: &Path) -> Result<Vec<String>> {
    let Some(items) = annotation.as_array() else {
        bail!(
            "schema {}: `metadata.x-vocabularies` must be an array of \
             vocabulary names (strings), but it is {}.\n\
             Fix: write e.g. `\"x-vocabularies\": [\"package_kind\"]`, then \
             run `cargo xtask codegen`.",
            schema.display(),
            json_kind(annotation)
        );
    };
    let mut names = Vec::with_capacity(items.len());
    for item in items {
        let Some(name) = item.as_str() else {
            bail!(
                "schema {}: `metadata.x-vocabularies` must be an array of \
                 vocabulary names (strings), but the array lists {}.\n\
                 Fix: write e.g. `\"x-vocabularies\": [\"package_kind\"]`, \
                 then run `cargo xtask codegen`.",
                schema.display(),
                json_kind(item)
            );
        };
        names.push(name.to_string());
    }
    Ok(names)
}

/// Refuse a dangling `ref` — a name that is not in `definitions` after
/// substitution. Measured: this exact input reaches jtd-codegen as a
/// panic (`no entry found for key`) that names neither the schema nor
/// the name, so the refusal belongs here, before the binary is spawned.
fn check_dangling_refs(doc: &Value, schema: &Path) -> Result<()> {
    let definitions = doc.get("definitions").and_then(Value::as_object);
    if let Some(name) = find_dangling_ref(doc, definitions) {
        bail!(
            "schema {}: `{{\"ref\": \"{name}\"}}` does not resolve — `{name}` \
             is in neither this schema's `definitions` nor the vocabularies \
             its `metadata.x-vocabularies` pulls in.\n\
             Fix: declare `{name}` in `metadata.x-vocabularies` (vocabularies \
             live in `formats/vocabularies.json`) or define it in \
             `definitions`, then run `cargo xtask codegen`.",
            schema.display()
        );
    }
    Ok(())
}

/// The first `{"ref": "X"}` whose `X` is not in `definitions`, walking
/// the whole document: references sit at any depth — inside
/// `properties`, `optionalProperties`, `elements`, `values`, `mapping`
/// and `definitions` themselves. `metadata` blocks are annotations the
/// JTD machinery never reads, so they are skipped: a `ref`-shaped object
/// inside one is data, not a reference.
fn find_dangling_ref(value: &Value, definitions: Option<&Map<String, Value>>) -> Option<String> {
    match value {
        Value::Object(fields) => fields.iter().find_map(|(key, field)| {
            if key == "metadata" {
                return None;
            }
            if key == "ref" {
                let Some(name) = field.as_str() else {
                    // Not the reference form; shape validation beyond the
                    // four refusals belongs to the generator.
                    return None;
                };
                let defined = definitions.is_some_and(|defs| defs.contains_key(name));
                return (!defined).then(|| name.to_string());
            }
            find_dangling_ref(field, definitions)
        }),
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_dangling_ref(item, definitions)),
        _ => None,
    }
}

/// The JSON kind of a value, for refusal texts — naming what was found
/// beats making the reader reconstruct it from a parse error.
fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
#[path = "vocabulary/tests.rs"]
mod tests;
