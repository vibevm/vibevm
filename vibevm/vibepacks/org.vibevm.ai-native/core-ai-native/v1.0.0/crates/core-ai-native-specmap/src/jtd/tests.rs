//! JTD-schema scanner tests, out-of-line per the file-length budget (the
//! `mdspec/tests.rs` pattern). Included via `#[cfg(test)] mod tests;`, so
//! `use super::*` reaches the string-level seam [`scan_schema_text`] and the
//! [`JtdScanner`] type.

use super::*;
use crate::config::Config;
use crate::generated::specmap::EdgeVerb;
use crate::scanner::CodeScanner;

const URI: &str = "spec://project/modules/demo/PROP-001#req-foo";

/// Format a warning slice one-per-line — the generated `Warning` has no
/// `Debug`, so tests render it themselves (the helper the sibling scanners
/// use).
fn warn_lines(w: &[Warning]) -> String {
    w.iter()
        .map(|x| format!("{}:{} [{}] {}", x.file, x.line, x.code, x.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Packet test 1: a `metadata.spec` tag on the root mints the root unit and
/// its edge.
#[test]
fn tagged_root_schema_yields_unit_and_edge() {
    let text = format!(r#"{{"metadata": {{"spec": {{"implements": "{URI}"}}}}}}"#);
    let (items, edges, warnings) = scan_schema_text("schemas/demo.jtd.json", "demo", &text);
    assert!(warnings.is_empty(), "{}", warn_lines(&warnings));
    assert_eq!(items.len(), 1, "just the root unit");
    assert_eq!(items[0].symbol, "demo");
    assert_eq!(items[0].itemKind, "schema");
    assert_eq!(items[0].crateName, "<schema>");
    assert_eq!(edges.len(), 1);
    assert!(matches!(edges[0].verb, EdgeVerb::Implements));
    assert_eq!(edges[0].uri, URI);
    assert_eq!(edges[0].fromSymbol, "demo");
    assert!(matches!(edges[0].provenance, EdgeProvenance::Authored));
    assert!(edges[0].pinnedR.is_none() && edges[0].reason.is_none());
}

/// Packet test 2: a `definitions` entry is its OWN unit with the composite
/// `stem::name` symbol, and its tag edges from that symbol.
#[test]
fn definition_yields_own_unit_with_composite_symbol() {
    let text = format!(
        r#"{{
  "definitions": {{
    "Foo": {{ "metadata": {{ "spec": {{ "implements": "{URI}" }} }} }}
  }}
}}"#
    );
    let (items, edges, warnings) = scan_schema_text("schemas/demo.jtd.json", "demo", &text);
    assert!(warnings.is_empty(), "{}", warn_lines(&warnings));
    assert_eq!(items.len(), 2, "root + the one definition");
    let foo = items
        .iter()
        .find(|i| i.itemKind == "schema-def")
        .expect("a schema-def unit");
    assert_eq!(foo.symbol, "demo::Foo");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].fromSymbol, "demo::Foo");
    assert_eq!(edges[0].uri, URI);
}

/// Packet test 3: a schema without tags still yields its units but ZERO
/// edges (and no warning) — the unit inventory is independent of tagging.
#[test]
fn untagged_schema_yields_units_but_zero_edges() {
    let text = r#"{"definitions": {"Foo": {"type": "string"}, "Bar": {"type": "object"}}}"#;
    let (items, edges, warnings) = scan_schema_text("schemas/demo.jtd.json", "demo", text);
    assert!(warnings.is_empty(), "{}", warn_lines(&warnings));
    assert!(edges.is_empty(), "no tags ⇒ no edges");
    assert_eq!(items.len(), 3, "root + Foo + Bar");
    let symbols: Vec<&str> = items.iter().map(|i| i.symbol.as_str()).collect();
    assert!(symbols.contains(&"demo"));
    assert!(symbols.contains(&"demo::Foo"));
    assert!(symbols.contains(&"demo::Bar"));
}

