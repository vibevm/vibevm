//! Public-API golden tests for the resolver cell — precedence,
//! get/get_section/inspect/origin, defaults, missing keys (predictions P1, P3;
//! PROP-040 §5 `#resolver` REQs). The algorithm-level tests live in
//! [`super::merge`].
//!
//! Split out of `mod.rs` to honour the ≤600-line AI-Native file budget.
//! Non-`#[test]` helpers carry `#[cfg(test)]` so file-grain scanners (the
//! conform frontend) scope their `unwrap`s as test code.

use super::*;
use crate::schema::{KeyMeta, KeyType, Scope};

// ── test helpers ────────────────────────────────────────────────────────────

/// Build a KeyMeta with a default value (test helper).
#[cfg(test)]
fn key_with_default(path: &str, ty: KeyType, scope: Scope, default: toml::Value) -> KeyMeta {
    KeyMeta::new(path, ty, scope, "a test setting")
        .unwrap()
        .with_default(default)
}

/// Build a `String` KeyMeta at `Scope::User` with a default (common test shape).
#[cfg(test)]
fn str_default_key(path: &str, default: &str) -> KeyMeta {
    key_with_default(
        path,
        KeyType::String,
        Scope::User,
        toml::Value::String(default.into()),
    )
}

/// Parse a TOML body into a table (test helper — body is fixture data).
#[cfg(test)]
fn tbl(body: &str) -> toml::Table {
    toml::from_str(body).unwrap()
}

/// Build `LayeredRaw` from three bodies (L1, L2, L3).
#[cfg(test)]
fn layered(l1: &str, l2: &str, l3: &str) -> LayeredRaw {
    LayeredRaw {
        l1: tbl(l1),
        l2: tbl(l2),
        l3: tbl(l3),
    }
}

/// Resolve `raw` against `schema` with empty CLI/env layers — the common test
/// shape for precedence checks that do not exercise cli/env.
#[cfg(test)]
fn resolve_plain(raw: LayeredRaw, schema: &Schema) -> ResolvedPrefs {
    resolve(raw, schema, toml::Table::new(), toml::Table::new())
}

/// Read an optional layer field as `Option<&str>` (the shape inspect exposes).
#[cfg(test)]
fn opt_str(v: &Option<toml::Value>) -> Option<&str> {
    v.as_ref().and_then(toml::Value::as_str)
}

/// The number of entries in an optional layer field's sub-table.
#[cfg(test)]
fn table_len(v: &Option<toml::Value>) -> Option<usize> {
    v.as_ref()
        .and_then(toml::Value::as_table)
        .map(toml::Table::len)
}

/// Test-only Schema convenience: build from a single KeyMeta.
#[cfg(test)]
fn schema_from_one(meta: KeyMeta) -> Schema {
    let mut s = Schema::new();
    s.register(meta).unwrap();
    s
}

// ── §2 #precedence-law: full chain default ⊂ L1 ⊂ L2 ⊂ L3 ⊂ cli ⊂ env ──────

#[test]
fn precedence_full_chain_each_layer_overrides_below() {
    let schema = schema_from_one(str_default_key("k", "def"));

    // Default wins when no file/cli/env sets the key.
    let rp = resolve_plain(LayeredRaw::new(), &schema);
    assert_eq!(rp.get("k").and_then(|v| v.as_str()), Some("def"));
    assert_eq!(rp.origin("k"), Some(Origin::Default));

    // L1 over default; L2 over L1; L3 over L2.
    let rp = resolve_plain(layered("k = \"L1\"\n", "", ""), &schema);
    assert_eq!(rp.get("k").and_then(|v| v.as_str()), Some("L1"));
    assert_eq!(rp.origin("k"), Some(Origin::L1));
    let rp = resolve_plain(layered("k = \"L1\"\n", "k = \"L2\"\n", ""), &schema);
    assert_eq!(rp.get("k").and_then(|v| v.as_str()), Some("L2"));
    let rp = resolve_plain(layered("", "k = \"L2\"\n", "k = \"L3\"\n"), &schema);
    assert_eq!(rp.get("k").and_then(|v| v.as_str()), Some("L3"));

    // CLI over L3; env over CLI (the top of the law).
    let rp = resolve(
        layered("", "", "k = \"L3\"\n"),
        &schema,
        tbl("k = \"cli\"\n"),
        toml::Table::new(),
    );
    assert_eq!(rp.get("k").and_then(|v| v.as_str()), Some("cli"));
    assert_eq!(rp.origin("k"), Some(Origin::Cli));
    let rp = resolve(
        LayeredRaw::new(),
        &schema,
        tbl("k = \"cli\"\n"),
        tbl("k = \"env\"\n"),
    );
    assert_eq!(rp.get("k").and_then(|v| v.as_str()), Some("env"));
    assert_eq!(rp.origin("k"), Some(Origin::Env));
}

