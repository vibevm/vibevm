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

    // Fresh gate: five findings (1 isolation + 4 unsafe), non-zero.
    let _ = std::fs::remove_file(root.join(baseline));
    let err = conform_cli_typescript::run_check(&root, baseline, None)
        .expect_err("dirty tree must fail the gate");
    assert!(
        err.to_string().contains("5 new finding(s)"),
        "unexpected: {err}"
    );

    // Freeze, then the same tree is ratchet-green.
    conform_cli_typescript::run_freeze(&root, baseline).expect("freeze");
    conform_cli_typescript::run_check(&root, baseline, None)
        .expect("frozen dirty tree passes the ratchet");
}

#[test]
fn clean_fixture_passes_with_zero_findings() {
    let root = fixture("clean");
    let baseline = "target/conform/test-baseline.json";
    let _ = std::fs::remove_file(root.join(baseline));
    conform_cli_typescript::run_check(&root, baseline, None)
        .expect("seam-only imports and zero unsafe tokens are green");
}