/// Packet test 4: broken JSON is a warning that names the file and never
/// crashes the run (B5: degrade, never error).
#[test]
fn broken_json_is_a_warning_not_a_crash() {
    let (items, edges, warnings) = scan_schema_text("schemas/demo.jtd.json", "demo", "{ not json");
    assert!(items.is_empty() && edges.is_empty());
    assert_eq!(warnings.len(), 1, "{}", warn_lines(&warnings));
    assert_eq!(warnings[0].code, "invalid-schema-json");
    assert_eq!(warnings[0].file, "schemas/demo.jtd.json");
    assert!(
        warnings[0].message.contains("JSON"),
        "names the failure: {}",
        warnings[0].message
    );
}

/// Packet test 5: an unknown `metadata.spec` verb is a finding naming the
/// verb, and mints no edge.
#[test]
fn unknown_verb_is_a_warning_naming_the_verb() {
    let text = format!(r#"{{"metadata": {{"spec": {{"supersedes": "{URI}"}}}}}}"#);
    let (items, edges, warnings) = scan_schema_text("schemas/demo.jtd.json", "demo", &text);
    assert!(edges.is_empty(), "an unknown verb mints no edge");
    let unknown: Vec<&Warning> = warnings
        .iter()
        .filter(|w| w.code == "unknown-schema-verb")
        .collect();
    assert_eq!(unknown.len(), 1, "{}", warn_lines(&warnings));
    assert!(
        unknown[0].message.contains("supersedes"),
        "the warning must name the verb: {}",
        unknown[0].message
    );
    assert_eq!(items.len(), 1, "the root unit is still inventoried");
}

/// Packet test 6: empty `schema_roots` is a no-op — the regression gate.
/// A schema file sits on disk under an unconfigured directory and is not
/// touched, so existing projects change by not one byte.
#[test]
fn empty_schema_roots_is_a_noop() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("schemas")).unwrap();
    std::fs::write(
        tmp.path().join("schemas/ghost.jtd.json"),
        r#"{"metadata":{"spec":{"implements":"spec://project/X#y"}}}"#,
    )
    .unwrap();
    // Default config: schema_roots is empty.
    let (items, edges, warnings) = JtdScanner.scan(tmp.path(), &Config::default());
    assert!(items.is_empty(), "no units from an unconfigured scan");
    assert!(edges.is_empty());
    assert!(warnings.is_empty());
}

/// A non-object JSON root (a valid JSON value, but not a schema) is a
/// warning, never a silent acceptance.
#[test]
fn non_object_root_is_a_warning() {
    let (items, edges, warnings) = scan_schema_text("schemas/demo.jtd.json", "demo", "[1, 2, 3]");
    assert!(items.is_empty() && edges.is_empty());
    assert_eq!(warnings.len(), 1, "{}", warn_lines(&warnings));
    assert_eq!(warnings[0].code, "schema-not-object");
}

/// Positions are MEASURED, not invented: the root unit spans the document,
/// and a definition unit's `line`/`end_line` are its key and its value's
/// closing brace. (Refinement #2 evidence.)
#[test]
fn positions_are_measured_from_the_source() {
    // line 1 `{` · 2 `"definitions": {` · 3 `"Foo": {` · 4 body · 5 `}` · 6 `}` · 7 `}`
    let text =
        "{\n  \"definitions\": {\n    \"Foo\": {\n      \"type\": \"string\"\n    }\n  }\n}\n";
    let (items, _, warnings) = scan_schema_text("schemas/demo.jtd.json", "demo", text);
    assert!(warnings.is_empty(), "{}", warn_lines(&warnings));
    let root = items
        .iter()
        .find(|i| i.itemKind == "schema")
        .expect("root unit");
    assert_eq!(root.line, 1, "root opens at the document's first brace");
    assert_eq!(
        *root.endLine.as_deref().unwrap(),
        7,
        "root closes at the document's last brace (line 7)"
    );
    let foo = items
        .iter()
        .find(|i| i.symbol == "demo::Foo")
        .expect("Foo unit");
    assert_eq!(foo.line, 3, "the definition key is on line 3");
    assert_eq!(
        *foo.endLine.as_deref().unwrap(),
        5,
        "the definition value closes on line 5"
    );
    // Refinement #2: a schema has no Rust token stream, so the fingerprint
    // the Rust scanner mints (tok1:<sha256>) is absent here — not invented.
    assert!(root.fingerprint.is_none());
}

