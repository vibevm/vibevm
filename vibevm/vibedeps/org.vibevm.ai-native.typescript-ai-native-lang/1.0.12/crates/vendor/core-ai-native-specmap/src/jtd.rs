//! The JTD-schema side of the scanner (B060) — the mechanism
//! `##RULE-GENERATED-CODE-IS-EXCLUDED` (PROP-014) names but no scanner
//! provided. A `.jtd.json` is a generator *input*, so it — not the
//! `/generated/` Rust it produces — is the taggable unit. Each schema file
//! yields a `schema` unit for its root object and a `schema-def` unit for
//! every `definitions` entry; the `metadata.spec` map ("verb → URI") mirrors
//! `#[spec(verb = "…")]` and mints the unit's edges. The verb dictionary is
//! the same closed set the Rust tags use; an unknown verb is a finding, not
//! silence (the law the markdown excludes already obey).
//!
//! B5 (monotone utility): unreadable or invalid JSON is a warning, never a
//! panic and never a silent skip — a skip would leave the corpus wider than
//! the config says.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#addressing-code");

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;
use walkdir::WalkDir;

use crate::config::Config;
use crate::fwd;
use crate::generated::specmap::{CodeItem, Edge, EdgeProvenance, EdgeVerb, Warning};

mod contained_path;
mod positions;
use contained_path::{bounded_declared_path, resolve_project_file};
use positions::{Span, schema_spans, top_level_member_spans};

#[cfg(test)]
mod tests;

/// The JTD-schema scanner: walks [`Config::schema_roots`] for `*.jtd.json`
/// and turns each into `schema` / `schema-def` units plus the edges their
/// `metadata.spec` declares. Stateless — the policy is read at `scan` time,
/// like [`crate::scanner::RustScanner`]. A no-op when `schema_roots` is
/// empty, so a project with no schema roots is byte-stable against the
/// Rust-only scan.
pub struct JtdScanner;

impl crate::scanner::CodeScanner for JtdScanner {
    fn id(&self) -> &'static str {
        "jtd-schema"
    }
    fn scan(&self, root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        scan_schemas(root, cfg)
    }
}

/// The closed verb dictionary — the same five the Rust `#[spec]` grammar
/// admits (see [`EdgeVerb`]). A `metadata.spec` key outside this set is an
/// `unknown-schema-verb` finding, not silence.
fn parse_verb(s: &str) -> Option<EdgeVerb> {
    use EdgeVerb::*;
    match s {
        "implements" => Some(Implements),
        "verifies" => Some(Verifies),
        "documents" => Some(Documents),
        "deviates" => Some(Deviates),
        "informs" => Some(Informs),
        _ => None,
    }
}

/// A schema is not a Rust crate, so a real crate name would lie; this
/// sentinel says so plainly. The orphan ratchet never reads `code_items`
/// (it walks `.rs` itself, grouping by the directory name — `ratchet.rs`),
/// so it cannot mis-gate a schema on this value.
const SCHEMA_CRATE: &str = "<schema>";

/// One unit's `metadata.spec` entries: `(verb, uri)` for every
/// `verb = "uri"` whose value is a string. Sorted for a deterministic local
/// order regardless of serde_json's map flavour; the index sorts globally
/// too, so this only keeps warnings readable in isolation.
fn spec_entries(unit: &Value) -> Vec<(String, String)> {
    let Some(obj) = unit
        .get("metadata")
        .and_then(|m| m.get("spec"))
        .and_then(|s| s.as_object())
    else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = obj
        .iter()
        .filter_map(|(verb, uri)| uri.as_str().map(|u| (verb.clone(), u.to_string())))
        .collect();
    out.sort();
    out
}

/// Turn one unit's `metadata.spec` entries into edges (known verbs) and
/// `unknown-schema-verb` warnings (the rest). Every edge is anchored at the
/// unit it belongs to — the assertion lives within the unit, and `line` is
/// that unit's measured line (a real position, not a placeholder).
fn record_edges(
    symbol: &str,
    file: &str,
    line: u32,
    entries: &[(String, String)],
    edges: &mut Vec<Edge>,
    warnings: &mut Vec<Warning>,
) {
    for (verb, uri) in entries {
        match parse_verb(verb) {
            Some(v) => edges.push(Edge {
                fromSymbol: symbol.to_string(),
                verb: v,
                uri: uri.clone(),
                // Hand-authored in the schema, mirroring `#[spec]`.
                provenance: EdgeProvenance::Authored,
                file: file.to_string(),
                line,
                // `metadata.spec` is a flat "verb → URI" map: no revision
                // pin and no deviation reason ride on it, so an edge minted
                // here is unpinned (it cannot go suspect) and reason-free.
                pinnedR: None,
                reason: None,
            }),
            None => warnings.push(Warning {
                code: "unknown-schema-verb".to_string(),
                message: format!(
                    "`metadata.spec` verb `{verb}` is not in the closed set \
                     {{implements, verifies, documents, deviates, informs}}"
                ),
                file: file.to_string(),
                line,
            }),
        }
    }
}

