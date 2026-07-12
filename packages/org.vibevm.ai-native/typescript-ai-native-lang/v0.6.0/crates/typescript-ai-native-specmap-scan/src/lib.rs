//! The scanner seam's TypeScript implementation (DEFERRALS-CLOSEOUT
//! D3): the §9 JSDoc spec markers — `@implements` / `@verifies` /
//! `@documents` / `@deviates` / `@informs` on declarations, `@scope`
//! at file level — lowered into the SAME PROP-014 index model the Rust
//! scanner feeds. One extractor run (the ts-extract bridge) already
//! carries the markers next to the conform facts, so the two gates
//! never parse a file twice.
//!
//! Semantics mirror rscan: a file-level `@scope` is a module-level
//! `implements` edge (the grammar's own scope default) that covers the
//! file's exports for the orphan gate; `deviates` requires a reason;
//! an `r=N` tail on any marker pins the edge's revision; a URI the
//! grammar rejects is a warning, never a panic. Symbols are
//! `<file>::<name>` — TypeScript modules ARE paths, so the file IS the
//! module qualifier.

use std::path::Path;

use specmap_core::config::Config;
use specmap_core::generated::specmap::{CodeItem, Edge, EdgeProvenance, EdgeVerb, Warning};
use specmap_core::scanner::CodeScanner;
use typescript_ai_native_extract_bridge::{FileRecord, RawMarker};

fn verb_of(tag: &str) -> Option<EdgeVerb> {
    match tag {
        // `@scope` defaults to implements — the grammar's
        // `into_scope_edge` rule, projected.
        "implements" | "scope" => Some(EdgeVerb::Implements),
        "verifies" => Some(EdgeVerb::Verifies),
        "documents" => Some(EdgeVerb::Documents),
        "deviates" => Some(EdgeVerb::Deviates),
        "informs" => Some(EdgeVerb::Informs),
        _ => None,
    }
}

/// Split a marker's free tail into `(pinned_r, reason)`: a bare `r=N`
/// is a revision pin; anything else is the (deviates) reason text; a
/// `r=N` prefix followed by text is both.
fn pin_and_reason(tail: Option<&str>) -> (Option<u32>, Option<String>) {
    let Some(tail) = tail else {
        return (None, None);
    };
    let tail = tail.trim();
    if let Some(rest) = tail.strip_prefix("r=") {
        let (digits, remainder) = match rest.split_once(char::is_whitespace) {
            Some((d, r)) => (d, r.trim()),
            None => (rest, ""),
        };
        if let Ok(r) = digits.parse::<u32>() {
            let reason = if remainder.is_empty() {
                None
            } else {
                Some(remainder.to_string())
            };
            return (Some(r), reason);
        }
    }
    (None, Some(tail.to_string()))
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
                format!("`@{tag} {uri}`: {e}"),
            ));
            continue;
        }
        let (pinned_r, reason_text) = pin_and_reason(reason.as_deref());
        if matches!(verb, EdgeVerb::Deviates) && reason_text.is_none() {
            warnings.push(warn(
                &record.file,
                *line,
                "deviates-missing-reason",
                format!("`@deviates {uri}` carries no reason — testimony is mandatory"),
            ));
            continue;
        }
        let (from_symbol, item_kind, item_symbol) = match (tag.as_str(), symbol) {
            ("scope", _) => (
                record.file.clone(),
                "module".to_string(),
                record.file.clone(),
            ),
            (_, Some(name)) => (
                format!("{}::{name}", record.file),
                "export".to_string(),
                format!("{}::{name}", record.file),
            ),
            (_, None) => (
                record.file.clone(),
                "module".to_string(),
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
            pinnedR: pinned_r.map(Box::new),
            reason: reason_text.map(Box::new),
        });
    }
}

/// Lower a whole extraction run. `root_name` becomes the items'
/// `crate_name` (the scan root's directory name — TS has no crates).
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
/// the CLI uses so the gate and the index share a single node run.
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
        "ts-tsc"
    }
    fn scan(&self, _root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        let scoped = scoped_records(self.records, cfg);
        records_to_index(&scoped, &self.root_name)
    }
}

