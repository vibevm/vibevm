//! The scanner seam's Go implementation: the §8 `//spec:` directive
//! markers — `implements` / `verifies` / `documents` / `deviates` /
//! `informs` on declarations, `scope` in the package doc block —
//! lowered into the SAME PROP-014 index model the Rust and TypeScript
//! scanners feed. One extractor run (the go-extract bridge) already
//! carries the markers next to the conform facts, so the two gates
//! never parse a file twice.
//!
//! Semantics mirror the siblings with one Go-shaped difference: a
//! `scope` directive is PACKAGE-grain (GUIDE-AI-NATIVE-GO §8 — it
//! lives in the package doc block and covers the package), so the
//! orphan gate treats every file in a scoped package's directory as
//! covered. `deviates` requires a reason; `r=N` pins the edge's
//! revision (carried as its own marker field — the go-extract protocol
//! parses it at the source); a URI the grammar rejects is a warning,
//! never a panic. Symbols are `<file>::<name>`.

specmark::scope!("spec://go-ai-native-lang/go/GUIDE-AI-NATIVE-GO#specmap");

use std::collections::BTreeSet;
use std::path::Path;

use go_ai_native_extract_bridge::{FileRecord, RawMarker};
use specmap_core::config::Config;
use specmap_core::generated::specmap::{CodeItem, Edge, EdgeProvenance, EdgeVerb, Warning};
use specmap_core::scanner::CodeScanner;

fn verb_of(tag: &str) -> Option<EdgeVerb> {
    match tag {
        // `scope` defaults to implements — the grammar's own
        // scope-edge rule, projected.
        "implements" | "scope" => Some(EdgeVerb::Implements),
        "verifies" => Some(EdgeVerb::Verifies),
        "documents" => Some(EdgeVerb::Documents),
        "deviates" => Some(EdgeVerb::Deviates),
        "informs" => Some(EdgeVerb::Informs),
        _ => None,
    }
}

fn warn(file: &str, line: u32, code: &str, message: String) -> Warning {
    Warning {
        code: code.to_string(),
        file: file.to_string(),
        line,
        message,
    }
}

/// Lower one file's markers into `(items, edges, warnings)`. Pure —
/// the replay tests drive it on recorded bridge output.
fn lower_record(
    record: &FileRecord,
    root_name: &str,
    items: &mut Vec<CodeItem>,
    edges: &mut Vec<Edge>,
    warnings: &mut Vec<Warning>,
) {
    for marker in &record.markers {
        let RawMarker {
            tag,
            uri,
            r,
            reason,
            symbol,
            line,
        } = marker;
        let Some(verb) = verb_of(tag) else {
            continue; // not a spec tag; the extractor already filters
        };
        if let Err(e) = specmark_grammar::parse_spec_uri(uri) {
            warnings.push(warn(
                &record.file,
                *line,
                "bad-spec-uri",
                format!("`//spec:{tag} {uri}`: {e}"),
            ));
            continue;
        }
        if matches!(verb, EdgeVerb::Deviates) && reason.is_none() {
            warnings.push(warn(
                &record.file,
                *line,
                "deviates-missing-reason",
                format!("`//spec:deviates {uri}` carries no reason — testimony is mandatory"),
            ));
            continue;
        }
        let (from_symbol, item_kind, item_symbol) = match (tag.as_str(), symbol) {
            ("scope", _) => (
                record.file.clone(),
                "package".to_string(),
                record.file.clone(),
            ),
            (_, Some(name)) => (
                format!("{}::{name}", record.file),
                "decl".to_string(),
                format!("{}::{name}", record.file),
            ),
            (_, None) => (
                record.file.clone(),
                "package".to_string(),
                record.file.clone(),
            ),
        };
        if !items.iter().any(|i: &CodeItem| i.symbol == item_symbol) {
            items.push(CodeItem {
                crateName: root_name.to_string(),
                file: record.file.clone(),
                itemKind: item_kind,
                line: *line,
                symbol: item_symbol,
            });
        }
        edges.push(Edge {
            file: record.file.clone(),
            fromSymbol: from_symbol,
            line: *line,
            provenance: EdgeProvenance::Authored,
            uri: uri.clone(),
            verb,
            pinnedR: r.map(Box::new),
            reason: reason.clone().map(Box::new),
        });
    }
}