/// Build one `CodeItem` for a unit. `line`/`end_line` are measured spans;
/// `fingerprint` is absent — the documented `tok1:<sha256>` is over a Rust
/// token stream (see `fingerprint.rs`), and a JSON schema has none, so
/// minting any hash here would be inventing a different semantic.
fn code_item(symbol: &str, kind: &str, file: &str, span: Span) -> CodeItem {
    CodeItem {
        symbol: symbol.to_string(),
        itemKind: kind.to_string(),
        crateName: SCHEMA_CRATE.to_string(),
        file: file.to_string(),
        line: span.line,
        endLine: Some(Box::new(span.end_line)),
        fingerprint: None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Done,
}

struct VocabularyMember {
    value: Value,
    span: Option<Span>,
}

/// One configured vocabulary, parsed and measured once per scan.
struct SharedVocabulary {
    file: String,
    members: BTreeMap<String, VocabularyMember>,
}

fn warning(code: &str, message: String, file: &str, line: u32) -> Warning {
    Warning {
        code: code.to_string(),
        message,
        file: file.to_string(),
        line,
    }
}

/// Load the configured vocabulary once; failures degrade to typed warnings.
fn load_shared_vocabulary(root: &Path, cfg: &Config) -> (Option<SharedVocabulary>, Vec<Warning>) {
    let Some(declared) = cfg.schema_vocabulary.as_deref() else {
        return (None, Vec::new());
    };
    let resolved = match resolve_project_file(root, declared) {
        Ok(resolved) => resolved,
        Err(error) => {
            return (
                None,
                vec![warning(
                    "invalid-schema-vocabulary-path",
                    format!(
                        "configured shared JTD vocabulary path `{}` is rejected: {error}",
                        bounded_declared_path(declared)
                    ),
                    Config::REL_PATH,
                    0,
                )],
            );
        }
    };
    let file = resolved.provenance;
    let text = match std::fs::read_to_string(&resolved.path) {
        Ok(text) => text,
        Err(err) => {
            return (
                None,
                vec![warning(
                    "unreadable-schema-vocabulary",
                    format!("could not read shared JTD vocabulary: {err}"),
                    &file,
                    0,
                )],
            );
        }
    };
    let value = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(err) => {
            return (
                None,
                vec![warning(
                    "invalid-schema-vocabulary-json",
                    format!("shared JTD vocabulary does not parse as JSON: {err}"),
                    &file,
                    0,
                )],
            );
        }
    };
    let Some(object) = value.as_object() else {
        return (
            None,
            vec![warning(
                "schema-vocabulary-not-object",
                "a shared JTD vocabulary's root must be a JSON object".to_string(),
                &file,
                0,
            )],
        );
    };
    let spans: BTreeMap<String, Span> = top_level_member_spans(&text).into_iter().collect();
    let members = object
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                VocabularyMember {
                    value: value.clone(),
                    span: spans.get(name).copied(),
                },
            )
        })
        .collect();
    (Some(SharedVocabulary { file, members }), Vec::new())
}

/// Narrow predicate: stem, root ref, and declared shared root all match.
fn thin_shared_root<'a>(stem: &str, root: &'a Value) -> Option<&'a str> {
    let object = root.as_object()?;
    if object.contains_key("definitions") {
        return None;
    }
    let root_ref = object.get("ref")?.as_str()?;
    if root_ref != stem {
        return None;
    }
    let declared = object
        .get("metadata")?
        .as_object()?
        .get("x-vocabularies")?
        .as_array()?;
    declared
        .iter()
        .any(|name| name.as_str() == Some(root_ref))
        .then_some(root_ref)
}

