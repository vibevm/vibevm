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
//! schema declares what it pulls in via `metadata.x-vocabularies`, and a
//! fragment may declare dependencies of its own by the same key.
//! [`Vocabularies::resolve`] materialises the document the generator
//! sees — the schema's own definitions plus the transitive closure of
//! the named fragments, each placed with its `x-vocabularies` key
//! stripped (the bookkeeping it names is already executed) and in
//! sorted name order — as a scratch copy, leaving the authored schema
//! untouched, and hands back the closure it placed beside the copy: the
//! same substitution seen from the schema side is the map the shared
//! module's phase consumes (a fragment placed into N schemas would
//! otherwise be emitted N times, and [`Vocabularies::shared_schema`]
//! exists so it is emitted once). The same pass refuses, with a
//! recipe, every input that
//! would otherwise reach jtd-codegen as a panic: a `{"ref": "x"}` with
//! no matching definition dies inside the binary with `no entry found
//! for key`, naming neither the schema nor the name — and a dependency
//! chain that leaves the home, or loops back on itself, is refused with
//! the route it took.

use std::collections::BTreeSet;
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

/// The scratch file name of the synthetic shared document. The
/// generator mints the parasitic root's name from the stem, so the stem
/// and the module name are one decision: `shared.jtd.json` →
/// `generated::shared` with the alias `pub type Shared = …` the
/// shared-module phase strips.
pub(crate) const SHARED_STEM: &str = "shared.jtd.json";

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

