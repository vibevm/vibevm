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
    assert!(warnings[0].message.contains("JSON"), "names the failure: {}", warnings[0].message);
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
