//! The fresh-Go-project acceptance, frozen (the wiring §14 walk,
//! engine calls end to end): bootstrap a temp module, tag one export,
//! and drive init → conform → specmap → orphan gate through the REAL
//! extractor and go. Requires go (PATH or `GO_AI_NATIVE_GO`) — a stack
//! obligation, never a skip.

use std::path::Path;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, body).expect("write");
}

#[test]
fn init_then_gates_catch_violations_and_the_tagged_tree_passes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write(root, "go.mod", "module fresh\n\ngo 1.24\n");
    write(
        root,
        "spec/PROP-001.md",
        "# PROP-001 — the demo spec {#root}\n\n## Hello {#req-hello}\n`req r1`\n\nIt MUST greet.\n",
    );
    // A materialised package slot with a spec tree (what vibe install
    // makes) so init discovers [[external_specs]].
    write(
        root,
        "vibedeps/flow-some-core/0.4.0/vibe.toml",
        "[package]\nname = \"some-core\"\n",
    );
    write(
        root,
        "vibedeps/flow-some-core/0.4.0/spec/mechanisms/ENGINE-X.md",
        "## Rules {#rules}\n`req r1`\n\nbody\n",
    );
    // One tagged export citing the local unit, one census site
    // (time.Now inside a cell), and — in a different, unscoped
    // package — one UNTAGGED export.
    write(
        root,
        "internal/cells/hello/hello.go",
        "// Package hello is the demo cell.\n\
         //\n\
         //spec:scope spec://fresh/PROP-001#req-hello r=1\n\
         package hello\n\
         \n\
         import \"time\"\n\
         \n\
         // Hello greets.\n\
         func Hello(name string) string { return \"hello \" + name }\n\
         \n\
         // Stamp leaks the ambient clock (the census site).\n\
         func Stamp() int64 { return time.Now().Unix() }\n",
    );
    write(
        root,
        "internal/registry/registry.go",
        "// Package registry wires cells.\n\
         package registry\n\
         \n\
         // Naked is an untagged export the ratchet must block.\n\
         func Naked() string { return \"naked\" }\n",
    );

    // 1. Bootstrap.
    go_ai_native_cli::run_init(
        root,
        &go_ai_native_cli::InitOptions {
            namespace: Some("fresh".into()),
            force: false,
        },
    )
    .expect("init");
    let policy = std::fs::read_to_string(root.join("specmap.toml")).expect("policy");
    assert!(
        policy.contains("root = \"vibedeps/flow-some-core/0.4.0/spec\""),
        "{policy}"
    );

    // 2. The conform gate catches the ambient call through the real
    // go run.
    let err = go_ai_native_conform::run_check(
        root,
        go_ai_native_conform::DEFAULT_GO_BASELINE,
        None,
    )
    .expect_err("the ambient call must fail the gate");
    assert!(err.to_string().contains("1 new finding(s)"), "{err}");

    // 3. The index mints (the ratchet is warn-only on a mint), then
    // `--check` blocks on the naked export (the scoped package passes
    // whole — package-grain scope).
    go_ai_native_specmap::run_specmap_go(root, false).expect("mint warns only");
    let err = go_ai_native_specmap::run_specmap_go(root, true)
        .expect_err("one naked export must block the check");
    assert!(err.to_string().contains("1 untagged export(s)"), "{err}");

    // 4. Tag it; mint again; check reproduces; the gate is green.
    write(
        root,
        "internal/registry/registry.go",
        "// Package registry wires cells.\n\
         //\n\
         //spec:scope spec://fresh/PROP-001#req-hello r=1\n\
         package registry\n\
         \n\
         // Naked is now covered by the package scope.\n\
         func Naked() string { return \"naked\" }\n",
    );
    go_ai_native_specmap::run_specmap_go(root, false).expect("mint");
    go_ai_native_specmap::run_specmap_go(root, true).expect("check clean");
}
