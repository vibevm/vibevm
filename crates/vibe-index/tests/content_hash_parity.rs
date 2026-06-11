//! Golden-hash parity test — locks the on-disk bytes of the
//! `crates/vibe-index/fixtures/golden-flow-wal-0.1.0/` fixture to
//! the content_hash that `vibe-registry::compute_content_hash`
//! produces for the same files. Algorithm: see PROP-005 §3.2.
//!
//! The golden fixture is a byte-for-byte copy of the main workspace's
//! `fixtures/registry/flow/wal/v0.1.0/` package; `.gitattributes`
//! (`* text=auto eol=lf`) keeps every text byte identical on Windows /
//! macOS / Linux, so the digest is stable cross-platform. The `GOLDEN`
//! constant below was re-derived 2026-05-22 after PROP-008 added the
//! mandatory `[package].group` field to the fixture's `vibe.toml`;
//! `vibe-index`'s own `content_hash.rs` (a verified byte-identical port
//! of `vibe-registry::compute_content_hash`) produces it, which is what
//! this test asserts.
//!
//! If this test fails after a git operation that touches the fixture
//! or the algorithm: re-sync the fixture from
//! `fixtures/registry/flow/wal/v0.1.0/`, re-derive `GOLDEN` from
//! `vibe-registry::compute_content_hash` against that directory, and
//! update the constant below.

use std::path::PathBuf;

use specmark::verifies;
use vibe_index::content_hash::compute_content_hash;

const GOLDEN: &str = "sha256:865d47fb41fb8590ef6f0780f7fe98c716b897dea494769dd37a0e5280bc55a5";

#[test]
#[verifies("spec://vibevm/modules/vibe-index/PROP-005#trust", r = 1)]
fn flow_wal_v0_1_0_matches_canonical_algorithm() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("golden-flow-wal-0.1.0");
    assert!(
        fixture.is_dir(),
        "fixture not present at `{}`",
        fixture.display()
    );
    let hash = compute_content_hash(&fixture).expect("hash computes");
    assert_eq!(
        hash, GOLDEN,
        "vibe-index content_hash diverged from the canonical algorithm — \
         either the fixture bytes changed or the algorithm did. See \
         spec://vibevm/modules/vibe-index/PROP-005#types"
    );
}
