//! End-to-end gate tests over the committed fixtures — the REAL path:
//! `run_check` → go-extract frontend → `go run` → the neutral engine →
//! baseline diff. Requires go on PATH or `GO_AI_NATIVE_GO` set (the
//! stack's documented toolchain floor; an absent go is a FAILURE with
//! the recipe, never a skip — GUIDE-AI-NATIVE-GO §14).
//!
//! Baselines are written under the fixtures' `target/` (gitignored) so
//! the committed tree is never mutated.

use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/go-extract/test/fixtures")
        .join(name)
}

#[test]
fn dirty_fixture_yields_the_ten_findings_then_freeze_ratchets_them() {
    let root = fixture("dirty");
    let baseline = "target/conform/test-baseline.json";

    // Fresh gate: ten findings (8 census on plan.go — blank_import,
    // seam_error_missing_req, init_decl, 2×ambient_call, naked_go,
    // 2×error_string_match, reasonless_suppression = 9, minus the
    // deviation-covered ambient in Sanctioned — plus t_skip in the
    // test file), non-zero.
    let _ = std::fs::remove_file(root.join(baseline));
    let err = go_ai_native_conform::run_check(&root, baseline, None)
        .expect_err("dirty tree must fail the gate");
    assert!(
        err.to_string().contains("10 new finding(s)"),
        "unexpected: {err}"
    );

    // Freeze, then the same tree is ratchet-green.
    go_ai_native_conform::run_freeze(&root, baseline).expect("freeze");
    go_ai_native_conform::run_check(&root, baseline, None)
        .expect("frozen dirty tree passes the ratchet");
}

#[test]
fn clean_fixture_passes_with_zero_findings() {
    let root = fixture("clean");
    let baseline = "target/conform/test-baseline.json";
    let _ = std::fs::remove_file(root.join(baseline));
    go_ai_native_conform::run_check(&root, baseline, None)
        .expect("injected capabilities and a Spec-carrying error set are green");
}