/// The document the generator reads for one schema, plus the vocabulary
/// closure that document carries — the pair the driver used to reduce to
/// the path alone, dropping the closure on the floor. The names are the
/// shared-module phase's map: the fragments a schema's module must
/// re-export from `generated::shared` rather than redeclare, so `a::
/// VersionEntry` and `b::VersionEntry` are one type, not two that happen
/// to look alike.
#[derive(Debug)]
pub(crate) struct Resolved {
    /// The path handed to the generator: the authored schema when it
    /// declares no vocabularies, the scratch copy otherwise.
    pub(crate) doc: PathBuf,
    /// Every fragment placed into the document — named by the schema's
    /// `metadata.x-vocabularies` or arriving with one it named.
    pub(crate) vocabularies: BTreeSet<String>,
    /// Fragments consumed through ordinary, unprojected references. Shared
    /// reader policy is computed from this set; projected-only consumption is
    /// validated separately and must not manufacture an ordinary mixed role.
    pub(crate) ordinary_vocabularies: BTreeSet<String>,
    /// Every consumer-site permissive projection, including the transitive
    /// fragment closure its generated adapter derives from.
    pub(crate) projections: Vec<super::reader_projection::ProjectionUse>,
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
        super::reader_projection::reject_vocabulary_markers(&fragments, home)?;
        Ok(Self {
            home: home.to_path_buf(),
            fragments,
            scratch: tempfile::tempdir().context("creating the codegen scratch dir")?,
            issued: 0,
        })
    }

    /// Resolve one schema to the document the generator should read: the
    /// schema's own path when it declares no vocabularies, otherwise a
    /// scratch copy whose `definitions` carry the transitive closure of
    /// the fragments named in `metadata.x-vocabularies` — a fragment may
    /// pull fragments of its own by the same key, and they arrive with
    /// it, unnamed by the schema, placed in sorted name order so one
    /// input renders one document. The schema on disk is never
    /// rewritten. The closure comes back beside the copy, because the
    /// driver's next question is which blocks of the emitted module that
    /// substitution makes redundant.
    ///
    /// Every schema passes the dangling-`ref` check, annotated or not —
    /// an unresolved reference is fatal inside the binary either way,
    /// and this is the only place positioned to say which file and which
    /// name, instead of letting a panic say nothing.
    pub(crate) fn resolve(&mut self, schema: &Path) -> Result<Resolved> {
        let text = std::fs::read_to_string(schema)
            .with_context(|| format!("reading schema {}", schema.display()))?;
        let mut doc: Value =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", schema.display()))?;
        let fragment_names: BTreeSet<String> = self.fragments.keys().cloned().collect();
        let mut projection = super::reader_projection::scan_schema(&doc, schema, &fragment_names)?;

        let Some(annotation) = doc
            .get("metadata")
            .and_then(|metadata| metadata.get("x-vocabularies"))
            .cloned()
        else {
            check_dangling_refs(&doc, schema)?;
            return Ok(Resolved {
                doc: schema.to_path_buf(),
                vocabularies: BTreeSet::new(),
                ordinary_vocabularies: BTreeSet::new(),
                projections: Vec::new(),
            });
        };
        let names = expect_name_array(&annotation, &format!("schema {}", schema.display()))?;
        // Every fragment the annotation reaches — through fragments' own
        // dependencies too. The cycle, chain and missing-name refusals
        // fire in there, before anything is placed.
        let closed = self.closure(&names, schema)?;
        let mut projected = BTreeSet::new();
        for usage in &mut projection.uses {
            if !closed.contains(&usage.target) {
                bail!(
                    "schema {} at {} projects `{}`, but that fragment is not in the closure declared by `metadata.x-vocabularies`.\nFix: add `{}` to the schema's vocabulary closure, then run `cargo xtask codegen`.",
                    schema.display(),
                    usage.location,
                    usage.target,
                    usage.target
                );
            }
            usage.closure = self.closure(std::slice::from_ref(&usage.target), schema)?;
            projected.extend(usage.closure.iter().cloned());
        }
        let mut ordinary_vocabularies = closed
            .difference(&projected)
            .cloned()
            .collect::<BTreeSet<_>>();
        for root in &projection.ordinary_roots {
            if !closed.contains(root) {
                // A schema-local definition may have the same name as an
                // unrelated global fragment. Only names this schema actually
                // pulled from the vocabulary home participate in shared policy.
                continue;
            }
            ordinary_vocabularies.extend(
                self.closure(std::slice::from_ref(root), schema)?
                    .into_iter(),
            );
        }
        // The schema's own definitions, snapshotted before placement: the
        // collision refusal below must fire on what the author wrote, not
        // on a fragment an earlier iteration placed (the closure may
        // deliver one name by several routes — a diamond — which is
        // legal; clobbering the author's own definition is not).
        let authored: BTreeSet<String> = doc
            .get("definitions")
            .and_then(Value::as_object)
            .map(|own| own.keys().cloned().collect())
            .unwrap_or_default();

        // Place the closed fragments. `definitions` may be absent — a
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
        // Sorted by name — the set's iteration order — so the traversal's
        // visiting order never shows up in the rendered document.
        for name in &closed {
            if authored.contains(name) {
                bail!(
                    "schema {}: vocabulary `{name}` collides with the \
                     schema's own `definitions.{name}` — a substitution must \
                     not silently overwrite a definition the schema carries.\n\
                     Fix: rename the definition, or stop pulling `{name}` in — \
                     directly in the schema's `metadata.x-vocabularies` or \
                     through the fragment that names it — then run \
                     `cargo xtask codegen`.",
                    schema.display()
                );
            }
            let fragment = self
                .fragments
                .get(name)
                .expect("the closure yields only names it saw in the home");
            definitions.insert(name.clone(), fragment_for_definitions(fragment));
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
        Ok(Resolved {
            doc: copy,
            vocabularies: closed,
            ordinary_vocabularies,
            projections: projection.uses,
        })
    }

    /// The synthetic document for the shared module: `definitions` are
    /// EVERY fragment of the home, and there is no root form — so the
    /// generator emits each fragment's type once (it emits all
    /// `definitions`, reachable or not, measured) plus one parasitic
    /// root alias, `pub type Shared = Option<Value>;`, the stem of the
    /// scratch copy minting the name, which the shared-module phase
    /// strips. Every fragment is placed through the same
    /// `fragment_for_definitions` cut `resolve` places through, so a
    /// block here comes out byte-identical to the copies the schema
    /// modules carry — the property the replacement pass stitches on.
    /// The document lives in the same scratch area as the resolved
    /// schema copies; nothing is written into the repository.
    pub(crate) fn shared_schema(&mut self) -> Result<PathBuf> {
        let mut definitions = Map::new();
        for (name, fragment) in &self.fragments {
            definitions.insert(name.clone(), fragment_for_definitions(fragment));
        }
        let mut root = Map::new();
        root.insert("definitions".to_string(), Value::Object(definitions));
        let doc = Value::Object(root);
        // A fragment nobody pulls can still dangle a `ref` — every
        // schema-side check above ran on documents that name their
        // fragments, and this one names them all. The home is the file
        // to name in the refusal.
        check_dangling_refs(&doc, &self.home)?;

        let copy_dir = self.scratch.path().join(format!("{:04}", self.issued));
        self.issued += 1;
        std::fs::create_dir_all(&copy_dir)
            .with_context(|| format!("creating {}", copy_dir.display()))?;
        let copy = copy_dir.join(SHARED_STEM);
        let rendered = serde_json::to_string_pretty(&doc)
            .context("rendering the synthetic shared document")?;
        std::fs::write(&copy, rendered)
            .with_context(|| format!("writing the synthetic shared document {}", copy.display()))?;
        Ok(copy)
    }

    /// The transitive closure of `names` over fragment dependencies: a
    /// fragment's own `metadata.x-vocabularies` names fragments the
    /// schema never mentioned, and they arrive with it, however deep the
    /// chain runs. Traversal follows declaration order, so a refusal
    /// retells the chain as the author wrote it; the set's sorted
    /// iteration is the placement order `resolve` relies on.
    fn closure(&self, names: &[String], schema: &Path) -> Result<BTreeSet<String>> {
        let mut closed = BTreeSet::new();
        for name in names {
            self.walk(name, &mut Vec::new(), &mut closed, schema)?;
        }
        Ok(closed)
    }

    /// One step of the closure walk: verify `name` in the home, take its
    /// dependencies, descend. `path` is the route from a name the schema
    /// declared down to here — the retelling every refusal along this
    /// walk needs; `closed` carries names an earlier branch already
    /// placed, which ends the walk where routes rejoin (a diamond)
    /// instead of following them again.
    fn walk(
        &self,
        name: &str,
        path: &mut Vec<String>,
        closed: &mut BTreeSet<String>,
        schema: &Path,
    ) -> Result<()> {
        if let Some(looped_at) = path.iter().position(|seen| seen.as_str() == name) {
            let mut route: Vec<&str> = path[looped_at..].iter().map(String::as_str).collect();
            route.push(name);
            bail!(
                "schema {}: the vocabulary chain `{}` is a cycle — a \
                 fragment cannot be substituted through itself.\n\
                 Fix: break the loop — remove `{}` from the \
                 `metadata.x-vocabularies` of one fragment along that chain \
                 in {} — then run `cargo xtask codegen`.",
                schema.display(),
                route.join(" -> "),
                name,
                self.home.display()
            );
        }
        if closed.contains(name) {
            return Ok(());
        }
        let Some(fragment) = self.fragments.get(name) else {
            if path.is_empty() {
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
            }
            // A fragment's dependency named a stranger: retell the chain
            // that led there, or the author hunts through the schema for
            // a word they never wrote in it.
            let parent = path[path.len() - 1].as_str();
            let mut chain = format!("`metadata.x-vocabularies` pulls `{}`", path[0]);
            for hop in &path[1..] {
                chain.push_str(&format!(", which pulls `{hop}`"));
            }
            chain.push_str(&format!(", which pulls `{name}`"));
            bail!(
                "schema {}: {chain}, but the vocabulary home {} has no \
                 `{name}`.\n\
                 Fix: add a `{name}` entry to {} (or drop `{name}` from \
                 `{parent}`'s `metadata.x-vocabularies`), then run \
                 `cargo xtask codegen`.",
                schema.display(),
                self.home.display(),
                self.home.display()
            );
        };
        let deps = dependencies(fragment, name, &self.home)?;
        path.push(name.to_string());
        for dependency in &deps {
            self.walk(dependency, path, closed, schema)?;
        }
        path.pop();
        closed.insert(name.to_string());
        Ok(())
    }
}

