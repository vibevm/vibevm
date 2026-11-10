//! Test-only access to the host checkout, for the two measurements the
//! campaign states as facts about *this* repository.
//!
//! Spike S1 sized the prefix at 278,092 bytes and `SP-FMT-103` sizes the
//! agent lane at about 310 KB, both on this tree. Those numbers are release
//! notes rather than invariants — `AGENTS.md` and `STATIC.xml` change with
//! every resolution — so the tests that use this module assert the
//! composition's *structure* and print the measurement, and skip entirely
//! outside a host checkout.

use std::path::{Path, PathBuf};

/// The repository root, found by walking up to the one file only a host
/// checkout has at its top.
pub(crate) fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while dir.pop() {
        if dir.join("formats/REGISTRY.toml").is_file() {
            return dir;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The `path = "…"` rows of the host repository's own generated INDEX, read
/// with a line scan because this is a measurement and not a parser.
pub(crate) fn index_rows(root: &Path) -> Vec<String> {
    std::fs::read_to_string(root.join("vibevm/vibespecs/boot/INDEX.md"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            line.strip_prefix("path = \"")
                .and_then(|rest| rest.strip_suffix('"'))
                .map(str::to_string)
        })
        .collect()
}
