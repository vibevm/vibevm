//! Loud death for the retired flat root keys (B-029 / B-034).
//!
//! The flat root keys that carried Rust-only policy (`roots`,
//! `exclude_substrings`, `gated_crates`, the Rust extras, `[[exempt]]`)
//! have moved under per-language sections. Each is declared on
//! [`Config`](super::Config) as an `Option<Value>` tombstone: its
//! presence parses, but [`check`] rejects it with a targeted move hint —
//! the `LegacyHostAuthority` house pattern, not serde's generic
//! unknown-field message. Three in-tree carriers, pre-publication, so
//! the loud tombstone is the whole migration aid (no alias grace
//! period; design `gate-parity-config.md` §`fork-legacy`).

use anyhow::{Result, bail};

use super::Config;

/// Reject any retired flat root key that survived into a parsed config,
/// each with its own move hint. Returns `Ok(())` when the surface is
/// clean. Order is the declaration order on `Config`; the first
/// tombstone hit wins (a file carrying several is rewritten wholesale
/// anyway, so which hint fires first is immaterial).
pub(crate) fn check(cfg: &Config) -> Result<()> {
    if cfg.roots.is_some() {
        bail!(
            "conform.toml: `roots` has moved — use `roots` under `[rust]` \
             (the config surface is per-language now)"
        );
    }
    if cfg.exclude_substrings.is_some() {
        bail!(
            "conform.toml: `exclude_substrings` has moved — use `exclude_substrings` under \
             `[rust]` (the config surface is per-language now)"
        );
    }
    if cfg.gated_crates.is_some() {
        bail!(
            "conform.toml: `gated_crates` has moved — use `gated` under `[rust]` \
             (the config surface is per-language now)"
        );
    }
    if cfg.gated_pub_doctest.is_some() {
        bail!(
            "conform.toml: `gated_pub_doctest` has moved under `[rust]` \
             (the config surface is per-language now)"
        );
    }
    if cfg.audit_crates.is_some() {
        bail!(
            "conform.toml: `audit_crates` has moved under `[rust]` \
             (the config surface is per-language now)"
        );
    }
    if cfg.env_roots.is_some() {
        bail!(
            "conform.toml: `env_roots` has moved under `[rust]` \
             (the config surface is per-language now)"
        );
    }
    if cfg.registry_file.is_some() {
        bail!(
            "conform.toml: `registry_file` has moved under `[rust]` \
             (the config surface is per-language now)"
        );
    }
    if cfg.registry_gated_crate.is_some() {
        bail!(
            "conform.toml: `registry_gated_crate` has moved under `[rust]` \
             (the config surface is per-language now)"
        );
    }
    if cfg.exempt.is_some() {
        bail!(
            "conform.toml: `[[exempt]]` has moved — use `[[rust.exempt]]` \
             (field `crate` → `unit`)"
        );
    }
    Ok(())
}
