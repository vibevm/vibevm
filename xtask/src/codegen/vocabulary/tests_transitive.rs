//! Tests for the transitive vocabulary closure (F41A2) — a fragment
//! may pull fragments of its own by `metadata.x-vocabularies`, and they
//! arrive unnamed. Split from `tests.rs` along that seam when the file
//! outgrew the 600-line budget; the shared fixtures stay there.

use super::tests::{home_with, inline_package_kind, read_json, write_schema};
use super::*;
use serde_json::json;
use std::collections::BTreeSet;

/// A composite fragment: it references `package_kind` and — the point of
/// F41A2 — declares that reference as its own dependency, so a schema
/// pulling `version_entry` needs no knowledge of `package_kind`. The
/// `description` witnesses that a placed fragment's other `metadata`
/// survives the strip.
fn version_entry() -> Value {
    json!({
        "metadata": {"description": "fixture", "x-vocabularies": ["package_kind"]},
        "properties": {"kind": {"ref": "package_kind"}}
    })
}

/// A home with the leaf and the composite that pulls it.
fn composite_home() -> Result<(tempfile::TempDir, PathBuf)> {
    home_with(json!({
        "package_kind": inline_package_kind(),
        "version_entry": version_entry()
    }))
}

/// The minimal composite fragment: it pulls `names` and references the
/// first of them.
fn pulling(names: &[&str]) -> Value {
    json!({
        "metadata": {"x-vocabularies": names},
        "properties": {"kind": {"ref": names[0]}}
    })
}

/// Every refusal must survive its own retelling: the message says what
/// broke, where, and the fix command. Assert all needles in one call so
/// a failure prints the whole message, not just the missing word.
fn expect_needles(err: &anyhow::Error, needles: &[&str]) {
    let msg = err.to_string();
    for needle in needles {
        assert!(
            msg.contains(needle),
            "message must mention {needle:?}: {msg}"
        );
    }
}

/// F41A2 §4.1: a schema pulling a composite fragment receives its
/// dependency too, unnamed. Before the transitive closure the
/// composite's internal `{"ref": "package_kind"}` dangled and the whole
/// resolution was refused — the exact red this test was written against.
#[test]
fn transitive_dependencies_arrive_without_being_named() -> Result<()> {
    let (_home_dir, home) = composite_home()?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "by_name.jtd.json",
        json!({
            "metadata": {"x-vocabularies": ["version_entry"]},
            "properties": {"entry": {"ref": "version_entry"}}
        }),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let doc = read_json(&vocabularies.resolve(&schema)?.doc)?;

    assert!(
        doc["definitions"].get("version_entry").is_some(),
        "the named fragment is placed: {doc}"
    );
    assert_eq!(
        doc["definitions"]["package_kind"],
        inline_package_kind(),
        "the dependency the schema never named arrives with it"
    );
    assert_eq!(
        doc["definitions"]["version_entry"]["properties"]["kind"],
        json!({"ref": "package_kind"}),
        "the ref inside the composite now resolves"
    );
    Ok(())
}

/// F41A2 §4.2: the closure runs deeper than one hop — `a` pulling `b`
/// pulling `c` places all three.
#[test]
fn dependencies_close_deeper_than_one_hop() -> Result<()> {
    let (_home_dir, home) = home_with(json!({
        "a": pulling(&["b"]),
        "b": pulling(&["c"]),
        "c": inline_package_kind()
    }))?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "deep.jtd.json",
        json!({"metadata": {"x-vocabularies": ["a"]}}),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let doc = read_json(&vocabularies.resolve(&schema)?.doc)?;
    for name in ["a", "b", "c"] {
        assert!(
            doc["definitions"].get(name).is_some(),
            "{name} must arrive through the chain: {doc}"
        );
    }
    Ok(())
}

/// F41A2 §4.3: a diamond (`a` → `b` → `d`, `a` → `c` → `d`) places the
/// shared tail once. JSON objects cannot duplicate keys, so "once" here
/// means the outcome: no refusal, and `definitions` holding exactly the
/// four members.
#[test]
fn a_diamond_places_the_shared_tail_once() -> Result<()> {
    let (_home_dir, home) = home_with(json!({
        "a": pulling(&["b", "c"]),
        "b": pulling(&["d"]),
        "c": pulling(&["d"]),
        "d": json!({"type": "string"})
    }))?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "diamond.jtd.json",
        json!({"metadata": {"x-vocabularies": ["a"]}}),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let doc = read_json(&vocabularies.resolve(&schema)?.doc)?;
    let placed: BTreeSet<&str> = doc["definitions"]
        .as_object()
        .expect("definitions is an object")
        .keys()
        .map(String::as_str)
        .collect();
    let expected: BTreeSet<&str> = ["a", "b", "c", "d"].into_iter().collect();
    assert_eq!(placed, expected, "each diamond member placed exactly once");
    Ok(())
}

