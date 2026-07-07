//! The live end-to-end suite: a REAL rust-analyzer against a scratch
//! cargo project. Installing this stack obliges the machine to carry
//! the component (ORACLE-RUST §1) — an absent rust-analyzer FAILS
//! these tests with the recipe, never skips (D11).

use std::time::Duration;

use tcg_oracle_bridge_rust::position::OuterPosition;
use tcg_oracle_bridge_rust::{RustOracle, resolve_rust_analyzer};

const CLEAN: &str = r#"fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

fn main() {
    let s = greet("world");
    println!("{s}");
}
"#;

fn scratch_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"live-scratch\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("Cargo.toml");
    std::fs::create_dir_all(dir.path().join("src")).expect("src");
    std::fs::write(dir.path().join("src/main.rs"), CLEAN).expect("main.rs");
    dir
}

#[test]
fn the_live_chain_answers_overlay_truth() {
    // Resolution first: the failure mode a fresh box hits, surfaced
    // with its recipe rather than skipped.
    let dir = scratch_project();
    resolve_rust_analyzer(dir.path()).expect(
        "rust-analyzer is a stack prerequisite (ORACLE-RUST §1): \
         `rustup component add rust-analyzer`",
    );

    let mut oracle =
        RustOracle::spawn(dir.path(), Duration::from_secs(60)).expect("spawn + handshake");
    assert!(
        oracle.capabilities().pull_diagnostics,
        "1.93.1 grants pull diagnostics"
    );

    // A seeded type error as a pure overlay — the disk stays clean.
    let seeded = CLEAN.replace("let s = greet", "let s: i32 = greet");
    let out = oracle
        .validate("src/main.rs", Some(seeded))
        .expect("validate overlay");
    assert!(
        out.diagnostics.iter().any(|d| d.code == "E0308"),
        "the E0308-class diagnostic surfaces through the overlay: {:?}",
        out.diagnostics
    );
    let disk = std::fs::read_to_string(dir.path().join("src/main.rs")).expect("read");
    assert_eq!(disk, CLEAN, "the overlay never touched disk");

    // The clean text clears it (version law: a NEW version, not a
    // reset).
    let clean = oracle
        .validate("src/main.rs", Some(CLEAN.to_string()))
        .expect("validate clean");
    assert!(
        clean.diagnostics.iter().all(|d| d.category != "error"),
        "clean text carries no error-grade diagnostics: {:?}",
        clean.diagnostics
    );

    // Quick info on the call site names the signature.
    let pos_line = 6u32; // 1-based: `    let s = greet("world");`
    let (display, _docs) = oracle
        .hover(
            "src/main.rs",
            OuterPosition {
                line: pos_line,
                character: 13,
            },
            None,
        )
        .expect("hover");
    assert!(
        display.contains("fn greet"),
        "hover names the signature: {display:?}"
    );

    // Completions at `let x = gre` include the in-scope fn with type
    // text.
    let entries = oracle
        .complete(
            "src/main.rs",
            OuterPosition {
                line: 6,
                character: 15,
            },
            Some(CLEAN.replace("let s = greet(\"world\")", "let x = gre")),
        )
        .expect("complete");
    let greet = entries
        .iter()
        .find(|e| e.name.starts_with("greet"))
        .expect("greet completes in scope");
    assert!(
        greet
            .type_text
            .as_deref()
            .is_some_and(|t| t.contains("String")),
        "the entry carries type text: {greet:?}"
    );

    // The graceful dance; kill-on-drop remains the backstop.
    oracle.shutdown().expect("shutdown");
}
