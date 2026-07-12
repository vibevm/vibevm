//! The fresh-project acceptance, frozen (SELF-SUFFICIENCY-PLAN Phase 4;
//! §9's library path): a project that has never heard of the dev tree
//! bootstraps with `init`, mints a resolving traceability index in its own
//! namespace, resolves a cross-package citation through
//! `[[external_specs]]`, and is CAUGHT by both gates when it violates the
//! discipline. Hermetic: temp dirs, no network, no git, no cargo — the
//! engine library calls only (the cargo-spawning floor steps are covered
//! by the dev repo's own gate runs).

use std::path::Path;

use rust_ai_native_cli::{InitOptions, run_init};

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, body).expect("write");
}

/// A minimal consumer tree: one workspace crate, one spec unit, one
/// materialised package slot carrying a mechanism spec — the shape
/// `vibe install` + `rust-ai-native init` leaves behind.
fn fresh_project() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write(root, "crates/app/Cargo.toml", "[package]\nname = \"app\"\n");
    write(
        root,
        "crates/app/src/lib.rs",
        r#"//! The demo crate.
specmark::scope!("spec://demo/PROP-001#req-hello");

/// The one seam.
///
/// ```
/// let _ = app::hello();
/// ```
pub fn hello() -> &'static str {
    "hello"
}
"#,
    );
    write(
        root,
        "spec/PROP-001.md",
        "# PROP-001 — the demo spec {#root}\n\n## Hello {#req-hello}\n`req r1`\n\nIt MUST greet.\n",
    );
    // A materialised package slot with a spec tree (what vibe install makes).
    write(
        root,
        "vibedeps/flow-core-ai-native/0.3.0/vibe.toml",
        "[package]\nname = \"core-ai-native\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\nversion = \"0.3.0\"\n",
    );
    write(
        root,
        "vibedeps/flow-core-ai-native/0.3.0/spec/mechanisms/ENGINE-X-v0.1.md",
        "## Rules {#rules}\n`req r1`\n\nbody\n",
    );
    tmp
}

#[test]
fn init_then_trace_resolves_and_the_gates_catch_violations() {
    let tmp = fresh_project();
    let root = tmp.path();

    // 1. Bootstrap. init discovers the workspace topology and the
    //    installed package's spec tree.
    run_init(
        root,
        &InitOptions {
            namespace: Some("demo".into()),
            force: false,
        },
    )
    .expect("init");
    let specmap_policy = std::fs::read_to_string(root.join("specmap.toml")).expect("policy");
    assert!(specmap_policy.contains("namespace = \"demo\""));
    assert!(
        specmap_policy.contains("root = \"vibedeps/flow-core-ai-native/0.3.0/spec\""),
        "{specmap_policy}"
    );

    // 2. The project's own unit + tag resolve in the project's own
    //    namespace — no dev-tree anywhere.
    let cfg = specmap_core::config::Config::load(root)
        .expect("load")
        .expect("present");
    let map = specmap_core::index::build(root, &cfg);
    assert_eq!(map.specUnits.len(), 2, "root + req-hello units");
    assert!(
        map.specUnits
            .iter()
            .any(|u| u.uri == "spec://demo/PROP-001#req-hello")
    );
    assert_eq!(map.edges.len(), 1);
    assert!(
        map.warnings.is_empty(),
        "fresh tree must have no dangling/warnings: {:?}",
        map.warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );

    // 3. A cross-package citation resolves through [[external_specs]].
    write(
        root,
        "crates/app/src/uses_mechanism.rs",
        "specmark::scope!(\"spec://org.vibevm.ai-native.core-ai-native/mechanisms/ENGINE-X-v0.1#rules\");\n",
    );
    // (registered as a module so the scanner sees it)
    write(
        root,
        "crates/app/src/lib.rs",
        r#"//! The demo crate.
specmark::scope!("spec://demo/PROP-001#req-hello");

pub mod uses_mechanism;

/// The one seam.
///
/// ```
/// let _ = app::hello();
/// ```
pub fn hello() -> &'static str {
    "hello"
}
"#,
    );
    let map = specmap_core::index::build(root, &cfg);
    assert_eq!(map.edges.len(), 2);
    assert!(
        map.warnings.is_empty(),
        "the package-hosted unit must resolve externally: {:?}",
        map.warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );

    // 4. The orphan ratchet catches an untagged public surface once the
    //    crate is gated (exempt lists are empty from init's specmap.toml,
    //    so `app` is gated already).
    write(root, "crates/app/src/orphan.rs", "pub fn untagged() {}\n");
    write(
        root,
        "crates/app/src/lib.rs",
        r#"//! The demo crate.
pub mod orphan;
pub mod uses_mechanism;

/// The one seam.
///
/// ```
/// let _ = app::hello();
/// ```
pub fn hello() -> &'static str {
    "hello"
}
"#,
    );
    let map = specmap_core::index::build(root, &cfg);
    let orphans = specmap_core::ratchet::orphans(root, &map, &cfg);
    assert!(
        orphans
            .iter()
            .any(|o| o.symbol.contains("untagged") || o.symbol.contains("orphan")),
        "the ratchet must flag the untagged module: {:?}",
        orphans.iter().map(|o| &o.symbol).collect::<Vec<_>>()
    );

    // 5. The conform gate catches a domain unwrap once the crate is
    //    flipped into gated_crates (the expand-as-you-conform move).
    write(
        root,
        "conform.toml",
        "roots = [\"crates/*\"]\ngated_crates = [\"app\"]\n",
    );
    write(
        root,
        "crates/app/src/lib.rs",
        r#"//! The demo crate.

/// A seam with a domain unwrap.
///
/// ```
/// let _ = app::parse("3");
/// ```
pub fn parse(s: &str) -> i32 {
    s.parse::<i32>().unwrap()
}
"#,
    );
    let _ = std::fs::remove_file(root.join("crates/app/src/orphan.rs"));
    let _ = std::fs::remove_file(root.join("crates/app/src/uses_mechanism.rs"));
    let err =
        rust_ai_native_conform::run_check(root, rust_ai_native_cli::DEFAULT_CONFORM_BASELINE, None)
            .expect_err("a domain unwrap in a gated crate must fail the gate");
    assert!(
        err.to_string().contains("new finding"),
        "expected a new-finding failure, got: {err}"
    );
}
