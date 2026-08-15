//! Tests for the vocabulary substitution — split out of `vocabulary.rs`
//! by the same `#[path]` idiom `index/memory.rs` uses, so neither half
//! sits against the 600-line budget. This half holds today's
//! substitution guarantees — placement, pass-through, the refusals; the
//! transitive-closure half (F41A2) lives in `tests_transitive.rs` and
//! shares these fixtures.

use super::*;
use crate::repo_root;
use serde_json::json;

/// The inline `definitions.package_kind` both report schemas carried
/// before the vocabulary home existed — the value the substitution
/// must reproduce exactly. The schemas no longer hold it inline, so
/// this literal is the only witness of the "before" side.
pub(super) fn inline_package_kind() -> Value {
    json!({"enum": ["flow", "feat", "stack", "tool", "mcp", "lang"]})
}

/// A vocabulary home built from a name → fragment map, in a tempdir —
/// the fixture base for every test; the composite shapes the later
/// catalog step brings (a fragment pulling other fragments) are just
/// entries in the same map.
pub(super) fn home_with(pairs: Value) -> Result<(tempfile::TempDir, PathBuf)> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("vocabularies.json");
    let body = serde_json::to_string(&pairs)?;
    std::fs::write(&path, body)?;
    Ok((dir, path))
}

/// A vocabulary home carrying the real `package_kind`, in a tempdir.
fn vocabulary_home() -> Result<(tempfile::TempDir, PathBuf)> {
    home_with(json!({"package_kind": inline_package_kind()}))
}

pub(super) fn write_schema(dir: &Path, name: &str, body: Value) -> Result<PathBuf> {
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_string(&body)?)?;
    Ok(path)
}

pub(super) fn read_json(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// §4.1: a schema that declares `package_kind` and references it
/// resolves to a document whose `definitions.package_kind` is the
/// fragment from the home — and the authored schema on disk keeps no
/// trace of the copy.
#[test]
fn substitution_places_the_fragment_into_definitions() -> Result<()> {
    let (_home_dir, home) = vocabulary_home()?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "list_report.jtd.json",
        json!({
            "metadata": {
                "description": "fixture",
                "x-vocabularies": ["package_kind"]
            },
            "properties": {
                "kind": {"ref": "package_kind"}
            }
        }),
    )?;
    let before = std::fs::read_to_string(&schema)?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let resolved = vocabularies.resolve(&schema)?;

    assert_ne!(
        resolved, schema,
        "the generator must read a copy, not the schema"
    );
    let doc = read_json(&resolved)?;
    assert_eq!(doc["definitions"]["package_kind"], inline_package_kind());
    assert_eq!(doc["properties"]["kind"], json!({"ref": "package_kind"}));
    assert_eq!(
        std::fs::read_to_string(&schema)?,
        before,
        "the authored schema is never rewritten"
    );
    Ok(())
}

/// §4.2: a schema without the annotation is handed over untouched —
/// the very same path, no copy at all.
#[test]
fn schema_without_the_annotation_passes_through() -> Result<()> {
    let (_home_dir, home) = vocabulary_home()?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "init_report.jtd.json",
        json!({
            "metadata": {"description": "fixture"},
            "definitions": {
                "kind": {"type": "string"}
            },
            "properties": {
                "kind": {"ref": "kind"}
            }
        }),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    assert_eq!(vocabularies.resolve(&schema)?, schema);
    Ok(())
}

/// §3(г).1: a name missing from the home is refused, and the refusal
/// names the schema, the vocabulary, the home and the fix.
#[test]
fn refuses_a_name_missing_from_the_home() -> Result<()> {
    let home_dir = tempfile::tempdir()?;
    let home = home_dir.path().join("vocabularies.json");
    std::fs::write(&home, r#"{"other": {"type": "string"}}"#)?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "list_report.jtd.json",
        json!({"metadata": {"x-vocabularies": ["package_kind"]}}),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let err = vocabularies
        .resolve(&schema)
        .expect_err("a name the home does not carry must be refused");
    let msg = err.to_string();
    assert!(msg.contains("package_kind"), "names the vocabulary: {msg}");
    assert!(
        msg.contains("list_report.jtd.json"),
        "names the schema: {msg}"
    );
    assert!(msg.contains("vocabularies.json"), "names the home: {msg}");
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the fix command: {msg}"
    );
    Ok(())
}

/// §3(г).2: a vocabulary that would overwrite the schema's own
/// definition of the same name is refused — substitution must not
/// silently clobber.
#[test]
fn refuses_a_vocabulary_colliding_with_a_schema_definition() -> Result<()> {
    let (_home_dir, home) = vocabulary_home()?;
    let schema_dir = tempfile::tempdir()?;
    let schema = write_schema(
        schema_dir.path(),
        "list_report.jtd.json",
        json!({
            "metadata": {"x-vocabularies": ["package_kind"]},
            "definitions": {
                "package_kind": {"type": "string"}
            }
        }),
    )?;

    let mut vocabularies = Vocabularies::load(&home)?;
    let err = vocabularies
        .resolve(&schema)
        .expect_err("overwriting a schema's own definition must be refused");
    let msg = err.to_string();
    assert!(msg.contains("package_kind"), "names the vocabulary: {msg}");
    assert!(
        msg.contains("definitions.package_kind"),
        "names the definition it would clobber: {msg}"
    );
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the fix command: {msg}"
    );
    Ok(())
}