/// The `.jtd.json` literal-extension law + recursive walk: only
/// `*.jtd.json` files are scanned (a bare `*.json` is not), and the walk
/// descends into subdirectories — mirroring the markdown scanner's `**/*.md`.
#[test]
fn scans_only_jtd_json_and_walks_recursively() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("schemas/sub")).unwrap();
    // a.jtd.json — tagged root, scanned.
    std::fs::write(
        tmp.path().join("schemas/a.jtd.json"),
        format!(r#"{{"metadata":{{"spec":{{"implements":"{URI}"}}}}}}"#),
    )
    .unwrap();
    // b.json — valid JSON, but NOT .jtd.json; must be ignored.
    std::fs::write(
        tmp.path().join("schemas/b.json"),
        r#"{"metadata":{"spec":{"implements":"spec://project/should-be-ignored"}}}"#,
    )
    .unwrap();
    // nested c.jtd.json — scanned recursively.
    std::fs::write(
        tmp.path().join("schemas/sub/c.jtd.json"),
        r#"{"definitions":{"C":{"type":"string"}}}"#,
    )
    .unwrap();
    let cfg = Config {
        schema_roots: vec!["schemas".into()],
        ..Config::default()
    };
    let (items, edges, warnings) = JtdScanner.scan(tmp.path(), &cfg);
    assert!(warnings.is_empty(), "{}", warn_lines(&warnings));
    let symbols: Vec<&str> = items.iter().map(|i| i.symbol.as_str()).collect();
    assert!(symbols.contains(&"a"), "root of a.jtd.json");
    assert!(symbols.contains(&"c"), "root of nested c.jtd.json");
    assert!(symbols.contains(&"c::C"), "definition under c.jtd.json");
    // b.json contributed nothing: no empty-stem unit and exactly one edge
    // (a's), where b.json alone would have added another.
    assert!(items.iter().all(|i| !i.symbol.is_empty()), "b.json leaked");
    assert_eq!(edges.len(), 1, "only a's root edge; b.json ignored");
    assert_eq!(edges[0].uri, URI);
}

fn write_fixture(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

/// A generic, non-product-named two-hop shared vocabulary proves the scanner
/// follows declared metadata rather than recognising a compiler filename.
/// Every projected position and edge is anchored in the vocabulary member.
#[test]
fn thin_shared_root_projects_transitive_closure_with_measured_positions_and_edges() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let schema = r#"{
  "metadata": {
    "x-vocabularies": ["shared"]
  },
  "ref": "shared"
}
"#;
    let vocabulary = format!(
        r#"{{
  "shared": {{
    "metadata": {{
      "x-vocabularies": ["branch"],
      "spec": {{"implements": "{URI}"}}
    }},
    "ref": "branch"
  }},
  "branch": {{
    "metadata": {{
      "x-vocabularies": ["leaf"],
      "spec": {{"verifies": "{URI}"}}
    }},
    "ref": "leaf"
  }},
  "leaf": {{
    "metadata": {{"spec": {{"documents": "{URI}"}}}},
    "type": "string"
  }}
}}
"#
    );
    write_fixture(root, "schemas/shared.jtd.json", schema);
    write_fixture(root, "formats/vocabulary.json", &vocabulary);
    let cfg = Config {
        schema_roots: vec!["schemas".into()],
        schema_vocabulary: Some("formats/vocabulary.json".into()),
        ..Config::default()
    };
    let (items, edges, warnings) = JtdScanner.scan(root, &cfg);
    assert!(warnings.is_empty(), "{}", warn_lines(&warnings));
    assert_eq!(items.len(), 3, "thin root + two projected fragments");
    let root_item = items.iter().find(|item| item.symbol == "shared").unwrap();
    assert_eq!(root_item.file, "schemas/shared.jtd.json");
    assert_eq!(
        (root_item.line, *root_item.endLine.as_deref().unwrap()),
        (1, 6)
    );
    let branch = items
        .iter()
        .find(|item| item.symbol == "shared::branch")
        .unwrap();
    assert_eq!(branch.file, "formats/vocabulary.json");
    assert_eq!((branch.line, *branch.endLine.as_deref().unwrap()), (9, 15));
    let leaf = items
        .iter()
        .find(|item| item.symbol == "shared::leaf")
        .unwrap();
    assert_eq!(leaf.file, "formats/vocabulary.json");
    assert_eq!((leaf.line, *leaf.endLine.as_deref().unwrap()), (16, 19));
    assert_eq!(edges.len(), 3);
    let roots: Vec<(&str, &str, u32)> = edges
        .iter()
        .map(|edge| (edge.fromSymbol.as_str(), edge.file.as_str(), edge.line))
        .collect();
    assert!(roots.contains(&("shared", "formats/vocabulary.json", 2)));
    assert!(roots.contains(&("shared::branch", "formats/vocabulary.json", 9)));
    assert!(roots.contains(&("shared::leaf", "formats/vocabulary.json", 16)));
}

