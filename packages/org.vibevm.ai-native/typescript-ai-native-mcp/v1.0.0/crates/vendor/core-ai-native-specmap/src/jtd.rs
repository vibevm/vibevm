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

use std::path::Path;

use serde_json::Value;
use walkdir::WalkDir;

use crate::config::Config;
use crate::fwd;
use crate::generated::specmap::{CodeItem, Edge, EdgeProvenance, EdgeVerb, Warning};

mod positions;
use positions::{Span, schema_spans};

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

/// Scan one schema file's text into units + edges + warnings. `file` is the
/// forward-slash repo-relative path; `stem` is the filename without
/// `.jtd.json` (the symbol prefix). The string-level seam the unit tests
/// drive, parallel to [`crate::rscan::scan_source`].
pub fn scan_schema_text(
    file: &str,
    stem: &str,
    text: &str,
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
    let mut warnings = Vec::new();
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
                    let (mut i, mut e, mut w) = scan_schema_text(&file, &stem, &text);
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