/// Lower a whole extraction run. `root_name` becomes the items'
/// `crate_name` (Go has no crates; the scan-root name stands in).
pub fn records_to_index(
    records: &[FileRecord],
    root_name: &str,
) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
    let mut items = Vec::new();
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    for record in records {
        lower_record(record, root_name, &mut items, &mut edges, &mut warnings);
    }
    (items, edges, warnings)
}

/// A [`CodeScanner`] over one already-extracted record set — the shape
/// the CLI uses so the gate and the index share a single go run.
pub struct RecordsScanner<'a> {
    records: &'a [FileRecord],
    root_name: String,
}

impl<'a> RecordsScanner<'a> {
    pub fn new(records: &'a [FileRecord], root_name: &str) -> RecordsScanner<'a> {
        RecordsScanner {
            records,
            root_name: root_name.to_string(),
        }
    }
}

impl CodeScanner for RecordsScanner<'_> {
    fn id(&self) -> &'static str {
        "go-extract"
    }
    fn scan(&self, _root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        let scoped = scoped_records(self.records, cfg);
        records_to_index(&scoped, &self.root_name)
    }
}

/// Restrict records to the policy's scan roots (repo-relative path
/// prefixes; `<dir>/*` counts as `<dir>/`; a `.` root keeps
/// everything).
fn scoped_records(records: &[FileRecord], cfg: &Config) -> Vec<FileRecord> {
    if cfg.scan_roots.iter().any(|r| r == ".") {
        return records.to_vec();
    }
    let prefixes: Vec<String> = cfg
        .scan_roots
        .iter()
        .map(|r| format!("{}/", r.trim_end_matches("/*").trim_end_matches('/')))
        .collect();
    records
        .iter()
        .filter(|r| prefixes.iter().any(|p| r.file.starts_with(p.as_str())))
        .cloned()
        .collect()
}