/// Restrict records to the policy's scan roots (repo-relative path
/// prefixes; `<dir>/*` counts as `<dir>/`).
fn scoped_records(records: &[FileRecord], cfg: &Config) -> Vec<FileRecord> {
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

/// One untagged export found by the TS orphan gate.
#[derive(Debug)]
pub struct TsOrphan {
    pub file: String,
    pub symbol: String,
    pub item_kind: String,
    pub line: u32,
}

/// The orphan gate, TS shape: an EXPORTED declaration in a
/// non-exempt scan root with no own marker and no file-level `@scope`
/// is an orphan (PROP-014 §2.3 projected: private helpers need no
/// annotation; the file-level scope covers the file's exports).
pub fn orphans(records: &[FileRecord], cfg: &Config) -> Vec<TsOrphan> {
    let scoped = scoped_records(records, cfg);
    let mut out = Vec::new();
    for record in &scoped {
        let root = record.file.split('/').next().unwrap_or_default();
        if cfg.exempt.iter().any(|e| e == root) {
            continue;
        }
        if record.in_test {
            continue; // tests verify; they are not public surface
        }
        let has_scope = record.markers.iter().any(|m| m.tag == "scope");
        if has_scope {
            continue;
        }
        for fact in &record.facts {
            let typescript_ai_native_extract_bridge::RawFact::Item {
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
                out.push(TsOrphan {
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
            .map(
                |(i, name)| typescript_ai_native_extract_bridge::RawFact::Item {
                    kind: "function".into(),
                    symbol: (*name).to_string(),
                    line: (i + 1) as u32,
                    is_exported: true,
                    has_doc_example: false,
                },
            )
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

    fn marker(tag: &str, uri: &str, reason: Option<&str>, symbol: Option<&str>) -> RawMarker {
        RawMarker {
            tag: tag.into(),
            uri: uri.into(),
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
            "src/cells/parse/index.ts",
            vec![
                marker(
                    "implements",
                    "spec://demo/PROP-001#req-parse",
                    Some("r=2"),
                    Some("parse"),
                ),
                marker(
                    "scope",
                    "spec://demo/PROP-001#cell-parse",
                    None,
                    Some("CELL"),
                ),
            ],
            &["parse"],
        )];
        let (items, edges, warnings) = records_to_index(&records, "src");
        let rendered: Vec<&str> = warnings.iter().map(|w| w.message.as_str()).collect();
        assert!(warnings.is_empty(), "{rendered:?}");
        assert_eq!(edges.len(), 2);
        assert_eq!(items.len(), 2); // the export + the module item
        let implements = &edges[0];
        assert_eq!(implements.fromSymbol, "src/cells/parse/index.ts::parse");
        assert_eq!(implements.pinnedR.as_deref(), Some(&2));
        assert!(matches!(edges[1].verb, EdgeVerb::Implements)); // scope default
        assert_eq!(edges[1].fromSymbol, "src/cells/parse/index.ts");
    }

    #[test]
    fn deviates_without_reason_is_a_warning_and_bad_uri_never_panics() {
        let records = vec![record(
            "src/a.ts",
            vec![
                marker("deviates", "spec://demo/PROP-001#req", None, Some("f")),
                marker("implements", "not-a-uri", None, Some("g")),
            ],
            &["f", "g"],
        )];
        let (_, edges, warnings) = records_to_index(&records, "src");
        assert!(edges.is_empty());
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.code == "deviates-missing-reason"));
        assert!(warnings.iter().any(|w| w.code == "bad-spec-uri"));
    }

    #[test]
    fn orphan_gate_flags_untagged_exports_and_honours_scope() {
        let records = vec![
            record(
                "src/tagged.ts",
                vec![marker(
                    "implements",
                    "spec://demo/PROP-001#req",
                    None,
                    Some("covered"),
                )],
                &["covered", "naked"],
            ),
            record(
                "src/scoped.ts",
                vec![marker("scope", "spec://demo/PROP-001#cell", None, None)],
                &["anything"],
            ),
        ];
        let found = orphans(&records, &cfg(&["src"]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].symbol, "src/tagged.ts::naked");
    }
}
