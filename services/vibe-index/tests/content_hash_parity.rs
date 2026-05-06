//! Golden-hash parity test — locks the on-disk bytes of the
//! `services/vibe-index/fixtures/golden-flow-wal-0.1.0/` fixture to
//! the content_hash that `vibe-registry::compute_content_hash`
//! produces for the same files. Algorithm: see PROP-005 §3.2.
//!
//! Verified 2026-05-06 against the canonical algorithm via Python
//! reimplementation + Rust port — both produce the same digest, and
//! both match what `vibe-registry::compute_content_hash` returns
//! against the byte-identical `fixtures/registry/flow/wal/v0.1.0/`
//! source in the main vibevm workspace.
//!
//! If this test fails after a git operation that touches the fixture
//! or the algorithm: re-derive the expected value by running
//! `cargo run --manifest-path ../../Cargo.toml -p vibe-cli -- registry vendor`
//! against the fixture and reading the lockfile, OR by recomputing
//! the algorithm via any reference implementation. Update both the
//! constant below and the divergence note.

use std::path::PathBuf;

use vibe_index::content_hash::compute_content_hash;

const GOLDEN: &str =
    "sha256:e9fedc632693ecbc3b041de8f553433349df498bfa4e1f19365f174dcd65b63d";

#[test]
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
