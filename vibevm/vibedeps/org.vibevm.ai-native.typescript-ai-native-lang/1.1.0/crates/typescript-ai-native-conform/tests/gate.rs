//! End-to-end gate tests over the committed fixtures — the REAL path:
//! `run_check` → ts-tsc frontend → node → the consumer-resolved
//! `typescript` → the neutral engine → baseline diff. Requires node on
//! PATH (the stack's documented toolchain floor); the fixtures resolve
//! `typescript` from tools/ts-extract's own devDependency install.
//!
//! Baselines are written under the fixtures' `target/` (gitignored) so
//! the committed tree is never mutated.

use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/ts-extract/test/fixtures")
        .join(name)
}

#[test]
fn dirty_fixture_yields_the_five_findings_then_freeze_ratchets_them() {
    let root = fixture("dirty");
    let baseline = "target/conform/test-baseline.json";

    // Fresh gate: eight NEW findings (1 isolation + 4 unsafe + 1
    // invariant-comment-position on src/invariant.ts, a `// INVARIANT:`
    // comment buried in the middle third of the 150-line exhibit file),
    // plus 2 declared-test-matrices (R-060: src/sweep.test.ts sweeps a
    // `1 << 3` bit-mask AND nests three C-style for-loops). B-025 adds a
    // NINTH finding — the reasoned `@ts-expect-error -- …` in logic.ts —
    // now MARKED `DeviationAcknowledged` (visible in the SARIF, never
    // `new`), so the gate still reports exactly 8 NEW and the freeze
    // ratchets only the eight Live fingerprints.
    let _ = std::fs::remove_file(root.join(baseline));
    let err = typescript_ai_native_conform::run_check(&root, baseline, None)
        .expect_err("dirty tree must fail the gate");
    assert!(
        err.to_string().contains("8 new finding(s)"),
        "unexpected: {err}"
    );

    // Freeze, then the same tree is ratchet-green.
    typescript_ai_native_conform::run_freeze(&root, baseline).expect("freeze");
    typescript_ai_native_conform::run_check(&root, baseline, None)
        .expect("frozen dirty tree passes the ratchet");
}

#[test]
fn clean_fixture_passes_with_zero_findings() {
    let root = fixture("clean");
    let baseline = "target/conform/test-baseline.json";
    let _ = std::fs::remove_file(root.join(baseline));
    typescript_ai_native_conform::run_check(&root, baseline, None)
        .expect("seam-only imports and zero unsafe tokens are green");
}
