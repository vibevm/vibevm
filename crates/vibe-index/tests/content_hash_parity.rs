//! Golden-hash parity test — locks the on-disk bytes of the
//! `crates/vibe-index/fixtures/golden-flow-wal-0.1.0/` fixture to
//! the content_hash that `vibe-registry::compute_content_hash`
//! produces for the same files. Algorithm: see PROP-005 §3.2.
//!
//! The golden fixture is a small, standalone synthetic package
//! (`com.example/golden-pkg`) kept frozen purely as a cross-impl hash
//! anchor — it deliberately reuses no real package's name (it was
//! de-collided from the shipped `org.vibevm.world/wal` package, whose
//! name it once mirrored). `.gitattributes` (`* text=auto eol=lf`) keeps
//! every text byte identical on Windows / macOS / Linux, so the digest is
//! stable cross-platform. `vibe-index`'s own `content_hash.rs` (a verified
//! byte-identical port of `vibe-registry::compute_content_hash`) must
//! reproduce the `GOLDEN` constant below, which is what this test asserts.
//!
//! If this test fails after a git operation that touches the fixture
//! or the algorithm: re-derive `GOLDEN` by running this test — the panic
//! prints the freshly-computed `left` digest — and paste it below.

use std::path::PathBuf;

use specmark::verifies;
use vibe_index::content_hash::compute_content_hash;

const GOLDEN: &str = "sha256:e10a49c0a8e1b35e3f0dc1e74d6ce26605052b2eead2225124051d67a2f76cb6";

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