fn dependency_names(
    name: &str,
    member: &VocabularyMember,
    vocabulary: &SharedVocabulary,
    warnings: &mut Vec<Warning>,
) -> Option<Vec<String>> {
    let Some(object) = member.value.as_object() else {
        warnings.push(warning(
            "schema-vocabulary-member-not-object",
            format!("shared JTD vocabulary member `{name}` must be an object"),
            &vocabulary.file,
            member.span.map_or(0, |span| span.line),
        ));
        return None;
    };
    let Some(metadata) = object.get("metadata") else {
        return Some(Vec::new());
    };
    let Some(metadata) = metadata.as_object() else {
        warnings.push(warning(
            "schema-vocabulary-metadata-not-object",
            format!("shared JTD vocabulary member `{name}` has non-object metadata"),
            &vocabulary.file,
            member.span.map_or(0, |span| span.line),
        ));
        return Some(Vec::new());
    };
    let Some(dependencies) = metadata.get("x-vocabularies") else {
        return Some(Vec::new());
    };
    let Some(dependencies) = dependencies.as_array() else {
        warnings.push(warning(
            "schema-vocabulary-dependencies-not-array",
            format!("shared JTD vocabulary member `{name}` has non-array `x-vocabularies`"),
            &vocabulary.file,
            member.span.map_or(0, |span| span.line),
        ));
        return Some(Vec::new());
    };
    let mut names = Vec::new();
    for dependency in dependencies {
        if let Some(dependency) = dependency.as_str() {
            names.push(dependency.to_string());
        } else {
            warnings.push(warning(
                "schema-vocabulary-dependency-not-string",
                format!("shared JTD vocabulary member `{name}` has a non-string dependency"),
                &vocabulary.file,
                member.span.map_or(0, |span| span.line),
            ));
        }
    }
    Some(names)
}

/// Deterministic closure; malformed/missing/back edges warn and are skipped.
fn visit_member(
    name: &str,
    vocabulary: &SharedVocabulary,
    states: &mut BTreeMap<String, VisitState>,
    closure: &mut Vec<String>,
    warnings: &mut Vec<Warning>,
) {
    match states.get(name) {
        Some(VisitState::Done) => return,
        Some(VisitState::Visiting) => {
            let line = vocabulary
                .members
                .get(name)
                .and_then(|member| member.span)
                .map_or(0, |span| span.line);
            warnings.push(warning(
                "schema-vocabulary-cycle",
                format!("shared JTD vocabulary dependency cycle returns to `{name}`"),
                &vocabulary.file,
                line,
            ));
            return;
        }
        None => {}
    }
    let Some(member) = vocabulary.members.get(name) else {
        warnings.push(warning(
            "missing-schema-vocabulary-member",
            format!("shared JTD vocabulary member `{name}` is not defined"),
            &vocabulary.file,
            0,
        ));
        return;
    };
    states.insert(name.to_string(), VisitState::Visiting);
    let Some(dependencies) = dependency_names(name, member, vocabulary, warnings) else {
        states.insert(name.to_string(), VisitState::Done);
        return;
    };
    for dependency in dependencies {
        visit_member(&dependency, vocabulary, states, closure, warnings);
    }
    states.insert(name.to_string(), VisitState::Done);
    closure.push(name.to_string());
}

fn project_shared_root(
    stem: &str,
    shared_root: &str,
    vocabulary: &SharedVocabulary,
    items: &mut Vec<CodeItem>,
    edges: &mut Vec<Edge>,
    warnings: &mut Vec<Warning>,
) {
    let mut states = BTreeMap::new();
    let mut closure = Vec::new();
    visit_member(shared_root, vocabulary, &mut states, &mut closure, warnings);
    for fragment in closure {
        let Some(member) = vocabulary.members.get(&fragment) else {
            continue;
        };
        let Some(span) = member.span else {
            warnings.push(warning(
                "unmeasured-schema-vocabulary-member",
                format!("could not measure shared JTD vocabulary member `{fragment}`"),
                &vocabulary.file,
                0,
            ));
            continue;
        };
        if fragment == shared_root {
            record_edges(
                stem,
                &vocabulary.file,
                span.line,
                &spec_entries(&member.value),
                edges,
                warnings,
            );
        } else {
            let symbol = format!("{stem}::{fragment}");
            items.push(code_item(&symbol, "schema-def", &vocabulary.file, span));
            record_edges(
                &symbol,
                &vocabulary.file,
                span.line,
                &spec_entries(&member.value),
                edges,
                warnings,
            );
        }
    }
}