/// Inline definitions and alias roots are ordinary schemas. Even when their
/// metadata mentions the shared vocabulary, neither may duplicate its units.
#[test]
fn ordinary_and_alias_schemas_do_not_project_shared_units() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_fixture(
        root,
        "formats/vocabulary.json",
        r#"{"shared":{"metadata":{"x-vocabularies":["tail"]}},"tail":{"type":"string"},"version_entry":{"type":"string"}}"#,
    );
    write_fixture(
        root,
        "schemas/shared.jtd.json",
        r#"{"metadata":{"x-vocabularies":["shared"]},"ref":"shared","definitions":{"local":{"type":"string"}}}"#,
    );
    write_fixture(
        root,
        "schemas/entry.jtd.json",
        r#"{"metadata":{"x-vocabularies":["version_entry"]},"ref":"version_entry"}"#,
    );
    let cfg = Config {
        schema_roots: vec!["schemas".into()],
        schema_vocabulary: Some("formats/vocabulary.json".into()),
        ..Config::default()
    };
    let (items, edges, warnings) = JtdScanner.scan(root, &cfg);
    assert!(warnings.is_empty(), "{}", warn_lines(&warnings));
    assert!(edges.is_empty());
    let symbols: Vec<&str> = items.iter().map(|item| item.symbol.as_str()).collect();
    assert_eq!(symbols.len(), 3);
    assert!(symbols.contains(&"entry"));
    assert!(symbols.contains(&"shared"));
    assert!(symbols.contains(&"shared::local"));
    assert!(
        items
            .iter()
            .all(|item| item.file != "formats/vocabulary.json")
    );
}

/// Merely placing a vocabulary beside a configured schema root changes
/// nothing when its explicit config path is absent: the scanner output is
/// byte-for-byte the string-level pre-vocabulary result.
#[test]
fn absent_vocabulary_config_preserves_inline_scanner_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let schema = r#"{"metadata":{"x-vocabularies":["shared"]},"ref":"shared"}"#;
    write_fixture(root, "schemas/shared.jtd.json", schema);
    write_fixture(
        root,
        "formats/vocabulary.json",
        r#"{"shared":{"metadata":{"spec":{"implements":"spec://project/ghost"}}}}"#,
    );
    let cfg = Config {
        schema_roots: vec!["schemas".into()],
        ..Config::default()
    };
    let scanned = JtdScanner.scan(root, &cfg);
    let direct = scan_schema_text("schemas/shared.jtd.json", "shared", schema);
    assert_eq!(
        serde_json::to_vec(&scanned.0).unwrap(),
        serde_json::to_vec(&direct.0).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&scanned.1).unwrap(),
        serde_json::to_vec(&direct.1).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&scanned.2).unwrap(),
        serde_json::to_vec(&direct.2).unwrap()
    );
}