/// The directory of a repo-relative file — Go's package unit.
fn dir_of(file: &str) -> &str {
    file.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

/// One untagged exported identifier found by the Go orphan gate.
#[derive(Debug)]
pub struct GoOrphan {
    pub file: String,
    pub symbol: String,
    pub item_kind: String,
    pub line: u32,
}

/// The orphan gate, Go shape: an EXPORTED identifier in a non-exempt
/// scan root with no own marker and no PACKAGE-level `//spec:scope`
/// is an orphan (PROP-014 §2.3 projected: unexported helpers need no
/// annotation; the package doc block's scope covers the package's
/// exports — GUIDE-AI-NATIVE-GO §8).
pub fn orphans(records: &[FileRecord], cfg: &Config) -> Vec<GoOrphan> {
    let scoped = scoped_records(records, cfg);
    // scope is package-grain: collect every directory that carries one.
    let scoped_dirs: BTreeSet<&str> = scoped
        .iter()
        .filter(|r| r.markers.iter().any(|m| m.tag == "scope"))
        .map(|r| dir_of(&r.file))
        .collect();
    let mut out = Vec::new();
    for record in &scoped {
        let root = record.file.split('/').next().unwrap_or_default();
        if cfg.exempt.iter().any(|e| e == root) {
            continue;
        }
        if record.in_test {
            continue; // tests verify; they are not public surface
        }
        if scoped_dirs.contains(dir_of(&record.file)) {
            continue;
        }
        for fact in &record.facts {
            let go_ai_native_extract_bridge::RawFact::Item {
                kind,
                symbol,
                line,
                is_exported,
                ..
            } = fact
            else {
                continue;
            };
            if !is_exported {
                continue;
            }
            let tagged = record
                .markers
                .iter()
                .any(|m| m.symbol.as_deref() == Some(symbol.as_str()));
            if !tagged {
                out.push(GoOrphan {
                    file: record.file.clone(),
                    symbol: format!("{}::{symbol}", record.file),
                    item_kind: kind.clone(),
                    line: *line,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(file: &str, markers: Vec<RawMarker>, exports: &[&str]) -> FileRecord {
        let facts = exports
            .iter()
            .enumerate()
            .map(|(i, name)| go_ai_native_extract_bridge::RawFact::Item {
                kind: "func".into(),
                symbol: (*name).to_string(),
                line: (i + 1) as u32,
                is_exported: true,
                has_doc_example: false,
                underlying: None,
            })
            .collect();
        FileRecord {
            protocol: 1,
            file: file.into(),
            in_test: false,
            degraded: false,
            facts,
            markers,
        }
    }

    fn marker(
        tag: &str,
        uri: &str,
        r: Option<u32>,
        reason: Option<&str>,
        symbol: Option<&str>,
    ) -> RawMarker {
        RawMarker {
            tag: tag.into(),
            uri: uri.into(),
            r,
            reason: reason.map(String::from),
            symbol: symbol.map(String::from),
            line: 1,
        }
    }

    fn cfg(roots: &[&str]) -> Config {
        Config {
            scan_roots: roots.iter().map(|s| s.to_string()).collect(),
            ..Config::default()
        }
    }

    #[test]
    fn markers_lower_to_items_edges_and_pins() {
        let records = vec![record(
            "internal/cells/plan/plan.go",
            vec![
                marker(
                    "implements",
                    "spec://demo/PROP-001#req-solve",
                    Some(2),
                    None,
                    Some("Solve"),
                ),
                marker("scope", "spec://demo/PROP-001#cells", Some(1), None, None),
            ],
            &["Solve"],
        )];
        let (items, edges, warnings) = records_to_index(&records, "go");
        let rendered: Vec<&str> = warnings.iter().map(|w| w.message.as_str()).collect();
        assert!(warnings.is_empty(), "{rendered:?}");
        assert_eq!(edges.len(), 2);
        assert_eq!(items.len(), 2); // the decl + the package item
        let implements = &edges[0];
        assert_eq!(implements.fromSymbol, "internal/cells/plan/plan.go::Solve");
        assert_eq!(implements.pinnedR.as_deref(), Some(&2));
        assert!(matches!(edges[1].verb, EdgeVerb::Implements)); // scope default
        assert_eq!(edges[1].fromSymbol, "internal/cells/plan/plan.go");
    }

    #[test]
    fn deviates_without_reason_is_a_warning_and_bad_uri_never_panics() {
        let records = vec![record(
            "internal/a.go",
            vec![
                marker("deviates", "spec://demo/PROP-001#req", Some(1), None, Some("F")),
                marker("implements", "not-a-uri", None, None, Some("G")),
            ],
            &["F", "G"],
        )];
        let (_, edges, warnings) = records_to_index(&records, "go");
        assert!(edges.is_empty());
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.code == "deviates-missing-reason"));
        assert!(warnings.iter().any(|w| w.code == "bad-spec-uri"));
    }

    #[test]
    fn orphan_gate_is_package_grain_for_scope() {
        let records = vec![
            // plan.go carries the package scope in its doc block…
            record(
                "internal/cells/plan/doc.go",
                vec![marker("scope", "spec://demo/PROP-001#cells", Some(1), None, None)],
                &[],
            ),
            // …so a SIBLING file's exports are covered too.
            record("internal/cells/plan/extra.go", vec![], &["Covered"]),
            // A different package with no scope and no tags: orphan.
            record("internal/registry/registry.go", vec![], &["Naked"]),
        ];
        let found = orphans(&records, &cfg(&["internal"]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].symbol, "internal/registry/registry.go::Naked");
    }
}