/// Scan one schema file's text into units + edges + warnings. `file` is the
/// forward-slash repo-relative path; `stem` is the filename without
/// `.jtd.json` (the symbol prefix). The string-level seam the unit tests
/// drive, parallel to [`crate::rscan::scan_source`].
pub fn scan_schema_text(
    file: &str,
    stem: &str,
    text: &str,
) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
    scan_schema_text_with_vocabulary(file, stem, text, None)
}

fn scan_schema_text_with_vocabulary(
    file: &str,
    stem: &str,
    text: &str,
    vocabulary: Option<&SharedVocabulary>,
) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
    let root_value = match serde_json::from_str::<Value>(text) {
        Ok(v) => v,
        Err(e) => {
            return (
                Vec::new(),
                Vec::new(),
                vec![Warning {
                    code: "invalid-schema-json".to_string(),
                    message: format!("JTD schema does not parse as JSON: {e}"),
                    file: file.to_string(),
                    line: 0,
                }],
            );
        }
    };
    let Some(obj) = root_value.as_object() else {
        return (
            Vec::new(),
            Vec::new(),
            vec![Warning {
                code: "schema-not-object".to_string(),
                message: "a JTD schema's root must be a JSON object".to_string(),
                file: file.to_string(),
                line: 0,
            }],
        );
    };

    // Positions: a second structural pass measures line numbers (serde_json
    // discards them). Valid JSON in, so the walker needs no error recovery.
    let (root_span, def_spans) = schema_spans(text);
    let root_span = root_span.unwrap_or(Span {
        line: 1,
        end_line: 1,
    });

    let mut items = Vec::new();
    let mut edges = Vec::new();
    let mut warnings = Vec::new();

    // Root unit — always inventoried (the schema document is itself a unit).
    items.push(code_item(stem, "schema", file, root_span));
    record_edges(
        stem,
        file,
        root_span.line,
        &spec_entries(&root_value),
        &mut edges,
        &mut warnings,
    );

    // Definition units — each `definitions` entry is its own unit.
    if let Some(defs) = obj.get("definitions").and_then(|d| d.as_object()) {
        for (name, def_value) in defs {
            let span = def_spans
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, s)| *s)
                .unwrap_or(root_span);
            let symbol = format!("{stem}::{name}");
            items.push(code_item(&symbol, "schema-def", file, span));
            record_edges(
                &symbol,
                file,
                span.line,
                &spec_entries(def_value),
                &mut edges,
                &mut warnings,
            );
        }
    }

    if let (Some(vocabulary), Some(shared_root)) = (vocabulary, thin_shared_root(stem, &root_value))
    {
        project_shared_root(
            stem,
            shared_root,
            vocabulary,
            &mut items,
            &mut edges,
            &mut warnings,
        );
    }

    (items, edges, warnings)
}

/// A `.jtd.json` file — the same literal-extension law the Rust and markdown
/// scanners apply (`rs` / `md`), for the schema family. A bare `.json` does
/// not match: the scanner targets JTD specifically, the format the engine's
/// own wire types are generated from.
fn is_jtd_json(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".jtd.json"))
}

/// The schema-file stem — the filename with `.jtd.json` stripped — the
/// symbol prefix for the root unit and every definition under it.
fn schema_stem(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".jtd.json"))
        .unwrap_or("")
        .to_string()
}

/// Walk each [`Config::schema_roots`] for `**/*.jtd.json`, deterministically
/// (sorted) — mirroring [`crate::mdspec::scan_spec_tree`]'s walk over
/// `spec_roots`, for the schema family.
fn scan_schemas(root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
    let mut items = Vec::new();
    let mut edges = Vec::new();
    let (vocabulary, mut warnings) = load_shared_vocabulary(root, cfg);
    for schema_root_rel in &cfg.schema_roots {
        let schema_root = root.join(schema_root_rel);
        for entry in WalkDir::new(&schema_root)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !is_jtd_json(path) {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(path);
            let file = fwd(rel);
            let stem = schema_stem(path);
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    let (mut i, mut e, mut w) =
                        scan_schema_text_with_vocabulary(&file, &stem, &text, vocabulary.as_ref());
                    items.append(&mut i);
                    edges.append(&mut e);
                    warnings.append(&mut w);
                }
                Err(err) => warnings.push(Warning {
                    code: "unreadable-file".to_string(),
                    message: format!("could not read: {err}"),
                    file,
                    line: 0,
                }),
            }
        }
    }
    (items, edges, warnings)
}
