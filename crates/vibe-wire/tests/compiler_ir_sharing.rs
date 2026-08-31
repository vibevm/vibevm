//! Canonical compiler-IR ownership and legacy-path identity.

use std::any::TypeId;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn schema_is_thin_and_the_complete_named_closure_lives_in_vocabularies() {
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root().join("schemas/compiler_ir/e1/ir.jtd.json")).unwrap(),
    )
    .unwrap();
    let vocabularies: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root().join("formats/vocabularies.json")).unwrap())
            .unwrap();
    assert_eq!(schema["ref"], "ir");
    assert_eq!(
        schema["metadata"]["x-vocabularies"],
        serde_json::json!(["ir"])
    );
    assert!(schema.get("definitions").is_none() && schema.get("mapping").is_none());
    let closure = vocabularies["ir"]["metadata"]["x-vocabularies"]
        .as_array()
        .unwrap();
    assert_eq!(closure.len(), 55);
    for name in closure {
        assert!(vocabularies.get(name.as_str().unwrap()).is_some(), "{name}");
    }
}

#[test]
fn legacy_module_is_only_reexports_of_the_strict_shared_family() {
    let legacy = std::fs::read_to_string(
        root().join("crates/vibe-wire/src/generated/compiler_ir/e1/ir/mod.rs"),
    )
    .unwrap();
    let shared =
        std::fs::read_to_string(root().join("crates/vibe-wire/src/generated/shared/mod.rs"))
            .unwrap();
    assert!(shared.contains("#[serde(tag = \"shape\")]\npub enum Ir"));
    assert!(shared.contains("#[serde(deny_unknown_fields)]\npub struct IrSourceDocument"));
    let items = legacy
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("// Code generated"))
        .collect::<Vec<_>>();
    assert!(
        items.iter().all(|line| {
            line.starts_with("pub use crate::generated::shared::") && line.ends_with(';')
        }),
        "the legacy module contains only exact shared re-exports: {items:?}"
    );
    assert!(legacy.contains("pub use crate::generated::shared::Ir;"));
    assert!(legacy.contains("pub use crate::generated::shared::IrSourceDocument;"));
    assert_eq!(
        TypeId::of::<vibe_wire::generated::compiler_ir::e1::ir::Ir>(),
        TypeId::of::<vibe_wire::generated::shared::Ir>()
    );
    assert_eq!(
        TypeId::of::<vibe_wire::generated::compiler_ir::e1::ir::Span>(),
        TypeId::of::<vibe_wire::generated::shared::Span>()
    );
    assert!(shared.contains("pub enum ArtifactTarget") && shared.contains("Unknown(String)"));

    let compiler_names: BTreeSet<&str> = items
        .iter()
        .filter_map(|line| {
            line.rsplit_once("::")
                .map(|(_, tail)| tail.trim_end_matches(';'))
        })
        .collect();
    assert!(!shared.contains("#[serde(flatten)]"));
    let mut compiler_type = false;
    for line in shared.lines() {
        if let Some(name) = declaration_name(line) {
            compiler_type = compiler_names.contains(name);
        }
        let trimmed = line.trim();
        if compiler_type
            && let Some((_, field_type)) = trimmed
                .strip_prefix("pub ")
                .and_then(|line| line.split_once(':'))
        {
            assert!(
                !field_type
                    .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                    .any(|token| token == "Value"),
                "canonical compiler member is a JSON catch-all: {trimmed}"
            );
        }
    }
}

fn declaration_name(line: &str) -> Option<&str> {
    ["pub struct ", "pub enum ", "pub type "]
        .into_iter()
        .find_map(|prefix| line.strip_prefix(prefix))
        .and_then(|tail| tail.split([' ', '{', '=']).next())
}