// ── P3: inspect round-trips the provenance winner ───────────────────────────

#[test]
fn inspect_roundtrips_origin_and_carries_each_layer() {
    let schema = schema_from_one(str_default_key("tree.palette", "def"));
    let raw = layered("", "tree.palette = \"rosé-pine\"\n", "");
    let cli = tbl("tree.palette = \"solarized\"\n");
    let rp = resolve(raw, &schema, cli, toml::Table::new());

    let iv = rp.inspect("tree.palette").unwrap();
    assert_eq!(iv.value.as_str(), Some("solarized"));
    assert_eq!(opt_str(&iv.default), Some("def"));
    assert!(iv.l1.is_none());
    assert_eq!(opt_str(&iv.l2), Some("rosé-pine"));
    assert!(iv.l3.is_none());
    assert_eq!(opt_str(&iv.cli), Some("solarized"));
    assert!(iv.env.is_none());
    // P3: the inspect-reported origin matches the provenance winner.
    assert_eq!(iv.origin, Origin::Cli);
    assert_eq!(rp.origin("tree.palette"), Some(iv.origin));
}

#[test]
fn inspect_for_table_path_returns_composed_subtree() {
    let rp = resolve_plain(
        layered("[tree]\na = 1\n", "[tree]\nb = 2\n", ""),
        &Schema::new(),
    );
    let iv = rp.inspect("tree").unwrap();
    assert_eq!(iv.value.as_table().map(toml::Table::len), Some(2));
    assert_eq!(table_len(&iv.l1), Some(1));
    assert_eq!(table_len(&iv.l2), Some(1));
    // A container has no single leaf provenance — fallback is Default.
    assert_eq!(iv.origin, Origin::Default);
}

// ── §5 #get-section ─────────────────────────────────────────────────────────

#[test]
fn get_section_returns_namespace_subtree() {
    let rp = resolve_plain(
        layered("[tree]\npalette = \"x\"\n[node]\nfold = true\n", "", ""),
        &Schema::new(),
    );
    let tree = rp.get_section("tree").unwrap();
    assert_eq!(tree.get("palette").and_then(|v| v.as_str()), Some("x"));
    let node = rp.get_section("node").unwrap();
    assert_eq!(node.get("fold").and_then(|v| v.as_bool()), Some(true));
    assert!(rp.get_section("ghost").is_none());
    // A leaf path is not a section.
    assert!(rp.get_section("tree.palette").is_none());
}

// ── §5 #resolved-prefs: missing keys ────────────────────────────────────────

#[test]
fn missing_key_returns_none_from_get_inspect_origin() {
    let rp = resolve_plain(LayeredRaw::new(), &Schema::new());
    assert!(rp.get("nope").is_none());
    assert!(rp.inspect("nope").is_none());
    assert!(rp.origin("nope").is_none());
    assert!(rp.diagnostics().is_empty());
}

// ── defaults materialise from the schema (not from a file) ──────────────────

#[test]
fn defaults_become_the_lowest_layer_when_file_layers_absent() {
    let schema = schema_from_one(key_with_default(
        "tree.fold",
        KeyType::Bool,
        Scope::User,
        toml::Value::Boolean(true),
    ));
    let rp = resolve_plain(LayeredRaw::new(), &schema);
    assert_eq!(rp.get("tree.fold").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(rp.origin("tree.fold"), Some(Origin::Default));

    // L3 overrides the default; Default is still visible via inspect.
    let rp = resolve_plain(layered("", "", "tree.fold = false\n"), &schema);
    assert_eq!(rp.get("tree.fold").and_then(|v| v.as_bool()), Some(false));
    let iv = rp.inspect("tree.fold").unwrap();
    assert_eq!(
        iv.default.as_ref().and_then(toml::Value::as_bool),
        Some(true)
    );
    assert_eq!(iv.l3.as_ref().and_then(toml::Value::as_bool), Some(false));
    assert_eq!(iv.origin, Origin::L3);
}

// ── Origin metadata smoke ───────────────────────────────────────────────────

#[test]
fn origin_labels_and_order_match_precedence_law() {
    // Labels match PROP-040 §2 spellings.
    assert_eq!(Origin::Default.label(), "default");
    assert_eq!(Origin::L1.label(), "L1");
    assert_eq!(Origin::L2.label(), "L2");
    assert_eq!(Origin::L3.label(), "L3");
    assert_eq!(Origin::Cli.label(), "cli");
    assert_eq!(Origin::Env.label(), "env");
    assert_eq!(Origin::Env.to_string(), "env");
    // Declaration order encodes §2 #precedence-law.
    assert!(Origin::Default < Origin::L1);
    assert!(Origin::L1 < Origin::L2);
    assert!(Origin::L2 < Origin::L3);
    assert!(Origin::L3 < Origin::Cli);
    assert!(Origin::Cli < Origin::Env);
}