/// §3(г).3: a malformed annotation is refused whichever way the
/// shape is broken — not an array, or an array listing a non-string.
#[test]
fn refuses_a_malformed_annotation() -> Result<()> {
    let (_home_dir, home) = vocabulary_home()?;
    let schema_dir = tempfile::tempdir()?;
    let mut vocabularies = Vocabularies::load(&home)?;

    for broken in [
        json!("package_kind"),
        json!(["package_kind", 7]),
        json!({"name": "package_kind"}),
    ] {
        let schema = write_schema(
            schema_dir.path(),
            "list_report.jtd.json",
            json!({"metadata": {"x-vocabularies": broken}}),
        )?;
        let err = vocabularies
            .resolve(&schema)
            .expect_err("a broken annotation shape must be refused");
        let msg = err.to_string();
        assert!(
            msg.contains("x-vocabularies"),
            "names the annotation: {msg}"
        );
        assert!(msg.contains("array"), "says what shape it needs: {msg}");
        assert!(
            msg.contains("list_report.jtd.json"),
            "names the schema: {msg}"
        );
    }
    Ok(())
}

/// §3(г).4: a `ref` nothing resolves — neither a definition nor a
/// pulled-in vocabulary — is refused here, where the schema and the
/// name can be named. Left unchecked, this is the input that reaches
/// jtd-codegen as a panic (`no entry found for key`).
#[test]
fn refuses_a_dangling_ref() -> Result<()> {
    let (_home_dir, home) = vocabulary_home()?;
    let schema_dir = tempfile::tempdir()?;
    let mut vocabularies = Vocabularies::load(&home)?;

    // The §2 shape: a reference and an empty `definitions` — today a
    // panic inside the binary, with neither file nor name in it.
    let schema = write_schema(
        schema_dir.path(),
        "list_report.jtd.json",
        json!({
            "definitions": {},
            "properties": {"kind": {"ref": "package_kind"}}
        }),
    )?;
    let err = vocabularies
        .resolve(&schema)
        .expect_err("a reference nothing resolves must be refused");
    let msg = err.to_string();
    assert!(msg.contains("package_kind"), "names the reference: {msg}");
    assert!(
        msg.contains("list_report.jtd.json"),
        "names the schema: {msg}"
    );
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the fix command: {msg}"
    );

    // The annotated variant of the same failure: vocabularies were
    // pulled in, but the body references a name none of them carries.
    let schema = write_schema(
        schema_dir.path(),
        "registry_sync_report.jtd.json",
        json!({
            "metadata": {"x-vocabularies": ["package_kind"]},
            "properties": {"kind": {"ref": "package_knid"}}
        }),
    )?;
    let err = vocabularies
        .resolve(&schema)
        .expect_err("a typo'd reference must be refused after substitution too");
    let msg = err.to_string();
    assert!(msg.contains("package_knid"), "names the reference: {msg}");
    Ok(())
}

/// §3(д)/§4.4: references sit at any depth; one inside
/// `properties.x.elements` is caught exactly like a top-level one —
/// and one that resolves passes at the same depth.
#[test]
fn finds_refs_at_any_depth() -> Result<()> {
    let (_home_dir, home) = vocabulary_home()?;
    let schema_dir = tempfile::tempdir()?;
    let mut vocabularies = Vocabularies::load(&home)?;

    let deep = write_schema(
        schema_dir.path(),
        "deep.jtd.json",
        json!({"properties": {"x": {"elements": {"ref": "nowhere"}}}}),
    )?;
    let err = vocabularies
        .resolve(&deep)
        .expect_err("a dangling ref at depth must be caught");
    assert!(err.to_string().contains("nowhere"), "names the reference");

    let top = write_schema(schema_dir.path(), "top.jtd.json", json!({"ref": "nowhere"}))?;
    let err = vocabularies
        .resolve(&top)
        .expect_err("a dangling ref at the top level must be caught");
    assert!(err.to_string().contains("nowhere"), "names the reference");

    let resolved = write_schema(
        schema_dir.path(),
        "resolved.jtd.json",
        json!({
            "metadata": {"x-vocabularies": ["package_kind"]},
            "properties": {"x": {"elements": {"ref": "package_kind"}}}
        }),
    )?;
    assert!(
        vocabularies.resolve(&resolved).is_ok(),
        "a deep reference that resolves passes"
    );
    Ok(())
}

/// §5 (green): the two report schemas as edited — no inline
/// `definitions.package_kind`, an `x-vocabularies` annotation —
/// resolve, against the real home, to documents whose
/// `definitions.package_kind` equals by value the fragment they
/// carried inline before the home existed. The wire does not move.
#[test]
fn report_schemas_resolve_to_the_inline_vocabulary_they_had() -> Result<()> {
    let root = repo_root()?;
    let mut vocabularies = Vocabularies::load(&vocabularies_path(&root))?;
    let home = read_json(&vocabularies_path(&root))?;
    assert_eq!(
        home["package_kind"],
        inline_package_kind(),
        "the home carries the fragment verbatim"
    );
    for name in ["list_report", "registry_sync_report"] {
        let schema = root.join("schemas").join(format!("{name}.jtd.json"));
        let resolved = vocabularies.resolve(&schema)?;
        assert_ne!(
            resolved, schema,
            "{name}: an annotated schema resolves to a copy"
        );
        let doc = read_json(&resolved)?;
        assert_eq!(
            doc["definitions"]["package_kind"],
            inline_package_kind(),
            "{name}: the resolved vocabulary equals the inline one"
        );
    }
    Ok(())
}