/// Missing, cyclic, and ill-typed vocabulary structure degrades into named
/// warnings while retaining every valid unit the traversal can measure.
#[test]
fn malformed_vocabulary_structure_is_typed_warnings_not_panics() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_fixture(
        root,
        "schemas/shared.jtd.json",
        r#"{"metadata":{"x-vocabularies":["shared"]},"ref":"shared"}"#,
    );
    write_fixture(
        root,
        "formats/vocabulary.json",
        r#"{
  "shared": {"metadata":{"x-vocabularies":["branch","missing",7,"bad","typed"]}},
  "branch": {"metadata":{"x-vocabularies":["shared"]}},
  "bad": 7,
  "typed": {"metadata":{"x-vocabularies":"tail"}}
}"#,
    );
    let cfg = Config {
        schema_roots: vec!["schemas".into()],
        schema_vocabulary: Some("formats/vocabulary.json".into()),
        ..Config::default()
    };
    let (items, _, warnings) = JtdScanner.scan(root, &cfg);
    assert!(items.iter().any(|item| item.symbol == "shared"));
    let codes: Vec<&str> = warnings
        .iter()
        .map(|warning| warning.code.as_str())
        .collect();
    for expected in [
        "schema-vocabulary-cycle",
        "missing-schema-vocabulary-member",
        "schema-vocabulary-dependency-not-string",
        "schema-vocabulary-member-not-object",
        "schema-vocabulary-dependencies-not-array",
    ] {
        assert!(codes.contains(&expected), "missing {expected}: {codes:?}");
    }
}

#[test]
fn malformed_vocabulary_json_is_one_typed_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_fixture(root, "schemas/shared.jtd.json", r#"{"type":"string"}"#);
    write_fixture(root, "formats/vocabulary.json", "{ broken");
    let cfg = Config {
        schema_roots: vec!["schemas".into()],
        schema_vocabulary: Some("formats/vocabulary.json".into()),
        ..Config::default()
    };
    let (items, edges, warnings) = JtdScanner.scan(root, &cfg);
    assert_eq!(items.len(), 1, "ordinary schema still scans");
    assert!(edges.is_empty());
    assert_eq!(warnings.len(), 1, "{}", warn_lines(&warnings));
    assert_eq!(warnings[0].code, "invalid-schema-vocabulary-json");
}

/// The live thin compiler root projects its exact former logical inventory:
/// one measured schema, 55 measured vocabulary definitions, and the root tag.
#[test]
fn live_thin_root_projects_exact_shared_inventory() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .ancestors()
        .find(|candidate| {
            candidate.join("formats/vocabularies.json").is_file()
                && candidate
                    .join("schemas/compiler_ir/e1/ir.jtd.json")
                    .is_file()
        })
        .expect("repository root containing the live wire inputs");
    let cfg = Config {
        schema_roots: vec!["schemas/compiler_ir/e1".into()],
        schema_vocabulary: Some("formats/vocabularies.json".into()),
        ..Config::default()
    };
    let (items, edges, warnings) = JtdScanner.scan(root, &cfg);
    assert!(warnings.is_empty(), "{}", warn_lines(&warnings));
    assert_eq!(items.len(), 56, "thin schema + 55 shared fragments");
    assert_eq!(
        items
            .iter()
            .filter(|item| item.itemKind == "schema-def")
            .count(),
        55
    );
    let schema = items.iter().find(|item| item.itemKind == "schema").unwrap();
    assert_eq!(schema.symbol, "ir");
    assert_eq!(schema.file, "schemas/compiler_ir/e1/ir.jtd.json");
    assert_eq!((schema.line, *schema.endLine.as_deref().unwrap()), (1, 11));
    assert!(
        items
            .iter()
            .filter(|item| item.itemKind == "schema-def")
            .all(|item| item.file == "formats/vocabularies.json"
                && item.line == *item.endLine.as_deref().unwrap())
    );
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].fromSymbol, "ir");
    assert_eq!(edges[0].file, "formats/vocabularies.json");
    assert_eq!(edges[0].line, 41);
}
