//! Completeness test: every `[format.*]` section in `formats/REGISTRY.toml`
//! maps to exactly one `FormatId` variant, and vice versa.
//!
//! Order-independent (it compares sets, not sequences). The format registry is
//! the source of truth for the enum (PROP-044 §4.1 `##M-FORMAT-REGISTRY`), so
//! this fails on drift in *either* direction:
//! - a TOML section with no variant — someone added a format and forgot
//!   `cargo xtask codegen`;
//! - a variant with no TOML section — the enum raced ahead of the registry.
//!
//! The registry lives at the repo root; vibe-wire is under `crates/vibe-wire/`,
//! so `CARGO_MANIFEST_DIR/../..` resolves it.

use std::collections::BTreeSet;

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