/// F41A2 §4.4 / §3(г): a dependency cycle is refused, and the refusal
/// walks the loop — participants in traversal order, the schema, the
/// home, the fix.
#[test]
fn refuses_a_dependency_cycle_naming_the_loop() -> Result<()> {
    let (_home_dir, home) = home_with(json!({
        "first": pulling(&["second"]),
        "second": pulling(&["first"])
    }))?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "looped.jtd.json",
        json!({"metadata": {"x-vocabularies": ["first"]}}),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let err = vocabularies
        .resolve(&schema)
        .expect_err("a cycle must be refused, not walked forever");
    expect_needles(
        &err,
        &[
            "first -> second -> first",
            "cycle",
            "looped.jtd.json",
            "cargo xtask codegen",
        ],
    );
    Ok(())
}

/// F41A2 §4.5 / §3(г): a self-referencing vocabulary is the same class
/// of refusal — the named loop just has one member.
#[test]
fn refuses_a_self_referencing_vocabulary() -> Result<()> {
    let (_home_dir, home) = home_with(json!({"solo": pulling(&["solo"])}))?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "self.jtd.json",
        json!({"metadata": {"x-vocabularies": ["solo"]}}),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let err = vocabularies
        .resolve(&schema)
        .expect_err("a self-reference must be refused like any cycle");
    expect_needles(
        &err,
        &[
            "solo -> solo",
            "cycle",
            "self.jtd.json",
            "cargo xtask codegen",
        ],
    );
    Ok(())
}

/// F41A2 §4.6 / §3(д): a name missing from the home is refused also
/// when a FRAGMENT names it — and the refusal tells the whole chain, or
/// the author goes hunting through the schema for a word they never
/// wrote there.
#[test]
fn refuses_a_chain_that_leaves_the_home_naming_the_chain() -> Result<()> {
    let (_home_dir, home) = home_with(json!({
        "package_kind": inline_package_kind(),
        "version_entry": pulling(&["group"])
    }))?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "by_name.jtd.json",
        json!({"metadata": {"x-vocabularies": ["version_entry"]}}),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let err = vocabularies
        .resolve(&schema)
        .expect_err("a dependency the home does not carry must be refused");
    expect_needles(
        &err,
        &[
            "version_entry", // the intermediary the schema did name
            "group",         // the name the home lacks
            "by_name.jtd.json",
            "cargo xtask codegen",
        ],
    );
    Ok(())
}

/// F41A2 §4.7 / §3(в): the placed fragment loses its `x-vocabularies`
/// key — the bookkeeping it names is already executed — while the rest
/// of its `metadata` survives, and a `metadata` emptied by the removal
/// goes entirely. The schema's own annotation stays: it is the schema's.
#[test]
fn placed_fragments_lose_their_x_vocabularies_key() -> Result<()> {
    let (_home_dir, home) = home_with(json!({
        "package_kind": inline_package_kind(),
        "version_entry": version_entry(),
        "bare": pulling(&["version_entry"])
    }))?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "strip.jtd.json",
        json!({"metadata": {"x-vocabularies": ["version_entry", "bare"]}}),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let doc = read_json(&vocabularies.resolve(&schema)?.doc)?;

    let placed = &doc["definitions"]["version_entry"];
    assert!(
        placed["metadata"].get("x-vocabularies").is_none(),
        "the executed instruction must not reach the reader: {placed}"
    );
    assert_eq!(
        placed["metadata"]["description"],
        json!("fixture"),
        "the fragment's own metadata survives the strip"
    );
    assert!(
        doc["definitions"]["bare"].get("metadata").is_none(),
        "a metadata emptied by the strip goes with it: {doc}"
    );
    assert_eq!(
        doc["metadata"]["x-vocabularies"],
        json!(["version_entry", "bare"]),
        "the schema's own annotation is untouched"
    );
    Ok(())
}

/// F41A2 §4.8 / §3(е): the resolved document is a function of its input
/// alone — resolving the same schema twice renders the same bytes, or
/// `check-codegen` would prove nothing about reproduction.
#[test]
fn resolving_twice_renders_identical_documents() -> Result<()> {
    let (_home_dir, home) = composite_home()?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "by_name.jtd.json",
        json!({
            "metadata": {"x-vocabularies": ["version_entry"]},
            "properties": {"entry": {"ref": "version_entry"}}
        }),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let first = vocabularies.resolve(&schema)?.doc;
    let second = vocabularies.resolve(&schema)?.doc;
    assert_ne!(first, second, "two resolutions, two scratch copies");
    assert_eq!(
        std::fs::read_to_string(&first)?,
        std::fs::read_to_string(&second)?,
        "one input, one document — byte for byte"
    );
    Ok(())
}
