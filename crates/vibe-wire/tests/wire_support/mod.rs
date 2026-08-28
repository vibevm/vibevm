//! The two file readers the R7.5 wire suites share.
//!
//! Cargo discovers only top-level `tests/*.rs`, so a directory like
//! this one is a support module several integration tests pull in by
//! `#[path]` — the same idiom `compiler_trace_index_support` and
//! `compiler_ir_close_oracle` already use. Each including test gets
//! its own copy, which is why nothing here holds state.
//!
//! Each includer uses the subset it needs, so an unused helper here is
//! the normal case rather than dead code to delete.
#![allow(dead_code)]

use std::path::PathBuf;

/// The repository root, from this crate's manifest directory.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One repo-relative JSON document, parsed. A missing or malformed
/// file is a broken checkout, so it panics naming the path rather than
/// returning an `Option` every call site would unwrap anyway.
pub fn read_json(relative: &str) -> serde_json::Value {
    let path = repo_root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} readable: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{relative} parses: {e}"))
}

/// Parse an authored corpus through a generated root and prove the
/// bytes survive the round trip unchanged.
pub fn round_trip<T: serde::de::DeserializeOwned + serde::Serialize>(relative: &str) -> T {
    let authored = read_json(relative);
    let value: T =
        serde_json::from_value(authored.clone()).unwrap_or_else(|e| panic!("{relative}: {e}"));
    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        authored,
        "{relative} loses data on generated round-trip"
    );
    value
}
