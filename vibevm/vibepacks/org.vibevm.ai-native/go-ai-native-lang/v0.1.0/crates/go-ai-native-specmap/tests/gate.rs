//! End-to-end index + gate tests over the committed fixtures and their
//! committed `specmap.json` goldens — the REAL path through `go run`,
//! the go-extract scanner, and the neutral index builder. `--check`
//! byte-compares against the goldens, so the index shape for Go trees
//! is frozen exactly the way the sibling stacks' is.

use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/go-extract/test/fixtures")
        .join(name)
}

#[test]
fn clean_fixture_check_is_byte_stable_and_gate_green() {
    let root = fixture("clean");
    go_ai_native_specmap::run_specmap_go(&root, true)
        .expect("clean fixture reproduces its golden and passes the gate");
}

#[test]
fn dirty_fixture_index_is_stable_but_the_orphan_gate_blocks() {
    let root = fixture("dirty");
    let err = go_ai_native_specmap::run_specmap_go(&root, true)
        .expect_err("the naked registry export must block the ratchet");
    assert!(
        err.to_string().contains("1 untagged export(s)"),
        "unexpected: {err}"
    );
}