/// `metadata.x-vocabularies` must be an array of vocabulary names —
/// anything else is a broken annotation, and tolerating it (say,
/// accepting a bare string) would only move the failure somewhere less
/// legible. Schemas and fragments carry the key alike, so the refusal
/// names whichever side is being read.
fn expect_name_array(annotation: &Value, where_: &str) -> Result<Vec<String>> {
    let Some(items) = annotation.as_array() else {
        bail!(
            "{where_}: `metadata.x-vocabularies` must be an array of \
             vocabulary names (strings), but it is {}.\n\
             Fix: write e.g. `\"x-vocabularies\": [\"package_kind\"]`, then \
             run `cargo xtask codegen`.",
            json_kind(annotation)
        );
    };
    let mut names = Vec::with_capacity(items.len());
    for item in items {
        let Some(name) = item.as_str() else {
            bail!(
                "{where_}: `metadata.x-vocabularies` must be an array of \
                 vocabulary names (strings), but the array lists {}.\n\
                 Fix: write e.g. `\"x-vocabularies\": [\"package_kind\"]`, \
                 then run `cargo xtask codegen`.",
                json_kind(item)
            );
        };
        names.push(name.to_string());
    }
    Ok(names)
}

/// The dependencies a fragment declares by its own
/// `metadata.x-vocabularies` — the same key, the same shape discipline,
/// read from the fragment side of the home. A fragment without the key
/// is a leaf and closes over nothing.
fn dependencies(fragment: &Value, name: &str, home: &Path) -> Result<Vec<String>> {
    let Some(annotation) = fragment
        .get("metadata")
        .and_then(|metadata| metadata.get("x-vocabularies"))
    else {
        return Ok(Vec::new());
    };
    expect_name_array(
        annotation,
        &format!("vocabulary `{name}` in {}", home.display()),
    )
}

/// The fragment as it may enter a schema's `definitions`: its
/// `metadata.x-vocabularies` is this layer's bookkeeping, already
/// executed by the time the fragment is placed, and a reader of the
/// resolved document must not see an instruction someone has carried
/// out. Only that key goes — the fragment's remaining `metadata` stays
/// as authored, and a `metadata` the removal empties goes with it.
fn fragment_for_definitions(fragment: &Value) -> Value {
    let mut placed = fragment.clone();
    let Some(root) = placed.as_object_mut() else {
        return placed;
    };
    if let Some(metadata) = root.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.remove("x-vocabularies");
        if metadata.is_empty() {
            root.remove("metadata");
        }
    }
    placed
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

// The transitive-closure half of the suite — split from `tests.rs` by
// the same `#[path]` idiom when it outgrew the 600-line budget, along
// the seam between today's substitution guarantees and F41A2's.
#[cfg(test)]
#[path = "vocabulary/tests_transitive.rs"]
mod tests_transitive;
