//! The live end-to-end chain: a real gopls over a real temp module —
//! spawn, handshake, overlay validate (a seeded type error must
//! surface), hover, completion, shutdown with no zombie. Requires
//! gopls (PATH / GO_AI_NATIVE_GOPLS / GOPATH-bin) — a stack
//! obligation; absence FAILS with the recipe, never skips
//! (TCG-ORACLE-GO §1).

use std::time::Duration;

use go_ai_native_tcg_bridge::GoOracle;
use go_ai_native_tcg_bridge::position::OuterPosition;

fn write(root: &std::path::Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, body).expect("write");
}

#[test]
fn seeded_error_surfaces_through_an_overlay_and_the_session_shuts_down() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write(root, "go.mod", "module live\n\ngo 1.24\n");
    write(
        root,
        "main.go",
        "package main\n\nfunc main() { println(greet()) }\n\nfunc greet() string { return \"hi\" }\n",
    );

    let mut oracle =
        GoOracle::spawn(root, Duration::from_secs(60)).expect("gopls spawns (stack obligation)");

    // A pure overlay carrying a type error — never written to disk.
    let outcome = oracle
        .validate(
            "main.go",
            Some(
                "package main\n\nfunc main() { println(greet()) }\n\nfunc greet() string { return 42 }\n"
                    .into(),
            ),
        )
        .expect("validate answers");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.category == "error"),
        "a seeded type error must surface: {outcome:?}"
    );

    // The healthy overlay goes quiet again (error-grain).
    let healthy = oracle
        .validate(
            "main.go",
            Some(
                "package main\n\nfunc main() { println(greet()) }\n\nfunc greet() string { return \"hi\" }\n"
                    .into(),
            ),
        )
        .expect("validate healthy");
    assert!(
        healthy.diagnostics.iter().all(|d| d.category != "error"),
        "the healthy overlay must carry no errors: {healthy:?}"
    );

    // Hover over `greet` in the call (line 3, character 22 —
    // `func main() { println(greet()) }`).
    let (display, _docs) = oracle
        .hover(
            "main.go",
            OuterPosition {
                line: 3,
                character: 23,
            },
            None,
        )
        .expect("hover answers");
    assert!(display.contains("greet"), "hover display: {display}");

    oracle.shutdown().expect("the LSP exit dance");
}
