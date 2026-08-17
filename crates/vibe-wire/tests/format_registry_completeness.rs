//! Consistency tests over `formats/REGISTRY.toml`: the registry is the
//! source of truth for the generated `FormatId` (PROP-044 §4.1
//! `##M-FORMAT-REGISTRY`), so drift fails here before anywhere else.
//!
//! - Completeness: every `[format.*]` section maps to exactly one
//!   `FormatId` variant, and vice versa. Order-independent (it compares
//!   sets, not sequences), and it fails on drift in *either* direction:
//!   a TOML section with no variant (someone added a format and forgot
//!   `cargo xtask codegen`), or a variant with no TOML section (the
//!   enum raced ahead of the registry).
//! - Epoch agreement: the catalog's format family (`index-*`) carries
//!   ONE epoch — the number `hello.json` announces for the world.
//!
//! The registry lives at the repo root; vibe-wire is under `crates/vibe-wire/`,
//! so `CARGO_MANIFEST_DIR/../..` resolves it.

use std::collections::{BTreeMap, BTreeSet};

use vibe_wire::generated::format_id::FormatId;

fn registry_path() -> std::path::PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set under cargo");
    std::path::PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .join("formats")
        .join("REGISTRY.toml")
}

#[test]
fn format_id_completeness() {
    let text = std::fs::read_to_string(registry_path()).expect("formats/REGISTRY.toml is readable");
    let parsed: toml::Value = toml::from_str(&text).expect("formats/REGISTRY.toml parses");
    let formats = parsed
        .get("format")
        .and_then(|v| v.as_table())
        .expect("formats/REGISTRY.toml has a `[format.*]` table");

    let registry_ids: BTreeSet<&str> = formats.keys().map(String::as_str).collect();
    let enum_ids: BTreeSet<&str> = FormatId::ALL.iter().copied().map(FormatId::id).collect();

    let in_toml_not_enum: Vec<&str> = registry_ids.difference(&enum_ids).copied().collect();
    let in_enum_not_toml: Vec<&str> = enum_ids.difference(&registry_ids).copied().collect();

    assert!(
        in_toml_not_enum.is_empty() && in_enum_not_toml.is_empty(),
        "FormatId <-> REGISTRY.toml drift:\n  \
         in TOML but no enum variant (run `cargo xtask codegen`): {in_toml_not_enum:?}\n  \
         enum variant but not in TOML: {in_enum_not_toml:?}"
    );
}

/// The catalog's format family — every `index-*` record — must agree on
/// its epoch, and that epoch must be the one the generated enum hands
/// the handshake writer. `hello.json` announces ONE epoch for the world,
/// read from `FormatId::IndexRepomd` (`repomd.json` is the world's
/// manifest); the first record that moved to a new epoch alone would
/// make the handshake a silent liar — vouching for an epoch part of the
/// world no longer matches. Without this sentinel that drift is
/// invisible; with it, the rule is a procedure, not a hope.
#[test]
fn catalog_format_family_agrees_on_its_epoch() {
    let text = std::fs::read_to_string(registry_path()).expect("formats/REGISTRY.toml is readable");
    let parsed: toml::Value = toml::from_str(&text).expect("formats/REGISTRY.toml parses");
    let formats = parsed
        .get("format")
        .and_then(|v| v.as_table())
        .expect("formats/REGISTRY.toml has a `[format.*]` table");

    let epochs: BTreeMap<&str, i64> = formats
        .iter()
        .filter(|(id, _)| id.starts_with("index-"))
        .map(|(id, table)| {
            let epoch = table
                .get("epoch")
                .and_then(|v| v.as_integer())
                .unwrap_or_else(|| panic!("format `{id}` carries no integer epoch"));
            (id.as_str(), epoch)
        })
        .collect();
    assert!(
        !epochs.is_empty(),
        "the catalog format family (`index-*`) must exist in the registry"
    );

    // The number the writer reads: `FormatId::IndexRepomd.epoch()`.
    // Comparing against the ENUM (not just the TOML) also catches a
    // registry edited without `cargo xtask codegen`.
    let expected = FormatId::IndexRepomd.epoch() as i64;
    let divergent: Vec<String> = epochs
        .iter()
        .filter(|(_, epoch)| **epoch != expected)
        .map(|(id, epoch)| format!("{id}: epoch = {epoch} (family says {expected})"))
        .collect();
    assert!(
        divergent.is_empty(),
        "the catalog format family disagrees on its epoch — `hello.json` announces \
         ONE epoch for the world, taken from `index-repomd` ({expected}):\n  {}",
        divergent.join("\n  ")
    );
}
